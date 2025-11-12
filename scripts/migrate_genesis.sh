#!/bin/bash
# Migration script to update genesis file from old to new format
#
# This script helps migrate from the old genesis format (simple transaction list)
# to the new format (complete block structure with preserved hash)
#
# Usage: ./scripts/migrate_genesis.sh [path_to_old_genesis.json]

set -e

OLD_GENESIS="${1:-config/genesis-testnet.json}"
BACKUP="${OLD_GENESIS}.backup.$(date +%Y%m%d_%H%M%S)"

echo "Genesis Migration Script"
echo "========================"
echo ""

# Check if file exists
if [ ! -f "$OLD_GENESIS" ]; then
    echo "Error: Genesis file not found: $OLD_GENESIS"
    exit 1
fi

# Check if already new format
if grep -q '"block":' "$OLD_GENESIS"; then
    echo "✓ Genesis file is already in new format"
    echo "  No migration needed."
    exit 0
fi

echo "Found old format genesis file: $OLD_GENESIS"
echo ""

# Backup old file
echo "Creating backup: $BACKUP"
cp "$OLD_GENESIS" "$BACKUP"

# Create new format
echo "Generating new genesis format..."

cat > "$OLD_GENESIS" << 'EOF'
{
  "network": "testnet",
  "version": 1,
  "message": "TIME Coin Testnet Launch - October 12, 2025 - 24 Hour Blocks, Instant Finality",
  "block": {
    "header": {
      "block_number": 0,
      "timestamp": "2025-10-12T00:00:00Z",
      "previous_hash": "0000000000000000000000000000000000000000000000000000000000000000",
      "merkle_root": "coinbase_0",
      "validator_signature": "genesis",
      "validator_address": "genesis"
    },
    "transactions": [
      {
        "txid": "coinbase_0",
        "version": 1,
        "inputs": [],
        "outputs": [
          {
            "amount": 11653781624,
            "address": "genesis"
          }
        ],
        "lock_time": 0,
        "timestamp": 1760227200
      }
    ],
    "hash": "9a81c7599d8eed9720282aa68dccbc76e92ac3770a1892a96e1d073f375d0aed"
  }
}
EOF

echo ""
echo "✓ Migration complete!"
echo ""
echo "Genesis Details:"
echo "  Network: testnet"
echo "  Date: October 12, 2025 00:00:00 UTC"
echo "  Hash: 9a81c7599d8eed9720282aa68dccbc76e92ac3770a1892a96e1d073f375d0aed"
echo "  Total Supply: 116.53781624 TIME"
echo ""
echo "Backup saved to: $BACKUP"
echo ""
echo "IMPORTANT: All nodes in the network must use the same genesis file"
echo "           to prevent 'genesis mismatch' errors."
