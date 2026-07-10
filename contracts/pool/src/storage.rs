use soroban_sdk::contracttype;

#[contracttype]
pub enum DataKey {
    Token0,
    Token1,
    /// The standalone SEP-41 LP token contract for this pool. LP token
    /// balances/allowances/supply all live over there now, not here.
    LpToken,
    Reserve0,
    Reserve1,
    TimestampLast,
    Price0CumulativeLast,
    Price1CumulativeLast,
    KLast,
    Locked,
    Initialized,
    FeeTo,
    FeeToSetter,
    Paused,
}
