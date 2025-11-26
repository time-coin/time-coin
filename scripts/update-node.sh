#!/bin/bash
# Quick deployment script for masternode updates
# Run this on each masternode to update to latest version

set -e  # Exit on error

echo "════════════════════════════════════════════════════════"
echo "  TIME Coin Masternode Update Script"
echo "════════════════════════════════════════════════════════"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo "⚠️  Please run as root (sudo ./update-node.sh)"
    exit 1
fi

# Get node directory
NODE_DIR="${NODE_DIR:-/root/time-coin-node}"

if [ ! -d "$NODE_DIR" ]; then
    echo "❌ Node directory not found: $NODE_DIR"
    echo "   Set NODE_DIR environment variable if using different path"
    exit 1
fi

cd "$NODE_DIR"

echo "📍 Working directory: $NODE_DIR"
echo ""

# Show current version
echo "📊 Current version:"
./target/release/timed --version 2>/dev/null || echo "   (not built yet)"
echo ""

# Backup current binary
if [ -f "target/release/timed" ]; then
    echo "💾 Backing up current binary..."
    cp target/release/timed target/release/timed.backup
    echo "   ✓ Backup saved to target/release/timed.backup"
    echo ""
fi

# Stop service
echo "⏸️  Stopping timed service..."
systemctl stop timed
echo "   ✓ Service stopped"
echo ""

# Pull latest code
echo "📥 Pulling latest code from main..."
git fetch origin
git reset --hard origin/main
COMMIT=$(git rev-parse --short HEAD)
echo "   ✓ Updated to commit: $COMMIT"
echo ""

# Build
echo "🔨 Building release binary (this may take a few minutes)..."
cargo build --release --bin timed
echo "   ✓ Build complete"
echo ""

# Show new version
echo "📊 New version:"
./target/release/timed --version
echo ""

# Start service
echo "▶️  Starting timed service..."
systemctl start timed
echo "   ✓ Service started"
echo ""

# Wait a moment for service to start
sleep 2

# Check status
echo "🔍 Service status:"
systemctl status timed --no-pager | head -n 10
echo ""

# Show recent logs
echo "📋 Recent logs:"
journalctl -u timed --since "30 seconds ago" --no-pager | tail -n 20
echo ""

echo "════════════════════════════════════════════════════════"
echo "✅ Update complete!"
echo ""
echo "Commands to monitor:"
echo "  sudo journalctl -u timed -f          # Follow logs"
echo "  sudo systemctl status timed          # Check status"
echo "  time-cli info                         # Check node info"
echo "════════════════════════════════════════════════════════"
