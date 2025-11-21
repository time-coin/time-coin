#!/bin/bash
#
# Reset testnet blockchain data
# This clears all blockchain and UTXO data while preserving wallet keys
#

set -e

echo "🔄 Resetting testnet blockchain data..."

# Stop the daemon if running
if systemctl is-active --quiet timed 2>/dev/null; then
    echo "   Stopping timed service..."
    sudo systemctl stop timed
fi

# Backup wallet if it exists
WALLET_FILE="/var/lib/time-coin/wallet.dat"
BACKUP_DIR="/tmp/time-coin-wallet-backup-$(date +%s)"

if [ -f "$WALLET_FILE" ]; then
    echo "   📦 Backing up wallet to $BACKUP_DIR"
    mkdir -p "$BACKUP_DIR"
    cp "$WALLET_FILE" "$BACKUP_DIR/"
fi

# Clear blockchain data
echo "   🗑️  Clearing blockchain data..."
sudo rm -rf /var/lib/time-coin/blocks/
sudo rm -rf /var/lib/time-coin/utxo_set/
sudo rm -f /var/lib/time-coin/current_height.txt
sudo rm -f /var/lib/time-coin/peers.json

# Restore wallet
if [ -f "$BACKUP_DIR/wallet.dat" ]; then
    echo "   📥 Restoring wallet..."
    sudo cp "$BACKUP_DIR/wallet.dat" "$WALLET_FILE"
    sudo chown time-coin:time-coin "$WALLET_FILE" 2>/dev/null || true
fi

echo "✅ Testnet data reset complete!"
echo ""
echo "Next steps:"
echo "  1. Ensure all nodes run this script"
echo "  2. Start timed: sudo systemctl start timed"
echo "  3. The network will recreate blocks from genesis via BFT consensus"

