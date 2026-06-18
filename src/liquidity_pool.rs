use crate::{errors::Error, math};

pub fn calculate_liquidity_to_mint(
    amount_0: i128,
    amount_1: i128,
    reserve_0: i128,
    reserve_1: i128,
    total_supply: i128,
) -> Result<i128, Error> {
    Ok(math::min(
        amount_0.checked_mul(total_supply).ok_or(Error::Overflow)? / reserve_0,
        amount_1.checked_mul(total_supply).ok_or(Error::Overflow)? / reserve_1,
    ))
}

pub fn calculate_initial_liquidity(amount_0: i128, amount_1: i128) -> Result<i128, Error> {
    let product = amount_0.checked_mul(amount_1).ok_or(Error::Overflow)?;
    let liquidity = math::sqrt(product);
    if liquidity <= math::MINIMUM_LIQUIDITY {
        return Err(Error::InsufficientLiquidityMinted);
    }
    Ok(liquidity - math::MINIMUM_LIQUIDITY)
}

pub fn calculate_optimal_amounts(
    amount_0_desired: i128,
    amount_1_desired: i128,
    amount_0_min: i128,
    amount_1_min: i128,
    reserve_0: i128,
    reserve_1: i128,
) -> Result<(i128, i128), Error> {
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

pub fn calculate_withdrawal_amounts(
    liquidity: i128,
    reserve_0: i128,
    reserve_1: i128,
    total_supply: i128,
) -> Result<(i128, i128), Error> {
    let amount_0 = liquidity.checked_mul(reserve_0).ok_or(Error::Overflow)? / total_supply;
    let amount_1 = liquidity.checked_mul(reserve_1).ok_or(Error::Overflow)? / total_supply;
    Ok((amount_0, amount_1))
}

pub fn verify_k_invariant(
    balance_0: i128,
    balance_1: i128,
    amount_0_in: i128,
    amount_1_in: i128,
    reserve_0: i128,
    reserve_1: i128,
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
