#!/bin/bash
# Example script for running tx-perf-test with testnet coin generation

# Generate a new test private key (or use an existing one)
PRIVATE_KEY="1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"

# Recipient address (can be the same wallet or a different one)
RECIPIENT="TIME1test_recipient_address"

# Amount to mint for testing (in smallest unit)
MINT_AMOUNT=10000000  # 10 million test coins

echo "🚀 Running TX Performance Test with Testnet Coin Generation"
echo "============================================================"
echo ""
echo "This example demonstrates:"
echo "  1. Generating testnet coins (minting)"
echo "  2. Running performance tests with the generated coins"
echo ""

# Example 1: Basic test with coin generation
echo "Example 1: Basic Performance Test"
echo "----------------------------------"
cargo run --release -- \
  --private-key $PRIVATE_KEY \
  --recipient $RECIPIENT \
  --network testnet \
  --mint-coins $MINT_AMOUNT \
  --tx-count 10 \
  --amount 1000 \
  --fee 100 \
  --verbose

echo ""
echo "Example 2: Burst Test (Maximum Throughput)"
echo "-------------------------------------------"
cargo run --release -- \
  --private-key $PRIVATE_KEY \
  --recipient $RECIPIENT \
  --network testnet \
  --mint-coins 5000000 \
  --tx-count 100 \
  --amount 1000 \
  --fee 100 \
  --delay-ms 0 \
  --output burst-test-results.json

echo ""
echo "Example 3: Sustained Load Test"
echo "-------------------------------"
cargo run --release -- \
  --private-key $PRIVATE_KEY \
  --recipient $RECIPIENT \
  --network testnet \
  --mint-coins 3000000 \
  --tx-count 50 \
  --amount 1000 \
  --fee 100 \
  --delay-ms 500 \
  --output sustained-test-results.json

echo ""
echo "✅ All tests completed!"
echo ""
echo "Note: The --mint-coins flag only works with testnet/devnet."
echo "      For mainnet testing, you need a pre-funded wallet."
