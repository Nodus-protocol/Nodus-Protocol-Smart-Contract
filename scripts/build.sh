#!/usr/bin/env bash
set -euo pipefail

echo "Building Nodus Protocol contracts for Stellar Soroban..."
echo "Prefer 'make build' -- it builds the LP token contract before the"
echo "pool, which the pool's build requires (see contracts/pool/src/lib.rs)."
echo "'stellar contract build' below builds the whole workspace and its"
echo "own internal ordering hasn't been verified against that requirement."

if ! command -v stellar &>/dev/null; then
    echo "Stellar CLI not found. Install: cargo install --locked stellar-cli --features opt"
    exit 1
fi

stellar contract build

POOL_WASM="target/wasm32v1-none/release/nodus_protocol_amm.wasm"
LP_TOKEN_WASM="target/wasm32v1-none/release/nodus_protocol_lp_token.wasm"
echo "Build complete:"
ls -lh "$POOL_WASM" "$LP_TOKEN_WASM"
