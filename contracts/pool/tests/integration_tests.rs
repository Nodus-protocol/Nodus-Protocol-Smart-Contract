#[cfg(test)]
#[cfg(feature = "testutils")]
mod common;

#[cfg(test)]
#[cfg(feature = "testutils")]
mod integration {
    use crate::common::{
        deploy_canonical_sacs, env_with_seq, register_hostile_at, HostileMode, HostileTokenClient,
    };
    use nodus_protocol_amm::{registry, NodusAmm, NodusAmmClient};
    use nodus_protocol_lp_token::{NodusLpToken, NodusLpTokenClient};
    use soroban_sdk::{
        testutils::{Address as _, Events as _, Ledger as _},
        token::Client as TokenClient,
        xdr::ContractEventBody,
        Address, Env, String, TryFromVal,
    };

    fn setup_initialized() -> (Env, Address, Address, Address) {
        let env = env_with_seq();
        let contract = env.register(NodusAmm, ());
        let client = NodusAmmClient::new(&env, &contract);
        let (t0, t1) = deploy_canonical_sacs(&env);
        let admin = Address::generate(&env);
        let lp_token = Address::generate(&env);
        client.initialize(&t0, &t1, &admin, &lp_token);
        (env, contract, t0, t1)
    }

    fn setup_initialized_with_admin() -> (Env, Address, Address) {
        let env = env_with_seq();
        let contract = env.register(NodusAmm, ());
        let client = NodusAmmClient::new(&env, &contract);
        let (t0, t1) = deploy_canonical_sacs(&env);
        let admin = Address::generate(&env);
        let lp_token = Address::generate(&env);
        client.initialize(&t0, &t1, &admin, &lp_token);
        (env, contract, admin)
    }

