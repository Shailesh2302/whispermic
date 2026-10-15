// Parse-check all browser JS via Bun.build (fails loudly on syntax errors).
// Run: bun tests/lint.ts  (or `turbo run lint --filter=@whispermic/website`)
import path from "node:path";
import os from "node:os";
import { fileURLToPath } from "node:url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const files = ["js/main.js", "js/auth.js", "js/config.js", "server.ts"];

const res = await Bun.build({
  entrypoints: files.map((f) => path.join(root, f)),
  outdir: path.join(os.tmpdir(), "whispermic-website-lint"),
});

if (!res.success) {
  for (const log of res.logs) console.error(log);
  process.exit(1);
}
console.log(`LINT OK: ${files.length} files parse`);
