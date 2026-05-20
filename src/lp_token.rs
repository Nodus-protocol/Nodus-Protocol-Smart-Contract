use ink::env::call::{build_call, ExecutionInput, Selector};
use ink::env::DefaultEnvironment;
use ink::primitives::AccountId;
use ink::prelude::vec::Vec;
use crate::errors::Error;

/// Cross-contract call wrapper for the PSP22 LP token.
pub struct LPTokenRef {
    address: AccountId,
}

impl LPTokenRef {
    pub fn new(address: AccountId) -> Self {
        Self { address }
    }

    pub fn total_supply(&self) -> u128 {
        build_call::<DefaultEnvironment>()
            .call(self.address)
            .exec_input(ExecutionInput::new(Selector::new(
                ink::selector_bytes!("PSP22::total_supply"),
            )))
            .returns::<u128>()
            .invoke()
    }

    pub fn balance_of(&self, owner: AccountId) -> u128 {
        build_call::<DefaultEnvironment>()
            .call(self.address)
            .exec_input(
                ExecutionInput::new(Selector::new(ink::selector_bytes!("PSP22::balance_of")))
                    .push_arg(owner),
            )
            .returns::<u128>()
            .invoke()
    }

    pub fn mint(&self, to: AccountId, amount: u128) -> Result<(), Error> {
        build_call::<DefaultEnvironment>()
            .call(self.address)
            .exec_input(
                ExecutionInput::new(Selector::new(ink::selector_bytes!("PSP22Mintable::mint")))
                    .push_arg(to)
                    .push_arg(amount),
            )
            .returns::<Result<(), Error>>()
            .invoke()
    }

    pub fn burn(&self, from: AccountId, amount: u128) -> Result<(), Error> {
        build_call::<DefaultEnvironment>()
            .call(self.address)
            .exec_input(
                ExecutionInput::new(Selector::new(ink::selector_bytes!("PSP22Burnable::burn")))
                    .push_arg(from)
                    .push_arg(amount),
            )
            .returns::<Result<(), Error>>()
            .invoke()
    }

    pub fn transfer(&self, to: AccountId, amount: u128) -> Result<(), Error> {
        build_call::<DefaultEnvironment>()
            .call(self.address)
            .exec_input(
                ExecutionInput::new(Selector::new(ink::selector_bytes!("PSP22::transfer")))
                    .push_arg(to)
                    .push_arg(amount)
                    .push_arg::<Vec<u8>>(Vec::new()),
            )
            .returns::<Result<(), Error>>()
            .invoke()
    }

    pub fn transfer_from(
        &self,
        from: AccountId,
        to: AccountId,
        amount: u128,
    ) -> Result<(), Error> {
        build_call::<DefaultEnvironment>()
            .call(self.address)
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
}
