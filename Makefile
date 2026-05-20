.PHONY: build test coverage deploy-local deploy-testnet lint format clean node help

help:
	@echo "Available commands:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
	    awk 'BEGIN {FS = ":.*?## "}; {printf "  %-20s %s\n", $$1, $$2}'

build: ## Build optimized WASM contract
	cargo contract build --release

test: ## Run unit and fuzz tests
	cargo test
	cargo test --features fuzzing

coverage: ## Generate HTML test coverage report (requires cargo-tarpaulin)
	cargo tarpaulin --out Html

lint: ## Run clippy and check formatting
	cargo clippy -- -D warnings
	cargo fmt -- --check

format: ## Format all source files
	cargo fmt

clean: ## Remove build artifacts
	cargo clean

node: ## Start a local substrate-contracts-node for development
	substrate-contracts-node --dev --tmp

deploy-local: ## Deploy contracts to local node (requires TOKEN_0, TOKEN_1, LP_TOKEN env vars)
	bash scripts/deploy.sh local

deploy-testnet: ## Deploy contracts to Aleph Zero testnet (requires TOKEN_0, TOKEN_1, LP_TOKEN, SURI)
	bash scripts/deploy.sh testnet
