#!/bin/sh
# Rust logic tests that need NO system libs (no Tauri/cpal/enigo linkage).
# Compiles vad.rs + refine.rs (pure Rust, with unit tests) via rustc and runs them.
# Run: npm test -w @whispermic/desktop  (or `turbo run test --filter=@whispermic/desktop`)
set -e
cd "$(dirname "$0")/.."
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
echo "--- vad.rs"
rustc --test src-tauri/src/vad.rs -o "$TMP/vad_test"
"$TMP/vad_test"
echo "--- refine.rs"
rustc --test src-tauri/src/refine.rs -o "$TMP/refine_test"
"$TMP/refine_test"
echo "RUST LOGIC TESTS: ALL PASS"
