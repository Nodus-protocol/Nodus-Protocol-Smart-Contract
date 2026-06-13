#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub mod errors;
pub mod events;
pub mod lp_token;
pub mod liquidity_pool;
pub mod math;
pub mod reentrancy_guard;
pub mod traits;

pub use errors::Error;
pub use events::{Burn, Mint, Swap, Sync};

#[ink::contract]
mod pool {
    use super::{
        errors::Error,
        events::{Burn, Mint, Swap, Sync},
        lp_token::LPTokenRef,
        liquidity_pool as pool_math,
        math::{self, MINIMUM_LIQUIDITY},
        reentrancy_guard::ReentrancyGuard,
        traits::ILiquidityPool,
    };
    use ink::env::call::{build_call, ExecutionInput, Selector};
    use ink::env::DefaultEnvironment;
    use ink::prelude::vec::Vec;

    #[ink(storage)]
    pub struct LiquidityPool {
        token_0: AccountId,
        token_1: AccountId,
        reserve_0: u128,
        reserve_1: u128,
        block_timestamp_last: u64,
        price_0_cumulative_last: u128,
        price_1_cumulative_last: u128,
        k_last: u128,
        lp_token: AccountId,
        locked: bool,
    }

    impl ReentrancyGuard for LiquidityPool {
        fn is_locked(&self) -> bool {
            self.locked
        }
        fn set_locked(&mut self, locked: bool) {
            self.locked = locked;
        }
    }

    impl LiquidityPool {
        #[ink(constructor)]
        pub fn new(
            token_0: AccountId,
            token_1: AccountId,
            lp_token: AccountId,
        ) -> Self {
            assert_ne!(token_0, token_1, "identical token addresses");
            Self {
                token_0,
                token_1,
                reserve_0: 0,
                reserve_1: 0,
                block_timestamp_last: 0,
                price_0_cumulative_last: 0,
                price_1_cumulative_last: 0,
                k_last: 0,
                lp_token,
                locked: false,
            }
        }

        fn update(
            &mut self,
            balance_0: u128,
            balance_1: u128,
            reserve_0: u128,
            reserve_1: u128,
        ) {
            let block_timestamp = self.env().block_timestamp();
            let time_elapsed = block_timestamp.saturating_sub(self.block_timestamp_last);

            if time_elapsed > 0 && reserve_0 > 0 && reserve_1 > 0 {
                self.price_0_cumulative_last = self
                    .price_0_cumulative_last
                    .saturating_add(
                        (reserve_1 / reserve_0).saturating_mul(time_elapsed as u128),
                    );
                self.price_1_cumulative_last = self
                    .price_1_cumulative_last
                    .saturating_add(
                        (reserve_0 / reserve_1).saturating_mul(time_elapsed as u128),
                    );
            }

            self.reserve_0 = balance_0;
            self.reserve_1 = balance_1;
            self.block_timestamp_last = block_timestamp;

            self.env().emit_event(Sync {
                reserve_0: self.reserve_0,
                reserve_1: self.reserve_1,
            });
        }

        fn token_balance(&self, token: AccountId) -> u128 {
            build_call::<DefaultEnvironment>()
                .call(token)
                .exec_input(
                    ExecutionInput::new(Selector::new(
                        ink::selector_bytes!("PSP22::balance_of"),
                    ))
                    .push_arg(self.env().account_id()),
                )
                .returns::<u128>()
                .invoke()
        }

        fn token_transfer_from(
            &self,
            token: AccountId,
            from: AccountId,
            to: AccountId,
            amount: u128,
        ) -> Result<(), Error> {
            build_call::<DefaultEnvironment>()
                .call(token)
                .exec_input(
                    ExecutionInput::new(Selector::new(
                        ink::selector_bytes!("PSP22::transfer_from"),
                    ))
                    .push_arg(from)
                    .push_arg(to)
                    .push_arg(amount)
                    .push_arg::<Vec<u8>>(Vec::new()),
                )
                .returns::<Result<(), Error>>()
                .invoke()
        }

