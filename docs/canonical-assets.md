# Canonical Asset Set (issue #122 release artifact)

The v1 pool is pinned to exactly one pair:

| Side | Asset | Derived SAC symbol | Derived SAC name | Decimals | Role |
|------|-------|--------------------|------------------|----------|------|
| `token_0` (base) | Native Stellar lumen (XLM) | `native` | `native` | 7 | Always the base asset |
| `token_1` (quote) | Circle USD Coin (USDC), issuer `GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN` | `USDC` | `USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN` | 7 | Always the quote asset |

These values are the single source of truth the contract enforces at
[`initialize`](../contracts/pool/src/lib.rs) — see
[`registry.rs`](../contracts/pool/src/registry.rs), which is the
contract-side copy of this table. **Ordering is part of the policy**: XLM is
always `token_0` and USDC always `token_1`. A reversed, impostor, or unknown
pair is rejected at initialization before any state is written.

> Note on metadata: the *SAC's* on-chain `name`/`symbol` are what the
> contract checks. The native SAC reports `name = "native"`, `symbol =
> "native"`; the USDC SAC reports `name = "USDC:GA5Z…"` (full
> `CODE:ISSUER` asset string) and `symbol = "USDC"`. These were captured
> empirically from the real SACs in the sandbox and are pinned as constants
> in `registry.rs`.

## Pinned canonical SAC contract addresses (release artifact)

Per CAP-46-3 the SAC contract address is a deterministic function of the
asset definition, so it is **network-independent**. The addresses below are
what `registry::derive_canonical_address` computes for the reviewed
definitions above; they are pinned in `registry.rs` (and cross-checked by a
unit test that re-derives them) as the shared release artifact for Backend /
Core Engine / Frontend / Mobile to consume:

| Side | Asset definition | Derived SAC address (`registry.rs` constant) |
|------|------------------|---------------------------------------------|
| `token_0` | Native XLM | `CB56OQJZFJXSSKFK3MXJZ4TLJAJFWH6KXN6BAWHQSJDZPHZFVBJ353HU` (`XLM_SAC_ADDRESS`) |
| `token_1` | USDC / `GA5ZSEJY…KZVN` | `CCAGPNTODUR2Z3JHN26WFINU3GUB3PIXBVQJIFN54CKZ3XWTGDJSSACA` (`USDC_SAC_ADDRESS`) |

### Machine-readable artifact for consumers

The validated identities above are also published in
[`docs/canonical-assets.json`](./canonical-assets.json) — the
machine-readable registry Backend / Core Engine / Frontend / Mobile should
pin (asset XDR, SAC address, name/symbol/decimals, issuer, and issuer
clawback/freeze capabilities, plus the canary limits). The published
artifact is described and validated by its
[`canonical-assets.schema.json`](./canonical-assets.schema.json) schema. The contract's own
[`registry.rs`](../contracts/pool/src/registry.rs) is the source of truth,
and a regression test
(`published_release_artifact_matches_contract_constants`) fails if the JSON
ever drifts from the on-chain constants, so the artifact stays authoritative.


## How identity is pinned on-chain (the proof)

Soroban derives a Stellar Asset Contract's address deterministically from
its asset definition (CAP-46-3). The host exposes that derivation as
`get_asset_contract_id`, surfaced by the SDK as
`Env::deployer().with_stellar_asset(xdr).deployed_address()`.

