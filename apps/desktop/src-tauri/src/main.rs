#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod history;
mod personal;
mod refine;
mod stt;
mod typer;
mod vad;

use anyhow::Result;
use crossbeam_channel::bounded;
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use stt::Provider;
use typer::Typer;
use vad::Vad;

// ---------------------------------------------------------------- Config/state

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictationConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_hide")]
    pub hide_after_secs: u64,
    #[serde(default = "default_true")]
    pub type_at_cursor: bool,
    #[serde(default = "default_shortcut")]
    pub shortcut: String,
    #[serde(default = "default_style")]
    pub style: String,
}
fn default_provider() -> String {
    std::env::var("STT_PROVIDER").unwrap_or_else(|_| "mock".into())
}
fn default_hide() -> u64 {
    3
}
fn default_true() -> bool {
    true
}
fn default_shortcut() -> String {
    "Ctrl+Alt+D".into()
}
fn default_style() -> String {
    "professional".into()
}
impl Default for DictationConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            hide_after_secs: 3,
            type_at_cursor: true,
            shortcut: default_shortcut(),
            style: default_style(),
        }
    }
}

pub struct AppState {
    config: Mutex<DictationConfig>,
    running: AtomicBool,
}

// ------------------------------------------------------------- Tauri commands

#[tauri::command]
fn get_config(state: tauri::State<AppState>) -> DictationConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn set_config(state: tauri::State<AppState>, config: DictationConfig) -> DictationConfig {
    *state.config.lock().unwrap() = config.clone();
    config
}

#[tauri::command]
fn start_dictation(app: AppHandle, _state: tauri::State<AppState>) -> Result<(), String> {
    start_pipeline(&app);
    Ok(())
}

#[tauri::command]
fn stop_dictation(app: AppHandle, _state: tauri::State<AppState>) -> Result<(), String> {
    stop_pipeline(&app);
    Ok(())
}

// ----------------------------------------------- Dictionary / snippets / history

#[tauri::command]
fn get_dictionary() -> std::collections::HashMap<String, String> {
    personal::get_dictionary()
}

