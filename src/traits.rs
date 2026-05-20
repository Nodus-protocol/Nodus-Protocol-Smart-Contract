use ink::primitives::AccountId;
use crate::errors::Error;

#[ink::trait_definition]
pub trait ILiquidityPool {
    #[ink(message)]
    fn add_liquidity(
        &mut self,
        amount_0_desired: u128,
        amount_1_desired: u128,
        amount_0_min: u128,
        amount_1_min: u128,
        to: AccountId,
        deadline: u64,
    ) -> Result<u128, Error>;

    #[ink(message)]
    fn remove_liquidity(
        &mut self,
        liquidity: u128,
        amount_0_min: u128,
        amount_1_min: u128,
        to: AccountId,
        deadline: u64,
    ) -> Result<(u128, u128), Error>;

    #[ink(message)]
    fn swap(
        &mut self,
        amount_0_out: u128,
        amount_1_out: u128,
        to: AccountId,
    ) -> Result<(), Error>;

    #[ink(message)]
    fn sync(&mut self) -> Result<(), Error>;

    #[ink(message)]
    fn get_reserves(&self) -> (u128, u128, u64);

    #[ink(message)]
    fn get_amount_out(
        &self,
        amount_in: u128,
        reserve_in: u128,
        reserve_out: u128,
    ) -> Result<u128, Error>;

    #[ink(message)]
    fn get_amount_in(
        &self,
        amount_out: u128,
        reserve_in: u128,
        reserve_out: u128,
    ) -> Result<u128, Error>;
}

#[ink::trait_definition]
pub trait IPSP22 {
    #[ink(message)]
    fn total_supply(&self) -> u128;

    #[ink(message)]
    fn balance_of(&self, owner: AccountId) -> u128;

    #[ink(message)]
    fn transfer(&mut self, to: AccountId, value: u128, data: ink::prelude::vec::Vec<u8>) -> Result<(), Error>;

    #[ink(message)]
    fn transfer_from(
        &mut self,
        from: AccountId,
        to: AccountId,
        value: u128,
        data: ink::prelude::vec::Vec<u8>,
    ) -> Result<(), Error>;

    #[ink(message)]
    fn approve(&mut self, spender: AccountId, value: u128) -> Result<(), Error>;

    #[ink(message)]
    fn allowance(&self, owner: AccountId, spender: AccountId) -> u128;
}
