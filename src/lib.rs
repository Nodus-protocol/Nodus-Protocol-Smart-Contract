#![no_std]
use soroban_sdk::{
    contract, contractimpl,
    token::Client as TokenClient,
    Address, Env, Symbol,
};

pub mod errors;
pub mod events;
pub mod lp_token;
pub mod liquidity_pool;
pub mod math;
pub mod storage;
pub mod traits;

pub use errors::Error;
use storage::DataKey;

const INSTANCE_TTL_THRESHOLD: u32 = 100;
const INSTANCE_TTL_BUMP: u32 = 500;

fn require_initialized(env: &Env) -> Result<(), Error> {
    if !env.storage().instance().get::<DataKey, bool>(&DataKey::Initialized).unwrap_or(false) {
        return Err(Error::NotInitialized);
    }
    Ok(())
}

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

fn lock(env: &Env) -> Result<(), Error> {
    if is_locked(env) { return Err(Error::ReentrancyDetected); }
    env.storage().instance().set(&DataKey::Locked, &true);
    Ok(())
}

fn unlock(env: &Env) {
    env.storage().instance().set(&DataKey::Locked, &false);
}

fn update(env: &Env, balance_0: i128, balance_1: i128, reserve_0: i128, reserve_1: i128) {
    let timestamp = env.ledger().timestamp();
    let time_elapsed = timestamp.saturating_sub(get_timestamp_last(env));

    if time_elapsed > 0 && reserve_0 > 0 && reserve_1 > 0 {
        let acc_0: u128 = env.storage().instance()
            .get(&DataKey::Price0CumulativeLast).unwrap_or(0u128);
        let acc_1: u128 = env.storage().instance()
            .get(&DataKey::Price1CumulativeLast).unwrap_or(0u128);
        let price_0 = ((reserve_1 as u128) << 32) / (reserve_0 as u128);
        let price_1 = ((reserve_0 as u128) << 32) / (reserve_1 as u128);
        env.storage().instance().set(
            &DataKey::Price0CumulativeLast,
            &acc_0.wrapping_add(price_0.saturating_mul(time_elapsed as u128)),
        );
        env.storage().instance().set(
            &DataKey::Price1CumulativeLast,
            &acc_1.wrapping_add(price_1.saturating_mul(time_elapsed as u128)),
        );
    }

    env.storage().instance().set(&DataKey::Reserve0, &balance_0);
    env.storage().instance().set(&DataKey::Reserve1, &balance_1);
    env.storage().instance().set(&DataKey::TimestampLast, &timestamp);
    events::emit_sync(env, balance_0, balance_1);
}

fn token_balance(env: &Env, token: &Address) -> i128 {
    TokenClient::new(env, token).balance(&env.current_contract_address())
}

fn token_pull(env: &Env, token: &Address, from: &Address, amount: i128) {
    TokenClient::new(env, token).transfer_from(
        &env.current_contract_address(),
        from,
        &env.current_contract_address(),
        &amount,
    );
}

fn token_push(env: &Env, token: &Address, to: &Address, amount: i128) {
    TokenClient::new(env, token).transfer(&env.current_contract_address(), to, &amount);
}

fn dead_address(env: &Env) -> Address {
    env.current_contract_address()
}

#[contract]
pub struct NodusAmm;

