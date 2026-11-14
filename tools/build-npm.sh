#!/usr/bin/env bash
# Builds the npm package: npm/tsdocrs's crate for wasm32-unknown-unknown, then wasm-bindgen's
# Node.js bindings and TypeScript types into npm/tsdocrs/pkg. The wasm-bindgen CLI has to be the
# version npm/tsdocrs/Cargo.toml pins.
set -euo pipefail
cd "$(dirname "$0")/.."

manifest=npm/tsdocrs/Cargo.toml
want=$(sed -n 's/^wasm-bindgen = "=\(.*\)"$/\1/p' "$manifest")
have=$(wasm-bindgen --version | cut -d' ' -f2)
if [[ $have != "$want" ]]; then
  echo "wasm-bindgen $have is installed, and npm/tsdocrs pins $want" >&2
  exit 1
fi
cargo build --locked --release --target wasm32-unknown-unknown --manifest-path "$manifest"
wasm-bindgen --target nodejs --out-dir npm/tsdocrs/pkg --out-name tsdocrs \
  npm/tsdocrs/target/wasm32-unknown-unknown/release/tsdocrs_wasm.wasm
