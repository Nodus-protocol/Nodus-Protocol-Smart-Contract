use scale::{Decode, Encode};

#[derive(Debug, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    InsufficientLiquidity,
    InsufficientLiquidityMinted,
    InsufficientLiquidityBurned,
    InsufficientOutputAmount,
    ExcessiveInputAmount,
    InvalidTokenPair,
    ReentrancyDetected,
    TransferFailed,
    Expired,
    KInvariantViolated,
    ZeroAmount,
    Overflow,
    /// Swap output fell below the caller-specified minimum (slippage guard).
    SlippageTooHigh,
    /// Caller is not the authorised admin for this operation.
    Unauthorized,
}