At `initialize` the pool **derives the canonical XLM and USDC SAC addresses
on-chain** from the reviewed asset definitions (native XLM; USDC with
Circle's Stellar issuer — both defined as XDR in `registry.rs`) and
**requires the supplied token addresses to match exactly**. So the identity
check is:

1. **Canonical asset derivation — the proof.** `token_0` must equal the SAC
   address derived from the native-XLM asset definition and `token_1` the
   SAC address derived from the USDC asset definition. Any other address,
   whatever metadata it reports, is rejected (`Error::UnsupportedAsset`).
   This closes the same-symbol impostor, wrong-network deployment,
   incompatible-contract, and reversed-pair vectors at the pool boundary:
   the pool only ever talks to the exact SAC addresses derived from the
   reviewed assets.
2. **Interface + pristine-state canary.** Each side must answer
   `balance(pool)` and report `0` (`Error::NotTokenContract` for a
   non-contract, uninitialized, or reverting address; `UnsupportedAsset`
   if a balance is already parked at the pool).
3. **Metadata — defense-in-depth, never the proof.** `name`/`symbol`/
   `decimals` must match the reviewed policy (`Error::UnsupportedAsset` /
   `Error::WrongDecimals`). Symbol/name alone are never sufficient; they
   are checked on top of the derived-address identity.

## What is NOT protected on-chain — and how it is covered

A SAC that has been **upgraded or replaced by a malicious implementation
still living at the canonical address** passes the derivation check (the
address is the address). Metadata checks catch static tampering; behavioral
tampering — fee-on-transfer, silently dropped transfers, broken
authorization, wrong decimals at runtime — is caught by the **post-deploy
transfer/allowance compatibility canary** implemented in-contract as
[`NodusAmm::verify_token_compatibility`](../contracts/pool/src/lib.rs).

Soroban 26 exposes no host function for reading another contract's
WASM/code hash, so the contract cannot fingerprint the official SAC binary
itself. The combination of (a) derived-address pinning and (b) the funded
transfer/allowance canary is the strongest verification achievable on-chain
today; WASM-hash fingerprinting should be revisited when the host exposes
it (see the issue's follow-ups).

## `verify_token_compatibility` (post-deploy canary)

**Enforced as an activation gate.** `add_liquidity` refuses the *first*
deposit until the canary has passed (`Error::CanaryNotCompleted`): the pool
cannot become liquid against a canonical SAC whose approve/transfer path
does not behave, so a tampered/upgraded implementation at a pinned address
is caught before any liquidity exists. Once reserves exist the canary has
already passed, so this is an activation-only gate.

Admin-only (`fee_to_setter`), reentrancy-locked, limited to `1..=10` stroops
per side (strict canary limits). For **each** pool token it runs a full
round trip:

1. `caller` approves the pool for exactly `amount`; approval must succeed
   (catches approve that reverts or is not implemented).
2. Pool pulls `amount` in via `transfer_from`; the pool's balance must then
   be exactly `amount` (catches fee-on-transfer and silently dropped or
   partial transfers).
3. Pool pushes `amount` back via `transfer`; the pool's balance must be
   exactly `0` again (catches push-side corruption).

Any failure maps to `Error::TokenCompatibilityFailed`, the pool is left
unactivated (`canary_verified() == false`), and liquidity should not be
enabled until it passes. Deploy automation should call this right after
activation, before the pool is exposed to users.

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

## Release checklist

- [x] `registry.rs` asset definitions (native XLM; USDC issuer
      `GA5ZSEJY…KZVN`) reviewed and pinned; pinned addresses
      (`XLM_SAC_ADDRESS` / `USDC_SAC_ADDRESS`) confirmed by the derivation
      test.
- [x] `verify_token_compatibility` canary implemented and tested (strict
      1–10 stroop limits); `canary_verified()` is a hard prerequisite
      enforced by `add_liquidity` before the first deposit.
- [x] Validated identities published for Backend / Core Engine / Frontend /
      Mobile in [`docs/canonical-assets.json`](./canonical-assets.json).
- [ ] `initialize` derived-address checks pass against the reviewed
      definitions on the target network (deploy-time).
- [ ] `name`/`symbol`/`decimals` of the live SACs on the target network
      match the table above (deploy-time).
- [ ] `verify_token_compatibility` canary executed on the target network
      (deploy-time).
- [ ] Issuer clawback/freeze implications surfaced in product UI
      (product follow-up).
