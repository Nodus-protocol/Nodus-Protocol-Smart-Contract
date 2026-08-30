#[cfg(test)]
#[cfg(feature = "testutils")]
mod integration {
    use nodus_protocol_amm::{registry, NodusAmm, NodusAmmClient};
    use nodus_protocol_lp_token::{NodusLpToken, NodusLpTokenClient};
    use soroban_sdk::{
        testutils::{Address as _, Events as _, Ledger as _},
        xdr::ContractEventBody,
        Address, Env, String, TryFromVal,
    };

    /// Deploys and initializes two SEP-41 token contracts whose metadata
    /// matches the canonical XLM/USDC policy (the stand-in for the real
    /// XLM and USDC Stellar Asset Contracts).
    fn deploy_canonical_pair(env: &Env) -> (Address, Address) {
        let xlm = env.register(NodusLpToken, ());
        let usdc = env.register(NodusLpToken, ());
        NodusLpTokenClient::new(env, &xlm).initialize(
            &Address::generate(env),
            &String::from_str(env, registry::XLM_NAME),
            &String::from_str(env, registry::XLM_SYMBOL),
            &registry::XLM_DECIMALS,
        );
        NodusLpTokenClient::new(env, &usdc).initialize(
            &Address::generate(env),
            &String::from_str(env, registry::USDC_NAME),
            &String::from_str(env, registry::USDC_SYMBOL),
            &registry::USDC_DECIMALS,
        );
        (xlm, usdc)
    }

    fn setup_initialized() -> (Env, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract = env.register(NodusAmm, ());
        let client = NodusAmmClient::new(&env, &contract);
        let (t0, t1) = deploy_canonical_pair(&env);
        let admin = Address::generate(&env);
        let lp_token = Address::generate(&env);
        client.initialize(&t0, &t1, &admin, &lp_token);
        (env, contract, t0, t1)
    }

    #[test]
    fn swap_without_reserves_fails() {
        let (env, contract, _, _) = setup_initialized();
        let client = NodusAmmClient::new(&env, &contract);
        let to = Address::generate(&env);
        assert!(client.try_swap(&to, &100, &0, &u64::MAX).is_err());
    }

    #[test]
    fn swap_zero_output_rejected() {
        let (env, contract, _, _) = setup_initialized();
        let client = NodusAmmClient::new(&env, &contract);
        let to = Address::generate(&env);
        assert!(client.try_swap(&to, &0, &0, &u64::MAX).is_err());
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
    fn expired_swap_rejected() {
        let (env, contract, _, _) = setup_initialized();
        let client = NodusAmmClient::new(&env, &contract);
        env.ledger().set_timestamp(5_000);
        let to = Address::generate(&env);
        assert_eq!(
            client.try_swap(&to, &100, &0, &1_000),
            Err(Ok(nodus_protocol_amm::Error::Expired))
        );
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
        let (t0, t1) = deploy_canonical_pair(&env);
        let admin = Address::generate(&env);
        let lp_token = Address::generate(&env);
        client.initialize(&t0, &t1, &admin, &lp_token);
        (env, contract, admin)
    }

    /// Activation must expose the canonical asset identities and pinned
    /// contract addresses through a pool-attributable registry event, so
    /// off-chain consumers can trust the pair without re-deriving it.
    #[test]
    fn initialize_emits_activation_event_with_canonical_identities() {
        let env = Env::default();
        env.mock_all_auths();
        let contract = env.register(NodusAmm, ());
        let client = NodusAmmClient::new(&env, &contract);
        let (xlm, usdc) = deploy_canonical_pair(&env);
        let admin = Address::generate(&env);
        let lp_token = Address::generate(&env);

        client.initialize(&xlm, &usdc, &admin, &lp_token);

        let filtered = env.events().all().filter_by_contract(&contract);
        let events = filtered.events();
        assert_eq!(events.len(), 1, "activation must emit exactly one event");
        let ContractEventBody::V0(v0) = &events[0].body;
        let data = &v0.data;
        let activated: nodus_protocol_amm::events::PoolActivatedEvent =
            nodus_protocol_amm::events::PoolActivatedEvent::try_from_val(&env, data).unwrap();
        assert_eq!(activated.token_0, xlm);
        assert_eq!(activated.token_1, usdc);
        assert_eq!(
            activated.canonical_symbol_0,
            String::from_str(&env, registry::XLM_SYMBOL)
        );
        assert_eq!(
            activated.canonical_symbol_1,
            String::from_str(&env, registry::USDC_SYMBOL)
        );
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
            client.try_swap(&to, &100, &0, &u64::MAX),
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
            client.try_swap(&to, &100, &0, &u64::MAX),
            Err(Ok(nodus_protocol_amm::Error::InsufficientLiquidity))
        );
    }

