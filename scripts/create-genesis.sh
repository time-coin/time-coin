#!/bin/bash
# Generate TIME Coin Testnet Genesis Block

set -e

GENESIS_DIR="${1:-./config}"
GENESIS_FILE="$GENESIS_DIR/genesis-testnet.json"

echo "Creating testnet genesis block..."
echo "Output: $GENESIS_FILE"

# Create config directory if it doesn't exist
mkdir -p "$GENESIS_DIR"

# Testnet genesis timestamp: October 24, 2024 00:00:00 UTC
GENESIS_TIMESTAMP=1729728000

# Create genesis block
cat > "$GENESIS_FILE" << 'GENESIS'
{
  "network": "testnet",
  "version": 1,
  "timestamp": 1729728000,
  "message": "TIME Coin Testnet - 24 Hour Blocks, Instant Finality",
  "hash": "00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048",
  "transactions": [
    {
      "amount": 1000000000000000,
      "address": "testnet_treasury",
      "description": "Testnet Treasury (10M TIME)"
    },
    {
      "amount": 500000000000000,
      "address": "testnet_dev_fund",
      "description": "Development Fund (5M TIME)"
    },
    {
      "amount": 500000000000000,
      "address": "testnet_faucet",
      "description": "Testnet Faucet (5M TIME)"
    }
  ]
}
GENESIS

echo ""
echo "✅ Genesis block created successfully!"
echo ""
echo "Details:"
echo "  Network:      testnet"
echo "  Timestamp:    $(date -d @$GENESIS_TIMESTAMP '+%Y-%m-%d %H:%M:%S UTC' 2>/dev/null || date -r $GENESIS_TIMESTAMP '+%Y-%m-%d %H:%M:%S UTC' 2>/dev/null || echo '2024-10-24 00:00:00 UTC')"
echo "  Total Supply: 20,000,000 TIME"
echo "  Allocations:"
echo "    - Treasury:    10,000,000 TIME"
echo "    - Dev Fund:     5,000,000 TIME"
echo "    - Faucet:       5,000,000 TIME"
echo ""
echo "Genesis file: $GENESIS_FILE"
