#![no_std]
use soroban_sdk::{contracttype, Address};

#[contracttype]
pub enum DataKey {
    Token0,
    Token1,
    Reserve0,
    Reserve1,
    TimestampLast,
    KLast,
    LpTotalSupply,
    LpBalance(Address),
    Locked,
    Initialized,
}
