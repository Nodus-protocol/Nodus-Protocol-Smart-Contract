# Canonical Asset Set (issue #122 release artifact)

The v1 pool is pinned to exactly one pair:

| Side | Asset | SAC symbol | SAC name | Decimals | Role |
|------|-------|-----------|----------|----------|------|
| `token_0` (base) | Native Stellar lumen (XLM) | `XLM` | `Stellar` | 7 | Always the base asset |
| `token_1` (quote) | Circle USD Coin (USDC) | `USDC` | `USD Coin` | 7 | Always the quote asset |

These values are the single source of truth the contract enforces at
[`initialize`](../contracts/pool/src/lib.rs) — see
[`registry.rs`](../contracts/pool/src/registry.rs), which is the
contract-side copy of this table. **Ordering is part of the policy**: XLM is
always `token_0` and USDC always `token_1`. A reversed or unknown pair is
rejected at initialization before any state is written.

## What the contract verifies on-chain

For each side, `initialize` requires the address to be a live, SEP-41
contract that:

1. answers `balance(pool)` and reports `0` (interface + pristine-state
   canary),
2. reports exactly the canonical `name` and `symbol` above,
3. reports exactly the canonical `decimals` above.

Failures map to `Error::NotTokenContract` (non-contract / reverting),
`Error::UnsupportedAsset` (wrong name/symbol — impostor, reversed, or
unknown asset), and `Error::WrongDecimals`.

## What the contract does NOT verify on-chain — and how it is covered

Soroban 26 exposes no host function for reading another contract's
WASM/code hash, so the contract cannot fingerprint the official Stellar
Asset Contract binary itself. Symbol/name/decimals are therefore
**necessary but never sufficient**: they are combined with **deploy-side
pinning of the reviewed canonical SAC contract addresses**. A fake contract
can copy the metadata; it cannot impersonate the reviewed address when the
deployment pipeline pins and verifies it.

### Required mainnet action (before any mainnet pool is deployed)

1. Confirm the XLM and USDC SAC contract addresses from reviewed Stellar
   asset definitions (e.g. the Stellar expert/lab asset pages and the
   official SAC deployment lists), per network.
2. Pin those addresses as `TOKEN_0` / `TOKEN_1` in the deploy pipeline and
   publish them for **Backend, Core Engine, Frontend, and Mobile**
   consumption so every consumer refers to the same identities.
3. Cross-check the on-chain `name`/`symbol`/`decimals` of the pinned
   addresses against the table above as part of the deploy checklist.

> The contract intentionally does **not** hard-code network-specific SAC
> addresses: testnet and mainnet SACs differ, and pinning an address that
> has not been reviewed would turn a placeholder into a security claim. The
> address pin lives with the deploy/release tooling, where it can be
> reviewed, signed off, and rotated.

## Issuer authorization / clawback / freeze implications

- **XLM (native)** has no issuer, no clawback, and no freeze/authorization
  flag. The pool cannot be clawed back from; balances are unconditional.
- **USDC on Stellar** is an issued asset (issuer: Circle). Like all issued
  Stellar assets it can carry authorization, clawback, and freeze
  capabilities, and the asset issuer has authority over them. Concretely,
  for users of this pool:
  - Circle (or any future admin) can `clawback` USDC held in the pool,
    reducing reserves and shifting LP value — a protocol-level risk that
    is inherent to holding USDC, not specific to this contract.
  - If Circle sets the pool's trustline/authorization flags, USDC in the
    pool could in principle be frozen. The pool cannot prevent this; it
    only holds whatever the asset contract permits it to hold.
  - LP holders should treat USDC-side exposure as subject to the issuer's
    powers under the asset's issuance settings, exactly as they would for
    any USDC balance on Stellar.
- These implications should be surfaced in the product UI (e.g. an asset
  description/risk note on the pool page) as part of the coordination work
  in the issue.

## Post-deploy compatibility canary (transfer / allowance)

`initialize` itself cannot perform a funded transfer/allowance round-trip —
it runs before anyone has deposited, and the pool holds no balance to move.
The strict, zero-cost canary it does run is the `balance(pool) == 0` +
metadata/decimals check above, which proves the addresses are live SEP-41
contracts and reserves start pristine.

After deployment, the deploy pipeline should additionally run a **canary
transfer/allowance check with strict canary limits** against each pinned
SAC before liquidity is enabled:

1. From a freshly funded operator account, `approve` a tiny canary amount
   (e.g. 1 stroop of the asset) to the pool.
2. `transfer_from` the canary amount into the pool, then back out.
3. Verify both calls succeed and the operator balance is restored.
4. Reject the deployment (keep the pool unactivated) if either call fails,
   reverts, or moves a different amount than requested.

This is intentionally not automated inside the pool contract: it needs a
funded actor and would add a fee-bearing call to activation. A
`scripts/verify-pool.sh` implementing this canary is a good follow-up.

## Release checklist

- [ ] Reviewed XLM/USDC SAC addresses pinned in deploy tooling (mainnet).
- [ ] `name`/`symbol`/`decimals` of pinned SACs match the table above.
- [ ] Validated identities published for Backend / Core Engine / Frontend /
      Mobile.
- [ ] Post-deploy transfer/allowance canary executed with strict canary
      limits before enabling liquidity.
- [ ] Issuer clawback/freeze implications surfaced in product UI.
