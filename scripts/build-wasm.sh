#!/usr/bin/env bash
# Build the Rust core to WebAssembly and place the wasm-bindgen output inside
# the frontend source tree, where Vite can import it.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/web/src/wasm"

mkdir -p "$OUT"

wasm-pack build "$ROOT/crates/wasm" \
  --target web \
  --out-dir "$OUT" \
  --out-name calcsim_wasm \
  --release

echo "✔ WASM built → $OUT"
