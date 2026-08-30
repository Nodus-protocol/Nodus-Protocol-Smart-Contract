#![no_std]
#![allow(clippy::too_many_arguments)]
use soroban_sdk::{contract, contractimpl, token::Client as TokenClient, Address, Env, String};

pub mod errors;
pub mod events;
pub mod liquidity_pool;
pub mod math;
pub mod registry;
pub mod storage;
pub mod traits;

pub use errors::Error;
use storage::DataKey;

/// Imports the LP token contract's interface from its own compiled WASM
/// (built separately -- see contracts/lp-token) rather than depending on
/// its crate directly. A regular Cargo dependency would link that
/// crate's own #[contractimpl]-generated WASM exports into this
/// contract's binary too: confirmed empirically, since both crates
/// export an `initialize` function, which fails the link with a
/// duplicate-symbol error. This only pulls in the client type and call
/// signatures, not the LP token's own contract code.
///
/// Build order requirement: contracts/lp-token must be built to WASM
/// before this crate, since contractimport! reads the file at compile
/// time. `make build` / CI handle this; see the workspace README.
mod lp_token_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/nodus_protocol_lp_token.wasm"
    );
}
use lp_token_contract::Client as LpTokenClient;

const INSTANCE_TTL_THRESHOLD: u32 = 100;
const INSTANCE_TTL_BUMP: u32 = 500;

/// Upper bound (in stroops) for the post-deploy transfer/allowance
/// compatibility canary. Canary amounts are deliberately tiny: the check
/// proves the tokens' write path works with negligible exposure.
const CANARY_MAX_AMOUNT: i128 = 10;

fn require_initialized(env: &Env) -> Result<(), Error> {
    if !env
        .storage()
        .instance()
        .get::<DataKey, bool>(&DataKey::Initialized)
        .unwrap_or(false)
    {
        return Err(Error::NotInitialized);
    }
    Ok(())
}

fn get_reserve_0(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::Reserve0)
        .unwrap_or(0)
}

fn get_reserve_1(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::Reserve1)
        .unwrap_or(0)
}

fn get_timestamp_last(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::TimestampLast)
        .unwrap_or(0)
}

fn is_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false)
}

fn require_not_paused(env: &Env) -> Result<(), Error> {
    if is_paused(env) {
        return Err(Error::ContractPaused);
    }
    Ok(())
}

fn require_fee_to_setter(env: &Env, caller: &Address) -> Result<(), Error> {
    caller.require_auth();
    let setter: Address = env
        .storage()
        .instance()
        .get(&DataKey::FeeToSetter)
        .ok_or(Error::NotInitialized)?;
    if *caller != setter {
        return Err(Error::Unauthorized);
    }
    Ok(())
}

fn is_locked(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::Locked)
        .unwrap_or(false)
}

fn lock(env: &Env) -> Result<(), Error> {
    if is_locked(env) {
        return Err(Error::ReentrancyDetected);
    }
    env.storage().instance().set(&DataKey::Locked, &true);
    Ok(())
}

fn unlock(env: &Env) {
    env.storage().instance().set(&DataKey::Locked, &false);
}

fn update(env: &Env, balance_0: i128, balance_1: i128, reserve_0: i128, reserve_1: i128) {
    let timestamp = env.ledger().timestamp();
    let time_elapsed = timestamp.saturating_sub(get_timestamp_last(env));

    if time_elapsed > 0 && reserve_0 > 0 && reserve_1 > 0 {
        let acc_0: u128 = env
            .storage()
            .instance()
            .get(&DataKey::Price0CumulativeLast)
            .unwrap_or(0u128);
        let acc_1: u128 = env
            .storage()
            .instance()
            .get(&DataKey::Price1CumulativeLast)
            .unwrap_or(0u128);
        let price_0 = ((reserve_1 as u128) << 32) / (reserve_0 as u128);
        let price_1 = ((reserve_0 as u128) << 32) / (reserve_1 as u128);
        env.storage().instance().set(
            &DataKey::Price0CumulativeLast,
            &acc_0.wrapping_add(price_0.saturating_mul(time_elapsed as u128)),
        );
        env.storage().instance().set(
            &DataKey::Price1CumulativeLast,
            &acc_1.wrapping_add(price_1.saturating_mul(time_elapsed as u128)),
        );
    }

    env.storage().instance().set(&DataKey::Reserve0, &balance_0);
    env.storage().instance().set(&DataKey::Reserve1, &balance_1);
    env.storage()
        .instance()
        .set(&DataKey::TimestampLast, &timestamp);
    events::emit_sync(env, balance_0, balance_1);
}

