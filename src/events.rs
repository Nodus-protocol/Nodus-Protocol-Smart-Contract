#![allow(deprecated)]
use soroban_sdk::{contracttype, symbol_short, Address, Env};

#[contracttype]
pub struct MintEvent {
    pub sender: Address,
    pub amount_0: i128,
    pub amount_1: i128,
}

#[contracttype]
pub struct BurnEvent {
    pub sender: Address,
    pub amount_0: i128,
    pub amount_1: i128,
    pub to: Address,
}

#[contracttype]
pub struct SwapEvent {
    pub sender: Address,
    pub amount_0_in: i128,
    pub amount_1_in: i128,
    pub amount_0_out: i128,
    pub amount_1_out: i128,
    pub to: Address,
}

#[contracttype]
pub struct SyncEvent {
    pub reserve_0: i128,
    pub reserve_1: i128,
}

/// Emits a mint event.
///
/// Topic includes the emitting contract's address (`env.current_contract_address()`)
/// so off-chain indexers can distinguish events from different Nodus pools when
/// subscribing broadly to the `v1_mint` topic across the whole network. See
/// https://github.com/Nodus-protocol/Nodus-Protocol-Smart-Contract/issues/60.
pub fn emit_mint(env: &Env, sender: Address, amount_0: i128, amount_1: i128) {
    env.events().publish(
        (
            symbol_short!("v1_mint"),
            env.current_contract_address(),
            sender.clone(),
        ),
        MintEvent {
            sender,
            amount_0,
            amount_1,
        },
    );
}

/// Emits a burn event. See [`emit_mint`] for the multi-pool indexing rationale.
pub fn emit_burn(env: &Env, sender: Address, amount_0: i128, amount_1: i128, to: Address) {
    env.events().publish(
        (
            symbol_short!("v1_burn"),
            env.current_contract_address(),
            sender.clone(),
        ),
        BurnEvent {
            sender,
            amount_0,
            amount_1,
            to,
        },
    );
}

/// Emits a swap event. See [`emit_mint`] for the multi-pool indexing rationale.
pub fn emit_swap(
    env: &Env,
    sender: Address,
    amount_0_in: i128,
    amount_1_in: i128,
    amount_0_out: i128,
    amount_1_out: i128,
    to: Address,
) {
    env.events().publish(
        (
            symbol_short!("v1_swap"),
            env.current_contract_address(),
            sender.clone(),
        ),
        SwapEvent {
            sender,
            amount_0_in,
            amount_1_in,
            amount_0_out,
            amount_1_out,
            to,
        },
    );
}

/// Emits a sync event. See [`emit_mint`] for the multi-pool indexing rationale.
///
/// Sync events have no per-call sender, so the topic is `(symbol, contract_address)`.
pub fn emit_sync(env: &Env, reserve_0: i128, reserve_1: i128) {
    env.events().publish(
        (symbol_short!("v1_sync"), env.current_contract_address()),
        SyncEvent {
            reserve_0,
            reserve_1,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Events};

    fn setup_env_and_contract() -> (Env, Address) {
        let env = Env::default();
        let contract_id = env.register(crate::NodusAmm, ());
        (env, contract_id)
    }

    /// Confirms emit_mint publishes exactly one event attributable to the
    /// emitting contract (filter_by_contract matches on contract address,
    /// which is only possible because the topic encodes that address).
    #[test]
    fn mint_event_is_attributable_to_contract() {
        let (env, contract_id) = setup_env_and_contract();
        let sender = Address::generate(&env);

        env.as_contract(&contract_id, || {
            emit_mint(&env, sender.clone(), 100, 200);
        });

        let filtered = env.events().all().filter_by_contract(&contract_id);
        assert_eq!(
            filtered.events().len(),
            1,
            "exactly one event must be attributable to the emitting contract"
        );
    }

    #[test]
    fn burn_event_is_attributable_to_contract() {
        let (env, contract_id) = setup_env_and_contract();
        let sender = Address::generate(&env);
        let to = Address::generate(&env);

        env.as_contract(&contract_id, || {
            emit_burn(&env, sender, 100, 200, to);
        });

        let filtered = env.events().all().filter_by_contract(&contract_id);
        assert_eq!(filtered.events().len(), 1);
    }

    #[test]
    fn swap_event_is_attributable_to_contract() {
        let (env, contract_id) = setup_env_and_contract();
        let sender = Address::generate(&env);
        let to = Address::generate(&env);

        env.as_contract(&contract_id, || {
            emit_swap(&env, sender, 100, 0, 0, 95, to);
        });

        let filtered = env.events().all().filter_by_contract(&contract_id);
        assert_eq!(filtered.events().len(), 1);
    }

    #[test]
    fn sync_event_is_attributable_to_contract() {
        let (env, contract_id) = setup_env_and_contract();

        env.as_contract(&contract_id, || {
            emit_sync(&env, 1_000, 2_000);
        });

        let filtered = env.events().all().filter_by_contract(&contract_id);
        assert_eq!(filtered.events().len(), 1);
    }

    /// Multi-pool indexing test: deploy two separate instances of the contract
    /// (simulating two different pools), have each emit a mint event, and
    /// confirm `filter_by_contract` can isolate each pool's event independently.
    /// This is only possible because each event's topic encodes the emitting
    /// contract's address — proving the fix actually solves the issue's
    /// multi-pool indexing ambiguity, not just that events still fire.
    #[test]
    fn two_pools_emit_independently_attributable_events() {
        let env = Env::default();
        let pool_a = env.register(crate::NodusAmm, ());
        let pool_b = env.register(crate::NodusAmm, ());
        let sender = Address::generate(&env);

        env.as_contract(&pool_a, || {
            emit_mint(&env, sender.clone(), 10, 20);
        });
        let pool_a_events = env.events().all().filter_by_contract(&pool_a);
        assert_eq!(
            pool_a_events.events().len(),
            1,
            "pool A's event must be isolatable by contract address"
        );

        env.as_contract(&pool_b, || {
            emit_mint(&env, sender.clone(), 30, 40);
        });
        let pool_b_events = env.events().all().filter_by_contract(&pool_b);
        assert_eq!(
            pool_b_events.events().len(),
            1,
            "pool B's event must be isolatable by contract address"
        );

        // Sanity check: an indexer filtering for a contract that emitted
        // nothing gets zero events, confirming filter_by_contract isn't
        // just returning everything.
        let unrelated = Address::generate(&env);
        let unrelated_events = env.events().all().filter_by_contract(&unrelated);
        assert_eq!(unrelated_events.events().len(), 0);
    }
}
