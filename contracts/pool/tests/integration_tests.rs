#[cfg(test)]
#[cfg(feature = "testutils")]
mod integration {
    use nodus_protocol_amm::{NodusAmm, NodusAmmClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger as _},
        Address, Env,
    };

    fn setup_initialized() -> (Env, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract = env.register(NodusAmm, ());
        let client = NodusAmmClient::new(&env, &contract);
        let t0 = Address::generate(&env);
        let t1 = Address::generate(&env);
        let admin = Address::generate(&env);
        client.initialize(&t0, &t1, &admin);
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
        assert!(client
            .try_remove_liquidity(&from, &to, &100, &0, &0, &1_000)
            .is_err());
    }

    #[test]
    fn not_initialized_token_query_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let contract = env.register(NodusAmm, ());
        let client = NodusAmmClient::new(&env, &contract);
        assert!(client.try_token_0().is_err());
        assert!(client.try_token_1().is_err());
    }

    fn setup_initialized_with_admin() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract = env.register(NodusAmm, ());
        let client = NodusAmmClient::new(&env, &contract);
        let t0 = Address::generate(&env);
        let t1 = Address::generate(&env);
        let admin = Address::generate(&env);
        client.initialize(&t0, &t1, &admin);
        (env, contract, admin)
    }

    #[test]
    fn contract_starts_unpaused() {
        let (env, contract, _) = setup_initialized_with_admin();
        let client = NodusAmmClient::new(&env, &contract);
        assert!(!client.is_paused());
    }

    #[test]
    fn admin_can_pause_and_unpause() {
        let (env, contract, admin) = setup_initialized_with_admin();
        let client = NodusAmmClient::new(&env, &contract);
        client.pause(&admin);
        assert!(client.is_paused());
        client.unpause(&admin);
        assert!(!client.is_paused());
    }

    #[test]
    fn non_admin_cannot_pause() {
        let (env, contract, _) = setup_initialized_with_admin();
        let client = NodusAmmClient::new(&env, &contract);
        let intruder = Address::generate(&env);
        assert!(client.try_pause(&intruder).is_err());
        assert!(!client.is_paused());
    }

    #[test]
    fn non_admin_cannot_unpause() {
        let (env, contract, admin) = setup_initialized_with_admin();
        let client = NodusAmmClient::new(&env, &contract);
        client.pause(&admin);
        let intruder = Address::generate(&env);
        assert!(client.try_unpause(&intruder).is_err());
        assert!(client.is_paused());
    }

    #[test]
    fn swap_rejected_while_paused() {
        let (env, contract, admin) = setup_initialized_with_admin();
        let client = NodusAmmClient::new(&env, &contract);
        client.pause(&admin);
        let to = Address::generate(&env);
        assert_eq!(
            client.try_swap(&to, &100, &0),
            Err(Ok(nodus_protocol_amm::Error::ContractPaused))
        );
    }

    #[test]
    fn add_liquidity_rejected_while_paused() {
        let (env, contract, admin) = setup_initialized_with_admin();
        let client = NodusAmmClient::new(&env, &contract);
        client.pause(&admin);
        let from = Address::generate(&env);
        let to = Address::generate(&env);
        assert_eq!(
            client.try_add_liquidity(&from, &to, &1_000, &1_000, &0, &0, &1_000_000_000),
            Err(Ok(nodus_protocol_amm::Error::ContractPaused))
        );
    }

    #[test]
    fn remove_liquidity_rejected_while_paused() {
        let (env, contract, admin) = setup_initialized_with_admin();
        let client = NodusAmmClient::new(&env, &contract);
        client.pause(&admin);
        let from = Address::generate(&env);
        let to = Address::generate(&env);
        assert_eq!(
            client.try_remove_liquidity(&from, &to, &100, &0, &0, &1_000_000_000),
            Err(Ok(nodus_protocol_amm::Error::ContractPaused))
        );
    }

    #[test]
    fn swap_exact_tokens_for_tokens_rejected_while_paused() {
        let (env, contract, admin) = setup_initialized_with_admin();
        let client = NodusAmmClient::new(&env, &contract);
        client.pause(&admin);
        let from = Address::generate(&env);
        let to = Address::generate(&env);
        assert_eq!(
            client.try_swap_exact_tokens_for_tokens(&from, &to, &100, &0, &true, &1_000_000_000),
            Err(Ok(nodus_protocol_amm::Error::ContractPaused))
        );
    }

    #[test]
    fn swap_tokens_for_exact_tokens_rejected_while_paused() {
        let (env, contract, admin) = setup_initialized_with_admin();
        let client = NodusAmmClient::new(&env, &contract);
        client.pause(&admin);
        let from = Address::generate(&env);
        let to = Address::generate(&env);
        assert_eq!(
            client.try_swap_tokens_for_exact_tokens(
                &from,
                &to,
                &100,
                &1_000_000,
                &true,
                &1_000_000_000
            ),
            Err(Ok(nodus_protocol_amm::Error::ContractPaused))
        );
    }

    #[test]
    fn unpause_restores_normal_operation() {
        let (env, contract, admin) = setup_initialized_with_admin();
        let client = NodusAmmClient::new(&env, &contract);
        client.pause(&admin);
        client.unpause(&admin);
        let to = Address::generate(&env);
        // No longer blocked by pause; fails for the ordinary reason (no reserves) instead.
        assert_eq!(
            client.try_swap(&to, &100, &0),
            Err(Ok(nodus_protocol_amm::Error::InsufficientLiquidity))
        );
    }

    #[test]
    fn sync_is_not_blocked_by_pause() {
        // sync() deliberately has no pause guard since it only reconciles
        // reserves and never moves funds. It still fails here because t0/t1
        // are bare addresses rather than deployed token contracts, but the
        // failure must not be ContractPaused -- proving pause isn't what
        // stopped it.
        let (env, contract, admin) = setup_initialized_with_admin();
        let client = NodusAmmClient::new(&env, &contract);
        client.pause(&admin);
        assert_ne!(
            client.try_sync(),
            Err(Ok(nodus_protocol_amm::Error::ContractPaused))
        );
    }
}
