# Nodus Protocol — AMM Liquidity Pool

Constant-product Automated Market Maker (AMM) smart contract written in **Rust** for **Stellar Soroban**.

---

## Overview

This contract implements a Uniswap V2-style AMM on Stellar Soroban. It holds reserves for two Stellar tokens, executes atomic swaps, and issues LP tokens representing each provider's proportional share.

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

## Repository Structure

```
src/
  lib.rs              Contract entry: initialize, add_liquidity, remove_liquidity, swap, sync
  liquidity_pool.rs   Pool math: optimal amounts, K invariant, LP minting/burning
  lp_token.rs         Internal LP balance helpers (mint, burn, transfer)
  math.rs             AMM formulas: get_amount_out, get_amount_in, sqrt
  events.rs           Event structs (Mint, Burn, Swap, Sync) + publish helpers
  errors.rs           Error enum
  storage.rs          DataKey enum for typed Soroban storage
  traits.rs           IAmmPool pure-Rust interface
  reentrancy_guard.rs ReentrancyGuard trait

tests/
  unit_tests.rs       Math and contract-level unit tests
  integration_tests.rs Integration tests (Soroban testutils)
  fuzz_tests.rs       Property-based invariant tests

scripts/
  build.sh            Build WASM
  test.sh             Run full test suite
  deploy.sh           Deploy to Stellar testnet or mainnet

.github/workflows/ci.yml  CI: build, test, lint on every PR
```

## Key Features

| Feature | Detail |
|---|---|
| Constant-product invariant | `x * y = k`; fee-adjusted check after every swap |
| Swap fee | 0.3% (997/1000 multiplier on input) |
| Minimum liquidity lock | `MINIMUM_LIQUIDITY = 1000` permanently minted to the zero address on first deposit |
| Slippage protection | `amount_0_min` / `amount_1_min` on all liquidity operations |
| Deadline | `deadline: u64` (ledger timestamp) rejects stale transactions |
| TWAP oracle | Q32.32 price accumulators updated on every reserve change |
| Reentrancy guard | `Locked` flag in instance storage; CEI pattern throughout |
| TTL management | Instance and persistent storage bumped on every call |

## Math

### Swap output
```
amount_out = (reserve_out × amount_in × 997) / (reserve_in × 1000 + amount_in × 997)
```

### Swap input (exact output)
```
amount_in = (reserve_in × amount_out × 1000) / ((reserve_out − amount_out) × 997) + 1
```

### LP minting (first deposit)
```
liquidity = sqrt(amount_0 × amount_1) − MINIMUM_LIQUIDITY
```

### LP minting (subsequent deposits)
```
liquidity = min(amount_0 × total_supply / reserve_0, amount_1 × total_supply / reserve_1)
```

## Build & Deploy

### Prerequisites

```bash
rustup target add wasm32-unknown-unknown
# optional: install Stellar CLI
cargo install stellar-cli
```

### Build

```bash
make build
# or directly:
cargo build --target wasm32-unknown-unknown --release
```

### Test

```bash
make test
# or:
cargo test --features testutils
```

### Deploy to Stellar testnet

```bash
export STELLAR_SECRET_KEY="SXXX..."
export TOKEN_0="CXXX..."
export TOKEN_1="CYYY..."
make deploy-testnet
```

## Contract API

### `initialize(token_0, token_1)`
Sets up the pool. Can only be called once.

### `add_liquidity(from, to, amount_0_desired, amount_1_desired, amount_0_min, amount_1_min, deadline) → i128`
- `from` — address that provides tokens (must pre-approve pool as spender)
- `to` — address that receives LP tokens
- Returns LP tokens minted

### `remove_liquidity(from, to, liquidity, amount_0_min, amount_1_min, deadline) → (i128, i128)`
- `from` — address that burns LP tokens
- `to` — address that receives underlying tokens

### `swap(to, amount_0_out, amount_1_out)`
Optimistic transfer; verifies K invariant after receiving input tokens.

### `sync()`
Reconciles tracked reserves with actual on-chain balances.

### View functions
- `get_reserves() → (i128, i128, u64)` — reserve_0, reserve_1, timestamp_last
- `get_price_cumulative() → (u128, u128)` — TWAP accumulators
- `get_amount_out(amount_in, reserve_in, reserve_out) → i128`
- `get_amount_in(amount_out, reserve_in, reserve_out) → i128`
- `lp_balance_of(owner) → i128`
- `lp_total_supply() → i128`
- `token_0() → Address`, `token_1() → Address`

## Security

| Threat | Mitigation |
|---|---|
| Reentrancy | `Locked` storage flag; CEI pattern |
| Integer overflow | `checked_*` throughout; `Error::Overflow` on failure |
| First-deposit manipulation | `MINIMUM_LIQUIDITY` permanently burned to address(0) |
| K invariant bypass | Fee-adjusted K check after every swap |
| Flash price manipulation | TWAP accumulators use pre-call reserve snapshots |
| Front-running | `amount_X_min` slippage bounds on all liquidity ops |
| Stale transactions | `deadline` parameter |
| State expiry | TTL bumped on every state-changing call |

## License

MIT
