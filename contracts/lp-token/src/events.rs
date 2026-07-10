#![allow(deprecated)]
use soroban_sdk::{contracttype, symbol_short, Address, Env};

#[contracttype]
pub struct MintEvent {
    pub to: Address,
    pub amount: i128,
}

#[contracttype]
pub struct BurnEvent {
    pub from: Address,
    pub amount: i128,
}

#[contracttype]
pub struct TransferEvent {
    pub from: Address,
    pub to: Address,
    pub amount: i128,
}

#[contracttype]
pub struct ApproveEvent {
    pub from: Address,
    pub spender: Address,
    pub amount: i128,
    pub expiration_ledger: u32,
}

/// Topics include the emitting contract's address so an indexer watching
/// every LP token instance across every pool can attribute each event to
/// the right one, matching the convention already used by the pool
/// contract's own events.
pub fn emit_mint(env: &Env, to: Address, amount: i128) {
    env.events().publish(
        (
            symbol_short!("v1_mint"),
            env.current_contract_address(),
            to.clone(),
        ),
        MintEvent { to, amount },
    );
}

pub fn emit_burn(env: &Env, from: Address, amount: i128) {
    env.events().publish(
        (
            symbol_short!("v1_burn"),
            env.current_contract_address(),
            from.clone(),
        ),
        BurnEvent { from, amount },
    );
}

pub fn emit_transfer(env: &Env, from: Address, to: Address, amount: i128) {
    env.events().publish(
        (
            symbol_short!("v1_xfer"),
            env.current_contract_address(),
            from.clone(),
        ),
        TransferEvent { from, to, amount },
    );
}

pub fn emit_approve(
    env: &Env,
    from: Address,
    spender: Address,
    amount: i128,
    expiration_ledger: u32,
) {
    env.events().publish(
        (
            symbol_short!("v1_appr"),
            env.current_contract_address(),
            from.clone(),
        ),
        ApproveEvent {
            from,
            spender,
            amount,
            expiration_ledger,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Events};

    fn setup() -> (Env, Address) {
        let env = Env::default();
        let contract_id = env.register(crate::NodusLpToken, ());
        (env, contract_id)
    }

    #[test]
    fn mint_event_is_attributable_to_contract() {
        let (env, contract_id) = setup();
        let to = Address::generate(&env);

        env.as_contract(&contract_id, || {
            emit_mint(&env, to, 100);
        });

        let filtered = env.events().all().filter_by_contract(&contract_id);
        assert_eq!(filtered.events().len(), 1);
    }

    #[test]
    fn burn_event_is_attributable_to_contract() {
        let (env, contract_id) = setup();
        let from = Address::generate(&env);

        env.as_contract(&contract_id, || {
            emit_burn(&env, from, 100);
        });

        let filtered = env.events().all().filter_by_contract(&contract_id);
        assert_eq!(filtered.events().len(), 1);
    }

    #[test]
    fn transfer_event_is_attributable_to_contract() {
        let (env, contract_id) = setup();
        let from = Address::generate(&env);
        let to = Address::generate(&env);

        env.as_contract(&contract_id, || {
            emit_transfer(&env, from, to, 50);
        });

        let filtered = env.events().all().filter_by_contract(&contract_id);
        assert_eq!(filtered.events().len(), 1);
    }

    #[test]
    fn approve_event_is_attributable_to_contract() {
        let (env, contract_id) = setup();
        let from = Address::generate(&env);
        let spender = Address::generate(&env);

        env.as_contract(&contract_id, || {
            emit_approve(&env, from, spender, 25, 1000);
        });

        let filtered = env.events().all().filter_by_contract(&contract_id);
        assert_eq!(filtered.events().len(), 1);
    }

    #[test]
    fn two_lp_tokens_emit_independently_attributable_events() {
        let env = Env::default();
        let token_a = env.register(crate::NodusLpToken, ());
        let token_b = env.register(crate::NodusLpToken, ());
        let holder = Address::generate(&env);

        env.as_contract(&token_a, || {
            emit_mint(&env, holder.clone(), 10);
        });
        let token_a_events = env.events().all().filter_by_contract(&token_a);
        assert_eq!(token_a_events.events().len(), 1);

        env.as_contract(&token_b, || {
            emit_mint(&env, holder, 20);
        });
        let token_b_events = env.events().all().filter_by_contract(&token_b);
        assert_eq!(token_b_events.events().len(), 1);
    }
}
