#!/bin/bash
# TIME Coin Genesis Migration Script
# Migrates masternode from old genesis to new Proof-of-Time genesis

set -e  # Exit on error

echo "════════════════════════════════════════════════════════════════"
echo "  TIME Coin Genesis Migration - Proof of Time"
echo "════════════════════════════════════════════════════════════════"
echo ""

# Configuration - Check common locations
if [ -d "$HOME/.timecoin" ]; then
    DEFAULT_DATA_DIR="$HOME/.timecoin/data"
elif [ -d "/var/lib/time-coin" ]; then
    DEFAULT_DATA_DIR="/var/lib/time-coin"
else
    DEFAULT_DATA_DIR="$HOME/.timecoin/data"
fi

DATA_DIR="${DATA_DIR:-$DEFAULT_DATA_DIR}"
CONFIG_FILE="${CONFIG_FILE:-$HOME/.timecoin/config/testnet.toml}"
REPO_DIR="${REPO_DIR:-$HOME/time-coin}"

echo "📂 Configuration:"
echo "   Data directory: $DATA_DIR"
echo "   Config file: $CONFIG_FILE"
echo "   Repo directory: $REPO_DIR"
echo ""

# Verify paths exist
if [ ! -d "$REPO_DIR" ]; then
    echo "❌ ERROR: Repository directory not found: $REPO_DIR"
    echo "   Set REPO_DIR environment variable to correct path"
    exit 1
fi

# Step 1: Stop service
echo "════════════════════════════════════════════════════════════════"
echo "STEP 1: Stopping timed service"
echo "════════════════════════════════════════════════════════════════"
if systemctl is-active --quiet timed; then
    echo "🛑 Stopping timed service..."
    sudo systemctl stop timed
    echo "✅ Service stopped"
else
    echo "ℹ️  Service already stopped"
fi
echo ""

# Step 2: Backup (optional)
echo "════════════════════════════════════════════════════════════════"
echo "STEP 2: Backup old blockchain (optional)"
echo "════════════════════════════════════════════════════════════════"
read -p "Create backup? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    BACKUP_FILE="$HOME/backup-old-chain-$(date +%Y%m%d-%H%M%S).tar.gz"
    echo "💾 Creating backup: $BACKUP_FILE"
    sudo tar -czf "$BACKUP_FILE" "$DATA_DIR" 2>/dev/null || true
    echo "✅ Backup created"
else
    echo "⏭️  Skipping backup"
fi
echo ""

# Step 3: Ensure directories exist
echo "════════════════════════════════════════════════════════════════"
echo "STEP 3: Creating necessary directories"
echo "════════════════════════════════════════════════════════════════"
echo "📁 Creating directory structure..."

# Create main .timecoin directory
sudo mkdir -p "$HOME/.timecoin"
sudo chown -R $USER:$USER "$HOME/.timecoin"

# Create subdirectories
sudo mkdir -p "$DATA_DIR"
sudo mkdir -p "$(dirname "$CONFIG_FILE")"
sudo mkdir -p "$HOME/.timecoin/logs"
sudo mkdir -p "$DATA_DIR/blockchain"

# Set ownership
sudo chown -R $USER:$USER "$HOME/.timecoin"
sudo chown -R $USER:$USER "$DATA_DIR"

echo "✅ Directories created:"
echo "   - $HOME/.timecoin"
echo "   - $DATA_DIR"
echo "   - $(dirname "$CONFIG_FILE")"
echo "   - $HOME/.timecoin/logs"
echo ""

