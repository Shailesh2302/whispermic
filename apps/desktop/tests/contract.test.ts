// Overlay ↔ Rust contract tests — bun:test style.
// Guarantees the Tauri command/event names used by src/main.js always match
// what src-tauri/src/main.rs exposes. Run: bun test
import { describe, test, expect } from "bun:test";
import path from "node:path";
import os from "node:os";
import { fileURLToPath } from "node:url";

const appDir = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const frontend = await Bun.file(path.join(appDir, "src/main.js")).text();
const backend = await Bun.file(path.join(appDir, "src-tauri/src/main.rs")).text();

const all = (src: string, re: RegExp): string[] =>
  [...src.matchAll(re)].map((m) => m[1]);

describe("overlay ↔ rust contract", () => {
  test("src/main.js bundles (syntax + @tauri-apps imports resolve)", async () => {
    const res = await Bun.build({
      entrypoints: [path.join(appDir, "src/main.js")],
      outdir: path.join(os.tmpdir(), "whispermic-contract-check"),
      external: ["@tauri-apps/*"],
    });
    if (!res.success) for (const log of res.logs) console.error(log);
    expect(res.success).toBe(true);
  });

  test("every invoke() has a Rust command", () => {
    const invoked = [...new Set(all(frontend, /invoke\("([a-z_]+)"/g))].sort();
    const gen = backend.match(/generate_handler!\[\s*([\s\S]*?)\]/);
    const handlers = new Set(
      (gen?.[1] ?? "").split(",").map((s) => s.trim()).filter(Boolean)
    );
    expect(invoked.length).toBeGreaterThan(0);
    // subset (not equality): removals go through invoke(removeCmd, …) dynamically
    for (const cmd of invoked) expect(handlers.has(cmd)).toBe(true);
  });

  test("dynamic remove commands exist in Rust", () => {
    // invoked as invoke(removeCmd, …) so covered separately from string literals
    const gen = backend.match(/generate_handler!\[\s*([\s\S]*?)\]/);
    const handlers = new Set(
      (gen?.[1] ?? "").split(",").map((s) => s.trim()).filter(Boolean)
    );
    expect(handlers.has("remove_dictionary_word")).toBe(true);
    expect(handlers.has("remove_snippet")).toBe(true);
  });

  test("every listened event is emitted by Rust", () => {
    const listened = new Set(all(frontend, /listen\("([a-z-]+)"/g));
    // NB: \s* — Rust formats long emit() calls across lines: emit(\n "name",
    const emitted = new Set(all(backend, /emit\(\s*"([a-z-]+)"/g));
    expect(listened.size).toBeGreaterThan(0);
    for (const e of listened) expect(emitted.has(e)).toBe(true);
  });
});
