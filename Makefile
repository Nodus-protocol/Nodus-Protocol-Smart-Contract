.PHONY: build build-lp-token test test-math lint format clean deploy-testnet deploy-mainnet help

help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
	    awk 'BEGIN {FS = ":.*?## "}; {printf "  %-22s %s\n", $$1, $$2}'

build-lp-token: ## Build the LP token contract WASM (must finish before the pool -- it imports this WASM via contractimport!)
	cargo build --release --target wasm32v1-none -p nodus-protocol-lp-token

build: build-lp-token ## Build all contract WASMs (LP token first, then everything else)
	cargo build --release --target wasm32v1-none --workspace

test: build-lp-token ## Run all tests (unit + integration; requires testutils feature)
	cargo test --workspace --features testutils

test-math: ## Run math-only unit tests (no Soroban env needed)
	cargo test -p nodus-protocol-amm math_tests
	cargo test -p nodus-protocol-amm liquidity_pool_tests
	cargo test -p nodus-protocol-amm fuzz_math

lint: build-lp-token ## Run clippy and check formatting
	cargo clippy --workspace --all-targets --features testutils -- -D warnings
	cargo fmt --all --check

format: ## Format all source files
	cargo fmt --all

clean: ## Remove build artifacts
	cargo clean

deploy-testnet: ## Deploy to Stellar testnet (requires STELLAR_SECRET_KEY env var)
	bash scripts/deploy.sh testnet

deploy-mainnet: ## Deploy to Stellar mainnet (requires STELLAR_SECRET_KEY env var)
	bash scripts/deploy.sh mainnet
