#![no_std]
use soroban_sdk::{
    contract, contractimpl,
    token::Client as TokenClient,
    Address, Env, Symbol,
};

mod errors;
mod events;
mod lp_token;
mod liquidity_pool;
mod math;
mod reentrancy_guard;
mod storage;
mod traits;

pub use errors::Error;
use storage::DataKey;

#[contract]
pub struct NodusAmm;

fn get_reserve_0(env: &Env) -> i128 {
    env.storage().instance().get(&DataKey::Reserve0).unwrap_or(0)
}

fn get_reserve_1(env: &Env) -> i128 {
    env.storage().instance().get(&DataKey::Reserve1).unwrap_or(0)
}

fn get_timestamp_last(env: &Env) -> u64 {
    env.storage().instance().get(&DataKey::TimestampLast).unwrap_or(0)
}

fn is_locked(env: &Env) -> bool {
    env.storage().instance().get(&DataKey::Locked).unwrap_or(false)
}

fn set_locked(env: &Env, locked: bool) {
    env.storage().instance().set(&DataKey::Locked, &locked);
}

fn lock(env: &Env) -> Result<(), Error> {
    if is_locked(env) { return Err(Error::ReentrancyDetected); }
    set_locked(env, true);
    Ok(())
}

fn unlock(env: &Env) {
    set_locked(env, false);
}

fn update(env: &Env, balance_0: i128, balance_1: i128, reserve_0: i128, reserve_1: i128) {
    let timestamp = env.ledger().timestamp();
    let time_elapsed = timestamp.saturating_sub(get_timestamp_last(env));
    if time_elapsed > 0 && reserve_0 > 0 && reserve_1 > 0 {
        let p0 = env.storage().instance()
            .get::<DataKey, i128>(&DataKey::LpTotalSupply).unwrap_or(0);
        let _ = p0;
    }
    env.storage().instance().set(&DataKey::Reserve0, &balance_0);
    env.storage().instance().set(&DataKey::Reserve1, &balance_1);
    env.storage().instance().set(&DataKey::TimestampLast, &timestamp);
    events::emit_sync(env, balance_0, balance_1);
}

fn token_balance(env: &Env, token: &Address) -> i128 {
    TokenClient::new(env, token).balance(&env.current_contract_address())
}

fn token_transfer(env: &Env, token: &Address, to: &Address, amount: i128) {
    TokenClient::new(env, token).transfer(&env.current_contract_address(), to, &amount);
}

fn token_transfer_from(env: &Env, token: &Address, from: &Address, amount: i128) {
    TokenClient::new(env, token)
        .transfer_from(&env.current_contract_address(), from, &env.current_contract_address(), &amount);
}

#[contractimpl]
impl NodusAmm {
    pub fn initialize(env: Env, token_0: Address, token_1: Address) -> Result<(), Error> {
        if env.storage().instance().get::<DataKey, bool>(&DataKey::Initialized).unwrap_or(false) {
            return Err(Error::AlreadyInitialized);
        }
        if token_0 == token_1 { return Err(Error::InvalidTokenPair); }
        env.storage().instance().set(&DataKey::Token0, &token_0);
        env.storage().instance().set(&DataKey::Token1, &token_1);
        env.storage().instance().set(&DataKey::Initialized, &true);
        Ok(())
    }

    pub fn add_liquidity(
        env: Env,
        amount_0_desired: i128,
        amount_1_desired: i128,
        amount_0_min: i128,
        amount_1_min: i128,
        to: Address,
        deadline: u64,
    ) -> Result<i128, Error> {
        if env.ledger().timestamp() > deadline { return Err(Error::Expired); }
        lock(&env)?;

        let token_0: Address = env.storage().instance().get(&DataKey::Token0)
            .ok_or_else(|| { unlock(&env); Error::NotInitialized })?;
        let token_1: Address = env.storage().instance().get(&DataKey::Token1)
            .ok_or_else(|| { unlock(&env); Error::NotInitialized })?;

        let reserve_0 = get_reserve_0(&env);
        let reserve_1 = get_reserve_1(&env);

        let (amount_0, amount_1) = if reserve_0 == 0 && reserve_1 == 0 {
            (amount_0_desired, amount_1_desired)
        } else {
            liquidity_pool::calculate_optimal_amounts(
                amount_0_desired, amount_1_desired,
                amount_0_min, amount_1_min,
                reserve_0, reserve_1,
            ).map_err(|e| { unlock(&env); e })?
        };

        let caller = to.clone();
        caller.require_auth();
        token_transfer_from(&env, &token_0, &caller, amount_0);
        token_transfer_from(&env, &token_1, &caller, amount_1);

        let total_supply = lp_token::total_supply(&env);

        let liquidity = if total_supply == 0 {
            let initial = liquidity_pool::calculate_initial_liquidity(amount_0, amount_1)
                .map_err(|e| { unlock(&env); e })?;
            lp_token::mint(&env, &Address::from_contract_id(&env, &env.current_contract_address()), math::MINIMUM_LIQUIDITY)
                .map_err(|e| { unlock(&env); e })?;
            initial
        } else {
            liquidity_pool::calculate_liquidity_to_mint(
                amount_0, amount_1, reserve_0, reserve_1, total_supply,
            ).map_err(|e| { unlock(&env); e })?
        };

        if liquidity == 0 { unlock(&env); return Err(Error::InsufficientLiquidityMinted); }

        lp_token::mint(&env, &to, liquidity)
            .map_err(|e| { unlock(&env); e })?;

        let b0 = token_balance(&env, &token_0);
        let b1 = token_balance(&env, &token_1);
        update(&env, b0, b1, reserve_0, reserve_1);

        events::emit_mint(&env, caller, amount_0, amount_1);
        unlock(&env);
        Ok(liquidity)
    }

