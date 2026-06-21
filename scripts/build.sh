#!/usr/bin/env bash
set -euo pipefail

echo "Building Nodus AMM contract for Stellar Soroban..."

if ! command -v stellar &>/dev/null; then
    echo "Stellar CLI not found. Install: cargo install --locked stellar-cli --features opt"
    exit 1
fi

stellar contract build

WASM="target/wasm32-unknown-unknown/release/nodus_protocol_amm.wasm"
echo "Build complete: $WASM"
ls -lh "$WASM"
