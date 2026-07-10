use soroban_sdk::{contracttype, Address};

#[contracttype]
pub enum DataKey {
    Pool,
    Initialized,
    Name,
    Symbol,
    Decimals,
    TotalSupply,
    Balance(Address),
    /// (owner, spender) -> approved amount.
    Allowance(Address, Address),
}