        fn token_transfer(
            &self,
            token: AccountId,
            to: AccountId,
            amount: u128,
        ) -> Result<(), Error> {
            build_call::<DefaultEnvironment>()
                .call(token)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!("PSP22::transfer")))
                        .push_arg(to)
                        .push_arg(amount)
                        .push_arg::<Vec<u8>>(Vec::new()),
                )
                .returns::<Result<(), Error>>()
                .invoke()
        }
    }

    impl ILiquidityPool for LiquidityPool {
        #[ink(message)]
        fn add_liquidity(
            &mut self,
            amount_0_desired: u128,
            amount_1_desired: u128,
            amount_0_min: u128,
            amount_1_min: u128,
            to: AccountId,
            deadline: u64,
        ) -> Result<u128, Error> {
            self.lock()?;

            if self.env().block_timestamp() > deadline {
                self.unlock();
                return Err(Error::Expired);
            }

            let lp = LPTokenRef::new(self.lp_token);
            let total_supply = lp.total_supply();

            let (amount_0, amount_1) = if total_supply == 0 {
                (amount_0_desired, amount_1_desired)
            } else {
                pool_math::calculate_optimal_amounts(
                    amount_0_desired,
                    amount_1_desired,
                    amount_0_min,
                    amount_1_min,
                    self.reserve_0,
                    self.reserve_1,
                )
                .map_err(|e| { self.unlock(); e })?
            };

            let caller = self.env().caller();
            let pool_id = self.env().account_id();

            self.token_transfer_from(self.token_0, caller, pool_id, amount_0)
                .map_err(|e| { self.unlock(); e })?;
            self.token_transfer_from(self.token_1, caller, pool_id, amount_1)
                .map_err(|e| { self.unlock(); e })?;

            let liquidity = if total_supply == 0 {
                let initial = pool_math::calculate_initial_liquidity(amount_0, amount_1)
                    .map_err(|e| { self.unlock(); e })?;
                lp.mint(AccountId::from([0u8; 32]), MINIMUM_LIQUIDITY)
                    .map_err(|e| { self.unlock(); e })?;
                initial
            } else {
                pool_math::calculate_liquidity_to_mint(
                    amount_0,
                    amount_1,
                    self.reserve_0,
                    self.reserve_1,
                    total_supply,
                )
                .map_err(|e| { self.unlock(); e })?
            };

            if liquidity == 0 {
                self.unlock();
                return Err(Error::InsufficientLiquidityMinted);
            }

            lp.mint(to, liquidity)
                .map_err(|e| { self.unlock(); e })?;

            let b0 = self.token_balance(self.token_0);
            let b1 = self.token_balance(self.token_1);
            self.update(b0, b1, self.reserve_0, self.reserve_1);

            self.env().emit_event(Mint {
                sender: caller,
                amount_0,
                amount_1,
            });

            self.unlock();
            Ok(liquidity)
        }

        #[ink(message)]
        fn remove_liquidity(
            &mut self,
            liquidity: u128,
            amount_0_min: u128,
            amount_1_min: u128,
            to: AccountId,
            deadline: u64,
        ) -> Result<(u128, u128), Error> {
            self.lock()?;

            if self.env().block_timestamp() > deadline {
                self.unlock();
                return Err(Error::Expired);
            }

            let lp = LPTokenRef::new(self.lp_token);
            let total_supply = lp.total_supply();

            let (amount_0, amount_1) = pool_math::calculate_withdrawal_amounts(
                liquidity,
                self.reserve_0,
                self.reserve_1,
                total_supply,
            )
            .map_err(|e| { self.unlock(); e })?;

            if amount_0 < amount_0_min || amount_1 < amount_1_min {
                self.unlock();
                return Err(Error::InsufficientLiquidityBurned);
            }

            let caller = self.env().caller();
            lp.burn(caller, liquidity)
                .map_err(|e| { self.unlock(); e })?;

            self.token_transfer(self.token_0, to, amount_0)
                .map_err(|e| { self.unlock(); e })?;
            self.token_transfer(self.token_1, to, amount_1)
                .map_err(|e| { self.unlock(); e })?;

            let b0 = self.token_balance(self.token_0);
            let b1 = self.token_balance(self.token_1);
            self.update(b0, b1, self.reserve_0, self.reserve_1);

            self.env().emit_event(Burn {
                sender: caller,
                amount_0,
                amount_1,
                to,
            });

            self.unlock();
            Ok((amount_0, amount_1))
        }

        #[ink(message)]
        fn swap(
            &mut self,
            amount_0_out: u128,
            amount_1_out: u128,
            to: AccountId,
        ) -> Result<(), Error> {
            self.lock()?;

            if amount_0_out == 0 && amount_1_out == 0 {
                self.unlock();
                return Err(Error::InsufficientOutputAmount);
            }
            if amount_0_out >= self.reserve_0 || amount_1_out >= self.reserve_1 {
                self.unlock();
                return Err(Error::InsufficientLiquidity);
            }

            // Optimistically transfer output tokens before verifying inputs (CEI pattern).
            if amount_0_out > 0 {
                self.token_transfer(self.token_0, to, amount_0_out)
                    .map_err(|e| { self.unlock(); e })?;
            }
            if amount_1_out > 0 {
                self.token_transfer(self.token_1, to, amount_1_out)
                    .map_err(|e| { self.unlock(); e })?;
            }

            let balance_0 = self.token_balance(self.token_0);
            let balance_1 = self.token_balance(self.token_1);

            let amount_0_in = balance_0
                .saturating_sub(self.reserve_0.saturating_sub(amount_0_out));
            let amount_1_in = balance_1
                .saturating_sub(self.reserve_1.saturating_sub(amount_1_out));

            if amount_0_in == 0 && amount_1_in == 0 {
                self.unlock();
                return Err(Error::InsufficientLiquidity);
            }

            pool_math::verify_k_invariant(
                balance_0,
                balance_1,
                amount_0_in,
                amount_1_in,
                self.reserve_0,
                self.reserve_1,
            )
            .map_err(|e| { self.unlock(); e })?;

            self.update(balance_0, balance_1, self.reserve_0, self.reserve_1);

            let caller = self.env().caller();
            self.env().emit_event(Swap {
                sender: caller,
                amount_0_in,
                amount_1_in,
                amount_0_out,
                amount_1_out,
                to,
            });

            self.unlock();
            Ok(())
        }

        #[ink(message)]
        fn sync(&mut self) -> Result<(), Error> {
            let b0 = self.token_balance(self.token_0);
            let b1 = self.token_balance(self.token_1);
            self.update(b0, b1, self.reserve_0, self.reserve_1);
            Ok(())
        }

        #[ink(message)]
        fn get_reserves(&self) -> (u128, u128, u64) {
            (self.reserve_0, self.reserve_1, self.block_timestamp_last)
        }

        #[ink(message)]
        fn get_amount_out(
            &self,
            amount_in: u128,
            reserve_in: u128,
            reserve_out: u128,
        ) -> Result<u128, Error> {
            math::get_amount_out(amount_in, reserve_in, reserve_out)
        }

        #[ink(message)]
        fn get_amount_in(
            &self,
            amount_out: u128,
            reserve_in: u128,
            reserve_out: u128,
        ) -> Result<u128, Error> {
            math::get_amount_in(amount_out, reserve_in, reserve_out)
        }

        #[ink(message)]
        fn swap_exact_tokens_for_tokens(
            &mut self,
            amount_in: u128,
            amount_out_min: u128,
            zero_for_one: bool,
            to: AccountId,
            deadline: u64,
        ) -> Result<u128, Error> {
            self.lock()?;

            if self.env().block_timestamp() > deadline {
                self.unlock();
                return Err(Error::Expired);
            }

            let (reserve_in, reserve_out, token_in, token_out) = if zero_for_one {
                (self.reserve_0, self.reserve_1, self.token_0, self.token_1)
            } else {
                (self.reserve_1, self.reserve_0, self.token_1, self.token_0)
            };

            let amount_out = math::get_amount_out(amount_in, reserve_in, reserve_out)
                .map_err(|e| { self.unlock(); e })?;

            if amount_out < amount_out_min {
                self.unlock();
                return Err(Error::SlippageTooHigh);
            }

            let caller = self.env().caller();
            let pool_id = self.env().account_id();

            self.token_transfer_from(token_in, caller, pool_id, amount_in)
                .map_err(|e| { self.unlock(); e })?;
            self.token_transfer(token_out, to, amount_out)
                .map_err(|e| { self.unlock(); e })?;

            let b0 = self.token_balance(self.token_0);
            let b1 = self.token_balance(self.token_1);
            let (amount_0_in, amount_1_in, amount_0_out, amount_1_out) = if zero_for_one {
                (amount_in, 0u128, 0u128, amount_out)
            } else {
                (0u128, amount_in, amount_out, 0u128)
            };

            pool_math::verify_k_invariant(b0, b1, amount_0_in, amount_1_in, self.reserve_0, self.reserve_1)
                .map_err(|e| { self.unlock(); e })?;

            self.update(b0, b1, self.reserve_0, self.reserve_1);

            self.env().emit_event(Swap {
                sender: caller,
                amount_0_in,
                amount_1_in,
                amount_0_out,
                amount_1_out,
                to,
            });

            self.unlock();
            Ok(amount_out)
        }

        #[ink(message)]
        fn swap_tokens_for_exact_tokens(
            &mut self,
            amount_out: u128,
            amount_in_max: u128,
            zero_for_one: bool,
            to: AccountId,
            deadline: u64,
        ) -> Result<u128, Error> {
            self.lock()?;

            if self.env().block_timestamp() > deadline {
                self.unlock();
                return Err(Error::Expired);
            }

            let (reserve_in, reserve_out, token_in, token_out) = if zero_for_one {
                (self.reserve_0, self.reserve_1, self.token_0, self.token_1)
            } else {
                (self.reserve_1, self.reserve_0, self.token_1, self.token_0)
            };

            let amount_in = math::get_amount_in(amount_out, reserve_in, reserve_out)
                .map_err(|e| { self.unlock(); e })?;

            if amount_in > amount_in_max {
                self.unlock();
                return Err(Error::SlippageTooHigh);
            }

            let caller = self.env().caller();
            let pool_id = self.env().account_id();

            self.token_transfer_from(token_in, caller, pool_id, amount_in)
                .map_err(|e| { self.unlock(); e })?;
            self.token_transfer(token_out, to, amount_out)
                .map_err(|e| { self.unlock(); e })?;

            let b0 = self.token_balance(self.token_0);
            let b1 = self.token_balance(self.token_1);
            let (amount_0_in, amount_1_in, amount_0_out, amount_1_out) = if zero_for_one {
                (amount_in, 0u128, 0u128, amount_out)
            } else {
                (0u128, amount_in, amount_out, 0u128)
            };

            pool_math::verify_k_invariant(b0, b1, amount_0_in, amount_1_in, self.reserve_0, self.reserve_1)
                .map_err(|e| { self.unlock(); e })?;

            self.update(b0, b1, self.reserve_0, self.reserve_1);

            self.env().emit_event(Swap {
                sender: caller,
                amount_0_in,
                amount_1_in,
                amount_0_out,
                amount_1_out,
                to,
            });

            self.unlock();
            Ok(amount_in)
        }

        #[ink(message)]
        fn get_spot_price(&self, zero_for_one: bool) -> Result<u128, Error> {
            if zero_for_one {
                math::get_spot_price(self.reserve_0, self.reserve_1)
            } else {
                math::get_spot_price(self.reserve_1, self.reserve_0)
            }
        }

        #[ink(message)]
        fn get_price_impact(&self, amount_in: u128, zero_for_one: bool) -> Result<u128, Error> {
            let (reserve_in, reserve_out) = if zero_for_one {
                (self.reserve_0, self.reserve_1)
            } else {
                (self.reserve_1, self.reserve_0)
            };
            let amount_out = math::get_amount_out(amount_in, reserve_in, reserve_out)?;
            math::calculate_price_impact(amount_in, amount_out, reserve_in, reserve_out)
        }
    }
}
