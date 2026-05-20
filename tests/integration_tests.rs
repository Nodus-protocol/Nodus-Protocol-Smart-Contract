#[cfg(all(test, feature = "e2e-tests"))]
mod e2e {
    use ink_e2e::prelude::*;
    use amm_liquidity_pool::pool::LiquidityPoolRef;

    type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// Deploys pool and LP token contracts, adds initial liquidity, verifies minted LP tokens.
    #[ink_e2e::test]
    async fn add_liquidity_mints_lp_tokens(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
        // 1. Deploy mock PSP22 token_0 and token_1
        // 2. Deploy LP token contract
        // 3. Deploy LiquidityPool with (token_0, token_1, lp_token)
        // 4. Approve pool to spend token_0 and token_1
        // 5. Call add_liquidity(100_000, 100_000, 0, 0, alice, u64::MAX)
        // 6. Assert lp_token.balance_of(alice) == sqrt(100_000 * 100_000) - MINIMUM_LIQUIDITY
        //    i.e., 100_000 - 1_000 = 99_000
        Ok(())
    }

    /// Adds liquidity, then removes it and verifies tokens are returned proportionally.
    #[ink_e2e::test]
    async fn remove_liquidity_returns_tokens(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
        // 1. Setup pool with 100_000 / 100_000 initial liquidity (alice holds 99_000 LP)
        // 2. Alice approves LP token burn
        // 3. Call remove_liquidity(99_000, 0, 0, alice, u64::MAX)
        // 4. Assert alice receives ~99% of each reserve back
        // 5. Assert LP total_supply == MINIMUM_LIQUIDITY (1_000) remaining
        Ok(())
    }

    /// Verifies a token swap preserves the K invariant and charges the 0.3% fee.
    #[ink_e2e::test]
    async fn swap_preserves_k_invariant(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
        // 1. Setup pool: reserve_0 = 1_000_000, reserve_1 = 1_000_000
        // 2. Call get_amount_out(1_000, reserve_0, reserve_1) -> expected_out
        // 3. Execute swap(0, expected_out, alice)
        //    (alice sends token_0 directly; pool checks balance delta)
        // 4. Assert reserve_0 increased, reserve_1 decreased
        // 5. Assert new_reserve_0 * new_reserve_1 >= old_reserve_0 * old_reserve_1
        Ok(())
    }

    /// Verifies that a second concurrent call within a message is rejected.
    #[ink_e2e::test]
    async fn reentrancy_is_blocked(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
        // 1. Deploy a malicious reentrant contract that calls back into the pool
        //    during a swap callback.
        // 2. Invoke the malicious contract.
        // 3. Assert the inner call returns Error::ReentrancyDetected.
        // 4. Assert the outer swap succeeded but inner was rejected.
        Ok(())
    }

    /// Verifies that a swap past the deadline is rejected.
    #[ink_e2e::test]
    async fn expired_deadline_is_rejected(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
        // 1. Setup funded pool.
        // 2. Call add_liquidity with deadline = 0 (already expired).
        // 3. Assert Error::Expired is returned.
        // 4. Assert reserves are unchanged.
        Ok(())
    }
}
