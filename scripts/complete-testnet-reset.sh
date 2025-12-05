#!/bin/bash
#
# Complete Testnet Reset Script
# Clears blockchain, mempool, and UTXO data for testing consensus from genesis
#
# Usage: ./scripts/complete-testnet-reset.sh
#

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔄  TIME COIN TESTNET - COMPLETE RESET"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "This will DELETE:"
echo "  ✗ All blockchain data"
echo "  ✗ All UTXO snapshots"
echo "  ✗ All mempool transactions"
echo "  ✗ Block height tracking"
echo "  ✗ Finalized transactions"
echo ""
echo "This will KEEP:"
echo "  ✓ Wallet keys (wallet.dat)"
echo "  ✓ Masternode configuration"
echo "  ✓ Peer list"
echo ""
echo "⚠️  WARNING: Run this on ALL nodes before restarting any node!"
echo ""
read -p "Are you sure you want to reset the testnet? (yes/no): " -r
echo

if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
    echo "❌ Reset cancelled"
    exit 1
fi

# Determine data directory
DATA_DIR="/var/lib/time-coin"
if [ ! -z "$1" ]; then
    DATA_DIR="$1"
fi

echo ""
echo "📂 Data directory: $DATA_DIR"
echo ""

# Check if running as root or with sudo
if [ "$EUID" -ne 0 ] && [ -z "$SUDO_USER" ]; then
    echo "⚠️  This script may need sudo privileges"
    echo "   If you see permission errors, run with: sudo $0"
    echo ""
fi

# Stop the daemon if running
echo "1️⃣  Stopping timed service..."
if systemctl is-active --quiet timed 2>/dev/null; then
    sudo systemctl stop timed
    echo "   ✓ Service stopped"
else
    echo "   ℹ️  Service not running"
fi
echo ""

# Backup wallet if it exists
WALLET_FILE="$DATA_DIR/wallet.dat"
BACKUP_DIR="/tmp/time-coin-wallet-backup-$(date +%s)"

if [ -f "$WALLET_FILE" ]; then
    echo "2️⃣  Backing up wallet..."
    mkdir -p "$BACKUP_DIR"
    cp "$WALLET_FILE" "$BACKUP_DIR/"
    echo "   ✓ Wallet backed up to: $BACKUP_DIR"
else
    echo "2️⃣  No wallet found (skipping backup)"
fi
echo ""

# Clear blockchain data (except genesis block)
echo "3️⃣  Clearing blockchain data (preserving genesis)..."

# Backup genesis block if it exists
GENESIS_BACKUP="/tmp/genesis_block_backup_$(date +%s).json"
GENESIS_FILE="$DATA_DIR/blockchain/block_0.json"

if [ -f "$GENESIS_FILE" ]; then
    echo "   📦 Backing up genesis block..."
    mkdir -p "$(dirname $GENESIS_BACKUP)"
    sudo cp "$GENESIS_FILE" "$GENESIS_BACKUP"
    echo "   ✓ Genesis backed up to: $GENESIS_BACKUP"
fi

# Clear blockchain data
sudo rm -rf "$DATA_DIR/blocks" 2>/dev/null || true
sudo rm -rf "$DATA_DIR/blockchain" 2>/dev/null || true
sudo rm -rf "$DATA_DIR/chainstate" 2>/dev/null || true
sudo rm -f "$DATA_DIR/current_height.txt" 2>/dev/null || true

# Restore genesis block
if [ -f "$GENESIS_BACKUP" ]; then
    echo "   📥 Restoring genesis block..."
    sudo mkdir -p "$DATA_DIR/blockchain"
    sudo cp "$GENESIS_BACKUP" "$GENESIS_FILE"
    sudo chown -R time-coin:time-coin "$DATA_DIR/blockchain" 2>/dev/null || true
    echo "   ✓ Genesis block restored"
fi

echo "   ✓ Blockchain data cleared (genesis preserved)"
echo ""

