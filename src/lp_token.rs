#![no_std]
use soroban_sdk::{Address, Env};
use crate::{errors::Error, storage::DataKey};

pub fn total_supply(env: &Env) -> i128 {
    env.storage().instance().get(&DataKey::LpTotalSupply).unwrap_or(0i128)
}

pub fn balance_of(env: &Env, owner: &Address) -> i128 {
    env.storage().instance().get(&DataKey::LpBalance(owner.clone())).unwrap_or(0i128)
}

pub fn mint(env: &Env, to: &Address, amount: i128) -> Result<(), Error> {
    if amount <= 0 { return Err(Error::ZeroAmount); }
    let new_bal = balance_of(env, to).checked_add(amount).ok_or(Error::Overflow)?;
    let new_supply = total_supply(env).checked_add(amount).ok_or(Error::Overflow)?;
    env.storage().instance().set(&DataKey::LpBalance(to.clone()), &new_bal);
    env.storage().instance().set(&DataKey::LpTotalSupply, &new_supply);
    Ok(())
}

pub fn burn(env: &Env, from: &Address, amount: i128) -> Result<(), Error> {
    if amount <= 0 { return Err(Error::ZeroAmount); }
    let bal = balance_of(env, from);
    if bal < amount { return Err(Error::InsufficientLiquidityBurned); }
    let supply = total_supply(env);
    env.storage().instance().set(&DataKey::LpBalance(from.clone()), &(bal - amount));
    env.storage().instance().set(&DataKey::LpTotalSupply, &(supply - amount));
    Ok(())
}

pub fn transfer(env: &Env, from: &Address, to: &Address, amount: i128) -> Result<(), Error> {
    if amount <= 0 { return Err(Error::ZeroAmount); }
    let from_bal = balance_of(env, from);
    if from_bal < amount { return Err(Error::InsufficientLiquidityBurned); }
    let to_bal = balance_of(env, to);
    env.storage().instance().set(&DataKey::LpBalance(from.clone()), &(from_bal - amount));
    env.storage().instance().set(&DataKey::LpBalance(to.clone()), &(to_bal.checked_add(amount).ok_or(Error::Overflow)?));
    Ok(())
}
