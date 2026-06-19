# Nodus Protocol — AMM Liquidity Pool

[![CI](https://github.com/Nodus-protocol/Nodus-Protocol-Smart-Contract/actions/workflows/ci.yml/badge.svg)](https://github.com/Nodus-protocol/Nodus-Protocol-Smart-Contract/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-violet.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-stable-orange?logo=rust)](rust-toolchain.toml)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

Constant-product Automated Market Maker (AMM) smart contract written in **Rust** for **Stellar Soroban**.

---

## Overview

This contract implements a Uniswap V2-style AMM on Stellar Soroban. It holds reserves for two SEP-41 Stellar tokens, executes atomic swaps, and issues LP tokens representing each provider's proportional share.

## Architecture

```
┌─────────────────────────────────────┐
│            NodusAmm                 │
│                                     │
│  reserve_0 ──── reserve_1           │
│       \              /              │
│        k = x * y (invariant)        │
│                                     │
│  add_liquidity() → mint LP tokens   │
│  remove_liquidity() → burn LP tokens│
│  swap()                             │
│  sync()  (drift correction)         │
│                                     │
│  TWAP price accumulators            │
│  (price_0_cumulative_last, …)       │
└─────────────────────────────────────┘
```

LP tokens are tracked internally in the pool's persistent storage — no separate token contract is required.

---

## Repository Structure

```
src/
  lib.rs              Contract entry point — all public functions
  liquidity_pool.rs   Pool math: optimal amounts, K-invariant, LP mint/burn
  lp_token.rs         Internal LP ledger: mint, burn, transfer, approve, allowance
  math.rs             AMM formulas: get_amount_out, get_amount_in, sqrt
  storage.rs          DataKey enum for all instance + persistent storage keys
  events.rs           Soroban event wrappers: Mint, Burn, Swap, Sync
  errors.rs           Stable #[contracterror] enum
  traits.rs           IAmmPool interface definition

tests/
  unit_tests.rs       Pure math + liquidity-pool unit tests (no Soroban env)
  integration_tests.rs Soroban testenv contract interaction tests
  fuzz_tests.rs       Property tests: k-invariant, sqrt floor, fee monotonicity
```

---

## Contract Functions

### Pool lifecycle

| Function | Auth | Description |
|----------|------|-------------|
| `initialize(token_0, token_1)` | — | One-time setup. Stores token addresses. |
| `sync()` | — | Reconcile reserves with actual contract token balances. |

### Liquidity

| Function | Auth | Description |
|----------|------|-------------|
| `add_liquidity(from, to, amount_0_desired, amount_1_desired, amount_0_min, amount_1_min, deadline)` | `from` | Deposit tokens, receive LP tokens. |
| `remove_liquidity(from, to, liquidity, amount_0_min, amount_1_min, deadline)` | `from` | Burn LP tokens, receive underlying tokens. |

### Swaps

| Function | Auth | Description |
|----------|------|-------------|
| `swap(to, amount_0_out, amount_1_out)` | — | Low-level swap. Caller must transfer tokens in before calling; K-invariant enforced post-swap. |
| `get_amount_out(amount_in, reserve_in, reserve_out)` | — | Quote output for a given input (0.3% fee). |
| `get_amount_in(amount_out, reserve_in, reserve_out)` | — | Quote input required to receive a given output. |

### LP token interface

| Function | Auth | Description |
|----------|------|-------------|
| `lp_balance_of(owner)` | — | Return LP token balance. |
| `lp_total_supply()` | — | Return total LP tokens in circulation. |
| `transfer_lp(from, to, amount)` | `from` | Transfer LP tokens directly. |
| `approve_lp(owner, spender, amount)` | `owner` | Approve `spender` to transfer up to `amount` LP tokens. |
| `lp_allowance(owner, spender)` | — | Return remaining approved LP amount. |
| `transfer_lp_from(spender, from, to, amount)` | `spender` | Transfer LP tokens using an existing allowance. |

### View

| Function | Description |
|----------|-------------|
| `get_reserves()` | Returns `(reserve_0, reserve_1, timestamp_last)`. |
| `get_price_cumulative()` | Returns `(price_0_cumulative_last, price_1_cumulative_last)` for TWAP. |
| `token_0()` / `token_1()` | Return the configured token contract addresses. |

---

## Build

```bash
# Install Stellar CLI
cargo install --locked stellar-cli --features opt

# Build (produces optimised WASM)
make build
# or: stellar contract build

# Build output used by the deploy scripts
target/wasm32-unknown-unknown/release/nodus_amm.wasm

# Run tests
make test

# Lint
make lint
```

---

## Deploy

```bash
# Testnet
STELLAR_SECRET_KEY=S... TOKEN_0=C... TOKEN_1=C... make deploy-testnet

# Mainnet
STELLAR_SECRET_KEY=S... TOKEN_0=C... TOKEN_1=C... make deploy-mainnet
```

The deploy script uploads the WASM, deploys a new contract instance, and calls `initialize`.

The crate name is `nodus-amm`, so the generated WASM artifact uses the
underscore form `nodus_amm.wasm`. This differs from the repository name
(`Nodus-Protocol-Smart-Contract`) by design. Keep deploy scripts and manual
commands pointed at `nodus_amm.wasm` unless the crate name is intentionally
changed.

---

## Math

The AMM uses the constant-product formula `k = x * y` with a **0.3% fee** (997/1000):

```
amount_out = (amount_in * 997 * reserve_out) / (reserve_in * 1000 + amount_in * 997)
```

The minimum liquidity constant (`MINIMUM_LIQUIDITY = 1000`) is permanently locked in the dead address on first deposit to prevent price manipulation attacks on empty pools.

TWAP price accumulators use Q32.32 fixed-point: `price_cumulative += (reserve_1 << 32) / reserve_0 * time_elapsed`.

---

## License

MIT
