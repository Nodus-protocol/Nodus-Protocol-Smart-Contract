#[cfg(test)]
mod math_tests {
    use nodus_protocol_amm::math;

    #[test]
    fn get_amount_out_standard_case() {
        let out = math::get_amount_out(1_000, 10_000, 10_000).unwrap();
        assert!(out > 0 && out < 1_000);
    }

    #[test]
    fn get_amount_out_zero_input() {
        assert!(math::get_amount_out(0, 10_000, 10_000).is_err());
    }

    #[test]
    fn get_amount_out_zero_reserves() {
        assert!(math::get_amount_out(1_000, 0, 10_000).is_err());
        assert!(math::get_amount_out(1_000, 10_000, 0).is_err());
    }

    #[test]
    fn get_amount_in_roundtrip() {
        let reserve = 1_000_000i128;
        let desired_out = 500i128;
        let amount_in = math::get_amount_in(desired_out, reserve, reserve).unwrap();
        let amount_out = math::get_amount_out(amount_in, reserve, reserve).unwrap();
        assert!(amount_out >= desired_out);
    }

    #[test]
    fn sqrt_known_values() {
        assert_eq!(math::sqrt(0), 0);
        assert_eq!(math::sqrt(1), 1);
        assert_eq!(math::sqrt(4), 2);
        assert_eq!(math::sqrt(9), 3);
        assert_eq!(math::sqrt(100), 10);
        assert_eq!(math::sqrt(10_000), 100);
        assert_eq!(math::sqrt(1_000_000), 1_000);
    }

    #[test]
    fn minimum_liquidity_is_1000() {
        assert_eq!(math::MINIMUM_LIQUIDITY, 1_000);
    }

    #[test]
    fn k_invariant_preserved_after_swap() {
        let reserve = 100_000i128;
        let amount_in = 1_000i128;
        let amount_out = math::get_amount_out(amount_in, reserve, reserve).unwrap();
        let new_k = (reserve + amount_in) * (reserve - amount_out);
        assert!(new_k >= reserve * reserve);
    }

    #[test]
    fn overflow_inputs_handled() {
        assert!(math::get_amount_out(i128::MAX, i128::MAX, i128::MAX).is_err());
    }

    #[test]
    fn fee_constants_correct() {
        assert_eq!(math::FEE_NUMERATOR, 997);
        assert_eq!(math::FEE_DENOMINATOR, 1_000);
    }
}

#[cfg(test)]
mod liquidity_pool_tests {
    use nodus_protocol_amm::liquidity_pool;

    #[test]
    fn calculate_initial_liquidity_geometric_mean() {
        let liq = liquidity_pool::calculate_initial_liquidity(100_000, 100_000).unwrap();
        assert_eq!(liq, 99_000);
    }

    #[test]
    fn calculate_initial_liquidity_too_small() {
        assert!(liquidity_pool::calculate_initial_liquidity(10, 10).is_err());
    }

    #[test]
    fn calculate_liquidity_to_mint_proportional() {
        let liq =
            liquidity_pool::calculate_liquidity_to_mint(1_000, 1_000, 10_000, 10_000, 100_000)
                .unwrap();
        assert_eq!(liq, 10_000);
    }

    #[test]
    fn calculate_withdrawal_amounts_proportional() {
        let (a0, a1) =
            liquidity_pool::calculate_withdrawal_amounts(5_000, 100_000, 200_000, 10_000).unwrap();
        assert_eq!(a0, 50_000);
        assert_eq!(a1, 100_000);
    }

    #[test]
    fn verify_k_invariant_holds_after_swap() {
        let r0 = 100_000i128;
        let r1 = 100_000i128;
        let amount_in = 1_000i128;
        let amount_out = nodus_protocol_amm::math::get_amount_out(amount_in, r0, r1).unwrap();
        let b0 = r0 + amount_in;
        let b1 = r1 - amount_out;
        assert!(liquidity_pool::verify_k_invariant(b0, b1, amount_in, 0, r0, r1).is_ok());
    }

    #[test]
    fn verify_k_invariant_violated() {
        assert!(liquidity_pool::verify_k_invariant(100, 100, 0, 0, 1_000, 1_000).is_err());
    }

