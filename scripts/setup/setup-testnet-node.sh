#!/usr/bin/env bash
set -e

# (top of file omitted - keep existing header content)

# Testnet: Skip real collateral requirement
testnet_mode = true
testnet_auto_fund = true

[rpc]
enabled = true
bind = "127.0.0.1"
port = 24101
# Only allow local connections for security

[logging]
level = "info"
file = "$HOME/time-coin-node/logs/node.log"
max_size = "100MB"
max_backups = 5
EOF

echo -e "${GREEN}\u2713 Configuration created${NC}"

# Create systemd service
echo -e "\n${BLUE}Creating systemd service...${NC}"

sudo tee /etc/systemd/system/time-node.service > /dev/null << EOF
[Unit]
Description=TIME Coin Testnet Masternode
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$HOME/time-coin-node
ExecStart=/usr/local/bin/time-node --config $HOME/time-coin-node/config/testnet.toml
Restart=always
RestartSec=10

# Logging
StandardOutput=append:$HOME/time-coin-node/logs/node.log
StandardError=append:$HOME/time-coin-node/logs/error.log

# Security
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
echo -e "${GREEN}\u2713 Systemd service created${NC}"

# (the rest of the original script remains unchanged)