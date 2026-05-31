#!/usr/bin/env bash
set -euo pipefail

echo "Building Nodus AMM contract for Soroban..."

rustup target add wasm32-unknown-unknown 2>/dev/null || true

cargo build --target wasm32-unknown-unknown --release

WASM="target/wasm32-unknown-unknown/release/nodus_amm.wasm"
echo "Build complete: $WASM"
ls -lh "$WASM"
