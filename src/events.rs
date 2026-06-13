#![no_std]
use soroban_sdk::{contractevent, Address, Env};

#[contractevent]
pub struct MintEvent {
    pub sender: Address,
    pub amount_0: i128,
    pub amount_1: i128,
}

#[contractevent]
pub struct BurnEvent {
    pub sender: Address,
    pub amount_0: i128,
    pub amount_1: i128,
    pub to: Address,
}

#[contractevent]
pub struct SwapEvent {
    pub sender: Address,
    pub amount_0_in: i128,
    pub amount_1_in: i128,
    pub amount_0_out: i128,
    pub amount_1_out: i128,
    pub to: Address,
}

#[contractevent]
pub struct SyncEvent {
    pub reserve_0: i128,
    pub reserve_1: i128,
}

pub fn emit_mint(env: &Env, sender: Address, amount_0: i128, amount_1: i128) {
    MintEvent { sender, amount_0, amount_1 }.emit(env);
}

pub fn emit_burn(env: &Env, sender: Address, amount_0: i128, amount_1: i128, to: Address) {
    BurnEvent { sender, amount_0, amount_1, to }.emit(env);
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
    SwapEvent { sender, amount_0_in, amount_1_in, amount_0_out, amount_1_out, to }.emit(env);
}

pub fn emit_sync(env: &Env, reserve_0: i128, reserve_1: i128) {
    SyncEvent { reserve_0, reserve_1 }.emit(env);
}
