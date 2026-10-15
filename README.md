# WhisperMic — Voice-to-Text for Every App

The voice-to-text that turns speech into clear, polished writing in every app.

Tiny mic overlay. Speak anywhere. Text appears where your cursor is. <300ms latency.

Managed as a **Turborepo monorepo on Bun**: one `bun install` at root, `turbo` runs builds/tests across apps with caching.

## Structure

```
dictation_app/
├── package.json          # workspaces (apps/*) + turbo scripts
├── turbo.json            # pipeline: build / dev / test / check / lint
├── .github/workflows/release.yml  # CI: builds .deb/.rpm/AppImage/msi/dmg on version tags
│
├── apps/
│   ├── website/          # @whispermic/website — Marketing + Auth + Docs (static, ultra-fast, no build)
│   │   ├── index.html    # Landing
│   │   ├── login.html    # Login
│   │   ├── signup.html   # Signup
│   │   ├── docs.html     # How to use
│   │   ├── install.html  # Installation guide
│   │   ├── css/style.css
│   │   ├── js/auth.js + main.js
│   │   └── tests/auth.test.mjs
│   │
│   └── desktop/          # @whispermic/desktop — Tauri v2 (Rust backend + Vanilla JS frontend)
│       ├── src/                  # Frontend: tiny mic overlay (no framework = fastest)
│       │   ├── index.html
│       │   ├── styles.css
│       │   └── main.js
│       ├── tests/logic.sh        # Rust logic tests (no system libs needed)
│       └── src-tauri/            # Rust backend: audio → VAD → STT → refine → type
│           ├── Cargo.toml
│           ├── tauri.conf.json
│           ├── build.rs
│           ├── src/
│               ├── main.rs       # Tauri setup, shortcuts, tray, commands
│               ├── audio.rs      # cpal mic capture @16kHz mono
│               ├── vad.rs        # Energy VAD (<1ms, realtime gate)
│               ├── stt.rs        # Streaming STT (Deepgram / OpenAI / local mock)
│               ├── refine.rs     # Instant text cleanup + punctuation + styles
│               ├── personal.rs   # Dictionary + snippets (offline JSON)
│               ├── history.rs    # Local dictation history hub
│               └── typer.rs      # enigo typing at cursor
```

## Why this stack = fast + low latency

| Layer | Choice | Why fast |
|---|---|---|
| Backend | **Rust** | Zero-GC, lock-free audio ring buffer, <5ms overhead |
| Frontend (overlay) | **Vanilla JS + Tauri WebView** | No React/Vue runtime. 60px DOM, ~15KB total. No framework diffing. |
| Audio | `cpal` 16kHz mono f32 | Native, no resampling in JS |
| VAD | Energy + ZCR gate in Rust | ~0.2ms/frame, drops silence before STT (saves 60% compute) |
| STT | Streaming: Deepgram Nova-2 / local whisper.cpp `tiny.en` | Partial results every ~300ms; local fallback works offline |
| Typing | `enigo` key injection | Types at OS cursor — works in Chrome, VS Code, Word, Terminal |
| Shortcut | Tauri global-shortcut | OS-level, works even when overlay hidden |

**Latency budget:** mic→VAD 30ms + STT partial 200-300ms + refine 1ms + type 5ms ≈ **250-350ms word-visible**.

## Turborepo commands (run from repo root, Bun 1.1+)

```bash
bun install          # install all workspaces + turbo (fast, single lockfile)
bun run test         # turbo run test   → website auth tests + overlay/rust contract + Rust logic tests
bun run build        # turbo run build  → per-app dist/ outputs (cached)
bun run dev          # turbo dev        → runs every app's dev server
bun run check        # turbo run check  → type/system checks per app
```

> Note: use `bun run test`, not bare `bun test` — bare `bun test` invokes Bun's
> own test runner in the current dir instead of the turbo pipeline.

Target one app:

```bash
bunx turbo dev --filter=@whispermic/desktop    # Tauri hot-reload overlay
bunx turbo dev --filter=@whispermic/website    # static site on :8000 (Bun.serve, zero deps)
bunx turbo run build --filter=@whispermic/desktop
```

## Quick start

### 1. Website (login/signup/docs)

```bash
bunx turbo dev --filter=@whispermic/website
# open http://localhost:8000  (override port: PORT=4000 bunx turbo dev --filter=@whispermic/website)
```

Auth is demo-backed by `localStorage` + ready to plug Supabase. Edit `apps/website/js/config.js` → set `API_URL`.

### 2. Desktop app (dev)

```bash
# prerequisites (Ubuntu/Debian)
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf libasound2-dev

bunx turbo dev --filter=@whispermic/desktop
```

Release build:

```bash
bunx turbo run build --filter=@whispermic/desktop
# binary in apps/desktop/src-tauri/target/release/
# installers in apps/desktop/src-tauri/target/release/bundle/{deb,rpm}/ (auto-copied to apps/website/downloads/)
```

Packaged installers (.deb / .rpm): `bundle.targets` is `"all"`, so a Linux
build always emits `.deb` + `.rpm` + `.AppImage` (the `rpm` CLI is needed on
Debian for the `.rpm`). Published builds come from CI — push a version tag:

```bash
git tag v0.1.0 && git push origin v0.1.0
# → GitHub Actions builds per OS and attaches installers to a draft Release:
#   Linux: WhisperMic_*_amd64.deb, WhisperMic-*.x86_64.rpm
#   Windows: *.msi · macOS: *.dmg
```

Rust-only check (no Node needed):

```bash
cd apps/desktop/src-tauri
cargo check
```

### 3. Configure STT

Copy `apps/desktop/.env.example` → `.env` (or set in app Settings):

```bash
STT_PROVIDER=deepgram   # deepgram | openai | mock
DEEPGRAM_API_KEY=...
OPENAI_API_KEY=...
```

Offline: download `ggml-tiny.en.bin` (whisper.cpp) into `~/.whispermic/models/` — see `apps/website/install.html`.

## Mic behavior (as requested)

- **Tiny:** 56×56px circle, always-on-top, transparent, draggable. `opacity: 0.55` idle.
- **Real-time:** interim transcripts stream word-by-word and are typed instantly at cursor.
- **Auto-hide:** after `silence_timeout` (default 3s) + no speech → overlay fades to 12px dot. Any audio or shortcut brings it back.
- **Shortcut (default):** `Ctrl+Alt+D` toggle · Hold `Ctrl+Alt+Space` = push-to-talk. Change in Settings → Shortcut (click + press keys).
- **Refine:** filler removal (`um`, `uh`), auto-capitalization, `. ? ! ,` restoration, double-space collapse — all in `refine.rs` (<1ms, no network).

## Keyboard shortcuts

| Action | Default | Notes |
|---|---|---|
| Toggle dictation | `Ctrl+Alt+D` | Global, works in any app |
| Push-to-talk (hold) | `Ctrl+Alt+Space` | Release = auto-stop + hide |
| Hide overlay | `Esc` (when focused) | Still listening via shortcut |

## Docs

Full docs on website: `apps/website/docs.html` + `apps/website/install.html`.
