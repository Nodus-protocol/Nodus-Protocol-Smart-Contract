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
}
