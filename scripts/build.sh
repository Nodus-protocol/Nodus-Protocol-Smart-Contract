#!/usr/bin/env bash
set -euo pipefail

echo "Building Nodus Protocol AMM contracts..."

if ! command -v cargo-contract &> /dev/null; then
    echo "cargo-contract not found. Installing..."
    cargo install cargo-contract --locked
fi

cargo contract build --release

echo "Build complete: target/ink/amm_liquidity_pool.contract"