    /// Activation must expose the canonical asset identifiers and the
    /// derived SAC contract addresses through a pool-attributable registry
    /// event, so off-chain consumers can trust the pair without re-deriving
    /// it.
    #[test]
    fn initialize_emits_activation_event_with_canonical_identities() {
        let env = env_with_seq();
        let contract = env.register(NodusAmm, ());
        let client = NodusAmmClient::new(&env, &contract);
        let (xlm, usdc) = deploy_canonical_sacs(&env);
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
            activated.canonical_id_0,
            String::from_str(&env, registry::XLM_NAME)
        );
        assert_eq!(
            activated.canonical_id_1,
            String::from_str(&env, registry::USDC_NAME)
        );
    }

    // ── Post-deploy transfer/allowance compatibility canary ───────────────

    /// The canary round trip must succeed against well-behaved tokens at the
    /// canonical addresses: approve → pull → exact balance → push back →
    /// zero balance, with strict canary limits, and record
    /// `canary_verified()`. (The sandbox cannot mint the native XLM SAC, so
    /// behavioral tests use SEP-41 stand-ins planted at the canonical
    /// addresses; identity/derivation is covered by the init-time tests on
    /// the real SACs.)
    #[test]
    fn canary_round_trip_succeeds_with_well_behaved_tokens() {
        let env = env_with_seq();
        let contract = env.register(NodusAmm, ());
        let client = NodusAmmClient::new(&env, &contract);
        let (xlm, usdc) = deploy_canonical_sacs(&env);
        let t0 = register_hostile_at(
            &env,
            &xlm,
            HostileMode::Normal,
            &String::from_str(&env, registry::XLM_NAME),
            &String::from_str(&env, registry::XLM_SYMBOL),
            registry::XLM_DECIMALS,
            &contract,
        );
        let t1 = register_hostile_at(
            &env,
            &usdc,
            HostileMode::Normal,
            &String::from_str(&env, registry::USDC_NAME),
            &String::from_str(&env, registry::USDC_SYMBOL),
            registry::USDC_DECIMALS,
            &contract,
        );
        let admin = Address::generate(&env);
        let lp_token = Address::generate(&env);
        client.initialize(&t0, &t1, &admin, &lp_token);

        // Fund the admin (fee_to_setter) with a canary-sized balance of both
        // assets and run the check.
        HostileTokenClient::new(&env, &t0).mint(&admin, &admin, &100);
        HostileTokenClient::new(&env, &t1).mint(&admin, &admin, &100);

        assert!(client.try_verify_token_compatibility(&admin, &10).is_ok());
        assert!(client.canary_verified());

        // Net zero movement: the pool ends with no balances and the admin is
        // whole.
        let t0_t = TokenClient::new(&env, &t0);
        let t1_t = TokenClient::new(&env, &t1);
        assert_eq!(t0_t.balance(&contract), 0);
        assert_eq!(t1_t.balance(&contract), 0);
        assert_eq!(t0_t.balance(&admin), 100);
        assert_eq!(t1_t.balance(&admin), 100);
    }

    #[test]
    fn canary_rejects_non_admin() {
        let (env, contract, _) = setup_initialized_with_admin();
        let client = NodusAmmClient::new(&env, &contract);
        let intruder = Address::generate(&env);
        assert_eq!(
            client.try_verify_token_compatibility(&intruder, &10),
            Err(Ok(nodus_protocol_amm::Error::Unauthorized))
        );
        assert!(!client.canary_verified());
    }

    #[test]
    fn canary_rejects_out_of_range_amount() {
        let (env, contract, admin) = setup_initialized_with_admin();
        let client = NodusAmmClient::new(&env, &contract);
        assert_eq!(
            client.try_verify_token_compatibility(&admin, &0),
            Err(Ok(nodus_protocol_amm::Error::InvalidCanaryAmount))
        );
        assert_eq!(
            client.try_verify_token_compatibility(&admin, &11),
            Err(Ok(nodus_protocol_amm::Error::InvalidCanaryAmount))
        );
        assert!(!client.canary_verified());
    }

    /// A canary caller without funds must fail: the pull step reverts, which
    /// surfaces as TokenCompatibilityFailed rather than passing silently.
    #[test]
    fn canary_fails_on_insufficient_balance() {
        let (env, contract, admin) = setup_initialized_with_admin();
        let client = NodusAmmClient::new(&env, &contract);
        assert_eq!(
            client.try_verify_token_compatibility(&admin, &10),
            Err(Ok(nodus_protocol_amm::Error::TokenCompatibilityFailed))
        );
        assert!(!client.canary_verified());
    }

    /// A canonical SAC replaced by a fee-on-transfer implementation must be
    /// caught by the canary: the pool balance after the pull is `amount - 1`,
    /// not exactly `amount`.
    #[test]
    fn canary_detects_fee_on_transfer_token_at_canonical_address() {
        let env = env_with_seq();
        let contract = env.register(NodusAmm, ());
        let client = NodusAmmClient::new(&env, &contract);
        let (xlm, usdc) = deploy_canonical_sacs(&env);
        // The XLM side is a well-behaved stand-in; the USDC side is replaced
        // by a fee-on-transfer implementation at the canonical address that
        // still reports canonical metadata.
        let t0 = register_hostile_at(
            &env,
            &xlm,
            HostileMode::Normal,
            &String::from_str(&env, registry::XLM_NAME),
            &String::from_str(&env, registry::XLM_SYMBOL),
            registry::XLM_DECIMALS,
            &contract,
        );
        let t1 = register_hostile_at(
            &env,
            &usdc,
            HostileMode::FeeOnTransfer,
            &String::from_str(&env, registry::USDC_NAME),
            &String::from_str(&env, registry::USDC_SYMBOL),
            registry::USDC_DECIMALS,
            &contract,
        );
        assert_eq!(t1, usdc);
        let admin = Address::generate(&env);
        let lp_token = Address::generate(&env);
        // Initialization still succeeds: address and metadata are canonical.
        client.initialize(&t0, &t1, &admin, &lp_token);
        HostileTokenClient::new(&env, &t0).mint(&admin, &admin, &100);
        HostileTokenClient::new(&env, &t1).mint(&admin, &admin, &100);

        assert_eq!(
            client.try_verify_token_compatibility(&admin, &10),
            Err(Ok(nodus_protocol_amm::Error::TokenCompatibilityFailed))
        );
        assert!(!client.canary_verified());
    }

    /// A canonical SAC replaced by a no-op implementation (transfers return
    /// Ok but move nothing) must be caught by the canary.
    #[test]
    fn canary_detects_noop_transfer_token_at_canonical_address() {
        let env = env_with_seq();
        let contract = env.register(NodusAmm, ());
        let client = NodusAmmClient::new(&env, &contract);
        let (xlm, usdc) = deploy_canonical_sacs(&env);
        let t0 = register_hostile_at(
            &env,
            &xlm,
            HostileMode::Normal,
            &String::from_str(&env, registry::XLM_NAME),
            &String::from_str(&env, registry::XLM_SYMBOL),
            registry::XLM_DECIMALS,
            &contract,
        );
        let t1 = register_hostile_at(
            &env,
            &usdc,
            HostileMode::NoOp,
            &String::from_str(&env, registry::USDC_NAME),
            &String::from_str(&env, registry::USDC_SYMBOL),
            registry::USDC_DECIMALS,
            &contract,
        );
        let admin = Address::generate(&env);
        let lp_token = Address::generate(&env);
        client.initialize(&t0, &t1, &admin, &lp_token);
        HostileTokenClient::new(&env, &t0).mint(&admin, &admin, &100);
        HostileTokenClient::new(&env, &t1).mint(&admin, &admin, &100);

        assert_eq!(
            client.try_verify_token_compatibility(&admin, &10),
            Err(Ok(nodus_protocol_amm::Error::TokenCompatibilityFailed))
        );
        assert!(!client.canary_verified());
    }

    // ── Reentrancy / malformed token behavior ─────────────────────────────

    /// A token that re-enters the pool from inside its transfer path must
    /// not be able to corrupt the pool: the nested swap is rejected (the
    /// Soroban host blocks contract re-entry at the protocol level, and the
    /// pool's own lock is defense-in-depth on top of that) and moves
    /// nothing, while the deposit that triggered it still completes with
    /// consistent reserves. Without the re-entry protection the nested
    /// swap would execute against real reserves, so the test only passes
    /// when the re-entry attempt is actually rejected.
    #[test]
    fn pool_blocks_reentrant_token_during_add_liquidity() {
        let env = env_with_seq();
        let pool = env.register(NodusAmm, ());
        let lp_token = env.register(NodusLpToken, ());
        let (xlm, usdc) = deploy_canonical_sacs(&env);
        let t0 = register_hostile_at(
            &env,
            &xlm,
            HostileMode::Normal,
            &String::from_str(&env, registry::XLM_NAME),
            &String::from_str(&env, registry::XLM_SYMBOL),
            registry::XLM_DECIMALS,
            &pool,
        );
        let t1 = register_hostile_at(
            &env,
            &usdc,
            HostileMode::Normal,
            &String::from_str(&env, registry::USDC_NAME),
            &String::from_str(&env, registry::USDC_SYMBOL),
            registry::USDC_DECIMALS,
            &pool,
        );
        let admin = Address::generate(&env);
        let provider = Address::generate(&env);

        let lp_client = NodusLpTokenClient::new(&env, &lp_token);
        lp_client.initialize(
            &pool,
            &String::from_str(&env, "Nodus LP"),
            &String::from_str(&env, "NODUS-LP"),
            &7,
        );

        let client = NodusAmmClient::new(&env, &pool);
        client.initialize(&t0, &t1, &admin, &lp_token);

        HostileTokenClient::new(&env, &t0).mint(&provider, &provider, &1_000_000);
        HostileTokenClient::new(&env, &t1).mint(&provider, &provider, &1_000_000);
        TokenClient::new(&env, &t0).approve(&provider, &pool, &1_000_000, &10_000);
        TokenClient::new(&env, &t1).approve(&provider, &pool, &1_000_000, &10_000);

        // First deposit creates reserves, so a reentrant swap would have
        // something to move if the lock did not stop it.
        client.add_liquidity(&provider, &provider, &100_000, &100_000, &0, &0, &u64::MAX);

        // Arm the USDC side to re-enter the pool from its transfer path.
        HostileTokenClient::new(&env, &t1).set_mode(&HostileMode::Reentrant);

        // Second deposit: the reentrant swap is rejected by the lock and
        // moves nothing; the deposit itself still completes and reserves
        // grow by exactly the deposited amounts.
        client.add_liquidity(&provider, &provider, &10_000, &10_000, &0, &0, &u64::MAX);
        assert!(
            HostileTokenClient::new(&env, &t1).reentry_observed(),
            "reentry result code: {}",
            HostileTokenClient::new(&env, &t1).reentry_result_code()
        );
        let (r0, r1, _) = client.get_reserves();
        assert_eq!(r0, 110_000);
        assert_eq!(r1, 110_000);
        assert_eq!(TokenClient::new(&env, &t0).balance(&pool), 110_000);
        assert_eq!(TokenClient::new(&env, &t1).balance(&pool), 110_000);
    }

    // ── Ordinary pool behavior (unchanged semantics, real SACs) ───────────

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
        let env = env_with_seq();
        let contract = env.register(NodusAmm, ());
        let client = NodusAmmClient::new(&env, &contract);
        assert!(client.try_token_0().is_err());
        assert!(client.try_token_1().is_err());
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
        // No longer blocked by pause; fails for the ordinary reason (no
        // reserves) instead.
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

    /// Exercises the cross-contract wiring end to end: SEP-41 tokens at the
    /// canonical addresses (the sandbox cannot mint the native XLM SAC, so
    /// behavioral flows use well-behaved stand-ins), pulled in and pushed
    /// out through real transfer_from/transfer calls, with a genuine
    /// NodusLpToken instance as the pool's LP token. Identity/derivation of
    /// the canonical addresses is covered by the init-time tests on the real
    /// SACs.
    #[test]
    fn add_liquidity_then_remove_liquidity_round_trips_through_tokens() {
        let env = env_with_seq();
        let pool = env.register(NodusAmm, ());
        let lp_token = env.register(NodusLpToken, ());
        let (xlm, usdc) = deploy_canonical_sacs(&env);
        let t0 = register_hostile_at(
            &env,
            &xlm,
            HostileMode::Normal,
            &String::from_str(&env, registry::XLM_NAME),
            &String::from_str(&env, registry::XLM_SYMBOL),
            registry::XLM_DECIMALS,
            &pool,
        );
        let t1 = register_hostile_at(
            &env,
            &usdc,
            HostileMode::Normal,
            &String::from_str(&env, registry::USDC_NAME),
            &String::from_str(&env, registry::USDC_SYMBOL),
            registry::USDC_DECIMALS,
            &pool,
        );

        let provider = Address::generate(&env);
        let admin = Address::generate(&env);

        let lp_client = NodusLpTokenClient::new(&env, &lp_token);
        lp_client.initialize(
            &pool,
            &String::from_str(&env, "Nodus LP"),
            &String::from_str(&env, "NODUS-LP"),
            &7,
        );

        // Fund the provider through the tokens and approve the pool.
        HostileTokenClient::new(&env, &t0).mint(&provider, &provider, &1_000_000);
        HostileTokenClient::new(&env, &t1).mint(&provider, &provider, &1_000_000);
        let t0_t = TokenClient::new(&env, &t0);
        let t1_t = TokenClient::new(&env, &t1);
        t0_t.approve(&provider, &pool, &1_000_000, &10_000);
        t1_t.approve(&provider, &pool, &1_000_000, &10_000);

        let pool_client = NodusAmmClient::new(&env, &pool);
        pool_client.initialize(&t0, &t1, &admin, &lp_token);

        let liquidity =
            pool_client.add_liquidity(&provider, &provider, &100_000, &100_000, &0, &0, &u64::MAX);

        // sqrt(100_000 * 100_000) - MINIMUM_LIQUIDITY(1_000) = 99_000;
        // the other 1_000 is permanently locked at the dead address.
        assert_eq!(liquidity, 99_000);
        assert_eq!(lp_client.balance(&provider), 99_000);
        assert_eq!(lp_client.total_supply(), 100_000);
        assert_eq!(t0_t.balance(&provider), 900_000);
        assert_eq!(t1_t.balance(&provider), 900_000);
        assert_eq!(t0_t.balance(&pool), 100_000);
        assert_eq!(t1_t.balance(&pool), 100_000);
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
        assert_eq!(t0_t.balance(&provider), 999_000);
        assert_eq!(t1_t.balance(&provider), 999_000);
    }

    /// Exercises the swap wiring end to end: SEP-41 tokens at the
    /// canonical addresses (the sandbox cannot mint the native XLM SAC, so
    /// behavioral flows use well-behaved stand-ins), pulled in and pushed
    /// out through real transfer_from/transfer calls. The K-invariant is
    /// enforced on actual balances, so a standard swap moves funds exactly
    /// and updates reserves. Identity/derivation of the canonical
    /// addresses is covered by the init-time tests on the real SACs.
    #[test]
    fn swap_exact_tokens_moves_balances_and_updates_reserves() {
        let env = env_with_seq();
        let pool = env.register(NodusAmm, ());
        let lp_token = env.register(NodusLpToken, ());
        let (xlm, usdc) = deploy_canonical_sacs(&env);
        let t0 = register_hostile_at(
            &env,
            &xlm,
            HostileMode::Normal,
            &String::from_str(&env, registry::XLM_NAME),
            &String::from_str(&env, registry::XLM_SYMBOL),
            registry::XLM_DECIMALS,
            &pool,
        );
        let t1 = register_hostile_at(
            &env,
            &usdc,
            HostileMode::Normal,
            &String::from_str(&env, registry::USDC_NAME),
            &String::from_str(&env, registry::USDC_SYMBOL),
            registry::USDC_DECIMALS,
            &pool,
        );

        let provider = Address::generate(&env);
        let admin = Address::generate(&env);

        NodusLpTokenClient::new(&env, &lp_token).initialize(
            &pool,
            &String::from_str(&env, "Nodus LP"),
            &String::from_str(&env, "NODUS-LP"),
            &7,
        );

        HostileTokenClient::new(&env, &t0).mint(&provider, &provider, &1_000_000);
        HostileTokenClient::new(&env, &t1).mint(&provider, &provider, &1_000_000);
        let xlm_t = TokenClient::new(&env, &t0);
        let usdc_t = TokenClient::new(&env, &t1);
        xlm_t.approve(&provider, &pool, &1_000_000, &10_000);
        usdc_t.approve(&provider, &pool, &1_000_000, &10_000);

        let pool_client = NodusAmmClient::new(&env, &pool);
        pool_client.initialize(&t0, &t1, &admin, &lp_token);
        pool_client.add_liquidity(&provider, &provider, &100_000, &100_000, &0, &0, &u64::MAX);

        // Swap 1_000 XLM for USDC (0.3% fee).
        let amount_out = pool_client.swap_exact_tokens_for_tokens(
            &provider,
            &provider,
            &1_000,
            &0,
            &true,
            &u64::MAX,
        );
        assert!(amount_out > 0 && amount_out < 1_000);
        assert_eq!(xlm_t.balance(&provider), 1_000_000 - 100_000 - 1_000);
        assert_eq!(usdc_t.balance(&provider), 900_000 + amount_out);
        let (r0, r1, _) = pool_client.get_reserves();
        assert_eq!(r0, 101_000);
        assert_eq!(r1, 100_000 - amount_out);
    }
}