# Clear UTXO data
echo "4️⃣  Clearing UTXO data..."
sudo rm -rf "$DATA_DIR/utxo_set" 2>/dev/null || true
sudo rm -rf "$DATA_DIR/utxo_db" 2>/dev/null || true
sudo rm -f "$DATA_DIR/utxo_snapshot.bin" 2>/dev/null || true
sudo rm -f "$DATA_DIR/utxo_snapshot.json" 2>/dev/null || true
echo "   ✓ UTXO data cleared"
echo ""

# Clear mempool
echo "5️⃣  Clearing mempool..."
MEMPOOL_FILE="$DATA_DIR/mempool.json"
if [ -f "$MEMPOOL_FILE" ]; then
    MEMPOOL_BACKUP="$DATA_DIR/mempool_backup_$(date +%Y%m%d_%H%M%S).json"
    sudo cp "$MEMPOOL_FILE" "$MEMPOOL_BACKUP" 2>/dev/null || true
    sudo rm -f "$MEMPOOL_FILE"
    echo "[]" | sudo tee "$MEMPOOL_FILE" > /dev/null
    echo "   ✓ Mempool cleared (backup: $MEMPOOL_BACKUP)"
else
    echo "   ℹ️  No mempool file found"
fi
echo ""

# Clear finalized transactions
echo "6️⃣  Clearing finalized transactions..."
sudo rm -f "$DATA_DIR/finalized_tx.json" 2>/dev/null || true
sudo rm -rf "$DATA_DIR/finalized_txs" 2>/dev/null || true
echo "   ✓ Finalized transactions cleared"
echo ""

# Clear any consensus state
echo "7️⃣  Clearing consensus state..."
sudo rm -f "$DATA_DIR/consensus_state.json" 2>/dev/null || true
sudo rm -rf "$DATA_DIR/proposals" 2>/dev/null || true
echo "   ✓ Consensus state cleared"
echo ""

# Restore wallet if backed up
if [ -f "$BACKUP_DIR/wallet.dat" ]; then
    echo "8️⃣  Restoring wallet..."
    sudo cp "$BACKUP_DIR/wallet.dat" "$WALLET_FILE"
    sudo chown time-coin:time-coin "$WALLET_FILE" 2>/dev/null || true
    echo "   ✓ Wallet restored"
else
    echo "8️⃣  No wallet to restore"
fi
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅  TESTNET RESET COMPLETE!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "📋 Summary of cleared data:"
echo "   • Blockchain: All blocks removed (EXCEPT genesis block)"
echo "   • Genesis: Preserved and restored"
echo "   • UTXO Set: Cleared, will rebuild from genesis"
echo "   • Mempool: Empty"
echo "   • Consensus: Fresh state"
echo ""
echo "📝 Next steps:"
echo ""
echo "   1️⃣  Run this script on ALL masternodes"
echo "      (NewYork, LW-Michigan, reitools.us, etc.)"
echo ""
echo "   2️⃣  After ALL nodes are reset, start them:"
echo "      sudo systemctl start timed"
echo ""
echo "   3️⃣  Monitor the first block creation:"
echo "      sudo journalctl -u timed -f"
echo ""
echo "   4️⃣  Verify consensus is working:"
echo "      - Nodes should start from genesis (block 0)"
echo "      - Wait for next midnight UTC block (block 1)"
echo "      - Check that votes are being exchanged"
echo "      - Verify block reaches 5/7 consensus"
echo ""
echo "⚠️  IMPORTANT:"
echo "   • Do NOT start any node until ALL nodes are reset"
echo "   • All nodes must have the latest code (git pull)"
echo "   • Genesis block is PRESERVED from backup"
echo "   • Blockchain will continue from block 1+"
echo ""
echo "🔗 Useful commands:"
echo "   • Check status: systemctl status timed"
echo "   • View logs: journalctl -u timed -f"
echo "   • Check height: curl http://localhost:24101/blockchain/height"
echo ""
