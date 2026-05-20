use ink::primitives::AccountId;

#[ink::event]
pub struct Mint {
    #[ink(topic)]
    pub sender: AccountId,
    pub amount_0: u128,
    pub amount_1: u128,
}

#[ink::event]
pub struct Burn {
    #[ink(topic)]
    pub sender: AccountId,
    pub amount_0: u128,
    pub amount_1: u128,
    #[ink(topic)]
    pub to: AccountId,
}

#[ink::event]
pub struct Swap {
    #[ink(topic)]
    pub sender: AccountId,
    pub amount_0_in: u128,
    pub amount_1_in: u128,
    pub amount_0_out: u128,
    pub amount_1_out: u128,
    #[ink(topic)]
    pub to: AccountId,
}

#[ink::event]
pub struct Sync {
    pub reserve_0: u128,
    pub reserve_1: u128,
}
