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
    style: $("style").value,
  }});
  panel.classList.add("hidden");
});

// --- Dictionary / snippets / history hub ---
function renderMap(list, map, removeCmd) {
  const entries = Object.entries(map);
  list.innerHTML = "";
  entries.slice(0, 30).forEach(([from, to]) => {
    const li = document.createElement("li");
    const b = document.createElement("button");
    b.className = "x";
    b.textContent = "×";
    b.title = "Remove";
    b.addEventListener("click", async (e) => { e.stopPropagation(); await invoke(removeCmd, { [removeCmd === "remove_snippet" ? "trigger" : "misheard"]: from }); await refreshPersonal(); });
    li.append(document.createTextNode(`${from} → ${to} `), b);
    list.append(li);
  });
  return entries.length;
}
async function refreshPersonal() {
  try {
    const [d, s] = await Promise.all([invoke("get_dictionary"), invoke("get_snippets")]);
    $("dictCount").textContent = `(${Object.keys(d).length})`;
    $("snipCount").textContent = `(${Object.keys(s).length})`;
    renderMap($("dictList"), d, "remove_dictionary_word");
    renderMap($("snipList"), s, "remove_snippet");
  } catch {}
}
$("dictAdd").addEventListener("click", async () => {
  const a = $("dictFrom").value.trim(), b = $("dictTo").value.trim();
  if (!a || !b) return;
  await invoke("add_dictionary_word", { misheard: a, correction: b });
  $("dictFrom").value = $("dictTo").value = "";
  await refreshPersonal();
});
$("snipAdd").addEventListener("click", async () => {
  const a = $("snipFrom").value.trim(), b = $("snipTo").value.trim();
  if (!a || !b) return;
  await invoke("add_snippet", { trigger: a, expansion: b });
  $("snipFrom").value = $("snipTo").value = "";
  await refreshPersonal();
});
async function refreshHistory() {
  try {
    const items = await invoke("get_history");
    $("histCount").textContent = `(${items.length})`;
    const ul = $("histList");
    ul.innerHTML = "";
    items.slice(0, 10).forEach((it) => {
      const li = document.createElement("li");
      li.textContent = it.text.length > 60 ? it.text.slice(0, 60) + "…" : it.text;
      li.title = "Click to type again at cursor";
      li.addEventListener("click", async () => { await invoke("retype_text", { text: it.text }); });
      ul.append(li);
    });
  } catch {}
}
$("histClear").addEventListener("click", async () => { await invoke("clear_history"); await refreshHistory(); });
$("histBox").addEventListener("toggle", () => { if ($("histBox").open) refreshHistory(); });

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
    $("style").value = cfg.style ?? "professional";
    hideAfterSec = cfg.hide_after_secs ?? 3;
    if (cfg.shortcut) $("sc").textContent = cfg.shortcut;
  }
} catch {}
await refreshPersonal();
await refreshHistory();
setState("idle");
scheduleHide();