fn token_balance(env: &Env, token: &Address) -> i128 {
    TokenClient::new(env, token).balance(&env.current_contract_address())
}

fn token_pull(env: &Env, token: &Address, from: &Address, amount: i128) {
    TokenClient::new(env, token).transfer_from(
        &env.current_contract_address(),
        from,
        &env.current_contract_address(),
        &amount,
    );
}

fn token_push(env: &Env, token: &Address, to: &Address, amount: i128) {
    TokenClient::new(env, token).transfer(&env.current_contract_address(), to, &amount);
}

/// Runs the strict transfer/allowance canary round trip against a single
/// pool token: approve → transfer_from → verify pool balance → transfer
/// back → verify the pool balance is zero again. Any failure — a revert, a
/// missing/consumed allowance, or a balance that doesn't move exactly as
/// requested — maps to [`Error::TokenCompatibilityFailed`]. See
/// [`NodusAmm::verify_token_compatibility`].
fn canary_token(env: &Env, token: &Address, caller: &Address, amount: i128) -> Result<(), Error> {
    let pool = env.current_contract_address();
    let client = TokenClient::new(env, token);

    // 1. Caller approves the pool for exactly `amount`.
    if client
        .try_approve(caller, &pool, &amount, &u32::MAX)
        .ok()
        .and_then(|r| r.ok())
        .is_none()
    {
        return Err(Error::TokenCompatibilityFailed);
    }

    // 2. Pool pulls the canary amount in via transfer_from.
    if client
        .try_transfer_from(&pool, caller, &pool, &amount)
        .ok()
        .and_then(|r| r.ok())
        .is_none()
    {
        return Err(Error::TokenCompatibilityFailed);
    }

    // 3. The pool balance must now be exactly `amount` — this is what
    //    catches fee-on-transfer, silently dropped, or partial transfers.
    let balance = registry::unwrap_token_call(client.try_balance(&pool))?;
    if balance != amount {
        return Err(Error::TokenCompatibilityFailed);
    }

    // 4. Pool pushes the canary amount back.
    if client
        .try_transfer(&pool, caller, &amount)
        .ok()
        .and_then(|r| r.ok())
        .is_none()
    {
        return Err(Error::TokenCompatibilityFailed);
    }

    // 5. The pool balance must be zero again, proving the full round trip.
    let balance = registry::unwrap_token_call(client.try_balance(&pool))?;
    if balance != 0 {
        return Err(Error::TokenCompatibilityFailed);
    }
    Ok(())
}

fn dead_address(env: &Env) -> Address {
    env.current_contract_address()
}

fn lp_token_client(env: &Env) -> LpTokenClient<'_> {
    let lp_token: Address = env.storage().instance().get(&DataKey::LpToken).unwrap();
    LpTokenClient::new(env, &lp_token)
}

#[contract]
pub struct NodusAmm;

