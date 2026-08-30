//! Canonical asset registry for the v1 pool.
//!
//! The pool is deliberately pinned to one reviewed pair: native Stellar
//! (XLM) as the base asset (token_0) and Circle's USD Coin (USDC) as the
//! quote asset (token_1). This module is the single source of truth for
//! the accepted mainnet asset set and doubles as the contract-side release
//! artifact for it: the exact SEP-41 metadata each side's token contract
//! must report, which [`initialize`](crate::NodusAmm::initialize) enforces
//! on-chain before a pool can ever become active.
//!
//! # Why metadata is verified on-chain, and what that does (and doesn't) prove
//!
//! Soroban 26 does not expose an on-chain way to read another contract's
//! WASM/code hash, so the contract cannot itself fingerprint the official
//! Stellar Asset Contract binary. What it *can* and does enforce is:
//!
//! 1. Each address is a live contract that speaks SEP-41 (metadata and
//!    `balance` calls must succeed, not revert);
//! 2. Its `name`/`symbol`/`decimals` exactly match the reviewed canonical
//!    asset policy below;
//! 3. `balance(pool)` works and is zero at activation, so reserves start
//!    from a pristine state.
//!
//! Symbol/name alone are never treated as proof (per the issue); they are
//! enforced *together with* deploy-side pinning of the reviewed canonical
//! SAC addresses (see `docs/canonical-assets.md`), which is what actually
//! defeats a same-symbol impostor or a wrong-network deployment. A contract
//! reporting the right metadata from the wrong address passes these checks
//! — that is exactly why the canonical contract addresses must be pinned
//! from reviewed Stellar asset definitions before mainnet deployment and
//! cross-checked off-chain at deploy time.

use crate::errors::Error;
use soroban_sdk::{token::Client as TokenClient, Address, Env, String};

/// Reviewed canonical metadata for native Stellar (XLM). XLM is always the
/// base asset (`token_0`); ordering is part of the pinned policy.
pub const XLM_NAME: &str = "Stellar";
pub const XLM_SYMBOL: &str = "XLM";
pub const XLM_DECIMALS: u32 = 7;

/// Reviewed canonical metadata for Circle's USD Coin (USDC) on Stellar.
/// USDC is always the quote asset (`token_1`).
pub const USDC_NAME: &str = "USD Coin";
pub const USDC_SYMBOL: &str = "USDC";
pub const USDC_DECIMALS: u32 = 7;

/// Unwraps the double-layer `Result` returned by a `try_*` token client
/// call, mapping any failure — callee reverted, non-contract address,
/// conversion failure — to [`Error::NotTokenContract`]. The client's
/// `try_` variants return `Result<Result<T, _>, Result<_, _>>`, where the
/// outer layer is the invocation result and the inner layer the value
/// conversion, so this collapses both.
fn unwrap_token_call<T, CE, IE>(res: Result<Result<T, CE>, IE>) -> Result<T, Error> {
    match res {
        Ok(Ok(value)) => Ok(value),
        _ => Err(Error::NotTokenContract),
    }
}

/// Verifies that `token` is a live, SEP-41-compatible contract whose
/// metadata and semantics match the expected canonical asset.
///
/// All calls go through the `try_*` client variants so that a bare account
/// address, an uninitialized contract, or a token that reverts on any of
/// these reads fails cleanly with [`Error::NotTokenContract`] instead of
/// aborting the whole initialization. A mismatch on the reviewed metadata
/// surfaces as [`Error::UnsupportedAsset`] (impostor / reversed / unknown
/// pair); a decimals mismatch as [`Error::WrongDecimals`].
pub fn verify_canonical_token(
    env: &Env,
    token: &Address,
    expected_name: &str,
    expected_symbol: &str,
    expected_decimals: u32,
) -> Result<(), Error> {
    let client = TokenClient::new(env, token);
    let pool = env.current_contract_address();

    // Canary that doubles as an interface check: a real SAC/SEP-41 contract
    // answers `balance(pool)` and reports 0 for an address that never held
    // the asset. A non-contract or reverting token surfaces as Err.
    let balance = unwrap_token_call(client.try_balance(&pool))?;
    if balance != 0 {
        // The pool must start from a pristine state; any balance parked at
        // this address before activation means it is not the pristine
        // canonical asset contract.
        return Err(Error::UnsupportedAsset);
    }

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