    #[test]
    fn sync_is_not_blocked_by_pause() {
        // sync() deliberately has no pause guard since it only reconciles
        // reserves and never moves funds; it runs (and succeeds) even while
        // paused. The assertion pins that it is not ContractPaused that
        // stops it.
        let (env, contract, admin) = setup_initialized_with_admin();
        let client = NodusAmmClient::new(&env, &contract);
        client.pause(&admin);
        assert_ne!(
            client.try_sync(),
            Err(Ok(nodus_protocol_amm::Error::ContractPaused))
        );
    }

    /// Exercises the real cross-contract wiring end to end: a genuine
    /// NodusLpToken instance (not a bare placeholder address) as the
    /// pool's LP token, and two more NodusLpToken instances standing in
    /// for token_0/token_1 -- close enough to a real SEP-41 token
    /// (mint/balance/transfer_from) to prove add_liquidity/
    /// remove_liquidity actually move real balances through real
    /// cross-contract calls, not just internal bookkeeping.
    #[test]
    fn add_liquidity_then_remove_liquidity_round_trips_through_real_lp_token() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_sequence_number(100);

        let pool = env.register(NodusAmm, ());
        let lp_token = env.register(NodusLpToken, ());
        let token_0 = env.register(NodusLpToken, ());
        let token_1 = env.register(NodusLpToken, ());

        let mint_authority = Address::generate(&env);
        let provider = Address::generate(&env);
        let admin = Address::generate(&env);

        let lp_client = NodusLpTokenClient::new(&env, &lp_token);
        lp_client.initialize(
            &pool,
            &String::from_str(&env, "Nodus LP"),
            &String::from_str(&env, "NODUS-LP"),
            &7,
        );

        let t0_client = NodusLpTokenClient::new(&env, &token_0);
        t0_client.initialize(
            &mint_authority,
            &String::from_str(&env, registry::XLM_NAME),
            &String::from_str(&env, registry::XLM_SYMBOL),
            &registry::XLM_DECIMALS,
        );
        let t1_client = NodusLpTokenClient::new(&env, &token_1);
        t1_client.initialize(
            &mint_authority,
            &String::from_str(&env, registry::USDC_NAME),
            &String::from_str(&env, registry::USDC_SYMBOL),
            &registry::USDC_DECIMALS,
        );

        // Give the liquidity provider tokens to deposit, and have them
        // approve the pool to pull them (add_liquidity uses
        // transfer_from, matching how it already worked against real
        // Stellar Asset Contract tokens before this refactor).
        t0_client.mint(&mint_authority, &provider, &1_000_000);
        t1_client.mint(&mint_authority, &provider, &1_000_000);
        t0_client.approve(&provider, &pool, &1_000_000, &10_000);
        t1_client.approve(&provider, &pool, &1_000_000, &10_000);

        let pool_client = NodusAmmClient::new(&env, &pool);
        pool_client.initialize(&token_0, &token_1, &admin, &lp_token);

        let liquidity =
            pool_client.add_liquidity(&provider, &provider, &100_000, &100_000, &0, &0, &u64::MAX);

        // sqrt(100_000 * 100_000) - MINIMUM_LIQUIDITY(1_000) = 99_000;
        // the other 1_000 is permanently locked at the dead address.
        assert_eq!(liquidity, 99_000);
        assert_eq!(lp_client.balance(&provider), 99_000);
        assert_eq!(lp_client.total_supply(), 100_000);
        assert_eq!(t0_client.balance(&provider), 900_000);
        assert_eq!(t1_client.balance(&provider), 900_000);
        assert_eq!(t0_client.balance(&pool), 100_000);
        assert_eq!(t1_client.balance(&pool), 100_000);
        let (r0, r1, _) = pool_client.get_reserves();
        assert_eq!(r0, 100_000);
        assert_eq!(r1, 100_000);

        let (amount_0, amount_1) =
            pool_client.remove_liquidity(&provider, &provider, &liquidity, &0, &0, &u64::MAX);

        // Proportional to the 99_000 of 100_000 total supply redeemed.
        assert_eq!(amount_0, 99_000);
        assert_eq!(amount_1, 99_000);
        assert_eq!(lp_client.balance(&provider), 0);
        assert_eq!(lp_client.total_supply(), 1_000);
        // Net down 1_000 of each token versus the starting 1_000_000 --
        // permanently locked in the pool via the dead-address LP shares.
        assert_eq!(t0_client.balance(&provider), 999_000);
        assert_eq!(t1_client.balance(&provider), 999_000);
    }
}
