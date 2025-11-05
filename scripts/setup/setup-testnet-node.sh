#!/bin/bash
# TIME Coin - Ubuntu Testnet Masternode Setup
# For Ubuntu 20.04 LTS or newer

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}════════════════════════════════════════════════${NC}"
echo -e "${BLUE}   TIME Coin Testnet Masternode Setup${NC}"
echo -e "${BLUE}════════════════════════════════════════════════${NC}\n"

# Check if running as root
if [[ $EUID -eq 0 ]]; then
   echo -e "${RED}Error: Do not run this script as root!${NC}"
   echo "Run as regular user with sudo privileges"
   exit 1
fi

# Check Ubuntu version
echo -e "${BLUE}Checking system...${NC}"
if [ -f /etc/os-release ]; then
    . /etc/os-release
    echo -e "${GREEN}✓ OS: $NAME $VERSION${NC}"
else
    echo -e "${RED}Error: Cannot determine OS version${NC}"
    exit 1
fi

# System requirements check
echo -e "\n${YELLOW}System Requirements:${NC}"
echo "  • Ubuntu 20.04+ (you have: $VERSION_ID)"
echo "  • 2+ CPU cores (minimum)"
echo "  • 4+ GB RAM (minimum)"
echo "  • 50+ GB disk space"
echo "  • Static IP address (recommended)"
echo ""
read -p "Continue with installation? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    exit 1
fi

# Get node configuration
echo -e "\n${BLUE}Node Configuration${NC}"
read -p "Enter node name [default: testnet-node]: " NODE_NAME
NODE_NAME=${NODE_NAME:-testnet-node}

echo -e "\nSelect masternode tier:"
echo "  1) Community (1,000 TIME) - 18% APY, 1x voting"
echo "  2) Verified (10,000 TIME) - 24% APY, 10x voting"
echo "  3) Professional (100,000 TIME) - 30% APY, 50x voting"
read -p "Choice [1-3]: " TIER_CHOICE

case $TIER_CHOICE in
    1)
        TIER="community"
        COLLATERAL="1000"
        ;;
    2)
        TIER="verified"
        COLLATERAL="10000"
        ;;
    3)
        TIER="professional"
        COLLATERAL="100000"
        ;;
    *)
        echo -e "${RED}Invalid choice${NC}"
        exit 1
        ;;
esac

echo -e "${GREEN}✓ Selected: $TIER tier ($COLLATERAL TIME)${NC}"

# Get public IP
PUBLIC_IP=$(curl -s ifconfig.me)
echo -e "${GREEN}✓ Public IP: $PUBLIC_IP${NC}"

# Install dependencies
echo -e "\n${BLUE}Installing dependencies...${NC}"
sudo apt-get update
sudo apt-get install -y \
    curl \
    wget \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    screen \
    htop

echo -e "${GREEN}✓ Dependencies installed${NC}"

# Install Rust
if ! command -v rustc &> /dev/null; then
    echo -e "\n${BLUE}Installing Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "${GREEN}✓ Rust installed: $(rustc --version)${NC}"
else
    echo -e "${GREEN}✓ Rust already installed: $(rustc --version)${NC}"
fi

# Create directory structure
echo -e "\n${BLUE}Creating directories...${NC}"
mkdir -p ~/time-coin-node/{config,data,logs}
cd ~/time-coin-node

echo -e "${GREEN}✓ Directories created${NC}"

# Clone TIME Coin repository
echo -e "\n${BLUE}Cloning TIME Coin repository...${NC}"
if [ ! -d "time-coin" ]; then
    git clone https://github.com/time-coin/time-coin.git
    cd time-coin
else
    echo -e "${YELLOW}⚠ Repository already exists, pulling latest${NC}"
    cd time-coin
    git pull origin main
fi

echo -e "${GREEN}✓ Repository ready${NC}"

# Build the project
echo -e "\n${BLUE}Building TIME Coin node (this may take 10-15 minutes)...${NC}"
cargo build --release --bin time-node

echo -e "${GREEN}✓ Build complete${NC}"

# Copy binary to bin directory
sudo cp target/release/time-node /usr/local/bin/
sudo chmod +x /usr/local/bin/time-node

echo -e "${GREEN}✓ Binary installed to /usr/local/bin/time-node${NC}"

# Create configuration file
echo -e "\n${BLUE}Creating configuration...${NC}"

cat > ~/time-coin-node/config/testnet.toml << EOF
# TIME Coin Testnet Node Configuration

[network]
mode = "testnet"
port = 9876
max_peers = 50
bootstrap_nodes = [
    "testnet-seed1.time-coin.io:9876",
    "testnet-seed2.time-coin.io:9876",
]

[node]
name = "$NODE_NAME"
data_dir = "$HOME/time-coin-node/data"
log_dir = "$HOME/time-coin-node/logs"

[masternode]
enabled = true
tier = "$TIER"
collateral = $COLLATERAL
public_ip = "$PUBLIC_IP"
public_port = 9876

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

echo -e "${GREEN}✓ Configuration created${NC}"

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
echo -e "${GREEN}✓ Systemd service created${NC}"

# Create management script
echo -e "\n${BLUE}Creating management script...${NC}"

