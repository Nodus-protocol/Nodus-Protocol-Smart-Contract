#[cfg(test)]
mod fuzz_math {
    use nodus_amm::math;

    #[test]
    fn fuzz_get_amount_out_never_exceeds_reserve_out() {
        let cases: &[(i128, i128, i128)] = &[
            (1, 1_000_000, 1_000_000),
            (999_999, 1_000_000, 1_000_000),
            (1, 1, 1_000_000),
            (1_000, 100_000, 200_000),
        ];
        for &(amount_in, reserve_in, reserve_out) in cases {
            if let Ok(out) = math::get_amount_out(amount_in, reserve_in, reserve_out) {
                assert!(out < reserve_out, "output must be less than reserve_out");
                assert!(out > 0, "output must be positive");
            }
        }
    }

    #[test]
    fn fuzz_sqrt_always_integer_floor() {
        for n in [0i128, 1, 2, 3, 4, 8, 15, 16, 99, 100, 10_000, 999_999, 1_000_000] {
            let s = math::sqrt(n);
            assert!(s * s <= n, "sqrt({n}) = {s}: s^2 must be <= n");
            assert!((s + 1) * (s + 1) > n, "sqrt({n}) = {s}: (s+1)^2 must be > n");
        }
    }

    #[test]
    fn fuzz_fee_reduces_output() {
        let reserve = 1_000_000i128;
        for amount_in in [100i128, 1_000, 10_000, 100_000] {
            let out = math::get_amount_out(amount_in, reserve, reserve).unwrap();
            assert!(out < amount_in, "output must be less than input due to 0.3% fee");
        }
    }
}
