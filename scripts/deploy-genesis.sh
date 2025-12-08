#!/bin/bash
# Deploy updated genesis block to data directory
# Usage: ./scripts/deploy-genesis.sh

set -e

echo "════════════════════════════════════════════════════════════════"
echo "  TIME Coin Genesis Deployment"
echo "  December 1, 2025 Genesis Update"
echo "════════════════════════════════════════════════════════════════"
echo ""

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
GENESIS_SOURCE="$REPO_DIR/config/genesis-testnet.json"

# Detect data directory
if [ -d "/var/lib/time-coin" ]; then
    DEFAULT_DATA_DIR="/var/lib/time-coin"
elif [ -d "$HOME/.timecoin" ]; then
    DEFAULT_DATA_DIR="$HOME/.timecoin"
else
    DEFAULT_DATA_DIR="$HOME/.timecoin"
fi

DATA_DIR="${DATA_DIR:-$DEFAULT_DATA_DIR}"
GENESIS_DEST="$DATA_DIR/genesis.json"

echo "📂 Paths:"
echo "   Source:      $GENESIS_SOURCE"
echo "   Destination: $GENESIS_DEST"
echo ""

# Verify source file exists
if [ ! -f "$GENESIS_SOURCE" ]; then
    echo "❌ ERROR: Genesis source file not found: $GENESIS_SOURCE"
    exit 1
fi

echo "✅ Genesis source file found"
echo ""

# Show genesis info
echo "📊 Genesis Block Info:"
if command -v jq &> /dev/null; then
    echo "   Timestamp: $(jq -r '.block.transactions[0].timestamp' "$GENESIS_SOURCE")"
    TIMESTAMP=$(jq -r '.block.transactions[0].timestamp' "$GENESIS_SOURCE")
    if command -v date &> /dev/null; then
        HUMAN_DATE=$(date -u -d "@$TIMESTAMP" '+%Y-%m-%d %H:%M:%S UTC' 2>/dev/null || date -r "$TIMESTAMP" -u '+%Y-%m-%d %H:%M:%S UTC' 2>/dev/null || echo "Date conversion unavailable")
        echo "   Date:      $HUMAN_DATE"
    fi
    echo "   Hash:      $(jq -r '.block.hash' "$GENESIS_SOURCE")"
    echo "   Message:   $(jq -r '.message' "$GENESIS_SOURCE")"
else
    echo "   (Install jq for detailed info)"
    grep -E '"timestamp"|"hash"|"message"' "$GENESIS_SOURCE" | head -5
fi
echo ""

# Check if destination exists
if [ -f "$GENESIS_DEST" ]; then
    echo "⚠️  Genesis file already exists at: $GENESIS_DEST"
    echo ""
    read -p "Backup and replace? (Y/n): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Nn]$ ]]; then
        echo "❌ Deployment cancelled"
        exit 0
    fi
    
    # Create backup
    BACKUP_FILE="${GENESIS_DEST}.backup.$(date +%s)"
    echo "💾 Creating backup: $BACKUP_FILE"
    cp "$GENESIS_DEST" "$BACKUP_FILE"
    echo "✅ Backup created"
    echo ""
fi

# Ensure data directory exists
if [ ! -d "$DATA_DIR" ]; then
    echo "📁 Creating data directory: $DATA_DIR"
    mkdir -p "$DATA_DIR"
fi

# Copy genesis file
echo "📋 Copying genesis file..."
cp "$GENESIS_SOURCE" "$GENESIS_DEST"
echo "✅ Genesis file deployed"
echo ""

# Verify
if [ -f "$GENESIS_DEST" ]; then
    echo "✅ Verification successful"
    echo "   Genesis file exists at: $GENESIS_DEST"
    
    if command -v jq &> /dev/null; then
        DEST_TIMESTAMP=$(jq -r '.block.transactions[0].timestamp' "$GENESIS_DEST")
        if [ "$DEST_TIMESTAMP" = "1764547200" ]; then
            echo "   ✅ Correct timestamp (December 1, 2025)"
        else
            echo "   ⚠️  Unexpected timestamp: $DEST_TIMESTAMP"
        fi
    fi
else
    echo "❌ ERROR: Verification failed - file not found at destination"
    exit 1
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "✅ GENESIS DEPLOYMENT COMPLETE"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "Next steps:"
echo "  1. Verify config has: load_genesis_from_file = true"
echo "  2. Verify config has: genesis_file = \"$GENESIS_DEST\""
echo "  3. Stop service:      sudo systemctl stop timed"
echo "  4. Clear blockchain:  rm -rf $DATA_DIR/blockchain/*"
echo "  5. Start service:     sudo systemctl start timed"
echo "  6. Check logs:        sudo journalctl -u timed -f"
echo ""
