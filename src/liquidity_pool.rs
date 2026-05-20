use crate::{errors::Error, math};

/// Calculates the LP tokens to mint when adding liquidity to an existing pool.
pub fn calculate_liquidity_to_mint(
    amount_0: u128,
    amount_1: u128,
    reserve_0: u128,
    reserve_1: u128,
    total_supply: u128,
) -> Result<u128, Error> {
    Ok(math::min(
        amount_0
            .checked_mul(total_supply)
            .ok_or(Error::Overflow)?
            / reserve_0,
        amount_1
            .checked_mul(total_supply)
            .ok_or(Error::Overflow)?
            / reserve_1,
    ))
}

/// Calculates the LP tokens to mint for the initial deposit using the geometric mean.
pub fn calculate_initial_liquidity(amount_0: u128, amount_1: u128) -> Result<u128, Error> {
    let product = amount_0.checked_mul(amount_1).ok_or(Error::Overflow)?;
    let liquidity = math::sqrt(product);
    liquidity
        .checked_sub(math::MINIMUM_LIQUIDITY)
        .ok_or(Error::InsufficientLiquidityMinted)
}

/// Calculates optimal deposit amounts given desired amounts and current reserves.
///
/// Finds the largest valid pair (amount_0, amount_1) within the desired bounds
/// that preserves the existing reserve ratio.
pub fn calculate_optimal_amounts(
    amount_0_desired: u128,
    amount_1_desired: u128,
    amount_0_min: u128,
    amount_1_min: u128,
    reserve_0: u128,
    reserve_1: u128,
) -> Result<(u128, u128), Error> {
    let amount_1_optimal = amount_0_desired
        .checked_mul(reserve_1)
        .ok_or(Error::Overflow)?
        / reserve_0;

    if amount_1_optimal <= amount_1_desired {
        if amount_1_optimal < amount_1_min {
            return Err(Error::InsufficientLiquidity);
        }
        return Ok((amount_0_desired, amount_1_optimal));
    }

    let amount_0_optimal = amount_1_desired
        .checked_mul(reserve_0)
        .ok_or(Error::Overflow)?
        / reserve_1;

    if amount_0_optimal < amount_0_min {
        return Err(Error::InsufficientLiquidity);
    }
    Ok((amount_0_optimal, amount_1_desired))
}

/// Calculates token amounts returned when burning LP tokens.
pub fn calculate_withdrawal_amounts(
    liquidity: u128,
    reserve_0: u128,
    reserve_1: u128,
    total_supply: u128,
) -> Result<(u128, u128), Error> {
    let amount_0 = liquidity
        .checked_mul(reserve_0)
        .ok_or(Error::Overflow)?
        / total_supply;
    let amount_1 = liquidity
        .checked_mul(reserve_1)
        .ok_or(Error::Overflow)?
        / total_supply;
    Ok((amount_0, amount_1))
}

/// Verifies the constant-product K invariant holds after a swap.
///
/// Checks: (balance_0 * 1000 - amount_0_in * 3) * (balance_1 * 1000 - amount_1_in * 3)
///         >= reserve_0 * reserve_1 * 1_000_000
pub fn verify_k_invariant(
    balance_0: u128,
    balance_1: u128,
    amount_0_in: u128,
    amount_1_in: u128,
    reserve_0: u128,
    reserve_1: u128,
) -> Result<(), Error> {
    let lhs_0 = balance_0
        .checked_mul(1_000)
        .ok_or(Error::Overflow)?
        .checked_sub(amount_0_in.checked_mul(3).ok_or(Error::Overflow)?)
        .ok_or(Error::KInvariantViolated)?;
    let lhs_1 = balance_1
        .checked_mul(1_000)
        .ok_or(Error::Overflow)?
        .checked_sub(amount_1_in.checked_mul(3).ok_or(Error::Overflow)?)
        .ok_or(Error::KInvariantViolated)?;
    let lhs = lhs_0.checked_mul(lhs_1).ok_or(Error::Overflow)?;
    let rhs = reserve_0
        .checked_mul(reserve_1)
        .ok_or(Error::Overflow)?
        .checked_mul(1_000_000)
        .ok_or(Error::Overflow)?;

    if lhs < rhs {
        return Err(Error::KInvariantViolated);
    }
    Ok(())
}
