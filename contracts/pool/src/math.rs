use crate::errors::Error;

pub const FEE_NUMERATOR: i128 = 997;
pub const FEE_DENOMINATOR: i128 = 1_000;
pub const MINIMUM_LIQUIDITY: i128 = 1_000;

pub fn get_amount_out(amount_in: i128, reserve_in: i128, reserve_out: i128) -> Result<i128, Error> {
    if amount_in == 0 {
        return Err(Error::ZeroAmount);
    }
    if reserve_in == 0 || reserve_out == 0 {
        return Err(Error::InsufficientLiquidity);
    }
    let fee_in = amount_in
        .checked_mul(FEE_NUMERATOR)
        .ok_or(Error::Overflow)?;
    let numerator = fee_in.checked_mul(reserve_out).ok_or(Error::Overflow)?;
    let denominator = reserve_in
        .checked_mul(FEE_DENOMINATOR)
        .ok_or(Error::Overflow)?
        .checked_add(fee_in)
        .ok_or(Error::Overflow)?;
    let amount_out = numerator / denominator;
    if amount_out == 0 {
        return Err(Error::InsufficientOutputAmount);
    }
    Ok(amount_out)
}

pub fn get_amount_in(amount_out: i128, reserve_in: i128, reserve_out: i128) -> Result<i128, Error> {
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

pub fn sqrt(y: i128) -> i128 {
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

pub fn min(a: i128, b: i128) -> i128 {
    if a < b {
        a
    } else {
        b
    }
}

/// Spot price of `token_in` denominated in `token_out`, scaled by 1e18.
///
/// Returns `(reserve_out * 1e18) / reserve_in` — the marginal price before fees.
/// Useful for displaying prices on a frontend without simulating a trade.
pub fn get_spot_price(reserve_in: i128, reserve_out: i128) -> Result<i128, crate::errors::Error> {
    if reserve_in <= 0 || reserve_out <= 0 {
        return Err(crate::errors::Error::InsufficientLiquidity);
    }
    const SCALE: i128 = 1_000_000_000_000_000_000; // 1e18
    reserve_out
        .checked_mul(SCALE)
        .ok_or(crate::errors::Error::Overflow)
        .map(|n| n / reserve_in)
}

/// Price impact of a swap in basis points (1 bp = 0.01 %).
///
/// Compares the marginal spot price against the effective execution price.
/// A value of 100 means 1 % slippage from market price.
pub fn calculate_price_impact(
    amount_in: i128,
    amount_out: i128,
    reserve_in: i128,
    reserve_out: i128,
) -> Result<i128, crate::errors::Error> {
    if reserve_in <= 0 || reserve_out <= 0 || amount_in <= 0 {
        return Err(crate::errors::Error::InsufficientLiquidity);
    }
    const BPS: i128 = 10_000;
    let ideal = reserve_out
        .checked_mul(amount_in)
        .ok_or(crate::errors::Error::Overflow)?;
    let actual = amount_out
        .checked_mul(reserve_in)
        .ok_or(crate::errors::Error::Overflow)?;
    if actual >= ideal {
        return Ok(0);
    }
    let diff = ideal - actual;
    diff.checked_mul(BPS)
        .ok_or(crate::errors::Error::Overflow)
        .map(|n| n / ideal)
}
