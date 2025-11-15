#!/bin/bash

# Script to clear mempool on all nodes
# This removes testnet-minted coins and any problematic transactions

echo "🧹 Clearing TIME Coin mempool..."
echo ""

# Default data directory
DATA_DIR="/var/lib/time-coin"

# Check if custom data directory provided
if [ ! -z "$1" ]; then
    DATA_DIR="$1"
fi

echo "Data directory: $DATA_DIR"
echo ""

# Check if data directory exists
if [ ! -d "$DATA_DIR" ]; then
    echo "❌ Error: Data directory not found: $DATA_DIR"
    echo "Usage: $0 [data_directory]"
    exit 1
fi

# Backup mempool before clearing (just in case)
MEMPOOL_FILE="$DATA_DIR/mempool.json"
if [ -f "$MEMPOOL_FILE" ]; then
    BACKUP_FILE="$DATA_DIR/mempool_backup_$(date +%Y%m%d_%H%M%S).json"
    echo "📦 Creating backup: $BACKUP_FILE"
    cp "$MEMPOOL_FILE" "$BACKUP_FILE"
    echo "✓ Backup created"
    echo ""
fi

# Clear mempool
if [ -f "$MEMPOOL_FILE" ]; then
    echo "🗑️  Removing mempool file..."
    rm "$MEMPOOL_FILE"
    echo "✓ Mempool cleared"
    echo ""
    
    # Create empty mempool file
    echo "📝 Creating empty mempool..."
    echo "[]" > "$MEMPOOL_FILE"
    echo "✓ Empty mempool created"
    echo ""
else
    echo "ℹ️  No mempool file found (already empty)"
    echo ""
fi

echo "✅ Done! Mempool has been cleared."
echo ""
echo "Next steps:"
echo "1. Restart the node: sudo systemctl restart timed"
echo "2. Check status: sudo systemctl status timed"
echo ""
echo "Note: Backup saved at: $BACKUP_FILE"
