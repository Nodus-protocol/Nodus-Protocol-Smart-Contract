#[cfg(test)]
#[cfg(feature = "testutils")]
mod integration {
    use nodus_protocol_lp_token::{Error, NodusLpToken, NodusLpTokenClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger as _},
        Address, Env, MuxedAddress, String,
    };

    fn setup() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract = env.register(NodusLpToken, ());
        let client = NodusLpTokenClient::new(&env, &contract);
        let pool = Address::generate(&env);
        client.initialize(
            &pool,
            &String::from_str(&env, "Nodus LP XLM/USDC"),
            &String::from_str(&env, "NODUS-LP"),
            &7,
        );
        (env, contract, pool)
    }

    #[test]
    fn initialize_sets_metadata() {
        let (env, contract, pool) = setup();
        let client = NodusLpTokenClient::new(&env, &contract);
        assert_eq!(client.pool(), pool);
        assert_eq!(client.name(), String::from_str(&env, "Nodus LP XLM/USDC"));
        assert_eq!(client.symbol(), String::from_str(&env, "NODUS-LP"));
        assert_eq!(client.decimals(), 7);
        assert_eq!(client.total_supply(), 0);
    }

    #[test]
    fn double_initialize_rejected() {
        let (env, contract, pool) = setup();
        let client = NodusLpTokenClient::new(&env, &contract);
        assert_eq!(
            client.try_initialize(
                &pool,
                &String::from_str(&env, "x"),
                &String::from_str(&env, "x"),
                &7,
            ),
            Err(Ok(Error::AlreadyInitialized)),
        );
    }

    #[test]
    fn metadata_queries_fail_before_initialize() {
        let env = Env::default();
        let contract = env.register(NodusLpToken, ());
        let client = NodusLpTokenClient::new(&env, &contract);
        assert!(client.try_name().is_err());
        assert!(client.try_symbol().is_err());
        assert!(client.try_decimals().is_err());
        assert!(client.try_pool().is_err());
    }

    #[test]
    fn pool_can_mint() {
        let (env, contract, pool) = setup();
        let client = NodusLpTokenClient::new(&env, &contract);
        let holder = Address::generate(&env);

        client.mint(&pool, &holder, &1_000);

        assert_eq!(client.balance(&holder), 1_000);
        assert_eq!(client.total_supply(), 1_000);
    }

    #[test]
    fn non_pool_cannot_mint() {
        let (env, contract, _pool) = setup();
        let client = NodusLpTokenClient::new(&env, &contract);
        let intruder = Address::generate(&env);
        let holder = Address::generate(&env);

        assert_eq!(
            client.try_mint(&intruder, &holder, &1_000),
            Err(Ok(Error::Unauthorized)),
        );
        assert_eq!(client.total_supply(), 0);
    }

    #[test]
    fn mint_rejects_non_positive_amount() {
        let (env, contract, pool) = setup();
        let client = NodusLpTokenClient::new(&env, &contract);
        let holder = Address::generate(&env);

        assert_eq!(
            client.try_mint(&pool, &holder, &0),
            Err(Ok(Error::ZeroAmount)),
        );
        assert_eq!(
            client.try_mint(&pool, &holder, &-5),
            Err(Ok(Error::ZeroAmount)),
        );
    }

    #[test]
    fn holder_can_burn_own_tokens() {
        let (env, contract, pool) = setup();
        let client = NodusLpTokenClient::new(&env, &contract);
        let holder = Address::generate(&env);
        client.mint(&pool, &holder, &1_000);

        client.burn(&holder, &400);

        assert_eq!(client.balance(&holder), 600);
        assert_eq!(client.total_supply(), 600);
    }

    #[test]
    fn burn_rejects_insufficient_balance() {
        let (env, contract, pool) = setup();
        let client = NodusLpTokenClient::new(&env, &contract);
        let holder = Address::generate(&env);
        client.mint(&pool, &holder, &100);

        assert_eq!(
            client.try_burn(&holder, &200),
            Err(Ok(Error::InsufficientBalance)),
        );
    }

    #[test]
    fn transfer_moves_balance() {
        let (env, contract, pool) = setup();
        let client = NodusLpTokenClient::new(&env, &contract);
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        client.mint(&pool, &alice, &1_000);

        client.transfer(&alice, MuxedAddress::from(bob.clone()), &300);

        assert_eq!(client.balance(&alice), 700);
        assert_eq!(client.balance(&bob), 300);
    }

    #[test]
    fn transfer_rejects_insufficient_balance() {
        let (env, contract, pool) = setup();
        let client = NodusLpTokenClient::new(&env, &contract);
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        client.mint(&pool, &alice, &100);

        assert_eq!(
            client.try_transfer(&alice, MuxedAddress::from(bob), &200),
            Err(Ok(Error::InsufficientBalance)),
        );
    }

    #[test]
    fn approve_and_transfer_from() {
        let (env, contract, pool) = setup();
        let client = NodusLpTokenClient::new(&env, &contract);
        let owner = Address::generate(&env);
        let spender = Address::generate(&env);
        let recipient = Address::generate(&env);
        client.mint(&pool, &owner, &1_000);
        env.ledger().set_sequence_number(100);

        client.approve(&owner, &spender, &500, &1_100);
        assert_eq!(client.allowance(&owner, &spender), 500);

        client.transfer_from(&spender, &owner, &recipient, &300);

        assert_eq!(client.balance(&owner), 700);
        assert_eq!(client.balance(&recipient), 300);
        assert_eq!(client.allowance(&owner, &spender), 200);
    }

    #[test]
    fn transfer_from_rejects_over_allowance() {
        let (env, contract, pool) = setup();
        let client = NodusLpTokenClient::new(&env, &contract);
        let owner = Address::generate(&env);
        let spender = Address::generate(&env);
        let recipient = Address::generate(&env);
        client.mint(&pool, &owner, &1_000);
        env.ledger().set_sequence_number(100);
        client.approve(&owner, &spender, &100, &1_100);

        assert_eq!(
            client.try_transfer_from(&spender, &owner, &recipient, &200),
            Err(Ok(Error::Unauthorized)),
        );
    }

    #[test]
    fn approve_rejects_an_already_expired_ledger_for_a_positive_amount() {
        let (env, contract, _pool) = setup();
        let client = NodusLpTokenClient::new(&env, &contract);
        let owner = Address::generate(&env);
        let spender = Address::generate(&env);
        env.ledger().set_sequence_number(1_000);

        assert_eq!(
            client.try_approve(&owner, &spender, &100, &500),
            Err(Ok(Error::ApprovalExpired)),
        );
    }

    #[test]
    fn approve_with_zero_amount_revokes_regardless_of_expiration() {
        let (env, contract, pool) = setup();
        let client = NodusLpTokenClient::new(&env, &contract);
        let owner = Address::generate(&env);
        let spender = Address::generate(&env);
        client.mint(&pool, &owner, &1_000);
        env.ledger().set_sequence_number(100);
        client.approve(&owner, &spender, &500, &1_100);

        // Revoking with an already-past expiration_ledger must still work.
        client.approve(&owner, &spender, &0, &1);

        assert_eq!(client.allowance(&owner, &spender), 0);
    }

    #[test]
    fn burn_from_spends_allowance_and_reduces_supply() {
        let (env, contract, pool) = setup();
        let client = NodusLpTokenClient::new(&env, &contract);
        let owner = Address::generate(&env);
        let spender = Address::generate(&env);
        client.mint(&pool, &owner, &1_000);
        env.ledger().set_sequence_number(100);
        client.approve(&owner, &spender, &400, &1_100);

        client.burn_from(&spender, &owner, &400);

        assert_eq!(client.balance(&owner), 600);
        assert_eq!(client.total_supply(), 600);
        assert_eq!(client.allowance(&owner, &spender), 0);
    }
}