#[contractimpl]
impl NodusAmm {
    pub fn initialize(
        env: Env,
        token_0: Address,
        token_1: Address,
        fee_to_setter: Address,
    ) -> Result<(), Error> {
        if env.storage().instance().get::<DataKey, bool>(&DataKey::Initialized).unwrap_or(false) {
            return Err(Error::AlreadyInitialized);
        }
        if token_0 == token_1 { return Err(Error::InvalidTokenPair); }
        env.storage().instance().set(&DataKey::Token0, &token_0);
        env.storage().instance().set(&DataKey::Token1, &token_1);
        env.storage().instance().set(&DataKey::FeeToSetter, &fee_to_setter);
        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_BUMP);
        Ok(())
    }

    pub fn add_liquidity(
        env: Env,
        from: Address,
        to: Address,
        amount_0_desired: i128,
        amount_1_desired: i128,
        amount_0_min: i128,
        amount_1_min: i128,
        deadline: u64,
    ) -> Result<i128, Error> {
        require_initialized(&env)?;
        if env.ledger().timestamp() > deadline { return Err(Error::Expired); }
        lock(&env)?;
        env.storage().instance().extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_BUMP);

        from.require_auth();

        let token_0: Address = env.storage().instance().get(&DataKey::Token0).unwrap();
        let token_1: Address = env.storage().instance().get(&DataKey::Token1).unwrap();

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

        token_pull(&env, &token_0, &from, amount_0);
        token_pull(&env, &token_1, &from, amount_1);

        let total_supply = lp_token::total_supply(&env);

        let liquidity = if total_supply == 0 {
            let initial = liquidity_pool::calculate_initial_liquidity(amount_0, amount_1)
                .map_err(|e| { unlock(&env); e })?;
            lp_token::mint(&env, &dead_address(&env), math::MINIMUM_LIQUIDITY)
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

        events::emit_mint(&env, from, amount_0, amount_1);
        unlock(&env);
        Ok(liquidity)
    }

    pub fn remove_liquidity(
        env: Env,
        from: Address,
        to: Address,
        liquidity: i128,
        amount_0_min: i128,
        amount_1_min: i128,
        deadline: u64,
    ) -> Result<(i128, i128), Error> {
        require_initialized(&env)?;
        if env.ledger().timestamp() > deadline { return Err(Error::Expired); }
        lock(&env)?;
        env.storage().instance().extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_BUMP);

        from.require_auth();

        let token_0: Address = env.storage().instance().get(&DataKey::Token0).unwrap();
        let token_1: Address = env.storage().instance().get(&DataKey::Token1).unwrap();

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

        lp_token::burn(&env, &from, liquidity)
            .map_err(|e| { unlock(&env); e })?;

        token_push(&env, &token_0, &to, amount_0);
        token_push(&env, &token_1, &to, amount_1);

        let b0 = token_balance(&env, &token_0);
        let b1 = token_balance(&env, &token_1);
        update(&env, b0, b1, reserve_0, reserve_1);

        events::emit_burn(&env, from, amount_0, amount_1, to);
        unlock(&env);
        Ok((amount_0, amount_1))
    }

    pub fn swap(
        env: Env,
        to: Address,
        amount_0_out: i128,
        amount_1_out: i128,
    ) -> Result<(), Error> {
        require_initialized(&env)?;
        lock(&env)?;
        env.storage().instance().extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_BUMP);

        if amount_0_out == 0 && amount_1_out == 0 {
            unlock(&env);
            return Err(Error::InsufficientOutputAmount);
        }

        let token_0: Address = env.storage().instance().get(&DataKey::Token0).unwrap();
        let token_1: Address = env.storage().instance().get(&DataKey::Token1).unwrap();

        let reserve_0 = get_reserve_0(&env);
        let reserve_1 = get_reserve_1(&env);

        if amount_0_out >= reserve_0 || amount_1_out >= reserve_1 {
            unlock(&env);
            return Err(Error::InsufficientLiquidity);
        }

        if amount_0_out > 0 { token_push(&env, &token_0, &to, amount_0_out); }
        if amount_1_out > 0 { token_push(&env, &token_1, &to, amount_1_out); }

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
        require_initialized(&env)?;
        env.storage().instance().extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_BUMP);
        let token_0: Address = env.storage().instance().get(&DataKey::Token0).unwrap();
        let token_1: Address = env.storage().instance().get(&DataKey::Token1).unwrap();
        let b0 = token_balance(&env, &token_0);
        let b1 = token_balance(&env, &token_1);
        update(&env, b0, b1, get_reserve_0(&env), get_reserve_1(&env));
        Ok(())
    }

    pub fn get_reserves(env: Env) -> (i128, i128, u64) {
        (get_reserve_0(&env), get_reserve_1(&env), get_timestamp_last(&env))
    }

    pub fn get_price_cumulative(env: Env) -> (u128, u128) {
        (
            env.storage().instance().get(&DataKey::Price0CumulativeLast).unwrap_or(0u128),
            env.storage().instance().get(&DataKey::Price1CumulativeLast).unwrap_or(0u128),
        )
    }

    pub fn get_amount_out(_env: Env, amount_in: i128, reserve_in: i128, reserve_out: i128) -> Result<i128, Error> {
        math::get_amount_out(amount_in, reserve_in, reserve_out)
    }

    pub fn get_amount_in(_env: Env, amount_out: i128, reserve_in: i128, reserve_out: i128) -> Result<i128, Error> {
        math::get_amount_in(amount_out, reserve_in, reserve_out)
    }

    // ── High-level swap entrypoints ─────────────────────────────────────────

    pub fn swap_exact_tokens_for_tokens(
        env: Env,
        from: Address,
        to: Address,
        amount_in: i128,
        amount_out_min: i128,
        zero_for_one: bool,
        deadline: u64,
    ) -> Result<i128, Error> {
        require_initialized(&env)?;
        if env.ledger().timestamp() > deadline { return Err(Error::Expired); }
        lock(&env)?;
        env.storage().instance().extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_BUMP);

        from.require_auth();

        let token_0: Address = env.storage().instance().get(&DataKey::Token0).unwrap();
        let token_1: Address = env.storage().instance().get(&DataKey::Token1).unwrap();

        let reserve_0 = get_reserve_0(&env);
        let reserve_1 = get_reserve_1(&env);

        let (reserve_in, reserve_out, token_in, token_out) = if zero_for_one {
            (reserve_0, reserve_1, token_0.clone(), token_1.clone())
        } else {
            (reserve_1, reserve_0, token_1.clone(), token_0.clone())
        };

        let amount_out = math::get_amount_out(amount_in, reserve_in, reserve_out)
            .map_err(|e| { unlock(&env); e })?;

        if amount_out < amount_out_min {
            unlock(&env);
            return Err(Error::SlippageTooHigh);
        }

        token_pull(&env, &token_in, &from, amount_in);
        token_push(&env, &token_out, &to, amount_out);

        let b0 = token_balance(&env, &token_0);
        let b1 = token_balance(&env, &token_1);

        let (amount_0_in, amount_1_in, amount_0_out, amount_1_out) = if zero_for_one {
            (amount_in, 0i128, 0i128, amount_out)
        } else {
            (0i128, amount_in, amount_out, 0i128)
        };

        liquidity_pool::verify_k_invariant(b0, b1, amount_0_in, amount_1_in, reserve_0, reserve_1)
            .map_err(|e| { unlock(&env); e })?;

        update(&env, b0, b1, reserve_0, reserve_1);
        events::emit_swap(&env, from, amount_0_in, amount_1_in, amount_0_out, amount_1_out, to);
        unlock(&env);
        Ok(amount_out)
    }

    pub fn swap_tokens_for_exact_tokens(
        env: Env,
        from: Address,
        to: Address,
        amount_out: i128,
        amount_in_max: i128,
        zero_for_one: bool,
        deadline: u64,
    ) -> Result<i128, Error> {
        require_initialized(&env)?;
        if env.ledger().timestamp() > deadline { return Err(Error::Expired); }
        lock(&env)?;
        env.storage().instance().extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_BUMP);

        from.require_auth();

        let token_0: Address = env.storage().instance().get(&DataKey::Token0).unwrap();
        let token_1: Address = env.storage().instance().get(&DataKey::Token1).unwrap();

        let reserve_0 = get_reserve_0(&env);
        let reserve_1 = get_reserve_1(&env);

        let (reserve_in, reserve_out, token_in, token_out) = if zero_for_one {
            (reserve_0, reserve_1, token_0.clone(), token_1.clone())
        } else {
            (reserve_1, reserve_0, token_1.clone(), token_0.clone())
        };

        let amount_in = math::get_amount_in(amount_out, reserve_in, reserve_out)
            .map_err(|e| { unlock(&env); e })?;

        if amount_in > amount_in_max {
            unlock(&env);
            return Err(Error::SlippageTooHigh);
        }

        token_pull(&env, &token_in, &from, amount_in);
        token_push(&env, &token_out, &to, amount_out);

        let b0 = token_balance(&env, &token_0);
        let b1 = token_balance(&env, &token_1);

        let (amount_0_in, amount_1_in, amount_0_out, amount_1_out) = if zero_for_one {
            (amount_in, 0i128, 0i128, amount_out)
        } else {
            (0i128, amount_in, amount_out, 0i128)
        };

        liquidity_pool::verify_k_invariant(b0, b1, amount_0_in, amount_1_in, reserve_0, reserve_1)
            .map_err(|e| { unlock(&env); e })?;

        update(&env, b0, b1, reserve_0, reserve_1);
        events::emit_swap(&env, from, amount_0_in, amount_1_in, amount_0_out, amount_1_out, to);
        unlock(&env);
        Ok(amount_in)
    }

    pub fn get_spot_price(env: Env, zero_for_one: bool) -> Result<i128, Error> {
        require_initialized(&env)?;
        let reserve_0 = get_reserve_0(&env);
        let reserve_1 = get_reserve_1(&env);
        if zero_for_one {
            math::get_spot_price(reserve_0, reserve_1)
        } else {
            math::get_spot_price(reserve_1, reserve_0)
        }
    }

    pub fn get_price_impact(env: Env, amount_in: i128, zero_for_one: bool) -> Result<i128, Error> {
        require_initialized(&env)?;
        let reserve_0 = get_reserve_0(&env);
        let reserve_1 = get_reserve_1(&env);
        let (reserve_in, reserve_out) = if zero_for_one {
            (reserve_0, reserve_1)
        } else {
            (reserve_1, reserve_0)
        };
        let amount_out = math::get_amount_out(amount_in, reserve_in, reserve_out)?;
        math::calculate_price_impact(amount_in, amount_out, reserve_in, reserve_out)
    }

    // ── Protocol fee collector ──────────────────────────────────────────────

    pub fn set_fee_to(env: Env, caller: Address, new_fee_to: Option<Address>) -> Result<(), Error> {
        caller.require_auth();
        let setter: Address = env.storage().instance()
            .get(&DataKey::FeeToSetter)
            .ok_or(Error::NotInitialized)?;
        if caller != setter {
            return Err(Error::Unauthorized);
        }
        match &new_fee_to {
            Some(addr) => env.storage().instance().set(&DataKey::FeeTo, addr),
            None => env.storage().instance().remove(&DataKey::FeeTo),
        }
        Ok(())
    }

    pub fn set_fee_to_setter(env: Env, caller: Address, new_setter: Address) -> Result<(), Error> {
        caller.require_auth();
        let setter: Address = env.storage().instance()
            .get(&DataKey::FeeToSetter)
            .ok_or(Error::NotInitialized)?;
        if caller != setter {
            return Err(Error::Unauthorized);
        }
        env.storage().instance().set(&DataKey::FeeToSetter, &new_setter);
        Ok(())
    }

    pub fn fee_to(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::FeeTo)
    }

    pub fn fee_to_setter(env: Env) -> Result<Address, Error> {
        env.storage().instance()
            .get(&DataKey::FeeToSetter)
            .ok_or(Error::NotInitialized)
    }

    // ── LP token interface ──────────────────────────────────────────────────

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

    pub fn approve_lp(env: Env, owner: Address, spender: Address, amount: i128) -> Result<(), Error> {
        owner.require_auth();
        lp_token::approve(&env, &owner, &spender, amount)
    }

    pub fn lp_allowance(env: Env, owner: Address, spender: Address) -> i128 {
        lp_token::allowance(&env, &owner, &spender)
    }

    pub fn transfer_lp_from(
        env: Env,
        spender: Address,
        from: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), Error> {
        spender.require_auth();
        lp_token::transfer_from(&env, &spender, &from, &to, amount)
    }

    pub fn token_0(env: Env) -> Result<Address, Error> {
        require_initialized(&env)?;
        Ok(env.storage().instance().get(&DataKey::Token0).unwrap())
    }

    pub fn token_1(env: Env) -> Result<Address, Error> {
        require_initialized(&env)?;
        Ok(env.storage().instance().get(&DataKey::Token1).unwrap())
    }
}
