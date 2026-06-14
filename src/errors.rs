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
}
