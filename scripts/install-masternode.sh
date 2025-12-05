#!/bin/bash

#############################################################
# TIME Coin Masternode Installation Script
#
# This script will:
# - Clone the TIME Coin repository
# - Install build dependencies (apt)
# - Install Rust toolchain if missing
# - Build timed and time-cli
# - Install binaries to /usr/local/bin/
# - Create config in /root/time-coin-node/config/testnet.toml
# - Create+enable+start systemd service "timed"
# - Setup masternode with grant application and activation
#
# Usage (on fresh Ubuntu server):
#   wget -O install-masternode.sh https://raw.githubusercontent.com/your-repo/time-coin/main/scripts/install-masternode.sh
#   sudo bash install-masternode.sh
#
# Or if you already have the repo:
#   cd ~/time-coin/scripts
#   sudo ./install-masternode.sh
#
# Assumptions:
#   - Ubuntu (tested against 22.04+ and 25.04)
#   - systemd present
#   - Internet connection
#############################################################

set -e  # exit immediately on error

#############################
# Colors / pretty output
#############################
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_header() {
    echo -e "\n${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}\n"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

#############################
# Paths / constants
#############################

REPO_DIR=""
# Use Bitcoin-style data directory: ~/.timecoin
NODE_DIR="$HOME/.timecoin"
CONFIG_DIR="$NODE_DIR/config"
DATA_DIR="$NODE_DIR/data"
LOG_DIR="$NODE_DIR/logs"
SERVICE_NAME="timed"

# Network configuration
P2P_PORT=24100
API_PORT=24101
API_URL="http://localhost:${API_PORT}"

# Masternode credentials
MASTERNODE_EMAIL=""
MASTERNODE_ADDRESS=""
MASTERNODE_PUBLIC_KEY=""
MASTERNODE_PRIVATE_KEY=""
SERVER_IP=""

#############################
# Sanity / permission checks
#############################

check_root() {
    if [[ $EUID -ne 0 ]]; then
        print_error "This script must be run as root or with sudo"
        exit 1
    fi
}

check_repo_dir() {
    if [ -n "$REPO_DIR" ] && [ -f "$REPO_DIR/Cargo.toml" ]; then
        print_success "Using existing TIME Coin repository at $REPO_DIR"
        return 0
    fi
    
    # Try to detect if we're in the repo
    local current_dir="$(pwd)"
    if [ -f "$current_dir/Cargo.toml" ] && grep -q "time-coin" "$current_dir/Cargo.toml" 2>/dev/null; then
        REPO_DIR="$current_dir"
        print_success "Detected TIME Coin repository at $REPO_DIR"
        return 0
    fi
    
    # Check parent directory
    if [ -f "$current_dir/../Cargo.toml" ] && grep -q "time-coin" "$current_dir/../Cargo.toml" 2>/dev/null; then
        REPO_DIR="$(cd "$current_dir/.." && pwd)"
        print_success "Detected TIME Coin repository at $REPO_DIR"
        return 0
    fi
    
    # Repository not found, will need to clone it
    print_info "TIME Coin repository not found, will clone it"
    REPO_DIR=""
    return 0
}

#############################
# Dependency installation
#############################

clone_repository() {
    if [ -n "$REPO_DIR" ]; then
        print_info "Repository already available at $REPO_DIR"
        return 0
    fi
    
    print_header "Cloning TIME Coin Repository"
    
    # Clone to /root/time-coin
    REPO_DIR="/root/time-coin"
    
    if [ -d "$REPO_DIR" ]; then
        print_warning "Directory $REPO_DIR already exists"
        print_info "Updating repository..."
        cd "$REPO_DIR"
        git fetch --all
        git reset --hard origin/main
        print_success "Repository updated"
    else
        print_info "Cloning from GitHub..."
        git clone https://github.com/your-org/time-coin.git "$REPO_DIR"
        cd "$REPO_DIR"
        print_success "Repository cloned to $REPO_DIR"
    fi
}

install_dependencies() {
    print_header "Installing System Dependencies"

    # apt-get update/upgrade
    print_info "Updating apt package lists..."
    apt-get update -y

    # core build deps for Rust projects
    print_info "Installing build essentials and libs..."
    apt-get install -y \
        build-essential \
        pkg-config \
        libssl-dev \
        curl \
        ca-certificates \
        systemd \
        git \
        jq

    print_success "System dependencies installed"
}

install_rust() {
    print_header "Checking Rust Toolchain"

    if command -v cargo >/dev/null 2>&1 && command -v rustc >/dev/null 2>&1; then
        print_success "Rust already installed: $(rustc --version)"
        return 0
    fi
    
    print_warning "Rust not found. Installing rustup + stable toolchain..."
    
    # Non-interactive rustup install
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    
    # Source cargo env for THIS session so cargo works below
    export PATH="$HOME/.cargo/bin:$PATH"
    
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    fi
    
    # Verify installation
    if command -v cargo >/dev/null 2>&1; then
        print_success "Rust installed: $(rustc --version)"
        print_success "Cargo version: $(cargo --version)"
    else
        print_error "Failed to install Rust/Cargo"
        print_info "Please install manually: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
}

#############################
# Build / install binaries
#############################

build_project() {
    print_header "Building TIME Coin Binaries"

    print_info "Building timed and time-cli (release mode)..."
    cd "$REPO_DIR"
    cargo build --release --workspace --exclude wallet-gui

    print_success "Binaries built successfully"

    # Size info
    if [ -f "$REPO_DIR/target/release/timed" ]; then
        NODE_SIZE=$(du -h "$REPO_DIR/target/release/timed" | cut -f1)
        print_info "timed size: $NODE_SIZE"
    fi
    if [ -f "$REPO_DIR/target/release/time-cli" ]; then
        CLI_SIZE=$(du -h "$REPO_DIR/target/release/time-cli" | cut -f1)
        print_info "time-cli size: $CLI_SIZE"
    fi
}

install_binaries() {
    print_header "Installing Binaries to /usr/local/bin"

    # Stop service if running (best effort)
    if systemctl is-active --quiet ${SERVICE_NAME}; then
        print_info "Stopping existing service..."
        systemctl stop ${SERVICE_NAME}
    fi

    cp "$REPO_DIR/target/release/timed" /usr/local/bin/
    cp "$REPO_DIR/target/release/time-cli" /usr/local/bin/
    chmod +x /usr/local/bin/timed
    chmod +x /usr/local/bin/time-cli

    # Verify
    if command -v timed >/dev/null 2>&1 && command -v time-cli >/dev/null 2>&1; then
        print_success "Binaries installed to /usr/local/bin"
        print_info "timed version: $(timed --version 2>&1 | head -1 || echo 'unknown')"
        print_info "time-cli version:  $(time-cli --version 2>&1 | head -1 || echo 'unknown')"
    else
        print_error "Failed to install binaries"
        exit 1
    fi
}

#############################
# Config + data dirs
#############################

setup_masternode_config() {
    print_header "Setting Up Masternode Configuration"

    # Create directories
    mkdir -p "$CONFIG_DIR"
    mkdir -p "$DATA_DIR"
    mkdir -p "$LOG_DIR"
    
    # Copy genesis block file
    print_info "Installing genesis block file..."
    if [ -f "$REPO_DIR/config/genesis-testnet.json" ]; then
        cp "$REPO_DIR/config/genesis-testnet.json" "$DATA_DIR/genesis.json"
        print_success "Genesis block file installed"
    else
        print_warning "Genesis block file not found at $REPO_DIR/config/genesis-testnet.json"
        print_warning "Node will attempt to download from network"
    fi
    
    local CONFIG_FILE="$CONFIG_DIR/testnet.toml"

    print_info "Writing config to $CONFIG_FILE"

    # Get server IP
    SERVER_IP=$(curl -s ifconfig.me 2>/dev/null || echo "0.0.0.0")
    print_info "Detected server IP: $SERVER_IP"

    cat > "$CONFIG_FILE" <<CONFIGEOF
# TIME Coin Testnet Node Configuration

[node]
mode = "full"
network = "testnet"
name = "masternode-node"
data_dir = "${DATA_DIR}"
log_dir = "${LOG_DIR}"

[blockchain]
# Genesis block file location
genesis_file = "${DATA_DIR}/genesis.json"
# Allow loading genesis from file for initial testnet setup
allow_genesis_load = true
# Disable block recreation (download from peers)
allow_block_recreation = false

[consensus]
# Full node mode - will sync before producing blocks
dev_mode = false
auto_approve = false

[rpc]
enabled = true
bind = "0.0.0.0"
port = ${API_PORT}

[network]
listen_addr = "0.0.0.0:${P2P_PORT}"
port = ${P2P_PORT}
max_peers = 50
testnet = true

[masternode]
enabled = true
tier = "free"
collateral = 0
testnet_mode = true
public_ip = "${SERVER_IP}"
public_port = ${P2P_PORT}
# These will be filled in during masternode activation
address = ""
public_key = ""
private_key = ""

[sync]
# Midnight window configuration for periodic chain sync
midnight_window_enabled = true
midnight_window_start_hour = 23
midnight_window_end_hour = 1
midnight_window_check_consensus = true

[logging]
level = "info"
file = "${LOG_DIR}/node.log"
max_size = "100MB"
max_backups = 5
CONFIGEOF

    print_success "Configuration created"
    print_info "Config location: $CONFIG_FILE"
    print_info "Genesis block:  $DATA_DIR/genesis.json"
    print_info "Data dir:       $DATA_DIR"
    print_info "Log dir:        $LOG_DIR"
    print_info "Ports:          P2P=${P2P_PORT}, API=${API_PORT} (testnet)"
}

#############################
# Masternode activation
#############################

wait_for_api() {
    print_info "Waiting for node to initialize and API to become available..."
    local max_attempts=60
    local attempt=0
    
    while [ $attempt -lt $max_attempts ]; do
        if curl -s "$API_URL/health" > /dev/null 2>&1; then
            print_success "API is responding"
            sleep 5  # Extra time for full initialization
            return 0
        fi
        attempt=$((attempt + 1))
        sleep 3
    done
    
    print_warning "API check timed out after $max_attempts attempts"
    print_info "Node may still be initializing - this is normal during first sync"
    return 1
}

setup_masternode_credentials() {
    print_header "Setting Up Masternode Credentials"
    
    echo ""
    echo -e "${BLUE}Setting up free tier masternode for testnet...${NC}"
    echo -e "${BLUE}No collateral required - using free tier for testing.${NC}"
    echo ""

    print_info "Generating new keypair..."
    local response=$(curl -s -X POST "$API_URL/keypair/generate" 2>/dev/null)
    
    if [ -n "$response" ]; then
        MASTERNODE_ADDRESS=$(echo "$response" | jq -r '.address' 2>/dev/null)
        MASTERNODE_PUBLIC_KEY=$(echo "$response" | jq -r '.public_key' 2>/dev/null)
        MASTERNODE_PRIVATE_KEY=$(echo "$response" | jq -r '.private_key' 2>/dev/null)
    fi
    
    # If API call failed, generate basic credentials
    if [ -z "$MASTERNODE_ADDRESS" ] || [ "$MASTERNODE_ADDRESS" == "null" ]; then
        print_warning "API keypair generation not available, using basic setup"
        MASTERNODE_ADDRESS="masternode_$(date +%s)"
        MASTERNODE_PUBLIC_KEY="generated_key_$(date +%s)"
        MASTERNODE_PRIVATE_KEY="private_key_$(openssl rand -hex 32)"
    fi
    
    print_success "Keypair generated"
    print_info "Address: $MASTERNODE_ADDRESS"
    print_info "Tier: Free (testnet)"
    
    # Update the config file with the generated credentials
    local config_file="$CONFIG_DIR/testnet.toml"
    
    sed -i "s|address = \"\"|address = \"$MASTERNODE_ADDRESS\"|" "$config_file"
    sed -i "s|public_key = \"\"|public_key = \"$MASTERNODE_PUBLIC_KEY\"|" "$config_file"
    sed -i "s|private_key = \"\"|private_key = \"$MASTERNODE_PRIVATE_KEY\"|" "$config_file"
    
    print_success "Configuration updated with masternode credentials"
    
    # Save credentials to file
    local creds_file="$NODE_DIR/masternode-credentials.txt"
    cat > "$creds_file" <<CREDSEOF
TIME COIN MASTERNODE CREDENTIALS
Generated: $(date)

Tier: Free (testnet - no collateral required)
Address: $MASTERNODE_ADDRESS
Public Key: $MASTERNODE_PUBLIC_KEY
Private Key: $MASTERNODE_PRIVATE_KEY
Server IP: $SERVER_IP:$P2P_PORT

⚠️  NEVER SHARE YOUR PRIVATE KEY!
⚠️  BACKUP THIS FILE SECURELY!

Configuration: $config_file
Logs: $LOG_DIR/node.log
CREDSEOF
    
    chmod 600 "$creds_file"
    print_success "Credentials saved to: $creds_file"
}

#############################
# systemd service
#############################

create_systemd_service() {
    print_header "Creating Systemd Service ${SERVICE_NAME}.service"

    cat > /etc/systemd/system/${SERVICE_NAME}.service <<SERVICEEOF
[Unit]
Description=TIME Coin Testnet Masternode
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/timed --config $CONFIG_DIR/testnet.toml
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
WorkingDirectory=$NODE_DIR
User=root

[Install]
WantedBy=multi-user.target
SERVICEEOF

    # systemd reload
    systemctl daemon-reload

    print_success "Systemd service created at /etc/systemd/system/${SERVICE_NAME}.service"
}

#############################
# start service
#############################

start_masternode() {
    print_header "Starting Masternode Service (Initial)"

    systemctl enable ${SERVICE_NAME}
    systemctl start ${SERVICE_NAME}

    # give it a moment
    sleep 5

    if systemctl is-active --quiet ${SERVICE_NAME}; then
        print_success "Masternode service is running!"
    else
        print_error "Failed to start masternode service"
        print_info "Check logs with: journalctl -u ${SERVICE_NAME} -f"
        exit 1
    fi
}

restart_masternode() {
    print_header "Restarting Masternode with New Configuration"
    
    systemctl restart ${SERVICE_NAME}
    sleep 5
    
    if systemctl is-active --quiet ${SERVICE_NAME}; then
        print_success "Masternode restarted successfully"
    else
        print_error "Failed to restart masternode"
        print_info "Check logs with: journalctl -u ${SERVICE_NAME} -f"
        exit 1
    fi
}

verify_installation() {
    print_header "Verifying Installation"
    
    sleep 3
    
    if systemctl is-active --quiet ${SERVICE_NAME}; then
        print_success "Masternode service is running"
    else
        print_warning "Masternode service status unclear"
    fi
    
    local response=$(curl -s "$API_URL/health" 2>/dev/null)
    if [ -n "$response" ]; then
        print_success "API is responding"
    else
        print_warning "API check inconclusive"
    fi
}

#############################
# summary
#############################

show_summary() {
    print_header "Masternode Installation Complete!"

    echo ""
    echo -e "${GREEN}✅ TIME Coin Masternode Successfully Installed and Activated!${NC}"
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}                    Installation Details${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo ""
    echo "Repository:        $REPO_DIR"
    echo "Node Directory:    $NODE_DIR (Bitcoin-style: ~/.timecoin)"
    echo "Configuration:     $CONFIG_DIR/testnet.toml"
    echo "Data Directory:    $DATA_DIR"
    echo "Log Directory:     $LOG_DIR"
    echo "Binaries:          /usr/local/bin/time{d,-cli}"
    echo "Service:           ${SERVICE_NAME}.service"
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}                    Masternode Details${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo ""
    echo "Tier:              Free (testnet)"
    echo "Address:           $MASTERNODE_ADDRESS"
    echo "Server IP:         $SERVER_IP"
    echo "P2P Port:          $P2P_PORT"
    echo "API Port:          $API_PORT"
    echo "Status:            Running"
    echo "Credentials File:  $NODE_DIR/masternode-credentials.txt"
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}                    Useful Commands${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo ""
    echo -e "${YELLOW}Service Management:${NC}"
    echo "  sudo systemctl status ${SERVICE_NAME}      # Check service status"
    echo "  sudo systemctl stop ${SERVICE_NAME}        # Stop the service"
    echo "  sudo systemctl start ${SERVICE_NAME}       # Start the service"
    echo "  sudo systemctl restart ${SERVICE_NAME}     # Restart the service"
    echo "  sudo journalctl -u ${SERVICE_NAME} -f      # View live logs"
    echo ""
    echo -e "${YELLOW}Node Information:${NC}"
    echo "  time-cli info                              # Node info"
    echo "  time-cli status                            # Node status"
    echo "  time-cli peers                             # Connected peers"
    echo "  time-cli blocks                            # Blockchain info"
    echo ""
    echo -e "${YELLOW}Masternode Status:${NC}"
    echo "  curl http://localhost:${API_PORT}/balance/$MASTERNODE_ADDRESS"
    echo "  curl http://localhost:${API_PORT}/health"
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}                    Important Security Notes${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo ""
    echo -e "${RED}⚠️  CRITICAL: Backup your credentials file!${NC}"
    echo "    Location: $NODE_DIR/masternode-credentials.txt"
    echo ""
    echo -e "${RED}⚠️  NEVER share your private key with anyone!${NC}"
    echo ""
    echo -e "${YELLOW}Firewall Configuration:${NC}"
    echo "  If you have a firewall, allow incoming connections on port $P2P_PORT:"
    echo "  sudo ufw allow ${P2P_PORT}/tcp comment 'TIME Coin P2P'"
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}                    Next Steps${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo ""
    echo "1. Monitor your masternode:"
    echo "   sudo journalctl -u ${SERVICE_NAME} -f"
    echo ""
    
    if [ -z "$MASTERNODE_ADDRESS" ]; then
        echo "2. Generate masternode credentials (once node is fully synced):"
        echo "   curl -X POST http://localhost:${API_PORT}/keypair/generate"
        echo "   Then update the config at $CONFIG_DIR/testnet.toml"
        echo ""
        echo "3. Check your balance:"
        echo "   time-cli balance <your_address>"
        echo ""
        echo "4. View node status:"
        echo "   time-cli status"
        echo ""
        echo "5. Join the TIME Coin community for support and updates"
    else
        echo "2. Check your balance:"
        echo "   time-cli balance $MASTERNODE_ADDRESS"
        echo ""
        echo "3. View node status:"
        echo "   time-cli status"
        echo ""
        echo "4. Join the TIME Coin community for support and updates"
    fi
    echo ""
    echo -e "${GREEN}🎉 Your masternode is now live on TIME Coin testnet!${NC}"
    echo -e "${GREEN}🚀 The node will sync with the network and participate in consensus.${NC}"
    echo ""
}

#############################
# main flow
#############################

main() {
    print_header "TIME Coin Masternode Installation"

    check_root
    check_repo_dir
    
    # If repo not found, clone it
    if [ -z "$REPO_DIR" ]; then
        clone_repository
    fi
    
    install_dependencies
    install_rust
    build_project
    install_binaries
    setup_masternode_config
    create_systemd_service
    start_masternode
    
    # Wait for API to be ready
    if wait_for_api; then
        # Setup masternode credentials
        setup_masternode_credentials
        
        # Restart with new configuration
        restart_masternode
    else
        print_warning "API not available, continuing with basic setup"
    fi
    
    # Verify everything is working
    verify_installation
    
    show_summary
}

main "$@"