#[cfg(test)]
#[cfg(feature = "testutils")]
mod integration {
    use soroban_sdk::{testutils::{Address as _, Ledger as _}, Address, Env};
    use nodus_amm::{NodusAmm, NodusAmmClient};

    fn setup_initialized() -> (Env, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract = env.register_contract(None, NodusAmm);
        let client = NodusAmmClient::new(&env, &contract);
        let t0 = Address::generate(&env);
        let t1 = Address::generate(&env);
        client.initialize(&t0, &t1);
        (env, contract, t0, t1)
    }

    #[test]
    fn swap_without_reserves_fails() {
        let (env, contract, _, _) = setup_initialized();
        let client = NodusAmmClient::new(&env, &contract);
        let to = Address::generate(&env);
        assert!(client.try_swap(&to, &100, &0).is_err());
    }

    #[test]
    fn swap_zero_output_rejected() {
        let (env, contract, _, _) = setup_initialized();
        let client = NodusAmmClient::new(&env, &contract);
        let to = Address::generate(&env);
        assert!(client.try_swap(&to, &0, &0).is_err());
    }

    #[test]
    fn lp_balance_starts_zero() {
        let (env, contract, _, _) = setup_initialized();
        let client = NodusAmmClient::new(&env, &contract);
        assert_eq!(client.lp_balance_of(&Address::generate(&env)), 0);
    }

    #[test]
    fn lp_total_supply_starts_zero() {
        let (env, contract, _, _) = setup_initialized();
        let client = NodusAmmClient::new(&env, &contract);
        assert_eq!(client.lp_total_supply(), 0);
    }

    #[test]
    fn token_0_and_1_readable_after_init() {
        let (env, contract, t0, t1) = setup_initialized();
        let client = NodusAmmClient::new(&env, &contract);
        assert_eq!(client.token_0(), t0);
        assert_eq!(client.token_1(), t1);
    }

    #[test]
    fn expired_remove_liquidity_rejected() {
        let (env, contract, _, _) = setup_initialized();
        let client = NodusAmmClient::new(&env, &contract);
        env.ledger().set_timestamp(5_000);
        let from = Address::generate(&env);
        let to = Address::generate(&env);
        assert!(client.try_remove_liquidity(&from, &to, &100, &0, &0, &1_000).is_err());
    }

    #[test]
    fn not_initialized_token_query_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let contract = env.register_contract(None, NodusAmm);
        let client = NodusAmmClient::new(&env, &contract);
        assert!(client.try_token_0().is_err());
        assert!(client.try_token_1().is_err());
    }
}
