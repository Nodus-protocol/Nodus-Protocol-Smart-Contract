#[cfg(test)]
mod fuzz_math {
    use nodus_protocol_amm::math;

    #[test]
    fn fuzz_amount_out_never_exceeds_reserve_out() {
        let cases: &[(i128, i128, i128)] = &[
            (1, 1_000_000, 1_000_000),
            (999_999, 1_000_000, 1_000_000),
            (1, 1, 1_000_000),
            (1_000, 100_000, 200_000),
            (50_000, 500_000, 250_000),
        ];
        for &(amount_in, reserve_in, reserve_out) in cases {
            if let Ok(out) = math::get_amount_out(amount_in, reserve_in, reserve_out) {
                assert!(
                    out < reserve_out,
                    "output must be strictly less than reserve_out"
                );
                assert!(out > 0, "output must be positive");
            }
        }
    }

    #[test]
    fn fuzz_sqrt_is_integer_floor() {
        for n in [
            0i128, 1, 2, 3, 4, 8, 15, 16, 99, 100, 10_000, 999_999, 1_000_000,
        ] {
            let s = math::sqrt(n);
            assert!(s * s <= n, "sqrt({n}) = {s}: s^2 must be <= n");
            if s > 0 {
                assert!(
                    (s + 1) * (s + 1) > n,
                    "sqrt({n}) = {s}: (s+1)^2 must exceed n"
                );
            }
        }
    }

    #[test]
    fn fuzz_fee_always_reduces_output_vs_input() {
        let reserve = 1_000_000i128;
        for amount_in in [100i128, 1_000, 10_000, 100_000] {
            let out = math::get_amount_out(amount_in, reserve, reserve).unwrap();
            assert!(
                out < amount_in,
                "output must be less than input due to 0.3% fee"
            );
        }
    }

    #[test]
    fn fuzz_get_amount_in_out_roundtrip() {
        let reserve_in = 1_000_000i128;
        let reserve_out = 1_000_000i128;
        for desired_out in [100i128, 500, 1_000, 10_000] {
            let amount_in = math::get_amount_in(desired_out, reserve_in, reserve_out).unwrap();
            let actual_out = math::get_amount_out(amount_in, reserve_in, reserve_out).unwrap();
            assert!(
                actual_out >= desired_out,
                "roundtrip must produce at least desired_out"
            );
        }
    }

    #[test]
    fn fuzz_k_invariant_holds_across_swap_sizes() {
        use nodus_protocol_amm::liquidity_pool;
        let reserve = 1_000_000i128;
        for amount_in in [100i128, 1_000, 5_000, 50_000, 200_000] {
            let amount_out = math::get_amount_out(amount_in, reserve, reserve).unwrap();
            let b0 = reserve + amount_in;
            let b1 = reserve - amount_out;
            assert!(
                liquidity_pool::verify_k_invariant(b0, b1, amount_in, 0, reserve, reserve).is_ok(),
                "K invariant must hold after swap of {amount_in}",
            );
        }
    }
}
