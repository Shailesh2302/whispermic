// WhisperMic overlay frontend — vanilla JS (no framework = fastest, ~6KB).
// Talks to Rust via Tauri invoke + events. Realtime: interim words stream in.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const $ = (id) => document.getElementById(id);
const mic = $("mic"), bubble = $("bubble"), dot = $("dot"),
  txt = $("txt"), stateEl = $("state"), live = $("live"),
  interimBox = $("interim"), panel = $("panel");

let listening = false;
let hideTimer = null;
let hideAfterSec = 3;

function setState(s) {
  stateEl.textContent = s;
  mic.classList.toggle("listening", s === "listening");
  mic.classList.toggle("processing", s === "processing");
  live.classList.toggle("hidden", s !== "listening");
  $("toggle").textContent = listening ? "⏹ Stop" : "▶ Start";
}

// Expand bubble on first speech; auto-hide after silence.
function poke() {
  bubble.classList.remove("hidden");
  dot.classList.add("hidden");
  mic.style.display = "";
  clearTimeout(hideTimer);
}
function scheduleHide() {
  clearTimeout(hideTimer);
  hideTimer = setTimeout(async () => {
    if (!listening) {
      bubble.classList.add("hidden");
      // shrink to dot (barely visible, still summonable via shortcut)
      mic.style.display = "none";
      dot.classList.remove("hidden");
    }
  }, hideAfterSec * 1000);
}

async function toggle() {
  listening = !listening;
  poke();
  if (listening) { txt.textContent = ""; setState("listening"); await invoke("start_dictation"); }
  else { setState("idle"); await invoke("stop_dictation"); scheduleHide(); }
}

mic.addEventListener("click", toggle);
dot.addEventListener("click", () => { poke(); toggle(); });
$("toggle").addEventListener("click", (e) => { e.stopPropagation(); toggle(); });
$("close").addEventListener("click", () => { bubble.classList.add("hidden"); mic.style.display = "none"; dot.classList.remove("hidden"); });
$("gear").addEventListener("click", (e) => { e.stopPropagation(); panel.classList.toggle("hidden"); });
mic.addEventListener("contextmenu", (e) => { e.preventDefault(); poke(); panel.classList.toggle("hidden"); });
document.addEventListener("keydown", (e) => { if (e.key === "Escape") $("close").click(); });
$("save").addEventListener("click", async () => {
  hideAfterSec = Math.max(1, Math.min(15, Number($("hideAfter").value) || 3));
  await invoke("set_config", { config: {
    provider: $("provider").value,
    hide_after_secs: hideAfterSec,
    type_at_cursor: $("typeAtCursor").checked,
  }});
  panel.classList.add("hidden");
});

// --- Rust events ---
await listen("transcript", (e) => {
  // { text, is_final } — partials stream word-by-word for realtime feel
  const { text, is_final } = e.payload;
  poke();
  txt.textContent = text;
  if (is_final) { setState("listening"); scheduleHide(); }
  else setState("listening");
});

await listen("dictation-state", (e) => {
  // "listening" | "processing" | "idle" | "error:..."
  if (e.payload === "idle") { listening = false; setState("idle"); scheduleHide(); }
  else setState(e.payload);
});

await listen("audio-level", (e) => {
  // 0.0..1.0 mic level → 5-bar meter + pulse ring
  const v = Math.max(0, Math.min(1, e.payload));
  const bars = document.querySelectorAll("#lvl i");
  const n = Math.round(v * bars.length);
  bars.forEach((b, i) => b.classList.toggle("on", i < n));
  drawRing(v);
  if (v > 0.02) poke();
});

await listen("toggle-dictation", () => { poke(); }); // Rust owns the toggle; overlay just wakes

function drawRing(v) {
  const c = document.getElementById("ring"), ctx = c.getContext("2d");
  ctx.clearRect(0, 0, 64, 64);
  if (!listening) return;
  ctx.beginPath();
  ctx.arc(32, 32, 26 + v * 6, 0, Math.PI * 2);
  ctx.strokeStyle = `rgba(45,212,191,${0.3 + v * 0.7})`;
  ctx.lineWidth = 2 + v * 3;
  ctx.stroke();
}

// init config
try {
  const cfg = await invoke("get_config");
  if (cfg) {
    $("provider").value = cfg.provider ?? "mock";
    $("hideAfter").value = cfg.hide_after_secs ?? 3;
    $("typeAtCursor").checked = cfg.type_at_cursor ?? true;
    hideAfterSec = cfg.hide_after_secs ?? 3;
    if (cfg.shortcut) $("sc").textContent = cfg.shortcut;
  }
} catch {}
setState("idle");
scheduleHide();
