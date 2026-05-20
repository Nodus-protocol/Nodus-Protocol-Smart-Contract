#[cfg(test)]
mod math_tests {
    use amm_liquidity_pool::math;

    #[test]
    fn get_amount_out_standard_case() {
        // 1 000 in with 10 000/10 000 reserves: expect ~996 out after 0.3% fee
        let out = math::get_amount_out(1_000, 10_000, 10_000).unwrap();
        assert!(out > 0 && out < 1_000, "output should be less than input due to fee");
    }

    #[test]
    fn get_amount_out_zero_input_returns_error() {
        assert!(math::get_amount_out(0, 10_000, 10_000).is_err());
    }

    #[test]
    fn get_amount_out_zero_reserves_returns_error() {
        assert!(math::get_amount_out(1_000, 0, 10_000).is_err());
        assert!(math::get_amount_out(1_000, 10_000, 0).is_err());
    }

    #[test]
    fn get_amount_in_roundtrip() {
        let reserve_in = 1_000_000u128;
        let reserve_out = 1_000_000u128;
        let desired_out = 500u128;
        let amount_in = math::get_amount_in(desired_out, reserve_in, reserve_out).unwrap();
        let amount_out = math::get_amount_out(amount_in, reserve_in, reserve_out).unwrap();
        assert!(amount_out >= desired_out, "roundtrip output must meet or exceed desired");
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
    fn minimum_liquidity_constant_is_1000() {
        assert_eq!(math::MINIMUM_LIQUIDITY, 1_000);
    }

    #[test]
    fn k_invariant_preserved_after_swap() {
        let reserve_in = 100_000u128;
        let reserve_out = 100_000u128;
        let amount_in = 1_000u128;

        let amount_out = math::get_amount_out(amount_in, reserve_in, reserve_out).unwrap();
        let new_k = (reserve_in + amount_in) * (reserve_out - amount_out);
        assert!(new_k >= reserve_in * reserve_out, "K must not decrease after swap");
    }

    #[test]
    fn overflow_inputs_handled_gracefully() {
        let result = math::get_amount_out(u128::MAX, u128::MAX, u128::MAX);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod liquidity_pool_tests {
    use amm_liquidity_pool::liquidity_pool;

    #[test]
    fn calculate_initial_liquidity_uses_geometric_mean() {
        // sqrt(100 * 100) - 1000 MINIMUM_LIQUIDITY = 10_000 - 1000 = 9_000
        // sqrt(10000) = 100, too small (< MINIMUM_LIQUIDITY), use larger values
        let liq = liquidity_pool::calculate_initial_liquidity(100_000, 100_000).unwrap();
        // sqrt(100_000 * 100_000) = 100_000; 100_000 - 1_000 = 99_000
        assert_eq!(liq, 99_000);
    }

    #[test]
    fn calculate_liquidity_to_mint_takes_minimum_ratio() {
        // Unequal amounts: the minimum ratio determines LP tokens minted
        let liq = liquidity_pool::calculate_liquidity_to_mint(
            1_000, 2_000, 10_000, 10_000, 100_000,
        )
        .unwrap();
        // min(1000*100_000/10_000, 2000*100_000/10_000) = min(10_000, 20_000) = 10_000
        assert_eq!(liq, 10_000);
    }

    #[test]
    fn calculate_withdrawal_amounts_proportional() {
        let (a0, a1) = liquidity_pool::calculate_withdrawal_amounts(
            500, 10_000, 20_000, 1_000,
        )
        .unwrap();
        assert_eq!(a0, 5_000);
        assert_eq!(a1, 10_000);
    }

    #[test]
    fn verify_k_invariant_passes_with_valid_swap() {
        // After a swap with fee, balances should satisfy the adjusted K check
        let reserve_0 = 100_000u128;
        let reserve_1 = 100_000u128;
        let amount_0_in = 1_000u128;
        let amount_1_out =
            amm_liquidity_pool::math::get_amount_out(amount_0_in, reserve_0, reserve_1).unwrap();

        let balance_0 = reserve_0 + amount_0_in;
        let balance_1 = reserve_1 - amount_1_out;

        assert!(liquidity_pool::verify_k_invariant(
            balance_0, balance_1, amount_0_in, 0, reserve_0, reserve_1,
        )
        .is_ok());
    }

    #[test]
    fn verify_k_invariant_rejects_manipulated_swap() {
        // Attempt to claim output without a corresponding input
        let result = liquidity_pool::verify_k_invariant(
            50_000, 50_000, 0, 0, 100_000, 100_000,
        );
        assert!(result.is_err());
    }
}
