#!/bin/bash
# Complete fix for systemd service environment and paths

set -e

echo "=== Fixing systemd service configuration ==="
echo ""

SERVICE_FILE="/etc/systemd/system/timed.service"
CONFIG_FILE="/root/.timecoin/config/testnet.toml"

# Step 1: Update service file to set HOME environment variable
echo "Step 1: Updating service file..."

cat > "$SERVICE_FILE" << 'EOF'
[Unit]
Description=TIME Coin Testnet Masternode
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/root/.timecoin
Environment="HOME=/root"
ExecStart=/usr/local/bin/timed --config /root/.timecoin/config/testnet.toml
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

echo "✅ Service file updated with HOME environment variable"
echo ""

# Step 2: Create config with absolute paths
echo "Step 2: Creating config file with absolute paths..."
mkdir -p /root/.timecoin/config

cat > "$CONFIG_FILE" << 'CONFIGEOF'
# TIME Coin Testnet Configuration

[node]
network = "testnet"
mode = "production"

[blockchain]
genesis_file = "/root/.timecoin/data/genesis-testnet.json"
load_genesis_from_file = true
data_dir = "/root/.timecoin/data"
allow_block_recreation = true

[network]
listen_addr = "0.0.0.0:24100"
external_addr = ""
bootstrap_nodes = []
peer_discovery_url = "https://time-coin.io/api/peers"

[masternode]
enabled = true

[rpc]
enabled = true
bind = "0.0.0.0"
port = 24101

[storage]
data_dir = "/root/.timecoin/data"

[logging]
level = "info"

[sync]
midnight_window_enabled = true
midnight_window_start_hour = 23
midnight_window_end_hour = 1
midnight_window_check_consensus = true
CONFIGEOF

echo "✅ Config file created"
echo ""

# Step 3: Ensure directories exist
echo "Step 3: Creating directory structure..."
mkdir -p /root/.timecoin/data/blockchain
mkdir -p /root/.timecoin/logs
mkdir -p /root/.timecoin/config
echo "✅ Directories created"
echo ""

# Step 4: Reload systemd
echo "Step 4: Reloading systemd daemon..."
systemctl daemon-reload
echo "✅ Daemon reloaded"
echo ""

# Step 5: Restart service
echo "Step 5: Restarting timed service..."
systemctl restart timed
echo "✅ Service restarted"
echo ""

sleep 3

echo "=== Service Status ==="
systemctl status timed --no-pager -l | head -20 || true
echo ""
echo "=== Recent Logs ==="
journalctl -u timed -n 20 --no-pager