    #[test]
    fn optimal_amounts_preserves_ratio() {
        let (a0, a1) =
            liquidity_pool::calculate_optimal_amounts(2_000, 2_000, 0, 0, 10_000, 20_000).unwrap();
        assert!(a0 > 0 && a1 > 0 && a0 <= 2_000 && a1 <= 2_000);
    }
}

#[cfg(test)]
#[cfg(feature = "testutils")]
mod soroban_contract_tests {
    use nodus_protocol_amm::{NodusAmm, NodusAmmClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger as _},
        Address, Env,
    };

    fn setup() -> (Env, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract = env.register(NodusAmm, ());
        (env, contract)
    }

    #[test]
    fn initialize_accepts_distinct_tokens() {
        let (env, contract) = setup();
        let client = NodusAmmClient::new(&env, &contract);
        let t0 = Address::generate(&env);
        let t1 = Address::generate(&env);
        let admin = Address::generate(&env);
        let lp_token = Address::generate(&env);
        assert!(client.try_initialize(&t0, &t1, &admin, &lp_token).is_ok());
    }

    #[test]
    fn initialize_rejects_identical_tokens() {
        let (env, contract) = setup();
        let client = NodusAmmClient::new(&env, &contract);
        let t = Address::generate(&env);
        let admin = Address::generate(&env);
        let lp_token = Address::generate(&env);
        assert!(client.try_initialize(&t, &t, &admin, &lp_token).is_err());
    }

    #[test]
    fn double_initialize_rejected() {
        let (env, contract) = setup();
        let client = NodusAmmClient::new(&env, &contract);
        let t0 = Address::generate(&env);
        let t1 = Address::generate(&env);
        let admin = Address::generate(&env);
        let lp_token = Address::generate(&env);
        client.initialize(&t0, &t1, &admin, &lp_token);
        assert!(client.try_initialize(&t0, &t1, &admin, &lp_token).is_err());
    }

    #[test]
    fn get_reserves_initial_zero() {
        let (env, contract) = setup();
        let client = NodusAmmClient::new(&env, &contract);
        let t0 = Address::generate(&env);
        let t1 = Address::generate(&env);
        client.initialize(&t0, &t1, &Address::generate(&env), &Address::generate(&env));
        let (r0, r1, _) = client.get_reserves();
        assert_eq!(r0, 0);
        assert_eq!(r1, 0);
    }

    #[test]
    fn expired_deadline_rejected() {
        let (env, contract) = setup();
        let client = NodusAmmClient::new(&env, &contract);
        let t0 = Address::generate(&env);
        let t1 = Address::generate(&env);
        client.initialize(&t0, &t1, &Address::generate(&env), &Address::generate(&env));
        env.ledger().set_timestamp(2_000);
        let from = Address::generate(&env);
        let to = Address::generate(&env);
        assert!(client
            .try_add_liquidity(&from, &to, &1_000, &1_000, &0, &0, &500)
            .is_err());
    }

    #[test]
    fn not_initialized_swap_rejected() {
        let (env, contract) = setup();
        let client = NodusAmmClient::new(&env, &contract);
        let to = Address::generate(&env);
        assert!(client.try_swap(&to, &100, &0).is_err());
    }

    #[test]
    fn lp_token_readable_after_init() {
        let (env, contract) = setup();
        let client = NodusAmmClient::new(&env, &contract);
        let t0 = Address::generate(&env);
        let t1 = Address::generate(&env);
        let lp_token = Address::generate(&env);
        client.initialize(&t0, &t1, &Address::generate(&env), &lp_token);
        assert_eq!(client.lp_token(), lp_token);
    }

    #[test]
    fn price_cumulatives_start_zero() {
        let (env, contract) = setup();
        let client = NodusAmmClient::new(&env, &contract);
        let t0 = Address::generate(&env);
        let t1 = Address::generate(&env);
        client.initialize(&t0, &t1, &Address::generate(&env), &Address::generate(&env));
        let (p0, p1) = client.get_price_cumulative();
        assert_eq!(p0, 0u128);
        assert_eq!(p1, 0u128);
    }
}
