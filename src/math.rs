use crate::errors::Error;

pub const FEE_NUMERATOR: u128 = 997;
pub const FEE_DENOMINATOR: u128 = 1_000;
pub const MINIMUM_LIQUIDITY: u128 = 1_000;

/// Returns the maximum output amount for a given input, applying the 0.3% fee.
///
/// Formula: amount_out = (reserve_out * amount_in * 997) / (reserve_in * 1000 + amount_in * 997)
pub fn get_amount_out(
    amount_in: u128,
    reserve_in: u128,
    reserve_out: u128,
) -> Result<u128, Error> {
    if amount_in == 0 {
        return Err(Error::ZeroAmount);
    }
    if reserve_in == 0 || reserve_out == 0 {
        return Err(Error::InsufficientLiquidity);
    }
    let amount_in_with_fee = amount_in
        .checked_mul(FEE_NUMERATOR)
        .ok_or(Error::Overflow)?;
    let numerator = amount_in_with_fee
        .checked_mul(reserve_out)
        .ok_or(Error::Overflow)?;
    let denominator = reserve_in
        .checked_mul(FEE_DENOMINATOR)
        .ok_or(Error::Overflow)?
        .checked_add(amount_in_with_fee)
        .ok_or(Error::Overflow)?;
    Ok(numerator / denominator)
}

/// Returns the minimum input amount required to receive a given output amount.
///
/// Formula: amount_in = (reserve_in * amount_out * 1000) / ((reserve_out - amount_out) * 997) + 1
pub fn get_amount_in(
    amount_out: u128,
    reserve_in: u128,
    reserve_out: u128,
) -> Result<u128, Error> {
    if amount_out == 0 {
        return Err(Error::ZeroAmount);
    }
    if reserve_in == 0 || reserve_out == 0 {
        return Err(Error::InsufficientLiquidity);
    }
    let numerator = reserve_in
        .checked_mul(amount_out)
        .ok_or(Error::Overflow)?
        .checked_mul(FEE_DENOMINATOR)
        .ok_or(Error::Overflow)?;
    let denominator = reserve_out
        .checked_sub(amount_out)
        .ok_or(Error::InsufficientLiquidity)?
        .checked_mul(FEE_NUMERATOR)
        .ok_or(Error::Overflow)?;
    (numerator / denominator)
        .checked_add(1)
        .ok_or(Error::Overflow)
}

/// Integer square root via Newton's method.
pub fn sqrt(y: u128) -> u128 {
    if y < 4 {
        return if y == 0 { 0 } else { 1 };
    }
    let mut z = y;
    let mut x = y / 2 + 1;
    while x < z {
        z = x;
        x = (y / x + x) / 2;
    }
    z
}

pub fn min(a: u128, b: u128) -> u128 {
    if a < b { a } else { b }
}
