//! Canonical asset registry for the v1 pool.
//!
//! The pool is deliberately pinned to one reviewed pair: native Stellar
//! (XLM) as the base asset (token_0) and Circle's USD Coin (USDC) on Stellar
//! as the quote asset (token_1). This module is the single source of truth
//! for the accepted mainnet asset set and doubles as the contract-side
//! release artifact for it:
//!
//! * the reviewed **asset definitions** (native XLM; USDC with Circle's
//!   Stellar issuer) from which the canonical SAC contract addresses are
//!   **derived on-chain** at initialization;
//! * the exact SEP-41 metadata each side's SAC must report.
//!
//! # How identity is pinned on-chain
//!
//! Soroban derives a Stellar Asset Contract's address deterministically from
//! the asset definition (CAP-46-3). The host exposes that derivation as
//! `get_asset_contract_id`, which the SDK surfaces via
//! `Env::deployer().with_stellar_asset(xdr).deployed_address()`. At
//! initialization the pool derives the canonical XLM and USDC SAC addresses
//! from the reviewed definitions below and **requires the supplied token
//! addresses to match exactly**. Symbol/name/decimals are verified as
//! defense-in-depth but are never the identity proof — a contract reporting
//! the right metadata from the wrong address is rejected, and so is any
//! contract that is not at the derived canonical address.
//!
//! This closes the "same-symbol impostor" / "wrong-network deployment" /
//! "incompatible contract" vectors at the pool boundary itself: the pool
//! only ever talks to the exact SAC addresses derived from the reviewed
//! assets. What this does *not* protect against — and what the post-deploy
//! compatibility canary (`NodusAmm::verify_token_compatibility`) is for — is
//! a canonical SAC that has been upgraded or replaced with a malicious
//! implementation that still lives at the canonical address: metadata checks
//! catch static tampering, and the funded transfer/allowance canary catches
//! behavioral tampering before liquidity is enabled.

#[cfg(test)]
extern crate std;

use crate::errors::Error;
use soroban_sdk::{token::Client as TokenClient, Address, Bytes, Env, String};

// ── Canonical asset definitions (reviewed; see docs/canonical-assets.md) ──

/// The native XLM asset serialized as XDR (`AssetType::Native`, u32 BE = 0).
/// Deriving the SAC address from this yields the canonical native-asset
/// contract, whose on-chain metadata is `name = "native"`, `symbol =
/// "native"`, `decimals = 7`.
pub const XLM_ASSET_XDR: [u8; 4] = [0, 0, 0, 0];

/// Circle's USDC issuer on Stellar, as a strkey G-address. This is the
/// reviewed issuer from the asset definition; the SAC address is derived
/// from it on-chain, so the pool can only ever reference the exact USDC
/// SAC.
pub const USDC_ISSUER_G: &str = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";

/// The ed25519 public key of [`USDC_ISSUER_G`] (kept alongside the strkey
/// form so both are reviewable; a unit test asserts they agree).
pub const USDC_ISSUER: [u8; 32] = [
    0x3b, 0x99, 0x11, 0x38, 0x0e, 0xfe, 0x98, 0x8b, 0xa0, 0xa8, 0x90, 0x0e, 0xb1, 0xcf, 0xe4, 0x4f,
    0x36, 0x6f, 0x7d, 0xbe, 0x94, 0x6b, 0xed, 0x07, 0x72, 0x40, 0xf7, 0xf6, 0x24, 0xdf, 0x15, 0xc5,
];

/// The USDC (credit alphanum4) asset serialized as XDR:
/// `[AssetType::CreditAlphanum4 u32 BE] [assetCode 4] [PublicKeyType::Ed25519
/// u32 BE] [issuer 32]` — 44 bytes. Deriving the SAC address from this
/// yields the canonical USDC SAC, whose on-chain metadata is `name =
/// "USDC:GA5Z…"`, `symbol = "USDC"`, `decimals = 7`.
pub const USDC_ASSET_XDR: [u8; 44] = {
    let mut b = [0u8; 44];
    // AssetType::CreditAlphanum4 = 1, serialized as u32 big-endian.
    b[3] = 1;
    // AssetCode4: "USDC".
    b[4] = b'U';
    b[5] = b'S';
    b[6] = b'D';
    b[7] = b'C';
    // AccountID: PublicKeyType::Ed25519 = 0, serialized as u32 big-endian
    // (already zero), followed by the 32-byte issuer key.
    let mut i = 0;
    while i < 32 {
        b[12 + i] = USDC_ISSUER[i];
        i += 1;
    }
    b
};

/// Reviewed canonical metadata the XLM SAC must report (the native SAC's
/// `name()`/`symbol()` are both the literal `"native"`).
pub const XLM_NAME: &str = "native";
pub const XLM_SYMBOL: &str = "native";
pub const XLM_DECIMALS: u32 = 7;

/// Reviewed canonical metadata the USDC SAC must report (the SAC's `name()`
/// is the full `CODE:ISSUER` asset string).
pub const USDC_NAME: &str = "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
pub const USDC_SYMBOL: &str = "USDC";
pub const USDC_DECIMALS: u32 = 7;

