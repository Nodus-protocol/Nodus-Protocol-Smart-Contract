# Architecture

## Overview

Nodus Protocol implements a constant-product AMM using two cooperating Soroban smart contracts:

1. **LiquidityPool** — holds token reserves, executes swaps, mints and burns LP tokens.
2. **LpToken (SEP-41)** — a standard fungible token representing a liquidity provider's share of the pool.

These contracts communicate through cross-contract calls. The pool is the sole authorized caller of the LP token's mint and burn functions.

---

## Contract Relationship

```
          ┌──────────────────────────────────────┐
          │           LiquidityPool               │
          │                                       │
          │  reserve_0 (token_0 balance)          │
          │  reserve_1 (token_1 balance)          │
          │  price_X_cumulative (TWAP oracle)     │
          │                                       │
          │  add_liquidity()  ─────────────────── │──► mint LP tokens
          │  remove_liquidity() ───────────────── │──► burn LP tokens
          │  swap()                               │
          │  sync()                               │
          └──────────────────────────────────────┘
                           │
                           │ cross-contract calls
                           ▼
          ┌──────────────────────────────────────┐
          │           LpToken (SEP-41)            │
          │                                       │
          │  balance: Mapping<Address, i128>      │
          │  allowance: Mapping<(Address,          │
          │               Address), i128>         │
          │  total_supply: i128                   │
          │                                       │
          │  mint()   (pool only)                 │
          │  burn()   (pool only)                 │
          │  transfer()                           │
          │  transfer_from()                      │
          └──────────────────────────────────────┘
```

---

## Module Breakdown

| File | Purpose |
|---|---|
| `lib.rs` | Crate root: module declarations and the `#[contract]` entry point |
| `liquidity_pool.rs` | Pure functions: optimal amounts, liquidity calculations, K invariant check |
| `math.rs` | `get_amount_out`, `get_amount_in`, `sqrt`, fee constants |
| `storage.rs` | `DataKey` enum for contract storage keys |
| `events.rs` | `Mint`, `Burn`, `Swap`, `Sync` event definitions |
| `errors.rs` | `Error` enum covering all failure modes |
| `traits.rs` | `IAmmPool` trait definition for interface abstraction |

---

## Key Flows

### Add Liquidity

```
caller ──► add_liquidity(amount_0_desired, amount_1_desired, ..., deadline)
              │
              ├─ lock() — reentrancy guard
              ├─ check deadline
              ├─ if first deposit:
              │     liquidity = sqrt(amount_0 * amount_1) - MINIMUM_LIQUIDITY
              │     mint MINIMUM_LIQUIDITY to zero address (permanently locked)
              │  else:
              │     (amount_0, amount_1) = calculate_optimal_amounts(...)
              │     liquidity = min(amount_0/reserve_0, amount_1/reserve_1) * total_supply
              ├─ transfer_from(token_0, caller → pool, amount_0)
              ├─ transfer_from(token_1, caller → pool, amount_1)
              ├─ lp_token.mint(to, liquidity)
              ├─ update reserves + TWAP accumulators
              ├─ emit Mint + Sync
              └─ unlock()
```

### Swap

```
caller ──► swap(amount_0_out, amount_1_out, to)
              │
              ├─ lock()
              ├─ validate: at least one output > 0, outputs < reserves
              ├─ optimistically transfer output tokens to `to`
              ├─ read new balances
              ├─ derive amount_X_in from balance delta
              ├─ verify K invariant:
              │     (balance_0*1000 - amount_0_in*3) * (balance_1*1000 - amount_1_in*3)
              │         >= reserve_0 * reserve_1 * 1_000_000
              ├─ update reserves + TWAP
              ├─ emit Swap + Sync
              └─ unlock()
```

### Remove Liquidity

```
caller ──► remove_liquidity(liquidity, amount_0_min, amount_1_min, to, deadline)
              │
              ├─ lock()
              ├─ check deadline
              ├─ amount_0 = liquidity * reserve_0 / total_supply
              ├─ amount_1 = liquidity * reserve_1 / total_supply
              ├─ assert amount_0 >= amount_0_min, amount_1 >= amount_1_min
              ├─ lp_token.burn(from, liquidity)
              ├─ transfer token_0, token_1 to `to`
              ├─ update reserves + TWAP
              ├─ emit Burn + Sync
              └─ unlock()
```

---

## TWAP Oracle

Cumulative price accumulators are updated on every reserve mutation:

```
time_elapsed = ledger_timestamp - timestamp_last

price_0_cumulative += (reserve_1 / reserve_0) * time_elapsed
price_1_cumulative += (reserve_0 / reserve_1) * time_elapsed
```

An off-chain indexer or on-chain consumer can snapshot these values at two points in time and derive the time-weighted average price:

```
twap_0 = (price_0_cumulative[t1] - price_0_cumulative[t0]) / (t1 - t0)
```

Single-block manipulation is not profitable because the accumulator updates only after the block's state is committed.

---

## Fee Model

The swap fee is 0.3% (30 basis points), implemented implicitly:

- The output formula uses a 997/1000 multiplier on `amount_in`.
- Fees remain in the reserves; they are not extracted to a separate address.
- LP token holders capture fees proportionally when they burn their shares.
- The protocol fee mechanism (which would redirect a fraction of fees to a `fee_to` address) is **not implemented**. The `fee_to` address and setter methods exist as reserved placeholders, but no protocol fees are collected or accrued.

---

## Security Model

See [SECURITY.md](SECURITY.md) for the full threat model and mitigations.