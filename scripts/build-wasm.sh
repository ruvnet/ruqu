#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export RUSTUP_TOOLCHAIN=1.90.0
test "$(wasm-bindgen --version)" = "wasm-bindgen 0.2.125"
cargo build --locked --release -p ruqu-wasm --target wasm32-unknown-unknown
wasm-bindgen target/wasm32-unknown-unknown/release/ruqu_wasm.wasm --target nodejs --out-dir cli/wasm
wasm-bindgen target/wasm32-unknown-unknown/release/ruqu_wasm.wasm --target web --out-dir explorer/wasm
wasm-bindgen target/wasm32-unknown-unknown/release/ruqu_wasm.wasm --target web --out-dir npm/packages/ruqu-wasm
node scripts/wasm-manifest.cjs --write
