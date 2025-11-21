#!/bin/bash
# Run transaction performance test with random addresses on testnet

# Generate a temporary test key (or use existing one)
PRIVATE_KEY="0000000000000000000000000000000000000000000000000000000000000001"
RECIPIENT="TIME0n1henHJ8n2MVCxiJMU4dYG3EusKA6czsGy"  # Dummy, will be random
API_URL="http://161.35.129.70:24101"

echo "🚀 Running transaction performance test with RANDOM addresses"
echo "=============================================================="
echo ""
echo "Configuration:"
echo "  API Node:     $API_URL"
echo "  Transactions: 50"
echo "  Mode:         Random recipient addresses"
echo "  Amount:       100 TIME per transaction"
echo "  Fee:          10 TIME per transaction"
echo ""

cd "$(dirname "$0")"

cargo run --release -- \
  --api-url "$API_URL" \
  --private-key "$PRIVATE_KEY" \
  --recipient "$RECIPIENT" \
  --network testnet \
  --mint-coins 10000000 \
  --tx-count 50 \
  --amount 100 \
  --fee 10 \
  --delay-ms 100 \
  --random-addresses \
  --output results.json \
  --verbose

echo ""
echo "✅ Test completed! Check results.json for detailed metrics"
