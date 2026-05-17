# AMM Liquidity Pool Smart Contracts (ink!)

Automated Market Maker (AMM) liquidity pool smart contracts written in **Rust** using the [**ink!**](https://use.ink/) framework. Deployable on Substrate-based blockchains with `pallet-contracts` or `pallet-revive`.

---

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Repository Structure](#repository-structure)
- [Core Contracts](#core-contracts)
  - [LiquidityPool](#liquiditypool)
  - [LPToken (PSP22)](#lptoken-psp22)
  - [ReentrancyGuard](#reentrancyguard)
- [Key Features](#key-features)
- [Math & Invariants](#math--invariants)
- [Security](#security)
- [Build & Deploy](#build--deploy)
- [Testing](#testing)
- [Events & Indexing](#events--indexing)
- [API Reference](#api-reference)
- [License](#license)

---

## Overview

This repository contains the on-chain logic for a constant-product AMM DEX. It manages token reserves, liquidity provider shares, and atomic token swaps while enforcing strict mathematical invariants and security guarantees.

**Chain Compatibility:** Substrate-based chains with Contracts pallet (e.g., Aleph Zero, Phala, Astar, Shiden)

## Repository Structure

amm-smart-contracts/
├── src/
│   ├── lib.rs                    # Contract entry point & module exports
│   ├── liquidity_pool.rs         # Core pool logic (add/remove/swap)
│   ├── lp_token.rs               # PSP22 LP token implementation
│   ├── reentrancy_guard.rs       # Reentrancy protection pattern
│   ├── math.rs                   # On-chain math helpers
│   ├── events.rs                 # Event definitions (Mint, Burn, Swap)
│   ├── errors.rs                 # Error enum types
│   └── traits.rs                 # Interface definitions
│
├── tests/
│   ├── unit_tests.rs             # Unit tests (100% coverage)
│   ├── integration_tests.rs      # Cross-contract integration
│   └── fuzz_tests.rs             # Extreme value fuzzing
│
├── scripts/
│   ├── build.sh                  # Build optimized WASM
│   ├── test.sh                   # Run full test suite
│   └── deploy.sh                 # Deploy to local/testnet node
│
├── docs/
│   ├── ARCHITECTURE.md           # Detailed architecture docs
│   └── SECURITY.md               # Security audit notes & vectors
│
├── Cargo.toml                    # Workspace manifest
├── rust-toolchain.toml           # Toolchain specification
├── Makefile                      # Common commands
├── README.md                     # This file
└── .gitignore



---

## Core Contracts

### LiquidityPool

The main contract holding token reserves and executing swaps.

#### Storage

```rust
#[ink(storage)]
pub struct LiquidityPool {
    token_0: AccountId,           // First token address
    token_1: AccountId,           // Second token address
    reserve_0: Balance,           // Token 0 reserves
    reserve_1: Balance,           // Token 1 reserves
    block_timestamp_last: u64,    // Last update timestamp
    price_0_cumulative_last: u128, // Cumulative price for TWAP
    price_1_cumulative_last: u128, // Cumulative price for TWAP
    k_last: u128,                 // Last invariant (for fee logic)
    lp_token: AccountId,          // LP token contract address
    locked: bool,                 // Reentrancy guard state
}


Execute Python code
Search
ink! smart contract 2026 latest version openbrush psp22 example 2026
15 results
Execute Python code
Done. Here's your smart contract-only repo with a comprehensive README. You can copy-paste everything below to scaffold it:
📁 Repo Structure
plain
Copy
amm-smart-contracts/
├── src/
│   ├── lib.rs                    # Entry point
│   ├── liquidity_pool.rs         # Core pool logic
│   ├── lp_token.rs               # PSP22 LP token
│   ├── reentrancy_guard.rs       # Mutex guard
│   ├── math.rs                   # On-chain math
│   ├── events.rs                 # Event definitions
│   ├── errors.rs                 # Error enums
│   └── traits.rs                 # Interfaces
├── tests/
│   ├── unit_tests.rs
│   ├── integration_tests.rs
│   └── fuzz_tests.rs
├── scripts/
│   ├── build.sh
│   ├── test.sh
│   └── deploy.sh
├── docs/
│   ├── ARCHITECTURE.md
│   └── SECURITY.md
├── Cargo.toml
├── rust-toolchain.toml
├── Makefile
├── README.md
├── LICENSE
└── .gitignore
📄 README.md
Copy and save as README.md:
Markdown
Copy
Code
Preview
# AMM Liquidity Pool Smart Contracts (ink!)

Automated Market Maker (AMM) liquidity pool smart contracts written in **Rust** using the [**ink!**](https://use.ink/) framework. Deployable on Substrate-based blockchains with `pallet-contracts` or `pallet-revive`.

---

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Repository Structure](#repository-structure)
- [Core Contracts](#core-contracts)
  - [LiquidityPool](#liquiditypool)
  - [LPToken (PSP22)](#lptoken-psp22)
  - [ReentrancyGuard](#reentrancyguard)
- [Key Features](#key-features)
- [Math & Invariants](#math--invariants)
- [Security](#security)
- [Build & Deploy](#build--deploy)
- [Testing](#testing)
- [Events & Indexing](#events--indexing)
- [API Reference](#api-reference)
- [License](#license)

---

## Overview

This repository contains the on-chain logic for a constant-product AMM DEX. It manages token reserves, liquidity provider shares, and atomic token swaps while enforcing strict mathematical invariants and security guarantees.

**Chain Compatibility:** Substrate-based chains with Contracts pallet (e.g., Aleph Zero, Phala, Astar, Shiden)

---

## Architecture
┌─────────────────────────────────────────┐
│           LiquidityPool                 │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐ │
│  │ Reserve │  │ Reserve │  │   K     │ │
│  │ Token 0 │  │ Token 1 │  │Invariant│ │
│  └────┬────┘  └────┬────┘  └─────────┘ │
│       │            │                    │
│  ┌────▼────────────▼────┐              │
│  │    LPToken (PSP22)   │              │
│  │  LP Share Tracking   │              │
│  └──────────────────────┘              │
│       │                                │
│  ┌────▼────────────────────┐           │
│  │   ReentrancyGuard       │           │
│  │   Mutex-style locking   │           │
│  └─────────────────────────┘           │
└─────────────────────────────────────────┘
plain
Copy

---

## Repository Structure
amm-smart-contracts/
├── src/
│   ├── lib.rs                    # Contract entry point & module exports
│   ├── liquidity_pool.rs         # Core pool logic (add/remove/swap)
│   ├── lp_token.rs               # PSP22 LP token implementation
│   ├── reentrancy_guard.rs       # Reentrancy protection pattern
│   ├── math.rs                   # On-chain math helpers
│   ├── events.rs                 # Event definitions (Mint, Burn, Swap)
│   ├── errors.rs                 # Error enum types
│   └── traits.rs                 # Interface definitions
│
├── tests/
│   ├── unit_tests.rs             # Unit tests (100% coverage)
│   ├── integration_tests.rs      # Cross-contract integration
│   └── fuzz_tests.rs             # Extreme value fuzzing
│
├── scripts/
│   ├── build.sh                  # Build optimized WASM
│   ├── test.sh                   # Run full test suite
│   └── deploy.sh                 # Deploy to local/testnet node
│
├── docs/
│   ├── ARCHITECTURE.md           # Detailed architecture docs
│   └── SECURITY.md               # Security audit notes & vectors
│
├── Cargo.toml                    # Workspace manifest
├── rust-toolchain.toml           # Toolchain specification
├── Makefile                      # Common commands
├── README.md                     # This file
└── .gitignore
plain
Copy

---

## Core Contracts

### LiquidityPool

The main contract holding token reserves and executing swaps.

#### Storage

```rust
#[ink(storage)]
pub struct LiquidityPool {
    token_0: AccountId,           // First token address
    token_1: AccountId,           // Second token address
    reserve_0: Balance,           // Token 0 reserves
    reserve_1: Balance,           // Token 1 reserves
    block_timestamp_last: u64,    // Last update timestamp
    price_0_cumulative_last: u128, // Cumulative price for TWAP
    price_1_cumulative_last: u128, // Cumulative price for TWAP
    k_last: u128,                 // Last invariant (for fee logic)
    lp_token: AccountId,          // LP token contract address
    locked: bool,                 // Reentrancy guard state
}
Messages
Table
Message	Description	Access
add_liquidity	Deposit token pair, mint LP tokens	External
remove_liquidity	Burn LP tokens, withdraw pair	External
swap	Execute token swap with invariant check	External
sync	Sync reserves with actual balances	External
skim	Recover excess tokens	External
get_reserves	Read current reserves	Read-only
get_amount_out	Preview swap output	Read-only
get_amount_in	Preview swap input required	Read-only

Add Liquidity Flow
1. Validate token pair & amounts
2. Calculate optimal amounts based on existing reserves
3. Transfer tokens from caller to pool
4. Mint LP tokens = min(amount0/reserve0, amount1/reserve1) * total_supply
5. Update reserves
6. Emit Mint event
7. Emit Sync event

1. Validate output amounts requested
2. Calculate required input using constant-product formula
3. Transfer output tokens to recipient
4. Verify invariant: (reserve0 + amount0_in) * (reserve1 + amount1_in) >= reserve0 * reserve1
5. Update reserves
6. Emit Swap event
7. Emit Sync event

LPToken (PSP22)
Standard PSP22 token representing liquidity provider shares.
rust
Copy
#[openbrush::contract]
pub mod lp_token {
    #[ink(storage)]
    pub struct LPToken {
        #[storage_field]
        psp22: PSP22Data,
        name: String,
        symbol: String,
        decimals: u8,
    }
    
    impl PSP22 for LPToken {}
    impl PSP22Metadata for LPToken {}
    impl PSP22Mintable for LPToken {}  // Only callable by pool
    impl PSP22Burnable for LPToken {}  // Only callable by pool
}
ReentrancyGuard
Manual mutex-style reentrancy protection (no nonReentrant modifier in ink!):
rust
Copy
pub trait ReentrancyGuard {
    fn _lock(&mut self) -> Result<(), Error>;
    fn _unlock(&mut self);
}

// Usage in every state-changing message:
fn add_liquidity(&mut self, ...) -> Result<<...> {
    self._lock()?;
    // ... logic ...
    self._unlock();
    Ok(...)
}
Key Features
Constant-Product Invariant
x×y=k 
Where x  and y  are token reserves, k  is the invariant. After every swap:
(x+Δx)×(y−Δy)≥x×y 
Fee Structure
Swap Fee: 0.3% (30 basis points)
Fee stays in reserves, implicitly accruing to LPs
Fee numerator: 3, denominator: 1000
Minimum Liquidity Lock
On first mint, MINIMUM_LIQUIDITY (1000 wei) is permanently locked to prevent division-by-zero exploits and ensure minimum share granularity.
TWAP (Time-Weighted Average Price)
Cumulative price tracking for oracle functionality:
rust
Copy
price_cumulative += (reserve_other / reserve_this) * time_elapsed
Math & Invariants
Swap Output Formula
plain
Copy
amount_out = (reserve_out * amount_in * 997) / (reserve_in * 1000 + amount_in * 997)
Swap Input Formula
plain
Copy
amount_in = (reserve_in * amount_out * 1000) / ((reserve_out - amount_out) * 997) + 1
LP Token Minting
plain
Copy
liquidity = min(
    (amount0 * total_supply) / reserve0,
    (amount1 * total_supply) / reserve1
)
LP Token Burning
plain
Copy
amount0 = (liquidity * reserve0) / total_supply
amount1 = (liquidity * reserve1) / total_supply
Security
Table
Attack Vector	Mitigation
Reentrancy	Manual locked boolean guard + CEI pattern
Flash Loans	No external callbacks during swap. Atomic execution.
Rounding Exploits	Ceiling division on LP minting. Floor on burning.
First Deposit Attack	MINIMUM_LIQUIDITY permanently locked
Integer Overflow	checked_* arithmetic with explicit error handling
Front-running	Minimum output / maximum input enforced
Re-orgs	Events include block context for indexer validation
Price Manipulation	TWAP resistant to single-block manipulation
See docs/SECURITY.md for detailed audit notes.
Build & Deploy
Prerequisites
Rust 1.70+ with wasm32-unknown-unknown target
cargo-contract CLI tool
Local Substrate node (e.g., substrate-contracts-node)
Install
bash
Copy
# Install ink! CLI
cargo install cargo-contract

# Add WASM target
rustup target add wasm32-unknown-unknown
Build
bash
Copy
# Standard build
cargo contract build

# Optimized release build
cargo contract build --release

# Output: target/ink/amm_liquidity_pool.contract (WASM + metadata)
Deploy Local
bash
Copy
# Start local node
substrate-contracts-node --dev --tmp

# Deploy (in another terminal)
cargo contract upload --suri //Alice
cargo contract instantiate --suri //Alice --args <token0> <token1>
Deploy Testnet
bash
Copy
cargo contract upload --url wss://ws.test.azero.dev --suri //Alice
cargo contract instantiate --url wss://ws.test.azero.dev --suri //Alice --args <token0> <token1>
Testing
Unit Tests
bash
Copy
# Run all unit tests
cargo test

# With output
cargo test -- --nocapture

# Specific module
cargo test liquidity_pool
Integration Tests
bash
Copy
# End-to-end tests (requires running node)
cargo test --features e2e-tests

# Or use ink! off-chain environment
cargo test --features ink-test
Fuzz Tests
bash
Copy
# Extreme value testing
cargo test --features fuzzing

# Proptest for invariant preservation
# Tests: near-zero liquidity, massive trades, decimal edge cases
Coverage
bash
Copy
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html

# Open: tarpaulin-report.html
Events & Indexing
All state changes emit structured events for backend indexing:
Mint Event
rust
Copy
#[ink::event]
pub struct Mint {
    #[ink(topic)]
    pub sender: AccountId,
    pub amount_0: Balance,
    pub amount_1: Balance,
}
Burn Event
rust
Copy
#[ink::event]
pub struct Burn {
    #[ink(topic)]
    pub sender: AccountId,
    pub amount_0: Balance,
    pub amount_1: Balance,
    #[ink(topic)]
    pub to: AccountId,
}
Swap Event
rust
Copy
#[ink::event]
pub struct Swap {
    #[ink(topic)]
    pub sender: AccountId,
    pub amount_0_in: Balance,
    pub amount_1_in: Balance,
    pub amount_0_out: Balance,
    pub amount_1_out: Balance,
    #[ink(topic)]
    pub to: AccountId,
}
Sync Event
rust
Copy
#[ink::event]
pub struct Sync {
    pub reserve_0: Balance,
    pub reserve_1: Balance,
}
Backend Integration: Events are indexed by the Go backend service to track historical volume, TVL, and price data.
API Reference
Constructor
rust
Copy
#[ink(constructor)]
pub fn new(token_0: AccountId, token_1: AccountId) -> Self
Messages
rust
Copy
// Add liquidity
#[ink(message)]
pub fn add_liquidity(
    &mut self,
    amount_0_desired: Balance,
    amount_1_desired: Balance,
    amount_0_min: Balance,
    amount_1_min: Balance,
    to: AccountId,
    deadline: u64,
) -> Result<<Balance, Error>

// Remove liquidity
#[ink(message)]
pub fn remove_liquidity(
    &mut self,
    liquidity: Balance,
    amount_0_min: Balance,
    amount_1_min: Balance,
    to: AccountId,
    deadline: u64,
) -> Result<(Balance, Balance), Error>

// Swap tokens
#[ink(message)]
pub fn swap(
    &mut self,
    amount_0_out: Balance,
    amount_1_out: Balance,
    to: AccountId,
    data: Vec<u8>,
) -> Result<(), Error>

// Read reserves
#[ink(message)]
pub fn get_reserves(&self) -> (Balance, Balance, u64)

// Preview swap output
#[ink(message)]
pub fn get_amount_out(&self, amount_in: Balance, reserve_in: Balance, reserve_out: Balance) -> Balance

// Preview swap input
#[ink(message)]
pub fn get_amount_in(&self, amount_out: Balance, reserve_in: Balance, reserve_out: Balance) -> Balance
Error Types
rust
Copy
pub enum Error {
    InsufficientLiquidity,
    InsufficientLiquidityMinted,
    InsufficientLiquidityBurned,
    InsufficientOutputAmount,
    ExcessiveInputAmount,
    InvalidTokenPair,
    ReentrancyDetected,
    TransferFailed,
    Expired,
    KInvariantViolated,
    ZeroAmount,
    Overflow,
}
Scripts
Quick Commands
bash
Copy
# Build everything
make build

# Run tests
make test

# Run with coverage
make coverage

# Deploy locally
make deploy-local

# Deploy to testnet
make deploy-testnet

# Lint
make lint

# Format code
make format

# Clean artifacts
make clean
License
MIT License - See LICENSE for details.
Contributing
Fork the repository
Create a feature branch: git checkout -b feature/amazing-feature
Commit changes: git commit -m 'Add amazing feature'
Push to branch: git push origin feature/amazing-feature
Open a Pull Request
Requirements:
All tests must pass (cargo test)
100% unit test coverage for math logic
Clippy linting must pass (cargo clippy -- -D warnings)
Security review required for state-changing code
Resources
ink! Documentation
OpenBrush Contracts
Substrate Contracts Node
Aleph Zero Testnet
PSP22 Standard
plain
Copy

---

## 📦 Config Files

**`Cargo.toml`:**

```toml
[package]
name = "amm-liquidity-pool"
version = "0.1.0"
authors = ["AMM Platform Team"]
edition = "2021"
license = "MIT"
description = "Constant-product AMM liquidity pool smart contract in ink!"
repository = "https://github.com/your-org/amm-smart-contracts"

[dependencies]
ink = { version = "5.0.0", default-features = false }
scale = { package = "parity-scale-codec", version = "3", default-features = false, features = ["derive"] }
scale-info = { version = "2.6", default-features = false, features = ["derive"], optional = true }
openbrush = { git = "https://github.com/Brushfam/openbrush-contracts", tag = "v4.0.0", default-features = false, features = ["psp22"] }
primitive-types = { version = "0.12", default-features = false }

[dev-dependencies]
ink_e2e = { version = "5.0.0" }
proptest = { version = "1.4" }

[lib]
name = "amm_liquidity_pool"
path = "src/lib.rs"
crate-type = ["cdylib"]

[features]
default = ["std"]
std = [
    "ink/std",
    "scale/std",
    "scale-info/std",
    "openbrush/std",
    "primitive-types/std",
]
ink-as-dependency = []
e2e-tests = []
fuzzing = []

[profile.release]
overflow-checks = false
panic = "abort"
lto = true
codegen-units = 1
opt-level = "z"
strip = true
rust-toolchain.toml:
toml
Copy
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy", "rust-src"]
targets = ["wasm32-unknown-unknown"]
profile = "minimal"
Makefile:
makefile
Copy
.PHONY: build test coverage deploy-local deploy-testnet lint format clean help

help: ## Show this help message
	@echo "Available commands:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

build: ## Build optimized WASM contract
	@echo "🔨 Building contract..."
	cargo contract build --release
	@echo "✅ Build complete: target/ink/amm_liquidity_pool.contract"

test: ## Run all tests
	@echo "🧪 Running unit tests..."
	cargo test
	@echo "🧪 Running integration tests..."
	cargo test --features e2e-tests
	@echo "✅ All tests passed"

coverage: ## Generate test coverage report
	@echo "📊 Generating coverage report..."
	cargo tarpaulin --out Html
	@echo "✅ Report: tarpaulin-report.html"

lint: ## Run clippy and rustfmt
	@echo "🔍 Running clippy..."
	cargo clippy -- -D warnings
	@echo "🔍 Checking formatting..."
	cargo fmt -- --check
	@echo "✅ Linting complete"

format: ## Format all code
	cargo fmt

clean: ## Clean build artifacts
	cargo clean
	rm -rf target/
	@echo "🧹 Cleaned build artifacts"

deploy-local: ## Deploy to local substrate-contracts-node
	@echo "🚀 Deploying to local node..."
	cargo contract upload --suri //Alice
	cargo contract instantiate --suri //Alice --args <token0> <token1>
	@echo "✅ Deployed to local node"

deploy-testnet: ## Deploy to Aleph Zero testnet
	@echo "🚀 Deploying to testnet..."
	cargo contract upload --url wss://ws.test.azero.dev --suri //Alice
	cargo contract instantiate --url wss://ws.test.azero.dev --suri //Alice --args <token0> <token1>
	@echo "✅ Deployed to testnet"

node: ## Start local development node
	@echo "🌐 Starting substrate-contracts-node..."
	substrate-contracts-node --dev --tmp
.gitignore:
gitignore
Copy
# Rust
target/
Cargo.lock
**/*.rs.bk
*.pdb

# ink! contract artifacts
*.contract
*.wasm
*.json
.metadata/

# IDE
.idea/
.vscode/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db

# Logs
logs/
*.log

# Coverage
tarpaulin-report.html
*.profraw
