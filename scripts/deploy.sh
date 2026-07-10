#!/usr/bin/env bash
set -euo pipefail

NETWORK="${1:-testnet}"
: "${STELLAR_SECRET_KEY:?Set STELLAR_SECRET_KEY to your Stellar secret (S...)}"

case "$NETWORK" in
  testnet)
    RPC_URL="https://soroban-testnet.stellar.org"
    NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
    ;;
  mainnet)
    RPC_URL="https://soroban-rpc.stellar.org"
    NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
    ;;
  *)
    echo "Unknown network: '$NETWORK'. Use 'testnet' or 'mainnet'." >&2
    exit 1
    ;;
esac

if ! command -v stellar &>/dev/null; then
    echo "Stellar CLI not found. Install: https://developers.stellar.org/docs/tools/cli"
    exit 1
fi

POOL_WASM="target/wasm32v1-none/release/nodus_protocol_amm.wasm"
LP_TOKEN_WASM="target/wasm32v1-none/release/nodus_protocol_lp_token.wasm"
if [ ! -f "$POOL_WASM" ] || [ ! -f "$LP_TOKEN_WASM" ]; then
    echo "WASM not found. Run: make build"
    exit 1
fi

: "${TOKEN_0:?Set TOKEN_0 to the first token contract address}"
: "${TOKEN_1:?Set TOKEN_1 to the second token contract address}"
: "${FEE_TO_SETTER:?Set FEE_TO_SETTER to the address allowed to set the protocol fee and pause the pool}"
LP_TOKEN_NAME="${LP_TOKEN_NAME:-Nodus LP Token}"
LP_TOKEN_SYMBOL="${LP_TOKEN_SYMBOL:-NODUS-LP}"
LP_TOKEN_DECIMALS="${LP_TOKEN_DECIMALS:-7}"

invoke() {
    stellar contract invoke \
        --source "$STELLAR_SECRET_KEY" \
        --rpc-url "$RPC_URL" \
        --network-passphrase "$NETWORK_PASSPHRASE" \
        "$@"
}

echo "Uploading pool contract to $NETWORK..."
POOL_HASH=$(stellar contract upload \
    --wasm "$POOL_WASM" \
    --source "$STELLAR_SECRET_KEY" \
    --rpc-url "$RPC_URL" \
    --network-passphrase "$NETWORK_PASSPHRASE")
echo "Pool contract hash: $POOL_HASH"

echo "Uploading LP token contract to $NETWORK..."
LP_TOKEN_HASH=$(stellar contract upload \
    --wasm "$LP_TOKEN_WASM" \
    --source "$STELLAR_SECRET_KEY" \
    --rpc-url "$RPC_URL" \
    --network-passphrase "$NETWORK_PASSPHRASE")
echo "LP token contract hash: $LP_TOKEN_HASH"

echo "Deploying pool..."
POOL_ID=$(stellar contract deploy \
    --wasm-hash "$POOL_HASH" \
    --source "$STELLAR_SECRET_KEY" \
    --rpc-url "$RPC_URL" \
    --network-passphrase "$NETWORK_PASSPHRASE")
echo "Pool deployed: $POOL_ID"

echo "Deploying LP token..."
LP_TOKEN_ID=$(stellar contract deploy \
    --wasm-hash "$LP_TOKEN_HASH" \
    --source "$STELLAR_SECRET_KEY" \
    --rpc-url "$RPC_URL" \
    --network-passphrase "$NETWORK_PASSPHRASE")
echo "LP token deployed: $LP_TOKEN_ID"

# The LP token must know its pool's address (the only address it will
# ever accept mint() calls from) before the pool is initialized, so this
# order matters: LP token first, then the pool.
echo "Initializing LP token..."
invoke --id "$LP_TOKEN_ID" \
    -- initialize \
    --pool "$POOL_ID" \
    --name "$LP_TOKEN_NAME" \
    --symbol "$LP_TOKEN_SYMBOL" \
    --decimals "$LP_TOKEN_DECIMALS"

echo "Initializing pool..."
invoke --id "$POOL_ID" \
    -- initialize \
    --token_0 "$TOKEN_0" \
    --token_1 "$TOKEN_1" \
    --fee_to_setter "$FEE_TO_SETTER" \
    --lp_token "$LP_TOKEN_ID"

echo "Done. Pool: $POOL_ID  LP token: $LP_TOKEN_ID"
