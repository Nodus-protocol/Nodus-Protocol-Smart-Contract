# Nodus Protocol — AMM Liquidity Pool

[![CI](https://github.com/Nodus-protocol/Nodus-Protocol-Smart-Contract/actions/workflows/ci.yml/badge.svg)](https://github.com/Nodus-protocol/Nodus-Protocol-Smart-Contract/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-violet.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-stable-orange?logo=rust)](rust-toolchain.toml)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

Constant-product Automated Market Maker (AMM) smart contract written in **Rust** for **Stellar Soroban**.

---

## Overview

This is a Uniswap V2-style AMM on Stellar Soroban, split across multiple
contracts rather than one monolithic one. It holds reserves for two SEP-41
Stellar tokens, executes atomic swaps, and issues LP tokens representing
each provider's proportional share via a standalone LP token contract.

## Architecture

```
┌─────────────────────────────────────┐      ┌──────────────────────────┐
│            NodusAmm (pool)          │      │  nodus-protocol-lp-token │
│                                      │      │                          │
│  reserve_0 ──── reserve_1           │ mint/ │  Standalone SEP-41-      │
│       \              /              │ burn  │  compatible token.       │
│        k = x * y (invariant)        │──────►│  mint/burn are pool-     │
│                                      │       │  gated; transfer/       │
│  add_liquidity() → mint LP tokens   │       │  approve/allowance are  │
│  remove_liquidity() → burn LP tokens│       │  standard and open to    │
│  swap()                             │       │  any holder.             │
│  sync()  (drift correction)         │       └──────────────────────────┘
│                                      │
│  TWAP price accumulators            │
│  (price_0_cumulative_last, …)       │
└──────────────────────────────────────┘
```

The pool talks to its LP token contract via `contractimport!` (see
`contracts/pool/src/lib.rs`) rather than a regular Cargo dependency on the
`nodus-protocol-lp-token` crate — depending on the crate directly links
its own `#[contractimpl]`-generated WASM exports into the pool's binary
too (confirmed empirically: both crates export an `initialize` function,
which fails the link with a duplicate-symbol error). `contractimport!`
reads the LP token's *compiled* WASM instead, so **the LP token contract
must be built before the pool** — `make build`/`make test`/`make lint`
all handle this ordering; see [Build](#build) below if you're running
`cargo` directly.

A factory contract (deploying and tracking a pool + LP token pair per
token combination, since today's pool still only supports one hard-coded
pair per deployed instance) and a router contract (multi-hop swaps once
more than one pool exists) are planned as follow-up PRs.

---

## Repository Structure

This is a Cargo workspace; each Soroban contract is its own crate under
`contracts/`.

```
contracts/
  pool/
    src/
      lib.rs              Contract entry point — all public functions
      liquidity_pool.rs   Pool math: optimal amounts, K-invariant
      math.rs             AMM formulas: get_amount_out, get_amount_in, sqrt
      storage.rs          DataKey enum for all instance + persistent storage keys
      events.rs           Soroban event wrappers: Mint, Burn, Swap, Sync
      errors.rs           Stable #[contracterror] enum
      traits.rs           IAmmPool interface definition
    tests/
      unit_tests.rs       Pure math + liquidity-pool unit tests (no Soroban env)
      integration_tests.rs Soroban testenv contract interaction tests, including
                           a full add_liquidity/remove_liquidity round trip
                           through a real LP token contract instance
      fuzz_tests.rs       Property tests: k-invariant, sqrt floor, fee monotonicity
  lp-token/
    src/
      lib.rs              Contract entry point: mint (pool-gated), plus the
                           standard transfer/transfer_from/approve/allowance/
                           burn/burn_from/balance/decimals/name/symbol interface
      storage.rs           DataKey enum
      errors.rs             Stable #[contracterror] enum
      events.rs              Mint, Burn, Transfer, Approve event wrappers
    tests/
      integration_tests.rs  Soroban testenv contract interaction tests
```

---

## Contract Functions

### Pool lifecycle

| Function | Auth | Description |
|----------|------|-------------|
| `initialize(token_0, token_1, fee_to_setter, lp_token)` | — | One-time setup. `lp_token` must already be a deployed, uninitialized `nodus-protocol-lp-token` instance — this contract never deploys or initializes it itself; that's the factory's job (planned). The token pair is pinned: `token_0` must be the canonical XLM Stellar Asset Contract and `token_1` the canonical USDC Stellar Asset Contract, in that order. Each side's address is **derived on-chain** from the reviewed asset definition and must match exactly, and must then pass SEP-41 interface, canonical `name`/`symbol`/`decimals`, and zero-balance checks before the pool becomes active; reversed, unknown, impostor, wrong-decimals, or non-contract pairs are rejected. See [Canonical assets](#canonical-assets) and `docs/canonical-assets.md`. |
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

### LP token

| Function | Description |
|----------|-------------|
| `lp_token()` | Returns the address of this pool's LP token contract. Balance, transfer, approve, and supply queries all live there now — interact with it directly rather than through the pool; see [LP Token Contract](#lp-token-contract) below. |

### View

| Function | Description |
|----------|-------------|
| `get_reserves()` | Returns `(reserve_0, reserve_1, timestamp_last)`. |
| `get_price_cumulative()` | Returns `(price_0_cumulative_last, price_1_cumulative_last)` for TWAP. |
| `token_0()` / `token_1()` | Return the configured token contract addresses. |

---

## LP Token Contract

`nodus-protocol-lp-token` is a standalone contract, one instance per pool.
`mint` is pool-gated (see [Pool lifecycle](#pool-lifecycle)); everything
else is the standard SEP-41 token interface (`soroban_sdk::token::Client`
can call it like any other token), open to any holder.

| Function | Auth | Description |
|----------|------|-------------|
| `initialize(pool, name, symbol, decimals)` | — | One-time setup. `pool` becomes the only address `mint` will ever accept. |
| `pool()` | — | Returns the authorized pool address. |
| `mint(caller, to, amount)` | `caller` (must be `pool`) | Mints new LP tokens. Not part of SEP-41 — minting is issuer-specific by design in that standard. |
| `balance(id)` | — | Return `id`'s LP token balance. |
| `total_supply()` | — | Return total LP tokens in circulation. |
| `transfer(from, to, amount)` | `from` | Standard transfer. `to` is a `MuxedAddress` per SEP-41, so a payment can carry a muxed id for the recipient's own bookkeeping. |
| `approve(from, spender, amount, expiration_ledger)` | `from` | Approve `spender` to move up to `amount`, expiring at `expiration_ledger`. `amount: 0` revokes regardless of `expiration_ledger`. |
| `allowance(from, spender)` | — | Return the remaining approved amount. |
| `transfer_from(spender, from, to, amount)` | `spender` | Transfer using an existing allowance. |
| `burn(from, amount)` | `from` | Burns `from`'s own tokens. When the pool calls this during `remove_liquidity`, `from`'s authorization for that top-level call covers this nested one too. A holder can also call it directly, bypassing the pool — that forfeits their claim on the underlying reserves with no payout, which only benefits every other LP holder proportionally. Unusual, not unsafe. |
| `burn_from(spender, from, amount)` | `spender` | Burns using an existing allowance. |
| `name()` / `symbol()` / `decimals()` | — | Standard metadata. |

---

## Build

```bash
# Install Stellar CLI
cargo install --locked stellar-cli --features opt

# Build (produces optimised WASM for every contract)
make build

# Build output
target/wasm32v1-none/release/nodus_protocol_amm.wasm
target/wasm32v1-none/release/nodus_protocol_lp_token.wasm

# Run tests
make test

# Lint
make lint
```

`make build`/`make test`/`make lint` all build the LP token contract's
WASM before touching the pool crate — required because the pool imports
it via `contractimport!` at compile time (see [Architecture](#architecture)).
If you're running `cargo` directly instead of through `make`, build
`nodus-protocol-lp-token` first as its own step:

```bash
cargo build --release --target wasm32v1-none -p nodus-protocol-lp-token
cargo build --release --target wasm32v1-none --workspace  # or test/clippy/fmt
```

A single `cargo build --workspace` from a clean `target/` **will not**
reliably do this for you — Cargo has no dependency-graph edge between the
two crates (that's the point of `contractimport!` over a regular
dependency), so it's free to compile them in parallel and sometimes does,
racing the pool's build against an LP token WASM that doesn't exist yet.

---

## Deploy

```bash
# Testnet
STELLAR_SECRET_KEY=S... TOKEN_0=C... TOKEN_1=C... FEE_TO_SETTER=G... make deploy-testnet

# Mainnet
STELLAR_SECRET_KEY=S... TOKEN_0=C... TOKEN_1=C... FEE_TO_SETTER=G... make deploy-mainnet
```

The deploy script uploads and deploys both contracts, initializes the LP
token first (it needs to know its pool's address before the pool can be
initialized with it), then initializes the pool. `LP_TOKEN_NAME` /
`LP_TOKEN_SYMBOL` / `LP_TOKEN_DECIMALS` are optional overrides.

This is manual, one-pair-at-a-time tooling. The planned factory contract
will do this deployment + wiring on-chain, for any token pair, without a
human running a script per pool.

---

## Canonical assets

The v1 pool is pinned to one reviewed pair: XLM (native Stellar) as
`token_0` and Circle's USDC on Stellar as `token_1`, both with 7 decimals
and canonical SEP-41 metadata defined in `contracts/pool/src/registry.rs`.
`initialize` derives the canonical XLM/USDC Stellar Asset Contract
addresses **on-chain** from the reviewed asset definitions
(`registry::derive_canonical_address`, the host's `get_asset_contract_id`)
and requires the supplied token addresses to match exactly — that derived
address is the identity proof, not metadata. On top of it each side must
speak SEP-41, report the canonical `name`/`symbol`/`decimals`, and hold a
zero balance at the pool. Reversed/unknown/impostor/wrong-decimals/
non-contract pairs are rejected before any state is written (issue #122).
Activation emits a registry event (`v1_reg`) exposing the canonical
identities and pinned contract addresses for off-chain consumers.

After activation, deploy automation should call the admin-only
`verify_token_compatibility` entrypoint — a strict, 1–10 stroop
transfer/allowance round trip against both tokens (approve → pull → exact
balance → push back → zero balance) that catches a canonical SAC upgraded
or replaced with a non-conforming implementation (fee-on-transfer, dropped
transfers, broken authorization) before liquidity is enabled. See
[`docs/canonical-assets.md`](docs/canonical-assets.md) for the full
verification model, the issuer clawback/freeze implications of holding
USDC, and the canary procedure.

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
