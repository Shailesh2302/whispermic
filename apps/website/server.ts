// Zero-dependency static server for the WhisperMic site (Bun-native, no `serve` pkg).
// Run: bun server.ts  (or `bun run dev`)  ·  PORT env override supported.
import path from "node:path";

const ROOT = import.meta.dir;
const PORT = Number(process.env.PORT ?? 8000);

const TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".json": "application/json",
  ".png": "image/png",
  ".svg": "image/svg+xml",
  ".ico": "image/x-icon",
};

Bun.serve({
  port: PORT,
  async fetch(req) {
    const url = new URL(req.url);
    let p = decodeURIComponent(url.pathname);
    if (p.endsWith("/")) p += "index.html";
    // normalize + strip traversal attempts, then confine under ROOT
    const safe = path.normalize(p).replace(/^(\.\.[\\/])+/, "").replace(/^[/\\]+/, "");
    const file = Bun.file(path.join(ROOT, safe));
    if (!(await file.exists())) return new Response("not found", { status: 404 });
    return new Response(file, {
      headers: { "content-type": TYPES[path.extname(safe)] ?? "application/octet-stream" },
    });
  },
});

console.log(`whispermic website → http://localhost:${PORT}`);