#[tauri::command]
fn add_dictionary_word(
    misheard: String,
    correction: String,
) -> Result<std::collections::HashMap<String, String>, String> {
    personal::set_word(misheard, correction).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_dictionary_word(
    misheard: String,
) -> Result<std::collections::HashMap<String, String>, String> {
    personal::remove_word(&misheard).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_snippets() -> std::collections::HashMap<String, String> {
    personal::get_snippets()
}

#[tauri::command]
fn add_snippet(
    trigger: String,
    expansion: String,
) -> Result<std::collections::HashMap<String, String>, String> {
    personal::set_snippet(trigger, expansion).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_snippet(
    trigger: String,
) -> Result<std::collections::HashMap<String, String>, String> {
    personal::remove_snippet(&trigger).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_history() -> Vec<history::HistoryItem> {
    let mut items = history::list();
    items.reverse(); // newest first for the hub list
    items.truncate(20);
    items
}

#[tauri::command]
fn clear_history() -> Result<(), String> {
    history::clear().map_err(|e| e.to_string())
}

#[tauri::command]
fn retype_text(app: AppHandle, text: String) -> Result<(), String> {
    let enabled = app
        .state::<AppState>()
        .config
        .lock()
        .unwrap()
        .type_at_cursor;
    Typer::new(enabled)
        .type_text(&text)
        .map_err(|e| e.to_string())
}

// ------------------------------------------------------- Pipeline control
// Single place that owns the running flag, so Tauri commands and the global
// shortcut handler can never disagree about state.

fn is_running(app: &AppHandle) -> bool {
    app.state::<AppState>().running.load(Ordering::SeqCst)
}

fn start_pipeline(app: &AppHandle) {
    if app
        .state::<AppState>()
        .running
        .swap(true, Ordering::SeqCst)
    {
        return; // already running
    }
    let cfg = app.state::<AppState>().config.lock().unwrap().clone();
    let handle = app.clone();
    std::thread::spawn(move || {
        if let Err(e) = run_pipeline(handle.clone(), cfg) {
            let _ = handle.emit("dictation-state", format!("error:{}", e));
        }
    });
    let _ = app.emit("dictation-state", "listening");
    // Wakes the overlay (frontend only shows itself — it never toggles back).
    let _ = app.emit("toggle-dictation", ());
}

fn stop_pipeline(app: &AppHandle) {
    app.state::<AppState>()
        .running
        .store(false, Ordering::SeqCst);
    let _ = app.emit("dictation-state", "idle");
    let _ = app.emit("toggle-dictation", ());
}

fn toggle_pipeline(app: &AppHandle) {
    if is_running(app) {
        stop_pipeline(app);
    } else {
        start_pipeline(app);
    }
}

// ------------------------------------------------------------------ Pipeline
// mic(16kHz,30ms frames) → VAD gate → 1.5s voiced windows → STT → refine → type+emit

const WINDOW_SAMPLES: usize = 24_000; // 1.5s @16kHz
const OVERLAP: usize = 4_000; // 0.25s overlap keeps word continuity
const SILENCE_FRAMES_TO_FLUSH: usize = 25; // ~0.75s silence → flush partial

fn run_pipeline(app: AppHandle, cfg: DictationConfig) -> Result<()> {
    let running = || {
        app.state::<AppState>()
            .running
            .load(Ordering::SeqCst)
    };

    let (sample_tx, sample_rx) = bounded::<Vec<f32>>(64);
    let (level_tx, level_rx) = bounded::<f32>(64);

    // Own the mic stream for the whole session (drop = mic closed).
    let _mic = audio::start_capture(sample_tx, level_tx)?;
    let _ = app.emit("dictation-state", "listening");

    let mut vad = Vad::new(0.02);
    let provider: Provider = cfg.provider.parse().unwrap_or(Provider::Mock);
    let mut stt = stt::Stt::new(provider);
    let typer = Typer::new(cfg.type_at_cursor);

    let mut window: Vec<f32> = Vec::with_capacity(WINDOW_SAMPLES + OVERLAP);
    let mut silence_frames = 0usize;
    let mut voiced_total = 0usize;

    loop {
        if !running() {
            break;
        }
        // Drain level meter without blocking transcript path.
        while let Ok(l) = level_rx.try_recv() {
            let _ = app.emit("audio-level", l.clamp(0.0, 1.0));
        }
        // Blocking-ish wait for next 30ms frame (100ms timeout keeps Esc/shortcut snappy).
        let frame = match sample_rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(f) => f,
            Err(_) => {
                // timeout: flush if we have speech + trailing silence
                if !window.is_empty() {
                    silence_frames += 3;
                    if silence_frames >= SILENCE_FRAMES_TO_FLUSH {
                        flush(&app, &mut stt, &typer, &mut window, true)?;
                        silence_frames = 0;
                    }
                }
                continue;
            }
        };

        let speech = vad.is_speech(&frame);
        if speech {
            silence_frames = 0;
            window.extend_from_slice(&frame);
            voiced_total += frame.len();
        } else {
            silence_frames += 1;
        }

        if window.len() >= WINDOW_SAMPLES {
            flush(&app, &mut stt, &typer, &mut window, false)?;
            silence_frames = 0;
        } else if !window.is_empty() && silence_frames >= SILENCE_FRAMES_TO_FLUSH {
            flush(&app, &mut stt, &typer, &mut window, true)?;
            silence_frames = 0;
        }

        // Safety: never grow unbounded on continuous speech (overlap slides).
        if window.len() > WINDOW_SAMPLES + OVERLAP + 4800 {
            flush(&app, &mut stt, &typer, &mut window, false)?;
        }
    }

    // Final flush of leftover speech on stop.
    if voiced_total > 0 && !window.is_empty() {
        let _ = flush(&app, &mut stt, &typer, &mut window, true);
    }
    let _ = app.emit("dictation-state", "idle");
    Ok(())
}

fn flush(
    app: &AppHandle,
    stt: &mut stt::Stt,
    typer: &Typer,
    window: &mut Vec<f32>,
    is_final: bool,
) -> Result<()> {
    if window.len() < 4_800 {
        // <0.3s — likely a click/cough; drop, keep realtime clean.
        window.clear();
        return Ok(());
    }
    let _ = app.emit("dictation-state", "processing");
    let chunk = window.clone();
    // Keep overlap for next window so words on the boundary aren't cut.
    let keep = chunk.len().saturating_sub(WINDOW_SAMPLES).min(OVERLAP);
    // Fresh per flush: tiny JSON files, always reflects latest Settings edits.
    let style = refine::Style::parse(&app.state::<AppState>().config.lock().unwrap().style);
    let dict = personal::get_dictionary();
    let snippets = personal::get_snippets();
    let _res = tauri::async_runtime::block_on(stt.transcribe(&chunk));
    match _res {
        Ok(t) => {
            let final_now = is_final || t.is_final;
            let polished = if final_now {
                refine::refine_final(&t.text, style, &dict, &snippets)
            } else {
                refine::refine_partial(&t.text, style, &dict)
            };
            if !polished.trim().is_empty() {
                let _ = typer.type_text(&polished);
                let _ = app.emit(
                    "transcript",
                    serde_json::json!({ "text": polished, "is_final": final_now }),
                );
                if final_now {
                    let _ = history::append(&polished);
                }
            }
        }
        Err(e) => {
            log::error!("stt error: {:#}", e);
            let _ = app.emit("dictation-state", format!("error:{}", e));
        }
    }
    window.clear();
    if keep > 0 && !is_final {
        window.extend_from_slice(&chunk[chunk.len() - keep..]);
    }
    let _ = app.emit("dictation-state", "listening");
    Ok(())
}

// ------------------------------------------------------------------ App setup

fn main() {
    env_logger::init();

    // Global shortcuts + always-on-top need X11. On Wayland sessions, run
    // through XWayland unless the user explicitly overrides the backend.
    // Override back to native: WINIT_UNIX_BACKEND=wayland (shortcuts won't work).
    if std::env::var("WAYLAND_DISPLAY").is_ok()
        && std::env::var("WINIT_UNIX_BACKEND").is_err()
    {
        std::env::set_var("WINIT_UNIX_BACKEND", "x11");
        log::info!("Wayland detected: using X11 backend for global shortcuts + pinned overlay");
    }

    tauri::Builder::default()
        .manage(AppState {
            config: Mutex::new(DictationConfig::default()),
            running: AtomicBool::new(false),
        })
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    let toggle =
                        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyD);
                    let ptt =
                        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space);
                    match event.state {
                        ShortcutState::Pressed => {
                            if *shortcut == ptt {
                                // push-to-talk: start now, stop on release
                                start_pipeline(app);
                            } else if *shortcut == toggle {
                                toggle_pipeline(app);
                            }
                        }
                        ShortcutState::Released => {
                            if *shortcut == ptt {
                                stop_pipeline(app);
                            }
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            // Global shortcuts: toggle + push-to-talk. Registration can still
            // fail (missing X11 portal, conflicts) — overlay click always works.
            let toggle = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyD);
            let ptt = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space);
            if let Err(e) = app.global_shortcut().register(toggle) {
                log::warn!("shortcut Ctrl+Alt+D unavailable: {}", e);
            }
            if let Err(e) = app.global_shortcut().register(ptt) {
                log::warn!("shortcut Ctrl+Alt+Space unavailable: {}", e);
            }
            // Pin overlay to bottom-right of the primary monitor, above the
            // taskbar, and keep it there (always on top of every window).
            if let Some(win) = app.get_webview_window("overlay") {
                let _ = win.set_shadow(false);
                let _ = win.set_always_on_top(true);
                if let Ok(Some(mon)) = app.primary_monitor() {
                    let ms = mon.size();
                    let mp = mon.position();
                    let x = mp.x + ms.width as i32 - 380 - 16;
                    let y = mp.y + ms.height as i32 - 170 - 72;
                    let _ =
                        win.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                            x,
                            y,
                        }));
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_dictation,
            stop_dictation,
            get_config,
            set_config,
            get_dictionary,
            add_dictionary_word,
            remove_dictionary_word,
            get_snippets,
            add_snippet,
            remove_snippet,
            get_history,
            clear_history,
            retype_text
        ])
        .run(tauri::generate_context!())
        .expect("failed to run WhisperMic");
}
