#![no_std]
#![allow(deprecated)]
use soroban_sdk::{contracttype, symbol_short, Address, Env};

#[contracttype]
pub struct MintEvent {
    pub sender: Address,
    pub amount_0: i128,
    pub amount_1: i128,
}

#[contracttype]
pub struct BurnEvent {
    pub sender: Address,
    pub amount_0: i128,
    pub amount_1: i128,
    pub to: Address,
}

#[contracttype]
pub struct SwapEvent {
    pub sender: Address,
    pub amount_0_in: i128,
    pub amount_1_in: i128,
    pub amount_0_out: i128,
    pub amount_1_out: i128,
    pub to: Address,
}

#[contracttype]
pub struct SyncEvent {
    pub reserve_0: i128,
    pub reserve_1: i128,
}

pub fn emit_mint(env: &Env, sender: Address, amount_0: i128, amount_1: i128) {
    env.events().publish((symbol_short!("mint"),), MintEvent { sender, amount_0, amount_1 });
}

pub fn emit_burn(env: &Env, sender: Address, amount_0: i128, amount_1: i128, to: Address) {
    env.events().publish((symbol_short!("burn"),), BurnEvent { sender, amount_0, amount_1, to });
}

pub fn emit_swap(
    env: &Env,
    sender: Address,
    amount_0_in: i128,
    amount_1_in: i128,
    amount_0_out: i128,
    amount_1_out: i128,
    to: Address,
) {
    env.events().publish(
        (symbol_short!("swap"),),
        SwapEvent { sender, amount_0_in, amount_1_in, amount_0_out, amount_1_out, to },
    );
}

pub fn emit_sync(env: &Env, reserve_0: i128, reserve_1: i128) {
    env.events().publish((symbol_short!("sync"),), SyncEvent { reserve_0, reserve_1 });
}
