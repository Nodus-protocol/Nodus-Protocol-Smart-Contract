.PHONY: build test lint format clean deploy-testnet deploy-mainnet help

help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
	    awk 'BEGIN {FS = ":.*?## "}; {printf "  %-22s %s\n", $$1, $$2}'

build: ## Build optimised contract WASM via Stellar CLI
	stellar contract build

test: ## Run all tests (unit + integration; requires testutils feature)
	cargo test --features testutils

test-math: ## Run math-only unit tests (no Soroban env needed)
	cargo test math_tests
	cargo test liquidity_pool_tests
	cargo test fuzz_math

lint: ## Run clippy and check formatting
	cargo clippy --all-targets --features testutils -- -D warnings
	cargo fmt --all --check

format: ## Format all source files
	cargo fmt --all

clean: ## Remove build artifacts
	cargo clean

deploy-testnet: ## Deploy to Stellar testnet (requires STELLAR_SECRET_KEY env var)
	bash scripts/deploy.sh testnet

deploy-mainnet: ## Deploy to Stellar mainnet (requires STELLAR_SECRET_KEY env var)
	bash scripts/deploy.sh mainnet
