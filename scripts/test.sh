#!/usr/bin/env bash
set -euo pipefail

echo "Running unit tests..."
cargo test

echo "Running fuzz tests..."
cargo test --features fuzzing

echo "All tests passed."
