use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    InsufficientLiquidity = 1,
    InsufficientLiquidityMinted = 2,
    InsufficientLiquidityBurned = 3,
    InsufficientOutputAmount = 4,
    ExcessiveInputAmount = 5,
    InvalidTokenPair = 6,
    ReentrancyDetected = 7,
    TransferFailed = 8,
    Expired = 9,
    KInvariantViolated = 10,
    ZeroAmount = 11,
    Overflow = 12,
    AlreadyInitialized = 13,
    NotInitialized = 14,
    SlippageTooHigh = 15,
    Unauthorized = 16,
    ContractPaused = 17,
    /// The token is not a member of the accepted canonical asset set (wrong
    /// symbol/name, or a reversed/unknown pair at initialization).
    UnsupportedAsset = 18,
    /// The address does not behave like a live SEP-41 token contract (not a
    /// contract at all, not initialized, or reverting metadata/balance calls).
    NotTokenContract = 19,
    /// The token reports decimals that differ from its canonical asset policy.
    WrongDecimals = 20,
    /// The post-deploy transfer/allowance canary amount is outside the
    /// permitted strict limits.
    InvalidCanaryAmount = 21,
    /// A post-deploy transfer/allowance compatibility check failed: one of
    /// the pool tokens did not behave as expected (transfer/transfer_from/
    /// approve reverted, or balances did not move exactly as requested).
    TokenCompatibilityFailed = 22,
    /// The post-deploy transfer/allowance compatibility canary has not
    /// completed successfully yet, and liquidity cannot be enabled until it
    /// passes.
    CanaryNotCompleted = 23,
}
