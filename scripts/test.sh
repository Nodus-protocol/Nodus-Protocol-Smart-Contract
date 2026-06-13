#!/usr/bin/env bash
set -euo pipefail

echo "Running Soroban contract tests..."
cargo test --features testutils

echo "Running math-only tests (no Soroban env)..."
cargo test math_tests
cargo test liquidity_pool_tests
cargo test fuzz_math

echo "All tests passed."
