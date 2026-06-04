#![no_std]
use soroban_sdk::{Address, Env};
use crate::{errors::Error, storage::DataKey};

const TTL_THRESHOLD: u32 = 100;
const TTL_BUMP: u32 = 500;

fn bump_balance(env: &Env, key: &DataKey) {
    env.storage().persistent().extend_ttl(key, TTL_THRESHOLD, TTL_BUMP);
}

pub fn total_supply(env: &Env) -> i128 {
    env.storage().instance().get(&DataKey::LpTotalSupply).unwrap_or(0i128)
}

pub fn balance_of(env: &Env, owner: &Address) -> i128 {
    let key = DataKey::LpBalance(owner.clone());
    let bal = env.storage().persistent().get(&key).unwrap_or(0i128);
    if bal > 0 { bump_balance(env, &key); }
    bal
}

pub fn allowance(env: &Env, owner: &Address, spender: &Address) -> i128 {
    let key = DataKey::LpAllowance(owner.clone(), spender.clone());
    env.storage().persistent().get(&key).unwrap_or(0i128)
}

pub fn approve(env: &Env, owner: &Address, spender: &Address, amount: i128) -> Result<(), Error> {
    if amount < 0 { return Err(Error::ZeroAmount); }
    let key = DataKey::LpAllowance(owner.clone(), spender.clone());
    env.storage().persistent().set(&key, &amount);
    env.storage().persistent().extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
    Ok(())
}

pub fn mint(env: &Env, to: &Address, amount: i128) -> Result<(), Error> {
    if amount <= 0 { return Err(Error::ZeroAmount); }
    let key = DataKey::LpBalance(to.clone());
    let new_bal = balance_of(env, to).checked_add(amount).ok_or(Error::Overflow)?;
    let new_supply = total_supply(env).checked_add(amount).ok_or(Error::Overflow)?;
    env.storage().persistent().set(&key, &new_bal);
    env.storage().persistent().extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
    env.storage().instance().set(&DataKey::LpTotalSupply, &new_supply);
    Ok(())
}

pub fn burn(env: &Env, from: &Address, amount: i128) -> Result<(), Error> {
    if amount <= 0 { return Err(Error::ZeroAmount); }
    let key = DataKey::LpBalance(from.clone());
    let bal = balance_of(env, from);
    if bal < amount { return Err(Error::InsufficientLiquidityBurned); }
    let supply = total_supply(env);
    env.storage().persistent().set(&key, &(bal - amount));
    env.storage().persistent().extend_ttl(&key, TTL_THRESHOLD, TTL_BUMP);
    env.storage().instance().set(&DataKey::LpTotalSupply, &(supply - amount));
    Ok(())
}

pub fn transfer(env: &Env, from: &Address, to: &Address, amount: i128) -> Result<(), Error> {
    if amount <= 0 { return Err(Error::ZeroAmount); }
    let from_bal = balance_of(env, from);
    if from_bal < amount { return Err(Error::InsufficientLiquidityBurned); }
    let to_bal = balance_of(env, to);
    let to_new = to_bal.checked_add(amount).ok_or(Error::Overflow)?;
    let from_key = DataKey::LpBalance(from.clone());
    let to_key = DataKey::LpBalance(to.clone());
    env.storage().persistent().set(&from_key, &(from_bal - amount));
    env.storage().persistent().extend_ttl(&from_key, TTL_THRESHOLD, TTL_BUMP);
    env.storage().persistent().set(&to_key, &to_new);
    env.storage().persistent().extend_ttl(&to_key, TTL_THRESHOLD, TTL_BUMP);
    Ok(())
}

/// Transfer on behalf of `from` using a pre-approved allowance.
pub fn transfer_from(
    env: &Env,
    spender: &Address,
    from: &Address,
    to: &Address,
    amount: i128,
) -> Result<(), Error> {
    if amount <= 0 { return Err(Error::ZeroAmount); }
    let allowed = allowance(env, from, spender);
    if allowed < amount { return Err(Error::InsufficientLiquidity); }
    // Deduct allowance first (checks-effects-interactions)
    approve(env, from, spender, allowed - amount)?;
    transfer(env, from, to, amount)
}
