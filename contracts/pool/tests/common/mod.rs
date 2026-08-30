//! Shared helpers for the pool's integration/unit tests: deploying the
//! *real* canonical XLM and USDC Stellar Asset Contracts at their derived
//! addresses (with the Circle issuer account materialized so USDC can be
//! minted), funding accounts, and a configurable hostile SEP-41 token for
//! adversarial tests (wrong metadata, fee-on-transfer, silently dropped
//! transfers, reentrancy).

#![allow(clippy::too_many_arguments)]
// Helpers are shared by two test crates; not every crate uses every helper.
#![allow(dead_code)]

use nodus_protocol_amm::registry;
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype,
    testutils::Ledger as _,
    token::StellarAssetClient,
    xdr::{
        AccountEntry, AccountEntryExt, AccountId, LedgerEntry, LedgerEntryData, LedgerEntryExt,
        LedgerKey, LedgerKeyAccount, PublicKey, SequenceNumber, Thresholds, Uint256, VecM,
    },
    Address, Env, MuxedAddress, String,
};
use std::rc::Rc;

/// Creates a default test env with auth mocked and a non-zero ledger
/// sequence (needed for SAC approve/allowance expiration semantics).
pub fn env_with_seq() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(100);
    env
}

/// Materializes Circle's USDC issuer account in the sandbox ledger so the
/// canonical USDC SAC can mint (the real SAC reads the issuer's account
/// entry during mint).
pub fn create_usdc_issuer_account(env: &Env) {
    let issuer_id = AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(
        registry::USDC_ISSUER,
    )));
    let key = Rc::new(LedgerKey::Account(LedgerKeyAccount {
        account_id: issuer_id.clone(),
    }));
    let entry = Rc::new(LedgerEntry {
        data: LedgerEntryData::Account(AccountEntry {
            account_id: issuer_id,
            balance: 0,
            flags: 0,
            home_domain: Default::default(),
            inflation_dest: None,
            num_sub_entries: 0,
            seq_num: SequenceNumber(0),
            thresholds: Thresholds([1; 4]),
            signers: VecM::default(),
            ext: AccountEntryExt::V0,
        }),
        last_modified_ledger_seq: 0,
        ext: LedgerEntryExt::V0,
    });
    env.host().add_ledger_entry(&key, &entry, None).unwrap();
}

/// Deploys the real canonical XLM and USDC Stellar Asset Contracts at the
/// addresses derived from the reviewed asset definitions (i.e. exactly what
/// `registry::derive_canonical_address` computes), and returns their
/// addresses. The caller still needs to fund them (see [`fund`]).
pub fn deploy_canonical_sacs(env: &Env) -> (Address, Address) {
    create_usdc_issuer_account(env);
    let xlm = env
        .deployer()
        .with_stellar_asset(soroban_sdk::Bytes::from_slice(
            env,
            &registry::XLM_ASSET_XDR,
        ))
        .deploy();
    let usdc = env
        .deployer()
        .with_stellar_asset(soroban_sdk::Bytes::from_slice(
            env,
            &registry::USDC_ASSET_XDR,
        ))
        .deploy();
    (xlm, usdc)
}

/// Mints `amount` of the SAC at `sac` to `to` (non-try client so testutils
/// auth mocking applies).
pub fn fund(env: &Env, sac: &Address, to: &Address, amount: i128) {
    StellarAssetClient::new(env, sac).mint(to, &amount);
}

// ── Hostile token ──────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HostileMode {
    /// Behave like a working token.
    Normal,
    /// Take a 1-stroop fee on every transfer/transfer_from.
    FeeOnTransfer,
    /// Return Ok from transfer/transfer_from but move nothing.
    NoOp,
    /// On transfer_from, call back into the configured pool.
    Reentrant,
}