cat > ~/time-coin-node/manage.sh << 'EOF'
#!/bin/bash
# TIME Coin Node Management Script

case "$1" in
    start)
        echo "Starting TIME node..."
        sudo systemctl start time-node
        sleep 2
        sudo systemctl status time-node --no-pager
        ;;
    stop)
        echo "Stopping TIME node..."
        sudo systemctl stop time-node
        ;;
    restart)
        echo "Restarting TIME node..."
        sudo systemctl restart time-node
        sleep 2
        sudo systemctl status time-node --no-pager
        ;;
    status)
        sudo systemctl status time-node --no-pager
        ;;
    logs)
        if [ "$2" == "follow" ] || [ "$2" == "-f" ]; then
            tail -f ~/time-coin-node/logs/node.log
        else
            tail -n 100 ~/time-coin-node/logs/node.log
        fi
        ;;
    errors)
        tail -n 50 ~/time-coin-node/logs/error.log
        ;;
    enable)
        echo "Enabling TIME node to start on boot..."
        sudo systemctl enable time-node
        ;;
    disable)
        echo "Disabling TIME node auto-start..."
        sudo systemctl disable time-node
        ;;
    update)
        echo "Updating TIME node..."
        cd ~/time-coin-node/time-coin
        git pull origin main
        cargo build --release --bin time-node
        sudo cp target/release/time-node /usr/local/bin/
        echo "Update complete. Restart the node to apply changes."
        ;;
    *)
        echo "TIME Coin Node Management"
        echo ""
        echo "Usage: $0 {start|stop|restart|status|logs|errors|enable|disable|update}"
        echo ""
        echo "Commands:"
        echo "  start    - Start the node"
        echo "  stop     - Stop the node"
        echo "  restart  - Restart the node"
        echo "  status   - Show node status"
        echo "  logs     - Show recent logs (add 'follow' to tail)"
        echo "  errors   - Show recent errors"
        echo "  enable   - Enable auto-start on boot"
        echo "  disable  - Disable auto-start"
        echo "  update   - Update node to latest version"
        echo ""
        echo "Examples:"
        echo "  $0 start"
        echo "  $0 logs follow"
        exit 1
        ;;
esac
EOF

chmod +x ~/time-coin-node/manage.sh

echo -e "${GREEN}✓ Management script created${NC}"

# Setup firewall
echo -e "\n${BLUE}Configuring firewall...${NC}"
if command -v ufw &> /dev/null; then
    sudo ufw allow 9876/tcp comment "TIME Coin P2P"
    sudo ufw allow 22/tcp comment "SSH"
    echo -e "${GREEN}✓ Firewall rules added${NC}"
else
    echo -e "${YELLOW}⚠ UFW not installed, skipping firewall setup${NC}"
fi

# Final setup summary
echo -e "\n${GREEN}════════════════════════════════════════════════${NC}"
echo -e "${GREEN}   ✅ Testnet Node Setup Complete!${NC}"
echo -e "${GREEN}════════════════════════════════════════════════${NC}\n"

echo -e "${YELLOW}Configuration Summary:${NC}"
echo "  • Node Name: $NODE_NAME"
echo "  • Tier: $TIER ($COLLATERAL TIME)"
echo "  • Public IP: $PUBLIC_IP"
echo "  • P2P Port: 9876"
echo "  • RPC Port: 24101 (local only)"
echo "  • Config: ~/time-coin-node/config/testnet.toml"
echo "  • Data: ~/time-coin-node/data"
echo "  • Logs: ~/time-coin-node/logs"

echo -e "\n${YELLOW}Management Commands:${NC}"
echo "  ~/time-coin-node/manage.sh start      # Start node"
echo "  ~/time-coin-node/manage.sh status     # Check status"
echo "  ~/time-coin-node/manage.sh logs       # View logs"
echo "  ~/time-coin-node/manage.sh logs follow # Follow logs"
echo "  ~/time-coin-node/manage.sh stop       # Stop node"
echo "  ~/time-coin-node/manage.sh restart    # Restart node"

echo -e "\n${YELLOW}Next Steps:${NC}"
echo "1. Start the node:"
echo -e "   ${BLUE}~/time-coin-node/manage.sh start${NC}"
echo ""
echo "2. Check if it's running:"
echo -e "   ${BLUE}~/time-coin-node/manage.sh status${NC}"
echo ""
echo "3. Watch the logs:"
echo -e "   ${BLUE}~/time-coin-node/manage.sh logs follow${NC}"
echo ""
echo "4. Enable auto-start on boot (recommended):"
echo -e "   ${BLUE}~/time-coin-node/manage.sh enable${NC}"

echo -e "\n${YELLOW}Testnet Notes:${NC}"
echo "  • This is TESTNET - coins have no real value"
echo "  • Testnet collateral is automatically provided"
echo "  • No real TIME coins required"
echo "  • Join testnet community: https://t.me/+CaN6EflYM-83OTY0"

echo -e "\n${GREEN}Happy testing! 🚀${NC}\n"

# Ask if they want to start now
read -p "Start the node now? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    ~/time-coin-node/manage.sh start
    echo ""
    echo -e "${GREEN}Node is starting! Watch logs with:${NC}"
    echo -e "${BLUE}~/time-coin-node/manage.sh logs follow${NC}"
fi
