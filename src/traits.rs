use crate::errors::Error;
use soroban_sdk::Address;

pub trait IAmmPool {
    fn add_liquidity(
        &self,
        amount_0_desired: i128,
        amount_1_desired: i128,
        amount_0_min: i128,
        amount_1_min: i128,
        to: Address,
        deadline: u64,
    ) -> Result<i128, Error>;

    fn remove_liquidity(
        &self,
        liquidity: i128,
        amount_0_min: i128,
        amount_1_min: i128,
        to: Address,
        deadline: u64,
    ) -> Result<(i128, i128), Error>;

    fn swap(&self, amount_0_out: i128, amount_1_out: i128, to: Address) -> Result<(), Error>;

    fn sync(&self) -> Result<(), Error>;

    fn get_reserves(&self) -> (i128, i128, u64);

    fn get_amount_out(
        &self,
        amount_in: i128,
        reserve_in: i128,
        reserve_out: i128,
    ) -> Result<i128, Error>;

    fn get_amount_in(
        &self,
        amount_out: i128,
        reserve_in: i128,
        reserve_out: i128,
    ) -> Result<i128, Error>;

    fn swap_exact_tokens_for_tokens(
        &self,
        amount_in: i128,
        amount_out_min: i128,
        zero_for_one: bool,
        to: Address,
        deadline: u64,
    ) -> Result<i128, Error>;

    fn swap_tokens_for_exact_tokens(
        &self,
        amount_out: i128,
        amount_in_max: i128,
        zero_for_one: bool,
        to: Address,
        deadline: u64,
    ) -> Result<i128, Error>;

    fn get_spot_price(&self, zero_for_one: bool) -> Result<i128, Error>;

    fn get_price_impact(&self, amount_in: i128, zero_for_one: bool) -> Result<i128, Error>;
}