    pub fn remove_liquidity(
        env: Env,
        liquidity: i128,
        amount_0_min: i128,
        amount_1_min: i128,
        to: Address,
        deadline: u64,
    ) -> Result<(i128, i128), Error> {
        if env.ledger().timestamp() > deadline { return Err(Error::Expired); }
        lock(&env)?;

        let token_0: Address = env.storage().instance().get(&DataKey::Token0)
            .ok_or_else(|| { unlock(&env); Error::NotInitialized })?;
        let token_1: Address = env.storage().instance().get(&DataKey::Token1)
            .ok_or_else(|| { unlock(&env); Error::NotInitialized })?;

        let caller = to.clone();
        caller.require_auth();

        let total_supply = lp_token::total_supply(&env);
        let reserve_0 = get_reserve_0(&env);
        let reserve_1 = get_reserve_1(&env);

        let (amount_0, amount_1) = liquidity_pool::calculate_withdrawal_amounts(
            liquidity, reserve_0, reserve_1, total_supply,
        ).map_err(|e| { unlock(&env); e })?;

        if amount_0 < amount_0_min || amount_1 < amount_1_min {
            unlock(&env);
            return Err(Error::InsufficientLiquidityBurned);
        }

        lp_token::burn(&env, &caller, liquidity)
            .map_err(|e| { unlock(&env); e })?;

        token_transfer(&env, &token_0, &to, amount_0);
        token_transfer(&env, &token_1, &to, amount_1);

        let b0 = token_balance(&env, &token_0);
        let b1 = token_balance(&env, &token_1);
        update(&env, b0, b1, reserve_0, reserve_1);

        events::emit_burn(&env, caller, amount_0, amount_1, to);
        unlock(&env);
        Ok((amount_0, amount_1))
    }

    pub fn swap(
        env: Env,
        amount_0_out: i128,
        amount_1_out: i128,
        to: Address,
    ) -> Result<(), Error> {
        lock(&env)?;

        if amount_0_out == 0 && amount_1_out == 0 {
            unlock(&env);
            return Err(Error::InsufficientOutputAmount);
        }

        let token_0: Address = env.storage().instance().get(&DataKey::Token0)
            .ok_or_else(|| { unlock(&env); Error::NotInitialized })?;
        let token_1: Address = env.storage().instance().get(&DataKey::Token1)
            .ok_or_else(|| { unlock(&env); Error::NotInitialized })?;

        let reserve_0 = get_reserve_0(&env);
        let reserve_1 = get_reserve_1(&env);

        if amount_0_out >= reserve_0 || amount_1_out >= reserve_1 {
            unlock(&env);
            return Err(Error::InsufficientLiquidity);
        }

        if amount_0_out > 0 { token_transfer(&env, &token_0, &to, amount_0_out); }
        if amount_1_out > 0 { token_transfer(&env, &token_1, &to, amount_1_out); }

        let balance_0 = token_balance(&env, &token_0);
        let balance_1 = token_balance(&env, &token_1);

        let amount_0_in = balance_0.saturating_sub(reserve_0.saturating_sub(amount_0_out));
        let amount_1_in = balance_1.saturating_sub(reserve_1.saturating_sub(amount_1_out));

        if amount_0_in == 0 && amount_1_in == 0 {
            unlock(&env);
            return Err(Error::InsufficientLiquidity);
        }

        liquidity_pool::verify_k_invariant(
            balance_0, balance_1,
            amount_0_in, amount_1_in,
            reserve_0, reserve_1,
        ).map_err(|e| { unlock(&env); e })?;

        update(&env, balance_0, balance_1, reserve_0, reserve_1);

        let caller = env.current_contract_address();
        events::emit_swap(&env, caller, amount_0_in, amount_1_in, amount_0_out, amount_1_out, to);
        unlock(&env);
        Ok(())
    }

    pub fn sync(env: Env) -> Result<(), Error> {
        let token_0: Address = env.storage().instance().get(&DataKey::Token0)
            .ok_or(Error::NotInitialized)?;
        let token_1: Address = env.storage().instance().get(&DataKey::Token1)
            .ok_or(Error::NotInitialized)?;
        let b0 = token_balance(&env, &token_0);
        let b1 = token_balance(&env, &token_1);
        update(&env, b0, b1, get_reserve_0(&env), get_reserve_1(&env));
        Ok(())
    }

    pub fn get_reserves(env: Env) -> (i128, i128, u64) {
        (get_reserve_0(&env), get_reserve_1(&env), get_timestamp_last(&env))
    }

    pub fn get_amount_out(env: Env, amount_in: i128, reserve_in: i128, reserve_out: i128) -> Result<i128, Error> {
        let _ = env;
        math::get_amount_out(amount_in, reserve_in, reserve_out)
    }

    pub fn get_amount_in(env: Env, amount_out: i128, reserve_in: i128, reserve_out: i128) -> Result<i128, Error> {
        let _ = env;
        math::get_amount_in(amount_out, reserve_in, reserve_out)
    }

    pub fn lp_balance_of(env: Env, owner: Address) -> i128 {
        lp_token::balance_of(&env, &owner)
    }

    pub fn lp_total_supply(env: Env) -> i128 {
        lp_token::total_supply(&env)
    }

    pub fn transfer_lp(env: Env, from: Address, to: Address, amount: i128) -> Result<(), Error> {
        from.require_auth();
        lp_token::transfer(&env, &from, &to, amount)
    }
}
