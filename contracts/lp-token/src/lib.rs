#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, MuxedAddress, String};

pub mod errors;
pub mod events;
pub mod storage;

pub use errors::Error;
use storage::DataKey;

const INSTANCE_TTL_THRESHOLD: u32 = 100;
const INSTANCE_TTL_BUMP: u32 = 500;
const BALANCE_TTL_THRESHOLD: u32 = 100;
const BALANCE_TTL_BUMP: u32 = 500;

fn require_initialized(env: &Env) -> Result<(), Error> {
    if !env
        .storage()
        .instance()
        .get::<DataKey, bool>(&DataKey::Initialized)
        .unwrap_or(false)
    {
        return Err(Error::NotInitialized);
    }
    Ok(())
}

fn read_balance(env: &Env, addr: &Address) -> i128 {
    let key = DataKey::Balance(addr.clone());
    let balance = env.storage().persistent().get(&key).unwrap_or(0i128);
    if balance > 0 {
        env.storage()
            .persistent()
            .extend_ttl(&key, BALANCE_TTL_THRESHOLD, BALANCE_TTL_BUMP);
    }
    balance
}

fn write_balance(env: &Env, addr: &Address, amount: i128) {
    let key = DataKey::Balance(addr.clone());
    env.storage().persistent().set(&key, &amount);
    env.storage()
        .persistent()
        .extend_ttl(&key, BALANCE_TTL_THRESHOLD, BALANCE_TTL_BUMP);
}

fn read_allowance(env: &Env, owner: &Address, spender: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::Allowance(owner.clone(), spender.clone()))
        .unwrap_or(0i128)
}

fn write_allowance(env: &Env, owner: &Address, spender: &Address, amount: i128, live_for: u32) {
    let key = DataKey::Allowance(owner.clone(), spender.clone());
    env.storage().persistent().set(&key, &amount);
    env.storage()
        .persistent()
        .extend_ttl(&key, live_for, live_for);
}

fn read_total_supply(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::TotalSupply)
        .unwrap_or(0i128)
}

fn write_total_supply(env: &Env, amount: i128) {
    env.storage().instance().set(&DataKey::TotalSupply, &amount);
}

fn spend_allowance(
    env: &Env,
    owner: &Address,
    spender: &Address,
    amount: i128,
) -> Result<(), Error> {
    let allowed = read_allowance(env, owner, spender);
    if allowed < amount {
        return Err(Error::Unauthorized);
    }
    // Deliberately doesn't extend the allowance's TTL on spend (only on a
    // fresh approve()) -- consuming less than the full amount shouldn't
    // resurrect an entry the owner otherwise let expire.
    let key = DataKey::Allowance(owner.clone(), spender.clone());
    env.storage().persistent().set(&key, &(allowed - amount));
    Ok(())
}

#[contract]
pub struct NodusLpToken;