#[contractimpl]
impl NodusAmm {
    /// `lp_token` must already be a deployed, uninitialized
    /// nodus-protocol-lp-token instance; the factory is responsible for
    /// deploying it and handing its address here. This contract never
    /// deploys or initializes the LP token itself.
    ///
    /// The token pair is not free-form: `token_0` must be the canonical XLM
    /// Stellar Asset Contract and `token_1` the canonical USDC Stellar
    /// Asset Contract, in that pinned order (see [`registry`]). Identity is
    /// established on-chain by **deriving** the expected SAC addresses from
    /// the reviewed canonical asset definitions (native XLM; USDC with
    /// Circle's Stellar issuer) and requiring the supplied addresses to
    /// match exactly — symbol/name/decimals alone are never proof. Each
    /// side is then additionally checked for SEP-41 behavior (metadata,
    /// decimals, zero balance at the pool) before any state is written.
    /// This rejects same-symbol impostors, incompatible contracts,
    /// wrong-network deployments, wrong-decimals tokens, reversed pairs,
    /// and unknown assets at initialization (issue #122). On success an
    /// activation event exposes the canonical asset identifiers and the
    /// pinned contract addresses.
    pub fn initialize(
        env: Env,
        token_0: Address,
        token_1: Address,
        fee_to_setter: Address,
        lp_token: Address,
    ) -> Result<(), Error> {
        if env
            .storage()
            .instance()
            .get::<DataKey, bool>(&DataKey::Initialized)
            .unwrap_or(false)
        {
            return Err(Error::AlreadyInitialized);
        }
        if token_0 == token_1 {
            return Err(Error::InvalidTokenPair);
        }
        registry::verify_canonical_token(
            &env,
            &token_0,
            &registry::XLM_ASSET_XDR,
            registry::XLM_NAME,
            registry::XLM_SYMBOL,
            registry::XLM_DECIMALS,
        )?;
        registry::verify_canonical_token(
            &env,
            &token_1,
            &registry::USDC_ASSET_XDR,
            registry::USDC_NAME,
            registry::USDC_SYMBOL,
            registry::USDC_DECIMALS,
        )?;
        env.storage().instance().set(&DataKey::Token0, &token_0);
        env.storage().instance().set(&DataKey::Token1, &token_1);
        env.storage().instance().set(&DataKey::LpToken, &lp_token);
        env.storage()
            .instance()
            .set(&DataKey::FeeToSetter, &fee_to_setter);
        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_BUMP);
        events::emit_pool_activated(
            &env,
            token_0,
            token_1,
            String::from_str(&env, registry::XLM_NAME),
            String::from_str(&env, registry::USDC_NAME),
        );
        Ok(())
    }

    /// Post-deploy transfer/allowance compatibility canary (issue #122).
    ///
    /// Runs a strict, tiny round trip against **both** pool tokens to prove
    /// their write path works end to end before liquidity is enabled:
    /// `caller` approves the pool, the pool pulls `amount` in via
    /// `transfer_from`, verifies its balance increased by exactly `amount`,
    /// pushes it back via `transfer`, and verifies the pool balance is again
    /// zero. Any revert, missing allowance, or balance that does not move
    /// exactly as requested fails with [`Error::TokenCompatibilityFailed`].
    /// This catches a canonical SAC that has been upgraded or replaced with
    /// a non-conforming implementation (fee-on-transfer, silently dropped
    /// transfers, broken authorization) even though it sits at the derived
    /// canonical address.
    ///
    /// Gated to the `fee_to_setter` admin, limited to `1..=10` stroops per
    /// side (a strict canary limit), and reentrancy-locked like the
    /// liquidity entrypoints. Recorded via `canary_verified()`.
    pub fn verify_token_compatibility(
        env: Env,
        caller: Address,
        amount: i128,
    ) -> Result<(), Error> {
        require_initialized(&env)?;
        require_fee_to_setter(&env, &caller)?;
        if !(1..=CANARY_MAX_AMOUNT).contains(&amount) {
            return Err(Error::InvalidCanaryAmount);
        }
        lock(&env)?;
        let result = (|| -> Result<(), Error> {
            let token_0: Address = env.storage().instance().get(&DataKey::Token0).unwrap();
            let token_1: Address = env.storage().instance().get(&DataKey::Token1).unwrap();
            canary_token(&env, &token_0, &caller, amount)?;
            canary_token(&env, &token_1, &caller, amount)?;
            Ok(())
        })();
        if result.is_err() {
            unlock(&env);
            return result;
        }
        env.storage()
            .instance()
            .set(&DataKey::CanaryVerified, &true);
        unlock(&env);
        events::emit_canary_passed(&env, caller);
        Ok(())
    }

    /// Whether the post-deploy transfer/allowance compatibility canary has
    /// completed successfully. Lets deploy automation and off-chain
    /// monitors confirm the pool passed its activation check.
    pub fn canary_verified(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::CanaryVerified)
            .unwrap_or(false)
    }

    pub fn add_liquidity(
        env: Env,
        from: Address,
        to: Address,
        amount_0_desired: i128,
        amount_1_desired: i128,
        amount_0_min: i128,
        amount_1_min: i128,
        deadline: u64,
    ) -> Result<i128, Error> {
        require_initialized(&env)?;
        require_not_paused(&env)?;
        if env.ledger().timestamp() > deadline {
            return Err(Error::Expired);
        }
        lock(&env)?;
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_BUMP);

        from.require_auth();

        let token_0: Address = env.storage().instance().get(&DataKey::Token0).unwrap();
        let token_1: Address = env.storage().instance().get(&DataKey::Token1).unwrap();

        let reserve_0 = get_reserve_0(&env);
        let reserve_1 = get_reserve_1(&env);

        let (amount_0, amount_1) = if reserve_0 == 0 && reserve_1 == 0 {
            (amount_0_desired, amount_1_desired)
        } else {
            liquidity_pool::calculate_optimal_amounts(
                amount_0_desired,
                amount_1_desired,
                amount_0_min,
                amount_1_min,
                reserve_0,
                reserve_1,
            )
            .inspect_err(|_| unlock(&env))?
        };

        token_pull(&env, &token_0, &from, amount_0);
        token_pull(&env, &token_1, &from, amount_1);

        let lp_client = lp_token_client(&env);
        let this_contract = env.current_contract_address();
        let total_supply = lp_client.total_supply();

        let liquidity = if total_supply == 0 {
            let initial = liquidity_pool::calculate_initial_liquidity(amount_0, amount_1)
                .inspect_err(|_| unlock(&env))?;
            lp_client.mint(
                &this_contract,
                &dead_address(&env),
                &math::MINIMUM_LIQUIDITY,
            );
            initial
        } else {
            liquidity_pool::calculate_liquidity_to_mint(
                amount_0,
                amount_1,
                reserve_0,
                reserve_1,
                total_supply,
            )
            .inspect_err(|_| unlock(&env))?
        };

        if liquidity == 0 {
            unlock(&env);
            return Err(Error::InsufficientLiquidityMinted);
        }

        lp_client.mint(&this_contract, &to, &liquidity);

        let b0 = token_balance(&env, &token_0);
        let b1 = token_balance(&env, &token_1);
        update(&env, b0, b1, reserve_0, reserve_1);

        events::emit_mint(&env, from, amount_0, amount_1);
        unlock(&env);
        Ok(liquidity)
    }

    pub fn remove_liquidity(
        env: Env,
        from: Address,
        to: Address,
        liquidity: i128,
        amount_0_min: i128,
        amount_1_min: i128,
        deadline: u64,
    ) -> Result<(i128, i128), Error> {
        require_initialized(&env)?;
        require_not_paused(&env)?;
        if env.ledger().timestamp() > deadline {
            return Err(Error::Expired);
        }
        lock(&env)?;
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_BUMP);

        from.require_auth();

        let token_0: Address = env.storage().instance().get(&DataKey::Token0).unwrap();
        let token_1: Address = env.storage().instance().get(&DataKey::Token1).unwrap();

        let lp_client = lp_token_client(&env);
        let total_supply = lp_client.total_supply();
        let reserve_0 = get_reserve_0(&env);
        let reserve_1 = get_reserve_1(&env);

        let (amount_0, amount_1) = liquidity_pool::calculate_withdrawal_amounts(
            liquidity,
            reserve_0,
            reserve_1,
            total_supply,
        )
        .inspect_err(|_| unlock(&env))?;

        if amount_0 < amount_0_min || amount_1 < amount_1_min {
            unlock(&env);
            return Err(Error::InsufficientLiquidityBurned);
        }

        // Burns from's own LP tokens; from already authorized this whole
        // call above, and that same authorization covers this nested
        // require_auth() on the LP token contract. Panics (reverting the
        // whole transaction, same as everywhere else in this contract
        // that unwraps an internal invariant) if from's real on-chain LP
        // balance is less than the amount they asked to redeem.
        lp_client.burn(&from, &liquidity);

        token_push(&env, &token_0, &to, amount_0);
        token_push(&env, &token_1, &to, amount_1);

        let b0 = token_balance(&env, &token_0);
        let b1 = token_balance(&env, &token_1);
        update(&env, b0, b1, reserve_0, reserve_1);

        events::emit_burn(&env, from, amount_0, amount_1, to);
        unlock(&env);
        Ok((amount_0, amount_1))
    }

    pub fn swap(
        env: Env,
        to: Address,
        amount_0_out: i128,
        amount_1_out: i128,
        deadline: u64,
    ) -> Result<(), Error> {
        require_initialized(&env)?;
        require_not_paused(&env)?;
        if env.ledger().timestamp() > deadline {
            return Err(Error::Expired);
        }
        lock(&env)?;
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_BUMP);

        if amount_0_out == 0 && amount_1_out == 0 {
            unlock(&env);
            return Err(Error::InsufficientOutputAmount);
        }

        let token_0: Address = env.storage().instance().get(&DataKey::Token0).unwrap();
        let token_1: Address = env.storage().instance().get(&DataKey::Token1).unwrap();

        let reserve_0 = get_reserve_0(&env);
        let reserve_1 = get_reserve_1(&env);

        if amount_0_out >= reserve_0 || amount_1_out >= reserve_1 {
            unlock(&env);
            return Err(Error::InsufficientLiquidity);
        }

        if amount_0_out > 0 {
            token_push(&env, &token_0, &to, amount_0_out);
        }
        if amount_1_out > 0 {
            token_push(&env, &token_1, &to, amount_1_out);
        }

        let balance_0 = token_balance(&env, &token_0);
        let balance_1 = token_balance(&env, &token_1);

        let amount_0_in = balance_0.saturating_sub(reserve_0.saturating_sub(amount_0_out));
        let amount_1_in = balance_1.saturating_sub(reserve_1.saturating_sub(amount_1_out));

        if amount_0_in == 0 && amount_1_in == 0 {
            unlock(&env);
            return Err(Error::InsufficientLiquidity);
        }

        liquidity_pool::verify_k_invariant(
            balance_0,
            balance_1,
            amount_0_in,
            amount_1_in,
            reserve_0,
            reserve_1,
        )
        .inspect_err(|_| unlock(&env))?;

        update(&env, balance_0, balance_1, reserve_0, reserve_1);

        let caller = env.current_contract_address();
        events::emit_swap(
            &env,
            caller,
            amount_0_in,
            amount_1_in,
            amount_0_out,
            amount_1_out,
            to,
        );
        unlock(&env);
        Ok(())
    }

    pub fn sync(env: Env) -> Result<(), Error> {
        require_initialized(&env)?;
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_BUMP);
        let token_0: Address = env.storage().instance().get(&DataKey::Token0).unwrap();
        let token_1: Address = env.storage().instance().get(&DataKey::Token1).unwrap();
        let b0 = token_balance(&env, &token_0);
        let b1 = token_balance(&env, &token_1);
        update(&env, b0, b1, get_reserve_0(&env), get_reserve_1(&env));
        Ok(())
    }

    pub fn get_reserves(env: Env) -> (i128, i128, u64) {
        (
            get_reserve_0(&env),
            get_reserve_1(&env),
            get_timestamp_last(&env),
        )
    }

    pub fn get_price_cumulative(env: Env) -> (u128, u128) {
        (
            env.storage()
                .instance()
                .get(&DataKey::Price0CumulativeLast)
                .unwrap_or(0u128),
            env.storage()
                .instance()
                .get(&DataKey::Price1CumulativeLast)
                .unwrap_or(0u128),
        )
    }

    pub fn get_amount_out(
        _env: Env,
        amount_in: i128,
        reserve_in: i128,
        reserve_out: i128,
    ) -> Result<i128, Error> {
        math::get_amount_out(amount_in, reserve_in, reserve_out)
    }

    pub fn get_amount_in(
        _env: Env,
        amount_out: i128,
        reserve_in: i128,
        reserve_out: i128,
    ) -> Result<i128, Error> {
        math::get_amount_in(amount_out, reserve_in, reserve_out)
    }

    // ── High-level swap entrypoints ─────────────────────────────────────────

    pub fn swap_exact_tokens_for_tokens(
        env: Env,
        from: Address,
        to: Address,
        amount_in: i128,
        amount_out_min: i128,
        zero_for_one: bool,
        deadline: u64,
    ) -> Result<i128, Error> {
        require_initialized(&env)?;
        require_not_paused(&env)?;
        if env.ledger().timestamp() > deadline {
            return Err(Error::Expired);
        }
        lock(&env)?;
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_BUMP);

        from.require_auth();

        let token_0: Address = env.storage().instance().get(&DataKey::Token0).unwrap();
        let token_1: Address = env.storage().instance().get(&DataKey::Token1).unwrap();

        let reserve_0 = get_reserve_0(&env);
        let reserve_1 = get_reserve_1(&env);

        let (reserve_in, reserve_out, token_in, token_out) = if zero_for_one {
            (reserve_0, reserve_1, token_0.clone(), token_1.clone())
        } else {
            (reserve_1, reserve_0, token_1.clone(), token_0.clone())
        };

        let amount_out = math::get_amount_out(amount_in, reserve_in, reserve_out)
            .inspect_err(|_| unlock(&env))?;

        if amount_out < amount_out_min {
            unlock(&env);
            return Err(Error::SlippageTooHigh);
        }

        token_pull(&env, &token_in, &from, amount_in);
        token_push(&env, &token_out, &to, amount_out);

        let b0 = token_balance(&env, &token_0);
        let b1 = token_balance(&env, &token_1);

        let (amount_0_in, amount_1_in, amount_0_out, amount_1_out) = if zero_for_one {
            (amount_in, 0i128, 0i128, amount_out)
        } else {
            (0i128, amount_in, amount_out, 0i128)
        };

        liquidity_pool::verify_k_invariant(b0, b1, amount_0_in, amount_1_in, reserve_0, reserve_1)
            .inspect_err(|_| unlock(&env))?;

        update(&env, b0, b1, reserve_0, reserve_1);
        events::emit_swap(
            &env,
            from,
            amount_0_in,
            amount_1_in,
            amount_0_out,
            amount_1_out,
            to,
        );
        unlock(&env);
        Ok(amount_out)
    }

    pub fn swap_tokens_for_exact_tokens(
        env: Env,
        from: Address,
        to: Address,
        amount_out: i128,
        amount_in_max: i128,
        zero_for_one: bool,
        deadline: u64,
    ) -> Result<i128, Error> {
        require_initialized(&env)?;
        require_not_paused(&env)?;
        if env.ledger().timestamp() > deadline {
            return Err(Error::Expired);
        }
        lock(&env)?;
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_BUMP);

        from.require_auth();

        let token_0: Address = env.storage().instance().get(&DataKey::Token0).unwrap();
        let token_1: Address = env.storage().instance().get(&DataKey::Token1).unwrap();

        let reserve_0 = get_reserve_0(&env);
        let reserve_1 = get_reserve_1(&env);

        let (reserve_in, reserve_out, token_in, token_out) = if zero_for_one {
            (reserve_0, reserve_1, token_0.clone(), token_1.clone())
        } else {
            (reserve_1, reserve_0, token_1.clone(), token_0.clone())
        };

        let amount_in = math::get_amount_in(amount_out, reserve_in, reserve_out)
            .inspect_err(|_| unlock(&env))?;

        if amount_in > amount_in_max {
            unlock(&env);
            return Err(Error::SlippageTooHigh);
        }

        token_pull(&env, &token_in, &from, amount_in);
        token_push(&env, &token_out, &to, amount_out);

        let b0 = token_balance(&env, &token_0);
        let b1 = token_balance(&env, &token_1);

        let (amount_0_in, amount_1_in, amount_0_out, amount_1_out) = if zero_for_one {
            (amount_in, 0i128, 0i128, amount_out)
        } else {
            (0i128, amount_in, amount_out, 0i128)
        };

        liquidity_pool::verify_k_invariant(b0, b1, amount_0_in, amount_1_in, reserve_0, reserve_1)
            .inspect_err(|_| unlock(&env))?;

        update(&env, b0, b1, reserve_0, reserve_1);
        events::emit_swap(
            &env,
            from,
            amount_0_in,
            amount_1_in,
            amount_0_out,
            amount_1_out,
            to,
        );
        unlock(&env);
        Ok(amount_in)
    }

    pub fn get_spot_price(env: Env, zero_for_one: bool) -> Result<i128, Error> {
        require_initialized(&env)?;
        let reserve_0 = get_reserve_0(&env);
        let reserve_1 = get_reserve_1(&env);
        if zero_for_one {
            math::get_spot_price(reserve_0, reserve_1)
        } else {
            math::get_spot_price(reserve_1, reserve_0)
        }
    }

    pub fn get_price_impact(env: Env, amount_in: i128, zero_for_one: bool) -> Result<i128, Error> {
        require_initialized(&env)?;
        let reserve_0 = get_reserve_0(&env);
        let reserve_1 = get_reserve_1(&env);
        let (reserve_in, reserve_out) = if zero_for_one {
            (reserve_0, reserve_1)
        } else {
            (reserve_1, reserve_0)
        };
        let amount_out = math::get_amount_out(amount_in, reserve_in, reserve_out)?;
        math::calculate_price_impact(amount_in, amount_out, reserve_in, reserve_out)
    }

    // ── Protocol fee collector ──────────────────────────────────────────────

    /// Sets the recipient of the protocol fee.
    ///
    /// # Important
    /// Protocol fee collection is currently **not implemented** in the pool.
    /// This is a reserved administrative endpoint; setting this value updates the configuration
    /// but has no functional impact and no fees will accrue to the configured address.
    pub fn set_fee_to(env: Env, caller: Address, new_fee_to: Option<Address>) -> Result<(), Error> {
        require_fee_to_setter(&env, &caller)?;
        match &new_fee_to {
            Some(addr) => env.storage().instance().set(&DataKey::FeeTo, addr),
            None => env.storage().instance().remove(&DataKey::FeeTo),
        }
        Ok(())
    }

    pub fn set_fee_to_setter(env: Env, caller: Address, new_setter: Address) -> Result<(), Error> {
        require_fee_to_setter(&env, &caller)?;
        env.storage()
            .instance()
            .set(&DataKey::FeeToSetter, &new_setter);
        Ok(())
    }

    /// Returns the protocol fee recipient address, if configured.
    ///
    /// # Important
    /// Protocol fee collection is currently **not implemented** in the pool.
    /// This function is non-functional/inert and is only a configuration query.
    pub fn fee_to(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::FeeTo)
    }

    pub fn fee_to_setter(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::FeeToSetter)
            .ok_or(Error::NotInitialized)
    }

    // ── Emergency pause ──────────────────────────────────────────────────────

    /// Halts add_liquidity, remove_liquidity, and all swap entrypoints.
    /// Callable only by the fee_to_setter admin address.
    pub fn pause(env: Env, caller: Address) -> Result<(), Error> {
        require_initialized(&env)?;
        require_fee_to_setter(&env, &caller)?;
        env.storage().instance().set(&DataKey::Paused, &true);
        events::emit_paused(&env, caller);
        Ok(())
    }

    /// Resumes normal operation after a [`Self::pause`].
    pub fn unpause(env: Env, caller: Address) -> Result<(), Error> {
        require_initialized(&env)?;
        require_fee_to_setter(&env, &caller)?;
        env.storage().instance().set(&DataKey::Paused, &false);
        events::emit_unpaused(&env, caller);
        Ok(())
    }

    pub fn is_paused(env: Env) -> bool {
        is_paused(&env)
    }

    // ── LP token ─────────────────────────────────────────────────────────────

    /// The standalone SEP-41 LP token contract for this pool. Balance,
    /// transfer, approve, and supply queries all live there now --
    /// interact with it directly rather than through this contract.
    pub fn lp_token(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::LpToken)
            .ok_or(Error::NotInitialized)
    }

    pub fn token_0(env: Env) -> Result<Address, Error> {
        require_initialized(&env)?;
        Ok(env.storage().instance().get(&DataKey::Token0).unwrap())
    }

    pub fn token_1(env: Env) -> Result<Address, Error> {
        require_initialized(&env)?;
        Ok(env.storage().instance().get(&DataKey::Token1).unwrap())
    }
}
