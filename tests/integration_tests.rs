#[cfg(test)]
mod integration {
    use soroban_sdk::{testutils::Address as _, Address, Env};
    use nodus_amm::NodusAmm;

    fn setup_initialized() -> (Env, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract = env.register_contract(None, NodusAmm);
        let client = nodus_amm::NodusAmmClient::new(&env, &contract);
        let t0 = Address::generate(&env);
        let t1 = Address::generate(&env);
        client.initialize(&t0, &t1).unwrap();
        (env, contract, t0, t1)
    }

    #[test]
    fn swap_without_liquidity_fails() {
        let (env, contract, _, _) = setup_initialized();
        let client = nodus_amm::NodusAmmClient::new(&env, &contract);
        let to = Address::generate(&env);
        assert!(client.swap(&100, &0, &to).is_err());
    }

    #[test]
    fn lp_balance_of_starts_zero() {
        let (env, contract, _, _) = setup_initialized();
        let client = nodus_amm::NodusAmmClient::new(&env, &contract);
        let user = Address::generate(&env);
        assert_eq!(client.lp_balance_of(&user), 0);
    }

    #[test]
    fn lp_total_supply_starts_zero() {
        let (env, contract, _, _) = setup_initialized();
        let client = nodus_amm::NodusAmmClient::new(&env, &contract);
        assert_eq!(client.lp_total_supply(), 0);
    }
}
