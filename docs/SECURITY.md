# Security

This document describes the threat model, known attack vectors, and the mitigations applied in Nodus Protocol's AMM contracts.

---

## Threat Model

The contracts assume:

- The Soroban runtime and execution environment are trusted.
- Tokens are well-behaved SEP-41 implementations (no fee-on-transfer, no rebase).
- The deployer-configured `token_0`, `token_1`, and `lp_token` addresses are correct.
- Users interact with the pool through standard Soroban contract calls.

---

## Attack Vectors and Mitigations

### Reentrancy

**Risk:** A malicious token or recipient contract calls back into the pool during an active message, allowing double-spending or invariant manipulation.

**Mitigation:**
- A `locked: bool` field acts as a mutex. Every state-changing message calls `lock()` at entry and `unlock()` before returning.
- If `lock()` is called while `locked == true`, `Error::ReentrancyDetected` is returned immediately.
- All state writes follow the Checks-Effects-Interactions (CEI) pattern: balances are read and invariants verified *after* output transfers but *before* updating stored reserves.

### Integer Overflow and Underflow

**Risk:** Arithmetic on token amounts and reserves can overflow `i128` or underflow to zero.

**Mitigation:**
- All arithmetic uses `checked_*` operations (`checked_mul`, `checked_add`, `checked_sub`).
- Errors propagate as `Error::Overflow` or `Error::InsufficientLiquidity` rather than panicking or wrapping.
- The Soroban SDK handles overflow checks; all safety is explicit via `checked_*`.

### First-Deposit Attack (Donation Attack)

**Risk:** The first liquidity provider can manipulate the initial reserve ratio by directly transferring tokens to the pool before calling `add_liquidity`, forcing unfavorable LP token issuance for subsequent depositors.

**Mitigation:**
- `MINIMUM_LIQUIDITY` (1,000 wei) of LP tokens is permanently burned to the zero address on the first mint.
- This prevents the initial LP from holding 100% of shares and sets a minimum pool value floor.

### Rounding Exploits

**Risk:** Floor division in LP token minting allows repeated small deposits to drain the pool by extracting slightly more than deposited across many transactions.

**Mitigation:**
- LP token minting uses `min(amount_0 * total / reserve_0, amount_1 * total / reserve_1)`, which floors rather than rounds up, ensuring new LPs cannot receive more value than they deposit.
- `MINIMUM_LIQUIDITY` permanently locked prevents zero-division and ensures a minimum `total_supply` denominator.

### K Invariant Manipulation

**Risk:** A swap caller claims output tokens without providing a proportional input, breaking the constant-product formula.

**Mitigation:**
- After every swap, the contract reads actual on-chain balances (not caller-supplied values) and verifies:

  ```
  (balance_0 * 1000 - amount_0_in * 3) * (balance_1 * 1000 - amount_1_in * 3)
      >= reserve_0 * reserve_1 * 1_000_000
  ```

  This is the fee-adjusted K check. If it fails, the entire message reverts with `Error::KInvariantViolated`.

### Price Manipulation (Flash Attacks)

**Risk:** An attacker manipulates reserves within a single block to exploit price-sensitive logic in integrated contracts.

**Mitigation:**
- The TWAP accumulator updates based on reserves *at the start* of the block (not end-of-block balances), making single-block manipulation ineffective for oracle consumers.
- Slippage parameters (`amount_0_min`, `amount_1_min` in `add_liquidity` / `remove_liquidity`) allow callers to enforce price bounds.

### Transaction Deadline Expiry

**Risk:** A pending transaction is executed at a future time when market conditions have changed, resulting in a bad trade or deposit.

**Mitigation:**
- `add_liquidity` and `remove_liquidity` accept a `deadline: u64` (ledger timestamp). If `env.ledger().timestamp() > deadline`, the message returns `Error::Expired` immediately.

### Unauthorized LP Token Minting and Burning

**Risk:** Any account mints LP tokens for free or burns another user's position.

**Mitigation:**
- The LP token contract restricts `mint` and `burn` to a single authorized caller — the pool contract's `Address`, set at LP token deployment.
- These are enforced inside the LP token contract; the pool contract does not need to enforce this itself.

### Front-Running

**Risk:** A validator or MEV searcher observes a pending swap and inserts their own transaction ahead of it to extract value.

**Mitigation:**
- `amount_0_min` / `amount_1_min` slippage parameters in all liquidity operations enforce a minimum acceptable output.
- For swaps, callers compute `get_amount_out` off-chain and set tight slippage bounds before submitting.

### Re-org Safety

**Risk:** A chain reorganization replays events out of order, causing indexers to display stale or incorrect state.

**Mitigation:**
- All events (`Mint`, `Burn`, `Swap`, `Sync`) include sufficient context for indexers to detect re-orgs and reprocess from the affected block.
- The on-chain state is always authoritative; the indexer is a derived view.

---

## Known Limitations

- **No fee-on-transfer token support.** The pool reads balance deltas to derive input amounts; deflationary tokens would cause the K check to fail unexpectedly.
- **No rebase token support.** Elastic supply tokens (e.g., AMPL) change balances outside of transfers; `sync()` must be called manually after each rebase to avoid reserve drift.
- **Single pool per token pair.** This implementation is a single-pair pool. A factory pattern to deploy many pools is a separate concern.
- **No protocol fee switch.** The optional `fee_to` mechanism (common in Uniswap V2) is not implemented; all fees accrue to LPs.

---

## Audit Checklist

- [ ] All `checked_*` arithmetic paths verified for completeness
- [ ] Reentrancy guard covers every state-changing message
- [ ] K invariant check reviewed for edge cases (zero reserves, single-sided input)
- [ ] LP token access control reviewed against pool `Address`
- [ ] TWAP accumulator overflow (wrapping on purpose) documented and accepted
- [ ] Fuzz tests pass for all math functions
- [ ] Integration tests cover reentrancy rejection and K violation rejection