use soroban_sdk::{contracttype, Address};

#[contracttype]
pub enum DataKey {
    Token0,
    Token1,
    Reserve0,
    Reserve1,
    TimestampLast,
    Price0CumulativeLast,
    Price1CumulativeLast,
    KLast,
    LpTotalSupply,
    LpBalance(Address),
    /// Approved LP-token spending allowance: (owner, spender) → amount.
    LpAllowance(Address, Address),
    Locked,
    Initialized,
    FeeTo,
    FeeToSetter,
    Paused,
}
