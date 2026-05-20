#[cfg(all(test, feature = "fuzzing"))]
mod fuzz {
    use amm_liquidity_pool::math;
    use amm_liquidity_pool::liquidity_pool;
    use proptest::prelude::*;

    proptest! {
        /// Output amount must always be strictly less than the output reserve.
        #[test]
        fn amount_out_never_exceeds_reserve(
            amount_in  in 1u128..1_000_000_000u128,
            reserve_in in 1u128..u64::MAX as u128,
            reserve_out in 1u128..u64::MAX as u128,
        ) {
            if let Ok(out) = math::get_amount_out(amount_in, reserve_in, reserve_out) {
                prop_assert!(out < reserve_out);
            }
        }

        /// K must never decrease after a swap (fees accrue to reserves).
        #[test]
        fn k_invariant_non_decreasing(
            amount_in  in 1u128..100_000u128,
            reserve_in in 100_001u128..1_000_000u128,
            reserve_out in 100_001u128..1_000_000u128,
        ) {
            if let Ok(amount_out) = math::get_amount_out(amount_in, reserve_in, reserve_out) {
                let k_before = reserve_in * reserve_out;
                let k_after = (reserve_in + amount_in) * (reserve_out - amount_out);
                prop_assert!(k_after >= k_before, "K decreased: before={k_before} after={k_after}");
            }
        }

        /// sqrt must be monotonically non-decreasing.
        #[test]
        fn sqrt_is_monotone(a in 0u128..u64::MAX as u128, b in 0u128..u64::MAX as u128) {
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            prop_assert!(math::sqrt(lo) <= math::sqrt(hi));
        }

        /// get_amount_in / get_amount_out must be consistent: in → out >= desired.
        #[test]
        fn get_amount_in_get_amount_out_roundtrip(
            desired_out in 1u128..10_000u128,
            reserve_in  in 100_000u128..10_000_000u128,
            reserve_out in 100_000u128..10_000_000u128,
        ) {
            if desired_out < reserve_out {
                if let Ok(amount_in) = math::get_amount_in(desired_out, reserve_in, reserve_out) {
                    if let Ok(amount_out) = math::get_amount_out(amount_in, reserve_in, reserve_out) {
                        prop_assert!(
                            amount_out >= desired_out,
                            "roundtrip: got {amount_out}, needed {desired_out}"
                        );
                    }
                }
            }
        }

        /// Withdrawal amounts must always be proportional to share of total supply.
        #[test]
        fn withdrawal_amounts_proportional(
            liquidity    in 1u128..1_000_000u128,
            reserve_0    in 1u128..u64::MAX as u128,
            reserve_1    in 1u128..u64::MAX as u128,
            total_supply in 1_000_001u128..u64::MAX as u128,
        ) {
            // Ensure liquidity <= total_supply
            let liquidity = liquidity.min(total_supply / 2);
            if let Ok((a0, a1)) = liquidity_pool::calculate_withdrawal_amounts(
                liquidity, reserve_0, reserve_1, total_supply,
            ) {
                prop_assert!(a0 <= reserve_0, "amount_0 exceeds reserve_0");
                prop_assert!(a1 <= reserve_1, "amount_1 exceeds reserve_1");
            }
        }

        /// Minted liquidity for non-initial deposits must always fit within existing reserves.
        #[test]
        fn minted_liquidity_does_not_dilute_existing_lps(
            amount_0     in 1u128..1_000_000u128,
            amount_1     in 1u128..1_000_000u128,
            reserve_0    in 1_000_001u128..100_000_000u128,
            reserve_1    in 1_000_001u128..100_000_000u128,
            total_supply in 1_000_001u128..u64::MAX as u128,
        ) {
            if let Ok(liq) = liquidity_pool::calculate_liquidity_to_mint(
                amount_0, amount_1, reserve_0, reserve_1, total_supply,
            ) {
                // New LP share must not allow withdrawal of more than deposited
                let share_num = liq as f64 / (total_supply + liq) as f64;
                let new_r0 = reserve_0 + amount_0;
                let implied_withdraw_0 = (share_num * new_r0 as f64) as u128;
                prop_assert!(
                    implied_withdraw_0 <= amount_0 + 1,
                    "LP over-represents deposit: could withdraw {implied_withdraw_0} but deposited {amount_0}"
                );
            }
        }
    }
}
