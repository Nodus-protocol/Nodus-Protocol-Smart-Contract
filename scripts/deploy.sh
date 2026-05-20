#!/usr/bin/env bash
set -euo pipefail

NETWORK="${1:-local}"
SURI="${SURI:-//Alice}"

: "${TOKEN_0:?TOKEN_0 environment variable is required}"
: "${TOKEN_1:?TOKEN_1 environment variable is required}"
: "${LP_TOKEN:?LP_TOKEN environment variable is required}"

case "$NETWORK" in
  local)
    URL="ws://127.0.0.1:9944"
    ;;
  testnet)
    URL="wss://ws.test.azero.dev"
    ;;
  *)
    echo "Unknown network: '$NETWORK'. Use 'local' or 'testnet'." >&2
    exit 1
    ;;
esac

echo "Deploying to $NETWORK ($URL)..."

cargo contract upload \
    --url "$URL" \
    --suri "$SURI" \
    --execute

cargo contract instantiate \
    --url "$URL" \
    --suri "$SURI" \
    --constructor new \
    --args "$TOKEN_0" "$TOKEN_1" "$LP_TOKEN" \
    --execute

echo "Deployment complete."