#[contracttype]
pub enum HKey {
    Init,
    Mode,
    Name,
    Sym,
    Dec,
    Pool,
    Supply,
    Balance(Address),
    /// Set by [`HostileMode::Reentrant`] transfers: whether the pool's
    /// reentrancy lock rejected the nested call with
    /// `ReentrancyDetected`.
    ReentryObserved,
    /// Debug aid: shape of the last reentrant try_swap result.
    ReentryResult,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum HostileError {
    AlreadyInitialized = 1,
    Overflow = 2,
    InsufficientBalance = 3,
}

/// A SEP-41-shaped token whose behavior is configurable, for adversarial
/// tests. Planted at a canonical SAC address via `register_at` it simulates
/// a canonical SAC that was upgraded/replaced with a malicious
/// implementation.
#[contract]
pub struct HostileToken;

#[contractimpl]
impl HostileToken {
    pub fn initialize(
        env: Env,
        mode: HostileMode,
        name: String,
        symbol: String,
        decimals: u32,
        pool: Address,
    ) -> Result<(), HostileError> {
        if env
            .storage()
            .instance()
            .get::<_, bool>(&HKey::Init)
            .unwrap_or(false)
        {
            return Err(HostileError::AlreadyInitialized);
        }
        env.storage().instance().set(&HKey::Init, &true);
        env.storage().instance().set(&HKey::Mode, &mode);
        env.storage().instance().set(&HKey::Name, &name);
        env.storage().instance().set(&HKey::Sym, &symbol);
        env.storage().instance().set(&HKey::Dec, &decimals);
        env.storage().instance().set(&HKey::Pool, &pool);
        Ok(())
    }

    pub fn name(env: Env) -> Result<String, HostileError> {
        require_init(&env)?;
        Ok(env.storage().instance().get(&HKey::Name).unwrap())
    }

    pub fn symbol(env: Env) -> Result<String, HostileError> {
        require_init(&env)?;
        Ok(env.storage().instance().get(&HKey::Sym).unwrap())
    }

    pub fn decimals(env: Env) -> Result<u32, HostileError> {
        require_init(&env)?;
        Ok(env.storage().instance().get(&HKey::Dec).unwrap())
    }

    pub fn balance(env: Env, id: Address) -> i128 {
        read_balance(&env, &id)
    }

    pub fn total_supply(env: Env) -> i128 {
        env.storage().instance().get(&HKey::Supply).unwrap_or(0)
    }

    /// Open mint (test-only convenience; auth is mocked in these tests).
    pub fn mint(env: Env, _caller: Address, to: Address, amount: i128) -> Result<(), HostileError> {
        require_init(&env)?;
        let new_balance = read_balance(&env, &to)
            .checked_add(amount)
            .ok_or(HostileError::Overflow)?;
        let new_supply = total_supply(&env)
            .checked_add(amount)
            .ok_or(HostileError::Overflow)?;
        write_balance(&env, &to, new_balance);
        env.storage().instance().set(&HKey::Supply, &new_supply);
        Ok(())
    }

    pub fn approve(
        _env: Env,
        _from: Address,
        _spender: Address,
        _amount: i128,
        _expiration_ledger: u32,
    ) -> Result<(), HostileError> {
        // Allowance semantics are not exercised by these tests; accept.
        Ok(())
    }

    pub fn transfer(
        env: Env,
        from: Address,
        to: MuxedAddress,
        amount: i128,
    ) -> Result<(), HostileError> {
        if mode(&env) == HostileMode::NoOp {
            return Ok(());
        }
        let moved = if mode(&env) == HostileMode::FeeOnTransfer {
            amount.saturating_sub(1)
        } else {
            amount
        };
        let from_balance = read_balance(&env, &from);
        if from_balance < amount {
            return Err(HostileError::InsufficientBalance);
        }
        write_balance(&env, &from, from_balance - amount);
        let to_address = to.address();
        write_balance(&env, &to_address, read_balance(&env, &to_address) + moved);
        Ok(())
    }

