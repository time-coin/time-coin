#!/bin/bash
#
# Reset Blockchain (Keep Genesis Only)
# This script removes all blocks except genesis block to allow
# the deterministic consensus system to recreate them properly
#

set -e

echo "╔════════════════════════════════════════════════════════════╗"
echo "║  TIME Coin - Reset Blockchain (Keep Genesis)              ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Confirm action
read -p "⚠️  This will DELETE all blocks except genesis. Continue? (yes/no): " confirm
if [ "$confirm" != "yes" ]; then
    echo "❌ Aborted"
    exit 1
fi

echo ""
echo "🛑 Stopping timed service..."
sudo systemctl stop timed || true

echo ""
echo "📁 Blockchain directory: /var/lib/time-coin/blockchain"

# Count existing blocks
block_count=$(ls -1 /var/lib/time-coin/blockchain/*.json 2>/dev/null | wc -l)
echo "   Found $block_count block files"

if [ $block_count -eq 0 ]; then
    echo "   ℹ️  No blocks found, nothing to delete"
    exit 0
fi

# Keep only genesis block (block_0.json)
echo ""
echo "🗑️  Removing blocks 1-N..."
for file in /var/lib/time-coin/blockchain/block_*.json; do
    filename=$(basename "$file")
    if [ "$filename" != "block_0.json" ]; then
        echo "   Deleting: $filename"
        sudo rm "$file"
    else
        echo "   Keeping: $filename (genesis)"
    fi
done

echo ""
echo "✅ Blockchain reset complete!"
echo "   Genesis block preserved"
echo "   All other blocks removed"
echo ""
echo "🔄 Starting timed service..."
sudo systemctl start timed

echo ""
echo "✓ Done! Node will recreate blocks via deterministic consensus"
echo ""
echo "📊 Monitor with: journalctl -u timed -f"
