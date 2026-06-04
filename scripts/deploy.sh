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

WASM="target/wasm32-unknown-unknown/release/nodus_amm.wasm"
if [ ! -f "$WASM" ]; then
    echo "WASM not found. Run: make build"
    exit 1
fi

echo "Uploading contract to $NETWORK..."
CONTRACT_HASH=$(stellar contract upload \
    --wasm "$WASM" \
    --source "$STELLAR_SECRET_KEY" \
    --rpc-url "$RPC_URL" \
    --network-passphrase "$NETWORK_PASSPHRASE")

echo "Contract hash: $CONTRACT_HASH"

: "${TOKEN_0:?Set TOKEN_0 to the first token contract address}"
: "${TOKEN_1:?Set TOKEN_1 to the second token contract address}"

echo "Deploying NodusAmm pool..."
CONTRACT_ID=$(stellar contract deploy \
    --wasm-hash "$CONTRACT_HASH" \
    --source "$STELLAR_SECRET_KEY" \
    --rpc-url "$RPC_URL" \
    --network-passphrase "$NETWORK_PASSPHRASE")

echo "Contract deployed: $CONTRACT_ID"

stellar contract invoke \
    --id "$CONTRACT_ID" \
    --source "$STELLAR_SECRET_KEY" \
    --rpc-url "$RPC_URL" \
    --network-passphrase "$NETWORK_PASSPHRASE" \
    -- initialize \
    --token_0 "$TOKEN_0" \
    --token_1 "$TOKEN_1"

echo "Pool initialized. Contract ID: $CONTRACT_ID"