#[contractimpl]
impl NodusLpToken {
    /// One-time setup, called by the factory (or pool) right after
    /// deployment. `pool` is the only address ever authorized to [`mint`].
    pub fn initialize(
        env: Env,
        pool: Address,
        name: String,
        symbol: String,
        decimals: u32,
    ) -> Result<(), Error> {
        if env
            .storage()
            .instance()
            .get::<DataKey, bool>(&DataKey::Initialized)
            .unwrap_or(false)
        {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Pool, &pool);
        env.storage().instance().set(&DataKey::Name, &name);
        env.storage().instance().set(&DataKey::Symbol, &symbol);
        env.storage().instance().set(&DataKey::Decimals, &decimals);
        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_BUMP);
        Ok(())
    }

    pub fn pool(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Pool)
            .ok_or(Error::NotInitialized)
    }

    // ── Pool-gated mint (not part of the standard token interface -- SEP-41
    // deliberately leaves minting out, since it's issuer-specific) ─────────

    /// Mints new LP tokens to `to`. `caller` must be the pool this token
    /// was initialized with; there is no other admin. Takes `caller`
    /// explicitly (rather than always requiring the stored pool's auth
    /// unconditionally) so the rejection is a plain identity comparison,
    /// matching how the pool contract itself gates set_fee_to/pause --
    /// and, unlike a bare require_auth() on the stored address, testable
    /// under mock_all_auths() without exotic per-address auth mocking.
    pub fn mint(env: Env, caller: Address, to: Address, amount: i128) -> Result<(), Error> {
        require_initialized(&env)?;
        if amount <= 0 {
            return Err(Error::ZeroAmount);
        }
        caller.require_auth();
        let pool: Address = env.storage().instance().get(&DataKey::Pool).unwrap();
        if caller != pool {
            return Err(Error::Unauthorized);
        }

        let new_balance = read_balance(&env, &to)
            .checked_add(amount)
            .ok_or(Error::Overflow)?;
        let new_supply = read_total_supply(&env)
            .checked_add(amount)
            .ok_or(Error::Overflow)?;
        write_balance(&env, &to, new_balance);
        write_total_supply(&env, new_supply);
        events::emit_mint(&env, to, amount);
        Ok(())
    }

    // ── SEP-41 token interface ───────────────────────────────────────────

    pub fn allowance(env: Env, from: Address, spender: Address) -> i128 {
        read_allowance(&env, &from, &spender)
    }

    /// `expiration_ledger` becomes the allowance entry's live-until ledger
    /// via `extend_ttl`; a lower value than the current ledger is only
    /// accepted when `amount` is 0 (revoking an approval never needs to
    /// extend anything).
    pub fn approve(
        env: Env,
        from: Address,
        spender: Address,
        amount: i128,
        expiration_ledger: u32,
    ) -> Result<(), Error> {
        require_initialized(&env)?;
        if amount < 0 {
            return Err(Error::ZeroAmount);
        }
        from.require_auth();

        let live_for = expiration_ledger.saturating_sub(env.ledger().sequence());
        if amount > 0 && live_for == 0 {
            return Err(Error::ApprovalExpired);
        }
        write_allowance(&env, &from, &spender, amount, live_for);
        events::emit_approve(&env, from, spender, amount, expiration_ledger);
        Ok(())
    }

    pub fn balance(env: Env, id: Address) -> i128 {
        read_balance(&env, &id)
    }

    /// `to` is a [`MuxedAddress`] per the standard token interface, so a
    /// payment can carry a muxed id for the recipient's own bookkeeping;
    /// the balance itself is always credited to the underlying `Address`.
    pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) -> Result<(), Error> {
        require_initialized(&env)?;
        if amount <= 0 {
            return Err(Error::ZeroAmount);
        }
        from.require_auth();

        let to_address = to.address();
        let from_balance = read_balance(&env, &from);
        if from_balance < amount {
            return Err(Error::InsufficientBalance);
        }
        let to_new = read_balance(&env, &to_address)
            .checked_add(amount)
            .ok_or(Error::Overflow)?;
        write_balance(&env, &from, from_balance - amount);
        write_balance(&env, &to_address, to_new);
        events::emit_transfer(&env, from, to_address, amount);
        Ok(())
    }

    pub fn transfer_from(
        env: Env,
        spender: Address,
        from: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), Error> {
        require_initialized(&env)?;
        if amount <= 0 {
            return Err(Error::ZeroAmount);
        }
        spender.require_auth();
        spend_allowance(&env, &from, &spender, amount)?;

        let from_balance = read_balance(&env, &from);
        if from_balance < amount {
            return Err(Error::InsufficientBalance);
        }
        let to_new = read_balance(&env, &to)
            .checked_add(amount)
            .ok_or(Error::Overflow)?;
        write_balance(&env, &from, from_balance - amount);
        write_balance(&env, &to, to_new);
        events::emit_transfer(&env, from, to, amount);
        Ok(())
    }

    /// Burns `from`'s own tokens. Authorized by `from` alone -- when the
    /// pool calls this as part of remove_liquidity, `from`'s own
    /// authorization for that top-level call covers this nested one too,
    /// the same way it already covers the pool's own token transfers.
    /// A holder can also call this directly, bypassing the pool; that
    /// forfeits their claim on the underlying reserves with no payout,
    /// which only benefits every other LP holder proportionally -- an
    /// unusual thing to do, not an unsafe one.
    pub fn burn(env: Env, from: Address, amount: i128) -> Result<(), Error> {
        require_initialized(&env)?;
        if amount <= 0 {
            return Err(Error::ZeroAmount);
        }
        from.require_auth();

        let balance = read_balance(&env, &from);
        if balance < amount {
            return Err(Error::InsufficientBalance);
        }
        let new_supply = read_total_supply(&env)
            .checked_sub(amount)
            .ok_or(Error::Overflow)?;
        write_balance(&env, &from, balance - amount);
        write_total_supply(&env, new_supply);
        events::emit_burn(&env, from, amount);
        Ok(())
    }

    pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) -> Result<(), Error> {
        require_initialized(&env)?;
        if amount <= 0 {
            return Err(Error::ZeroAmount);
        }
        spender.require_auth();
        spend_allowance(&env, &from, &spender, amount)?;

        let balance = read_balance(&env, &from);
        if balance < amount {
            return Err(Error::InsufficientBalance);
        }
        let new_supply = read_total_supply(&env)
            .checked_sub(amount)
            .ok_or(Error::Overflow)?;
        write_balance(&env, &from, balance - amount);
        write_total_supply(&env, new_supply);
        events::emit_burn(&env, from, amount);
        Ok(())
    }

    pub fn decimals(env: Env) -> Result<u32, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Decimals)
            .ok_or(Error::NotInitialized)
    }

    pub fn name(env: Env) -> Result<String, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Name)
            .ok_or(Error::NotInitialized)
    }

    pub fn symbol(env: Env) -> Result<String, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Symbol)
            .ok_or(Error::NotInitialized)
    }

    pub fn total_supply(env: Env) -> i128 {
        read_total_supply(&env)
    }
}
