#![no_std]
use crate::errors::Error;

pub const FEE_NUMERATOR: i128 = 997;
pub const FEE_DENOMINATOR: i128 = 1_000;
pub const MINIMUM_LIQUIDITY: i128 = 1_000;

pub fn get_amount_out(amount_in: i128, reserve_in: i128, reserve_out: i128) -> Result<i128, Error> {
    if amount_in == 0 { return Err(Error::ZeroAmount); }
    if reserve_in == 0 || reserve_out == 0 { return Err(Error::InsufficientLiquidity); }
    let fee_in = amount_in.checked_mul(FEE_NUMERATOR).ok_or(Error::Overflow)?;
    let numerator = fee_in.checked_mul(reserve_out).ok_or(Error::Overflow)?;
    let denominator = reserve_in
        .checked_mul(FEE_DENOMINATOR).ok_or(Error::Overflow)?
        .checked_add(fee_in).ok_or(Error::Overflow)?;
    Ok(numerator / denominator)
}

pub fn get_amount_in(amount_out: i128, reserve_in: i128, reserve_out: i128) -> Result<i128, Error> {
    if amount_out == 0 { return Err(Error::ZeroAmount); }
    if reserve_in == 0 || reserve_out == 0 { return Err(Error::InsufficientLiquidity); }
    let numerator = reserve_in
        .checked_mul(amount_out).ok_or(Error::Overflow)?
        .checked_mul(FEE_DENOMINATOR).ok_or(Error::Overflow)?;
    let denominator = reserve_out
        .checked_sub(amount_out).ok_or(Error::InsufficientLiquidity)?
        .checked_mul(FEE_NUMERATOR).ok_or(Error::Overflow)?;
    (numerator / denominator).checked_add(1).ok_or(Error::Overflow)
}

pub fn sqrt(y: i128) -> i128 {
    if y < 4 { return if y == 0 { 0 } else { 1 }; }
    let mut z = y;
    let mut x = y / 2 + 1;
    while x < z { z = x; x = (y / x + x) / 2; }
    z
}

pub fn min(a: i128, b: i128) -> i128 {
    if a < b { a } else { b }
}
