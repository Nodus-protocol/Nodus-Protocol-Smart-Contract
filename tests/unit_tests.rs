#[cfg(test)]
mod math_tests {
    use nodus_amm::math;

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
        let reserve_in = 1_000_000i128;
        let reserve_out = 1_000_000i128;
        let desired_out = 500i128;
        let amount_in = math::get_amount_in(desired_out, reserve_in, reserve_out).unwrap();
        let amount_out = math::get_amount_out(amount_in, reserve_in, reserve_out).unwrap();
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
        let reserve_in = 100_000i128;
        let reserve_out = 100_000i128;
        let amount_in = 1_000i128;
        let amount_out = math::get_amount_out(amount_in, reserve_in, reserve_out).unwrap();
        let new_k = (reserve_in + amount_in) * (reserve_out - amount_out);
        assert!(new_k >= reserve_in * reserve_out);
    }

    #[test]
    fn overflow_inputs_handled() {
        assert!(math::get_amount_out(i128::MAX, i128::MAX, i128::MAX).is_err());
    }
}

#[cfg(test)]
mod liquidity_pool_tests {
    use nodus_amm::liquidity_pool;

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
    fn calculate_liquidity_to_mint() {
        let liq = liquidity_pool::calculate_liquidity_to_mint(1_000, 1_000, 10_000, 10_000, 100_000).unwrap();
        assert_eq!(liq, 10_000);
    }

    #[test]
    fn calculate_withdrawal_amounts() {
        let (a0, a1) = liquidity_pool::calculate_withdrawal_amounts(5_000, 100_000, 200_000, 10_000).unwrap();
        assert_eq!(a0, 50_000);
        assert_eq!(a1, 100_000);
    }

    #[test]
    fn verify_k_invariant_holds() {
        let r0 = 100_000i128;
        let r1 = 100_000i128;
        let amount_in = 1_000i128;
        let amount_out = nodus_amm::math::get_amount_out(amount_in, r0, r1).unwrap();
        let b0 = r0 + amount_in;
        let b1 = r1 - amount_out;
        assert!(liquidity_pool::verify_k_invariant(b0, b1, amount_in, 0, r0, r1).is_ok());
    }

    #[test]
    fn verify_k_invariant_violated() {
        assert!(liquidity_pool::verify_k_invariant(100, 100, 0, 0, 1_000, 1_000).is_err());
    }
}

#[cfg(test)]
mod soroban_contract_tests {
    use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env};
    use nodus_amm::NodusAmm;

    fn setup() -> (Env, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract = env.register_contract(None, NodusAmm);
        (env, contract)
    }

    fn mock_token(env: &Env) -> Address {
        Address::generate(env)
    }

    #[test]
    fn initialize_sets_tokens() {
        let (env, contract) = setup();
        let client = nodus_amm::NodusAmmClient::new(&env, &contract);
        let t0 = mock_token(&env);
        let t1 = mock_token(&env);
        assert!(client.initialize(&t0, &t1).is_ok());
    }

    #[test]
    fn initialize_rejects_identical_tokens() {
        let (env, contract) = setup();
        let client = nodus_amm::NodusAmmClient::new(&env, &contract);
        let t0 = mock_token(&env);
        assert!(client.initialize(&t0, &t0).is_err());
    }

    #[test]
    fn double_initialize_rejected() {
        let (env, contract) = setup();
        let client = nodus_amm::NodusAmmClient::new(&env, &contract);
        let t0 = mock_token(&env);
        let t1 = mock_token(&env);
        client.initialize(&t0, &t1).unwrap();
        assert!(client.initialize(&t0, &t1).is_err());
    }

    #[test]
    fn get_reserves_initial_zero() {
        let (env, contract) = setup();
        let client = nodus_amm::NodusAmmClient::new(&env, &contract);
        let t0 = mock_token(&env);
        let t1 = mock_token(&env);
        client.initialize(&t0, &t1).unwrap();
        let (r0, r1, _) = client.get_reserves();
        assert_eq!(r0, 0);
        assert_eq!(r1, 0);
    }

    #[test]
    fn expired_deadline_rejected() {
        let (env, contract) = setup();
        let client = nodus_amm::NodusAmmClient::new(&env, &contract);
        let t0 = mock_token(&env);
        let t1 = mock_token(&env);
        client.initialize(&t0, &t1).unwrap();
        let mut info = env.ledger().get();
        info.timestamp = 2_000;
        env.ledger().set(info);
        let to = Address::generate(&env);
        assert!(client.add_liquidity(&1_000, &1_000, &0, &0, &to, &500).is_err());
    }
}
