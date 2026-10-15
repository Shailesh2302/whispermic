// Landing demo typing animation
const SENTENCES = [
  "hello team, shipping the release notes today.",
  "remember to refine that audio and text in real time...",
  "press Control Alt D anywhere, and just speak.",
];
let si = 0, ci = 0, deleting = false;
function tick() {
  const el = document.getElementById("demo-text");
  if (!el) return;
  const full = SENTENCES[si];
  if (!deleting) {
    ci++;
    if (ci >= full.length) { ci = full.length; deleting = true; setTimeout(tick, 1600); render(); return; }
  } else {
    ci -= 2;
    if (ci <= 0) { ci = 0; deleting = false; si = (si + 1) % SENTENCES.length; }
  }
  render();
  setTimeout(tick, deleting ? 18 : 42);
  function render() { el.innerHTML = escapeHtml(full.slice(0, ci)) + '<span class="caret"></span>'; }
}
function escapeHtml(s){return s.replace(/&/g,"&amp;").replace(/</g,"&lt;")}
document.addEventListener("DOMContentLoaded", () => { tick(); updateYear(); });
function updateYear(){document.querySelectorAll("[data-year]").forEach(e=>e.textContent=new Date().getFullYear());}
function detectOS(){
  const p = ((navigator.platform || "") + "").toLowerCase(),
        ua = ((navigator.userAgent || "") + "").toLowerCase();
  if (p.includes("win")||ua.includes("windows")) return { key: "windows", label: "Windows (.msi)" };
  if (p.includes("mac")||ua.includes("mac")) return { key: "macos", label: "macOS (.dmg)" };
  return { key: "linux", label: "Linux (.deb / .rpm)" };
}
// Filenames shipped in downloads/ (copied there by the desktop build).
const LOCAL_FILES = {
  windows: "WhisperMic_0.1.0_x64.msi",
  macos: "WhisperMic_0.1.0_aarch64.dmg",
};
document.addEventListener("DOMContentLoaded",()=>{
  const os = detectOS();
  document.querySelectorAll("[data-os-dl]").forEach(e=>e.textContent=os.label);
  // highlight the visitor's OS row in the download table
  document.querySelectorAll("[data-os-row]").forEach((r) => {
    if (r.getAttribute("data-os-row") === os.key) r.classList.add("picked");
  });
});
// Wire download buttons.
// - Releases published (RELEASES_URL set): everything points at GitHub release assets.
// - Local mode: Windows/macOS buttons download straight from downloads/;
//   Linux keeps the install-guide link (user picks .deb vs .rpm).
document.addEventListener("DOMContentLoaded", () => {
  const cfg = (window.APP_CONFIG || {});
  const base = (cfg.RELEASES_URL || "").replace(/\/$/, "");
  const os = detectOS();
  if (base && !base.includes("YOUR_USER")) {
    const tag = "v" + (cfg.VERSION || "0.1.0");
    document.querySelectorAll("[data-dl]").forEach((a) => { a.href = base; });
    document.querySelectorAll("[data-dl-file]").forEach((a) => {
      a.href = `${base}/download/${tag}/${a.getAttribute("data-dl-file")}`;
    });
    return;
  }
  if (LOCAL_FILES[os.key]) {
    document.querySelectorAll("[data-dl]").forEach((a) => { a.href = "downloads/" + LOCAL_FILES[os.key]; });
  }
});
