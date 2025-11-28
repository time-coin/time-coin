#!/bin/bash
# Reset testnet blockchain on all nodes

echo "🔄 Resetting TIME Coin Testnet..."
echo ""
echo "This will:"
echo "  1. Stop all nodes"
echo "  2. Clear blockchain data"
echo "  3. Keep wallet/masternode data"
echo ""
read -p "Continue? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]
then
    exit 1
fi

# Stop timed service
echo "Stopping timed service..."
sudo systemctl stop timed

# Backup wallet if exists
if [ -f "/var/lib/time-coin/wallet.dat" ]; then
    echo "Backing up wallet..."
    sudo cp /var/lib/time-coin/wallet.dat /var/lib/time-coin/wallet.dat.backup
fi

# Clear blockchain data but keep wallet
echo "Clearing blockchain data..."
sudo rm -rf /var/lib/time-coin/blockchain/*
sudo rm -rf /var/lib/time-coin/blocks
sudo rm -rf /var/lib/time-coin/chainstate
sudo rm -rf /var/lib/time-coin/block_height.txt
sudo rm -rf /var/lib/time-coin/utxo_snapshot.bin
sudo rm -rf /var/lib/time-coin/quarantine.json

# Keep these:
# - /var/lib/time-coin/wallet.dat
# - /var/lib/time-coin/masternode.key
# - /var/lib/time-coin/peers.json

echo "✓ Blockchain data cleared"
echo ""
echo "Restart the node with: sudo systemctl start timed"
echo "All nodes must be reset before restarting"
