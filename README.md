# Nodus Protocol — AMM Liquidity Pool Smart Contracts

Constant-product Automated Market Maker (AMM) smart contracts written in **Rust** using the [ink!](https://use.ink/) framework. Deployable on Substrate-based blockchains with `pallet-contracts` or `pallet-revive`.

**Chain Compatibility:** Substrate chains with the Contracts pallet — Aleph Zero, Phala, Astar, Shiden, and compatible networks.

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
- [Math and Invariants](#math-and-invariants)
- [Security](#security)
- [Build and Deploy](#build-and-deploy)
- [Testing](#testing)
- [Events and Indexing](#events-and-indexing)
- [API Reference](#api-reference)
- [License](#license)

---

## Overview

This repository contains the on-chain logic for Nodus Protocol's constant-product AMM DEX. It manages token reserves, liquidity provider shares, and atomic token swaps while enforcing strict mathematical invariants and security guarantees.

The protocol is modelled after the Uniswap V2 design and adapted for the ink! execution model on Substrate. Two cooperating contracts form the core:

1. **LiquidityPool** — holds reserves, executes swaps, delegates LP token minting and burning.
2. **LPToken** — a PSP22-compatible fungible token representing each provider's proportional share.

---

## Architecture

```text
┌───────────────────────────────────────────┐
│              LiquidityPool                │
│                                           │
│  reserve_0 ──── reserve_1                 │
│       \              /                    │
│        k = x * y (invariant)              │
│                                           │
│  add_liquidity()  ─────────────────────── │──► LPToken.mint()
│  remove_liquidity() ───────────────────── │──► LPToken.burn()
│  swap()                                   │
│  sync()   (drift correction)              │
│                                           │
│  price_0_cumulative_last (TWAP oracle)    │
│  price_1_cumulative_last (TWAP oracle)    │
└───────────────────────────────────────────┘
                    │
                    │ cross-contract calls
                    ▼
┌───────────────────────────────────────────┐
│            LPToken (PSP22)                │
│                                           │
│  balances    Mapping<AccountId, u128>     │
│  allowances  Mapping<(AccountId,          │
│               AccountId), u128>           │
│  total_supply: u128                       │
│                                           │
│  mint()  — pool contract only             │
│  burn()  — pool contract only             │
│  transfer(), transfer_from(), approve()   │
└───────────────────────────────────────────┘
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed flow diagrams.

---

## Repository Structure

```text
nodus-protocol-smart-contract/
├── src/
│   ├── lib.rs                  # Crate root: module declarations and #[ink::contract] entry
│   ├── liquidity_pool.rs       # Pure business logic: optimal amounts, K invariant, LP math
│   ├── lp_token.rs             # Cross-contract call wrapper for the PSP22 LP token
│   ├── reentrancy_guard.rs     # Mutex-style lock / unlock trait
│   ├── math.rs                 # get_amount_out, get_amount_in, sqrt, fee constants
│   ├── events.rs               # Mint, Burn, Swap, Sync event definitions
│   ├── errors.rs               # Error enum covering all failure modes
│   └── traits.rs               # ILiquidityPool and IPSP22 ink! trait definitions
│
├── tests/
│   ├── unit_tests.rs           # Unit tests for math and pool logic functions
│   ├── integration_tests.rs    # End-to-end tests against a live substrate node
│   └── fuzz_tests.rs           # Property-based fuzz tests using proptest
│
├── scripts/
│   ├── build.sh                # Build optimized WASM artifact
│   ├── test.sh                 # Run full test suite
│   └── deploy.sh               # Deploy to local node or testnet
│
├── docs/
│   ├── ARCHITECTURE.md         # Detailed flow diagrams and module breakdown
│   └── SECURITY.md             # Threat model, mitigations, and audit checklist
│
├── Cargo.toml                  # Package manifest and dependency declarations
├── rust-toolchain.toml         # Pinned Rust toolchain with WASM target
├── Makefile                    # Common development commands
├── README.md                   # This file
└── .gitignore
```

---

## Core Contracts

### LiquidityPool

The main contract. It holds reserves for both tokens, executes swaps, and coordinates LP token issuance through cross-contract calls.

#### Storage

```rust
#[ink(storage)]
pub struct LiquidityPool {
    token_0: AccountId,               // First token contract address
    token_1: AccountId,               // Second token contract address
    reserve_0: u128,                  // Tracked reserve for token_0
    reserve_1: u128,                  // Tracked reserve for token_1
    block_timestamp_last: u64,        // Timestamp of last reserve update
    price_0_cumulative_last: u128,    // Cumulative price for TWAP oracle
    price_1_cumulative_last: u128,    // Cumulative price for TWAP oracle
    k_last: u128,                     // Last K value (reserved for protocol fee)
    lp_token: AccountId,              // LP token contract address
    locked: bool,                     // Reentrancy guard state
}
```

#### Messages

| Message | Description | Mutates State |
| --- | --- | --- |
| `add_liquidity` | Deposit token pair, receive LP tokens | Yes |
| `remove_liquidity` | Burn LP tokens, withdraw token pair | Yes |
| `swap` | Execute a token swap with invariant check | Yes |
| `sync` | Reconcile tracked reserves with actual balances | Yes |
| `get_reserves` | Read current reserves and last timestamp | No |
| `get_amount_out` | Preview the output for a given input amount | No |
| `get_amount_in` | Preview the input required for a desired output | No |

#### Add Liquidity Flow

1. Validate deadline has not passed.
2. If first deposit, accept desired amounts directly and burn `MINIMUM_LIQUIDITY` to the zero address.
3. Otherwise, calculate the optimal deposit amounts that preserve the current reserve ratio.
4. Transfer tokens from caller to the pool via `PSP22::transfer_from`.
5. Mint LP tokens: `min(amount_0 / reserve_0, amount_1 / reserve_1) * total_supply`.
6. Update reserves and TWAP accumulators.
7. Emit `Mint` and `Sync` events.

#### Swap Flow

1. Validate at least one output amount is non-zero and within available reserves.
2. Optimistically transfer output tokens to the recipient.
3. Read the new on-chain balances to derive the actual input amounts.
4. Verify the fee-adjusted K invariant: `(b0*1000 - in0*3) * (b1*1000 - in1*3) >= reserve_0 * reserve_1 * 1_000_000`.
5. Update reserves and TWAP accumulators.
6. Emit `Swap` and `Sync` events.

---

### LPToken (PSP22)

A standard PSP22 token representing a liquidity provider's proportional share of the pool reserves. The pool contract is the sole authorized minter and burner.

```rust
#[openbrush::contract]
pub mod lp_token {
    #[ink(storage)]
    pub struct LPToken {
        #[storage_field]
        psp22: PSP22Data,
        pool: AccountId,     // Authorized minter/burner
        name: String,
        symbol: String,
        decimals: u8,
    }

    impl PSP22 for LPToken {}
    impl PSP22Metadata for LPToken {}
    impl PSP22Mintable for LPToken {}   // Restricted to pool AccountId
    impl PSP22Burnable for LPToken {}   // Restricted to pool AccountId
}
```

---

### ReentrancyGuard

A mutex-style reentrancy protection pattern implemented as a Rust trait over the pool's `locked: bool` storage field.

```rust
pub trait ReentrancyGuard {
    fn is_locked(&self) -> bool;
    fn set_locked(&mut self, locked: bool);

    fn lock(&mut self) -> Result<(), Error> {
        if self.is_locked() {
            return Err(Error::ReentrancyDetected);
        }
        self.set_locked(true);
        Ok(())
    }

    fn unlock(&mut self) {
        self.set_locked(false);
    }
}
```

Every state-changing message acquires the lock at entry and releases it before returning, preventing any reentrant call from proceeding.

---

## Key Features

### Constant-Product Invariant

```
x * y = k
```

Where `x` and `y` are token reserves and `k` is the invariant. After every swap, `k` must not decrease (fees cause it to grow slightly):

```
(x + dx) * (y - dy) >= x * y
```

### Fee Structure

- Swap fee: **0.3%** (30 basis points).
- The fee multiplier applied to input amounts is `997 / 1000`.
- Fees remain inside the reserves; they accrue to LP token holders proportionally when they withdraw.

### Minimum Liquidity Lock

On the first deposit, `MINIMUM_LIQUIDITY` (1,000 wei) of LP tokens is permanently minted to the zero address. This prevents the pool from being fully drained, avoids division-by-zero in LP calculations, and sets a minimum floor on the pool's value.

### TWAP Oracle

Cumulative price accumulators track the time-weighted average price for both tokens:

```rust
price_0_cumulative += (reserve_1 / reserve_0) * time_elapsed
price_1_cumulative += (reserve_0 / reserve_1) * time_elapsed
```

An off-chain service or on-chain consumer reads these accumulators at two points in time and computes the TWAP over that window, which is resistant to single-block price manipulation.

---

## Math and Invariants

### Swap Output Formula

```
amount_out = (reserve_out * amount_in * 997) / (reserve_in * 1000 + amount_in * 997)
```

### Swap Input Formula

```
amount_in = (reserve_in * amount_out * 1000) / ((reserve_out - amount_out) * 997) + 1
```

The `+ 1` ensures the output will be met after rounding.

### LP Token Minting (subsequent deposits)

```
liquidity = min(
    (amount_0 * total_supply) / reserve_0,
    (amount_1 * total_supply) / reserve_1
)
```

### LP Token Minting (initial deposit)

```
liquidity = sqrt(amount_0 * amount_1) - MINIMUM_LIQUIDITY
```

### LP Token Burning

```
amount_0 = (liquidity * reserve_0) / total_supply
amount_1 = (liquidity * reserve_1) / total_supply
```

---

## Security

| Attack Vector | Mitigation |
| --- | --- |
| Reentrancy | `locked` boolean guard on every state-changing message; CEI pattern |
| Integer overflow | `checked_*` arithmetic throughout; errors propagate as `Error::Overflow` |
| First-deposit manipulation | `MINIMUM_LIQUIDITY` permanently burned to the zero address |
| Rounding exploit | Floor division on LP minting; ceiling division on input calculation |
| K invariant bypass | On-chain balance verification after every swap; fee-adjusted K check |
| Flash price manipulation | TWAP accumulators use pre-block reserve snapshots |
| Front-running | Slippage parameters (`amount_X_min`) enforced on all liquidity operations |
| Deadline expiry | `deadline: u64` parameter rejects stale transactions |
| Unauthorized mint/burn | LP token restricts mint and burn to the pool's `AccountId` |

See [docs/SECURITY.md](docs/SECURITY.md) for the full threat model and audit checklist.

---

## Build and Deploy

### Prerequisites

- Rust with the `wasm32-unknown-unknown` target
- `cargo-contract` CLI
- A local Substrate node for development (e.g., `substrate-contracts-node`)

### Install Tools

```bash
# Install the ink! contract build tool
cargo install cargo-contract --locked

# Add the WASM compilation target
rustup target add wasm32-unknown-unknown
```

### Build

```bash
# Standard debug build
cargo contract build

# Optimized release build (smaller WASM, suitable for deployment)
cargo contract build --release

# Output: target/ink/amm_liquidity_pool.contract
```

### Deploy to a Local Node

```bash
# Terminal 1 — start a local development node
substrate-contracts-node --dev --tmp

# Terminal 2 — upload and instantiate the contract
cargo contract upload --suri //Alice
cargo contract instantiate \
    --suri //Alice \
    --constructor new \
    --args <token_0_address> <token_1_address> <lp_token_address>
```

### Deploy to Testnet

```bash
export SURI="your secret phrase"
export TOKEN_0="<token_0_address>"
export TOKEN_1="<token_1_address>"
export LP_TOKEN="<lp_token_address>"

bash scripts/deploy.sh testnet
```

---

## Testing

### Unit Tests

```bash
# Run all unit tests
cargo test

# With printed output
cargo test -- --nocapture

# Run a specific module
cargo test math_tests
cargo test liquidity_pool_tests
```

### Integration Tests

Integration tests require a running `substrate-contracts-node`:

```bash
# End-to-end tests against a live node
cargo test --features e2e-tests
```

### Fuzz Tests

Property-based tests verify invariants across thousands of randomized inputs:

```bash
cargo test --features fuzzing
```

Properties tested:
- Output amount is always less than the output reserve.
- K invariant never decreases after a swap.
- `get_amount_in` / `get_amount_out` are consistent (roundtrip).
- Withdrawal amounts are always proportional and within reserves.
- `sqrt` is monotonically non-decreasing.

### Coverage

```bash
# Install coverage tool
cargo install cargo-tarpaulin

# Generate HTML report
cargo tarpaulin --out Html

# Open tarpaulin-report.html in a browser
```

---

## Events and Indexing

All state-changing operations emit structured events for off-chain indexing.

### Mint

Emitted when liquidity is added.

```rust
#[ink::event]
pub struct Mint {
    #[ink(topic)]
    pub sender: AccountId,
    pub amount_0: u128,
    pub amount_1: u128,
}
```

### Burn

Emitted when liquidity is removed.

```rust
#[ink::event]
pub struct Burn {
    #[ink(topic)]
    pub sender: AccountId,
    pub amount_0: u128,
    pub amount_1: u128,
    #[ink(topic)]
    pub to: AccountId,
}
```

### Swap

Emitted on every token swap.

```rust
#[ink::event]
pub struct Swap {
    #[ink(topic)]
    pub sender: AccountId,
    pub amount_0_in: u128,
    pub amount_1_in: u128,
    pub amount_0_out: u128,
    pub amount_1_out: u128,
    #[ink(topic)]
    pub to: AccountId,
}
```

### Sync

Emitted whenever reserves are updated.

```rust
#[ink::event]
pub struct Sync {
    pub reserve_0: u128,
    pub reserve_1: u128,
}
```

Events are indexed by a backend service to track historical swap volume, total value locked (TVL), and price history.

---

## API Reference

### Constructor

```rust
#[ink(constructor)]
pub fn new(token_0: AccountId, token_1: AccountId, lp_token: AccountId) -> Self
```

### Messages

```rust
// Add liquidity to the pool
#[ink(message)]
pub fn add_liquidity(
    &mut self,
    amount_0_desired: u128,
    amount_1_desired: u128,
    amount_0_min: u128,
    amount_1_min: u128,
    to: AccountId,
    deadline: u64,
) -> Result<u128, Error>

// Remove liquidity from the pool
#[ink(message)]
pub fn remove_liquidity(
    &mut self,
    liquidity: u128,
    amount_0_min: u128,
    amount_1_min: u128,
    to: AccountId,
    deadline: u64,
) -> Result<(u128, u128), Error>

// Execute a token swap
#[ink(message)]
pub fn swap(
    &mut self,
    amount_0_out: u128,
    amount_1_out: u128,
    to: AccountId,
) -> Result<(), Error>

// Synchronise tracked reserves with actual on-chain balances
#[ink(message)]
pub fn sync(&mut self) -> Result<(), Error>

// Read current reserves and last update timestamp
#[ink(message)]
pub fn get_reserves(&self) -> (u128, u128, u64)

// Preview output amount for a given input
#[ink(message)]
pub fn get_amount_out(
    &self,
    amount_in: u128,
    reserve_in: u128,
    reserve_out: u128,
) -> Result<u128, Error>

// Preview input amount required for a desired output
#[ink(message)]
pub fn get_amount_in(
    &self,
    amount_out: u128,
    reserve_in: u128,
    reserve_out: u128,
) -> Result<u128, Error>
```

### Error Types

```rust
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
```

---

## Contributing

1. Fork the repository.
2. Create a feature branch: `git checkout -b feature/your-feature`.
3. Commit your changes: `git commit -m 'Add your feature'`.
4. Push to the branch: `git push origin feature/your-feature`.
5. Open a pull request.

Requirements:
- All tests must pass (`cargo test`).
- Unit test coverage for all math logic must be maintained.
- Clippy linting must pass with no warnings: `cargo clippy -- -D warnings`.
- Security review is required for any change to state-mutating messages.

---

## Resources

- [ink! Documentation](https://use.ink/)
- [OpenBrush Contracts](https://github.com/Brushfam/openbrush-contracts)
- [Substrate Contracts Node](https://github.com/paritytech/substrate-contracts-node)
- [PSP22 Token Standard](https://github.com/w3f/PSPs/blob/master/PSPs/psp-22.md)
- [Aleph Zero Testnet](https://test.azero.dev/)

---

## License

MIT License — see [LICENSE](LICENSE) for details.