/// Derives the canonical Stellar Asset Contract address for a serialized
/// asset (the host's `get_asset_contract_id`). Pure computation — no auth,
/// no deployment.
pub fn derive_canonical_address(env: &Env, asset_xdr: &[u8]) -> Address {
    env.deployer()
        .with_stellar_asset(Bytes::from_slice(env, asset_xdr))
        .deployed_address()
}

/// Unwraps the double-layer `Result` returned by a `try_*` token client
/// call, mapping any failure — callee reverted, non-contract address,
/// conversion failure — to [`Error::NotTokenContract`]. The client's `try_`
/// variants return `Result<Result<T, _>, Result<_, _>>`, where the outer
/// layer is the invocation result and the inner layer the value conversion,
/// so this collapses both.
pub fn unwrap_token_call<T, CE, IE>(res: Result<Result<T, CE>, IE>) -> Result<T, Error> {
    match res {
        Ok(Ok(value)) => Ok(value),
        _ => Err(Error::NotTokenContract),
    }
}

/// Verifies that `token` is the canonical SAC for the given reviewed asset
/// definition and that it behaves like a live, SEP-41-compatible contract.
///
/// 1. **Identity (the proof):** `token` must equal the SAC address derived
///    on-chain from `asset_xdr` — any other address, whatever its metadata,
///    is rejected with [`Error::UnsupportedAsset`].
/// 2. **Interface + pristine-state canary:** `balance(pool)` must succeed
///    and report 0 (a non-contract, uninitialized, or reverting address
///    yields [`Error::NotTokenContract`]).
/// 3. **Metadata (defense-in-depth):** `name`/`symbol`/`decimals` must match
///    the reviewed canonical policy ([`Error::UnsupportedAsset`] /
///    [`Error::WrongDecimals`]).
pub fn verify_canonical_token(
    env: &Env,
    token: &Address,
    asset_xdr: &[u8],
    expected_name: &str,
    expected_symbol: &str,
    expected_decimals: u32,
) -> Result<(), Error> {
    // 1. Canonical asset derivation — the identity proof.
    let canonical = derive_canonical_address(env, asset_xdr);
    if *token != canonical {
        return Err(Error::UnsupportedAsset);
    }

    let client = TokenClient::new(env, token);
    let pool = env.current_contract_address();

    // 2. Interface canary: a real SAC answers `balance(pool)` and reports 0
    //    for an address that never held the asset; anything else (not a
    //    contract, uninitialized, reverting) surfaces as Err.
    let balance = unwrap_token_call(client.try_balance(&pool))?;
    if balance != 0 {
        // The pool must start from a pristine state; any balance parked at
        // this address before activation means it is not the pristine
        // canonical asset contract.
        return Err(Error::UnsupportedAsset);
    }

    // 3. Metadata defense-in-depth.
    let name = unwrap_token_call(client.try_name())?;
    if name != String::from_str(env, expected_name) {
        return Err(Error::UnsupportedAsset);
    }

    let symbol = unwrap_token_call(client.try_symbol())?;
    if symbol != String::from_str(env, expected_symbol) {
        return Err(Error::UnsupportedAsset);
    }

    let decimals = unwrap_token_call(client.try_decimals())?;
    if decimals != expected_decimals {
        return Err(Error::WrongDecimals);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::xdr::ToXdr;

    #[test]
    fn usdc_issuer_strkey_matches_raw_pubkey() {
        let env = Env::default();
        let issuer = Address::from_str(&env, USDC_ISSUER_G);
        // `Address::to_xdr` serializes the wrapping ScVal (ScValType::Address)
        // followed by the ScAddress: the ed25519 public key is the last 32
        // bytes of the 44-byte buffer.
        let xdr = issuer.to_xdr(&env);
        let n = xdr.len();
        assert_eq!(n, 44, "ScVal-wrapped ScAddress XDR length");
        let pk_xdr = xdr.slice((n - 32)..n);
        let mut pk = [0u8; 32];
        for (i, byte) in pk_xdr.iter().enumerate() {
            pk[i] = byte;
        }
        assert_eq!(pk, USDC_ISSUER);
    }

    #[test]
    fn asset_xdr_layout_is_canonical() {
        // Cross-check the manual XDR against the xdr crate's own
        // serialization, which is what the host deserializes.
        let asset = soroban_sdk::xdr::Asset::CreditAlphanum4(soroban_sdk::xdr::AlphaNum4 {
            asset_code: soroban_sdk::xdr::AssetCode4(*b"USDC"),
            issuer: soroban_sdk::xdr::AccountId(soroban_sdk::xdr::PublicKey::PublicKeyTypeEd25519(
                soroban_sdk::xdr::Uint256(USDC_ISSUER),
            )),
        });
        let mut buf = std::vec::Vec::new();
        let mut limited =
            soroban_sdk::xdr::Limited::new(&mut buf, soroban_sdk::xdr::Limits::none());
        soroban_sdk::xdr::WriteXdr::write_xdr(&asset, &mut limited).unwrap();
        assert_eq!(buf, USDC_ASSET_XDR.to_vec());
        assert_eq!(USDC_ASSET_XDR.len(), 44);
        assert_eq!(XLM_ASSET_XDR.len(), 4);
    }
}