# Step 4: Delete old blockchain
echo "════════════════════════════════════════════════════════════════"
echo "STEP 4: Deleting old blockchain"
echo "════════════════════════════════════════════════════════════════"
if [ -d "$DATA_DIR/blockchain" ] && [ "$(ls -A $DATA_DIR/blockchain 2>/dev/null)" ]; then
    echo "🗑️  Found blockchain at: $DATA_DIR/blockchain"
    echo "   This will delete all existing blocks"
    echo ""
    read -p "Confirm deletion? (Y/n): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Nn]$ ]]; then
        echo "🗑️  Deleting $DATA_DIR/blockchain/*..."
        sudo rm -rf "$DATA_DIR/blockchain"/*
        echo "✅ Old blockchain deleted"
    else
        echo "❌ Migration cancelled"
        exit 1
    fi
else
    echo "ℹ️  No blockchain data found (already clean)"
fi
echo ""

# Step 5: Pull latest code
echo "════════════════════════════════════════════════════════════════"
echo "STEP 5: Pulling latest code"
echo "════════════════════════════════════════════════════════════════"
cd "$REPO_DIR"
echo "📥 Fetching latest code..."
git fetch origin
git reset --hard origin/main
git pull origin main
echo "✅ Code updated"
echo ""
echo "📊 Current commit:"
git log --oneline -1
echo ""

# Step 6: Verify genesis file
echo "════════════════════════════════════════════════════════════════"
echo "STEP 6: Verifying genesis file"
echo "════════════════════════════════════════════════════════════════"
GENESIS_FILE="$REPO_DIR/config/genesis-testnet.json"
if [ -f "$GENESIS_FILE" ]; then
    echo "📄 Genesis file found: $GENESIS_FILE"
    
    # Check for proof_of_time field
    if grep -q "proof_of_time" "$GENESIS_FILE"; then
        echo "✅ Genesis has proof_of_time field"
        
        # Show genesis details
        echo ""
        echo "📊 Genesis details:"
        cat "$GENESIS_FILE" | grep -A 6 "proof_of_time" || true
    else
        echo "❌ ERROR: Genesis file missing proof_of_time field!"
        echo "   Your code may not be up to date"
        exit 1
    fi
else
    echo "❌ ERROR: Genesis file not found: $GENESIS_FILE"
    exit 1
fi
echo ""

# Step 6: Verify genesis file
echo "════════════════════════════════════════════════════════════════"
echo "STEP 6: Verifying genesis file"
echo "════════════════════════════════════════════════════════════════"
if [ -f "$CONFIG_FILE" ]; then
    echo "📄 Config file: $CONFIG_FILE"
    
    # Check load_genesis_from_file setting
    if grep -q "load_genesis_from_file = true" "$CONFIG_FILE"; then
        echo "✅ Genesis loading is enabled"
    elif grep -q "load_genesis_from_file = false" "$CONFIG_FILE"; then
        echo "⚠️  Genesis loading is DISABLED"
        echo ""
        read -p "Enable genesis loading? (Y/n): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Nn]$ ]]; then
            echo "✏️  Updating config..."
            sudo sed -i 's/load_genesis_from_file = false/load_genesis_from_file = true/' "$CONFIG_FILE"
            echo "✅ Config updated"
        else
            echo "❌ ERROR: Genesis loading must be enabled"
            exit 1
        fi
    else
        echo "⚠️  Cannot find load_genesis_from_file in config"
        echo "   Manually verify config file"
    fi
    
    # Check genesis_file path
    echo ""
    echo "📊 Genesis file path in config:"
    grep "genesis_file" "$CONFIG_FILE" | grep -v "^#" || echo "   Not set (will use default)"
else
    echo "❌ ERROR: Config file not found: $CONFIG_FILE"
    exit 1
fi
echo ""

# Step 7: Check config
echo "════════════════════════════════════════════════════════════════"
echo "STEP 7: Checking config"
echo "════════════════════════════════════════════════════════════════"
cd "$REPO_DIR"
echo "🔨 Building release binary..."
echo "   (This will take 5-10 minutes)"
echo ""
cargo build --release
echo ""
echo "✅ Build complete"
echo ""

# Step 8: Rebuild
echo "════════════════════════════════════════════════════════════════"
echo "STEP 8: Rebuilding"
echo "════════════════════════════════════════════════════════════════"
echo "🚀 Starting timed service..."
sudo systemctl start timed
echo "✅ Service started"
echo ""

# Step 9: Restart service
echo "════════════════════════════════════════════════════════════════"
echo "STEP 9: Restarting service"
echo "════════════════════════════════════════════════════════════════"
echo "📊 Service status:"
sudo systemctl status timed --no-pager -l | head -n 10 || true
echo ""
echo "Waiting 5 seconds for startup..."
sleep 5
echo ""
echo "📋 Recent logs:"
sudo journalctl -u timed -n 20 --no-pager | tail -n 20
echo ""

# Final checklist
echo "════════════════════════════════════════════════════════════════"
echo "✅ MIGRATION COMPLETE!"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "Verification checklist:"
echo "  [ ] Service is running (check above)"
echo "  [ ] Logs show: 🔍 Genesis loading is enabled"
echo "  [ ] Logs show: 📥 Loading genesis block from file..."
echo "  [ ] Logs show: ✅ Genesis block loaded"
echo "  [ ] No errors in logs"
echo ""
echo "Watch live logs with:"
echo "  sudo journalctl -u timed -f"
echo ""
echo "Check genesis block:"
echo "  curl -s http://localhost:24101/api/blockchain/block/0 | jq ."
echo ""
echo "════════════════════════════════════════════════════════════════"
