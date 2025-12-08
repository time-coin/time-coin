#!/bin/bash
# Quick Deployment Script for Approval Manager Fix
# Run this on each node: LW-Arizona, LW-Michigan, LW-Amsterdam, LW-London, LW-Paris

set -e  # Exit on error

echo "╔════════════════════════════════════════════════════════════╗"
echo "║                                                            ║"
echo "║       Deploying Approval Manager + UTXO State Fix         ║"
echo "║                                                            ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Get node name
NODE_NAME=$(hostname)
echo "🖥️  Node: $NODE_NAME"
echo ""

# Step 1: Pull latest code
echo "📥 Step 1/7: Pulling latest code from GitHub..."
cd ~/time-coin || { echo "❌ Directory ~/time-coin not found!"; exit 1; }
git pull origin main
echo "✅ Code updated"
echo ""

# Step 2: Build release binary
echo "🔨 Step 2/7: Building release binary (this takes 3-5 minutes)..."
cargo build --release
if [ $? -eq 0 ]; then
    echo "✅ Build successful"
else
    echo "❌ Build failed! Check errors above."
    exit 1
fi
echo ""

# Step 3: Stop the node
echo "🛑 Step 3/7: Stopping timed service..."
sudo systemctl stop timed
echo "✅ Service stopped"
echo ""

# Step 4: Backup old binary
echo "💾 Step 4/7: Backing up old binary..."
BACKUP_NAME="timed.backup.$(date +%Y%m%d_%H%M%S)"
sudo cp /usr/local/bin/timed /usr/local/bin/$BACKUP_NAME
echo "✅ Backup saved: /usr/local/bin/$BACKUP_NAME"
echo ""

# Step 5: Deploy new binary
echo "🚀 Step 5/7: Deploying new binary..."
sudo cp target/release/timed /usr/local/bin/timed
sudo chmod +x /usr/local/bin/timed
echo "✅ Binary deployed"
echo ""

# Step 6: Start the node
echo "▶️  Step 6/7: Starting timed service..."
sudo systemctl start timed
sleep 2
echo "✅ Service started"
echo ""

# Step 7: Verify deployment
echo "🔍 Step 7/7: Verifying deployment..."
echo ""
echo "Service status:"
sudo systemctl status timed --no-pager | head -5
echo ""

# Wait a few seconds for startup
echo "⏳ Waiting 10 seconds for node to initialize..."
sleep 10
echo ""

echo "📊 Recent logs (last 30 lines):"
sudo journalctl -u timed -n 30 --no-pager
echo ""

echo "╔════════════════════════════════════════════════════════════╗"
echo "║                                                            ║"
echo "║                  ✅ DEPLOYMENT COMPLETE! ✅               ║"
echo "║                                                            ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "🎯 NEXT STEPS:"
echo ""
echo "1. Watch logs for approval messages:"
echo "   sudo journalctl -u timed -f | grep -E 'approval|APPROVED'"
echo ""
echo "2. Look for SUCCESS indicators:"
echo "   ✅ Approval recorded successfully   ← NEW! This means fix is working"
echo ""
echo "3. Should NOT see:"
echo "   ❌ Failed to record approval: Unauthorized masternode"
echo ""
echo "4. Test by sending a transaction from any node"
echo ""
echo "📝 Node: $NODE_NAME deployed successfully at $(date)"
echo ""