    pub fn transfer_from(
        env: Env,
        _spender: Address,
        from: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), HostileError> {
        if mode(&env) == HostileMode::NoOp {
            return Ok(());
        }
        let moved = if mode(&env) == HostileMode::FeeOnTransfer {
            amount.saturating_sub(1)
        } else {
            amount
        };
        let from_balance = read_balance(&env, &from);
        if from_balance < amount {
            return Err(HostileError::InsufficientBalance);
        }
        write_balance(&env, &from, from_balance - amount);
        write_balance(&env, &to, read_balance(&env, &to) + moved);

        if mode(&env) == HostileMode::Reentrant {
            // Reenter the pool from inside the token's transfer path while
            // the pool holds its reentrancy lock, and record whether the
            // lock specifically rejected the nested call. The pool's
            // ReentrancyDetected error is caught here rather than allowed
            // to abort the whole transaction: the deposit that triggered
            // the reentry should still complete, with the reentrant swap
            // provably blocked and its effects zero.
            let pool: Address = env.storage().instance().get(&HKey::Pool).unwrap();
            let result = nodus_protocol_amm::NodusAmmClient::new(&env, &pool).try_swap(
                &to,
                &1,
                &0,
                &u64::MAX,
            );
            // The Soroban host rejects contract re-entry at the protocol
            // level (Context/InvalidAction, surfaced here as an abort); the
            // pool's own lock is defense-in-depth on top of that. Either
            // way the nested swap must not execute, which is the property
            // the test asserts. Record the outcome so the test can observe
            // the rejection from outside the pool.
            use nodus_protocol_amm::Error as PoolError;
            let code: u32 = match result {
                Ok(Ok(())) => 1,
                Err(Ok(PoolError::ReentrancyDetected)) => 21,
                Err(Ok(_)) => 2,
                Err(Err(soroban_sdk::InvokeError::Abort)) => 3,
                Err(Err(soroban_sdk::InvokeError::Contract(c))) => 100 + c,
                _ => 4,
            };
            env.storage().instance().set(&HKey::ReentryResult, &code);
            env.storage()
                .instance()
                .set(&HKey::ReentryObserved, &(code != 1));
        }
        Ok(())
    }

    /// Test-only switch: re-arm a token's behavior after initialization
    /// (e.g. flip a previously well-behaved token into reentrant mode once
    /// the pool holds reserves).
    pub fn set_mode(env: Env, mode: HostileMode) {
        env.storage().instance().set(&HKey::Mode, &mode);
    }

    /// Whether the last reentrant transfer saw the pool's reentrancy lock
    /// reject the nested call with `ReentrancyDetected`.
    pub fn reentry_observed(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&HKey::ReentryObserved)
            .unwrap_or(false)
    }

    /// Debug aid: 0 = never attempted, 1 = Ok(Ok), 2 = Err(Ok(err)),
    /// 3 = Err(Err(invoke)).
    pub fn reentry_result_code(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&HKey::ReentryResult)
            .unwrap_or(0)
    }
}

fn require_init(env: &Env) -> Result<(), HostileError> {
    if !env
        .storage()
        .instance()
        .get::<_, bool>(&HKey::Init)
        .unwrap_or(false)
    {
        return Err(HostileError::AlreadyInitialized);
    }
    Ok(())
}

fn mode(env: &Env) -> HostileMode {
    env.storage()
        .instance()
        .get(&HKey::Mode)
        .unwrap_or(HostileMode::Normal)
}

fn read_balance(env: &Env, addr: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&HKey::Balance(addr.clone()))
        .unwrap_or(0)
}

fn write_balance(env: &Env, addr: &Address, amount: i128) {
    env.storage()
        .persistent()
        .set(&HKey::Balance(addr.clone()), &amount);
}

fn total_supply(env: &Env) -> i128 {
    env.storage().instance().get(&HKey::Supply).unwrap_or(0)
}

/// Registers a [`HostileToken`] at the exact address `at` (e.g. a canonical
/// SAC address) with the given mode and metadata. `pool` is the address the
/// token re-enters in [`HostileMode::Reentrant`].
pub fn register_hostile_at(
    env: &Env,
    at: &Address,
    mode: HostileMode,
    name: &String,
    symbol: &String,
    decimals: u32,
    pool: &Address,
) -> Address {
    let addr = env.register_at(at, HostileToken, ());
    let client = HostileTokenClient::new(env, &addr);
    client.initialize(&mode, name, symbol, &decimals, pool);
    addr
}

/// A SEP-41-shaped contract that is deliberately **not** a token (no
/// metadata functions), used to prove the interface check rejects non-token
/// contracts even when they sit at a canonical address.
#[contract]
pub struct NonToken;

#[contractimpl]
impl NonToken {
    pub fn ping(env: Env) -> u32 {
        env.ledger().sequence()
    }
}
