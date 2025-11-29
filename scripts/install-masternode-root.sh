#!/bin/bash

#############################################################
# TIME Coin Masternode Installation Script
#
# This script will:
# - Install build dependencies (apt)
# - Install Rust toolchain if missing
# - Build timed and time-cli
# - Install binaries to /usr/local/bin/
# - Create config in /root/time-coin-node/config/testnet.toml
# - Create+enable+start systemd service "timed"
#
# Usage:
#   cd ~/time-coin
#   sudo ./install-masternode.sh
#
# Assumptions:
#   - Ubuntu (tested against 22.04+ and 25.04)
#   - systemd present
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

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NODE_DIR="/root/time-coin-node"
CONFIG_DIR="$NODE_DIR/config"
SERVICE_NAME="timed"

# Network configuration
P2P_PORT=24100
API_PORT=24101

# Data directory - this is where blockchain data will be stored
DATA_DIR="/var/lib/time-coin"

# We'll leave masternode address empty for now and let operator configure
MASTERNODE_ADDRESS=""

# Peer discovery endpoint
PEER_DISCOVERY_URL="https://time-coin.io/api/peers"

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
    if [ ! -f "$REPO_DIR/Cargo.toml" ]; then
        print_error "Not in TIME Coin repository directory!"
        print_info "Please run this script from: ~/time-coin/"
        exit 1
    fi
}

#############################
# Dependency installation
#############################

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
        git

    print_success "System dependencies installed"
}

install_rust() {
    print_header "Checking Rust Toolchain"

    if command -v cargo >/dev/null 2>&1 && command -v rustc >/dev/null 2>&1; then
        print_success "Rust already installed: $(rustc --version)"
    else
        print_warning "Rust not found. Installing rustup + stable toolchain..."
        # Non-interactive rustup install
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

        # Source cargo env for THIS session so cargo works below
        if [ -f "$HOME/.cargo/env" ]; then
            source "$HOME/.cargo/env"
        fi

        print_success "Rust installed: $(rustc --version)"
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

    # Create config directory
    mkdir -p "$CONFIG_DIR"
    
    # NOTE: We don't create data directories here - the node will create them on first run
    # This ensures proper permissions and structure
    
    # Copy genesis block file
    print_info "Installing genesis block file..."
    if [ -f "$REPO_DIR/config/genesis-testnet.json" ]; then
        cp "$REPO_DIR/config/genesis-testnet.json" "$CONFIG_DIR/genesis-testnet.json"
        print_success "Genesis block file installed"
    else
        print_warning "Genesis block file not found at $REPO_DIR/config/genesis-testnet.json"
        print_warning "Node will attempt to download from network"
    fi
    
    local CONFIG_FILE="$CONFIG_DIR/testnet.toml"

    print_info "Writing config to $CONFIG_FILE"

    cat > "$CONFIG_FILE" <<CONFIGEOF
# TIME Coin Testnet Node Configuration

[node]
network = "testnet"
mode = "masternode"
# Data directory - node will create subdirectories automatically
data_dir = "${DATA_DIR}"

[blockchain]
# Genesis block file location
genesis_file = "${CONFIG_DIR}/genesis-testnet.json"

[consensus]
dev_mode = false

[rpc]
enabled = true
bind = "0.0.0.0"
port = ${API_PORT}

[network]
listen_addr = "0.0.0.0:${P2P_PORT}"
api_port = ${API_PORT}
testnet = true
peer_discovery_url = "${PEER_DISCOVERY_URL}"

[masternode]
enabled = true
# IMPORTANT: Generate this address in wallet-gui (hot wallet), NOT on this server!
# The masternode only references the address - it never holds your private keys.
# All rewards will be sent to this address in your hot wallet.
# Testnet addresses start with TIME0, mainnet addresses start with TIME1
address = "${MASTERNODE_ADDRESS}"
CONFIGEOF

    print_success "Configuration created"
    print_info "Config location: $CONFIG_FILE"
    print_info "Genesis block:  $CONFIG_DIR/genesis-testnet.json"
    print_info "Data dir:       $DATA_DIR (will be created on first run)"
    print_info "Ports:          P2P=${P2P_PORT}, API=${API_PORT} (testnet)"
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
    print_header "Starting Masternode Service"

    systemctl enable ${SERVICE_NAME}
    systemctl start ${SERVICE_NAME}

    # give it a moment
    sleep 3

    if systemctl is-active --quiet ${SERVICE_NAME}; then
        print_success "Masternode is running!"

        echo ""
        print_info "Service Status:"
        systemctl status ${SERVICE_NAME} --no-pager -l | head -15

        echo ""
        print_info "Recent Logs:"
        journalctl -u ${SERVICE_NAME} -n 10 --no-pager
    else
        print_error "Failed to start masternode"
        print_info "Check logs with: journalctl -u ${SERVICE_NAME} -f"
        exit 1
    fi
}

#############################
# summary
#############################

show_summary() {
    print_header "Masternode Installation Complete!"

    echo ""
    echo -e "${GREEN}✅ TIME Coin Masternode Successfully Installed!${NC}"
    echo ""
    echo -e "${BLUE}Installation Details:${NC}"
    echo "────────────────────────────────────────────────"
    echo "Node Directory:    $NODE_DIR"
    echo "Configuration:     $CONFIG_DIR/testnet.toml"
    echo "Data Directory:    $DATA_DIR"
    echo "Binaries:          /usr/local/bin/time{d,-cli}"
    echo "Service:           ${SERVICE_NAME}.service"
    echo "Network Ports:     ${P2P_PORT} (P2P), ${API_PORT} (API, testnet)"
    echo "────────────────────────────────────────────────"
    echo ""
    echo -e "${BLUE}Useful Commands:${NC}"
    echo "────────────────────────────────────────────────"
    echo "Node Control:"
    echo "  sudo systemctl status ${SERVICE_NAME}"
    echo "  sudo systemctl stop ${SERVICE_NAME}"
    echo "  sudo systemctl start ${SERVICE_NAME}"
    echo "  sudo systemctl restart ${SERVICE_NAME}"
    echo "  sudo journalctl -u ${SERVICE_NAME} -f"
    echo ""
    echo "Blockchain Queries:"
    echo "  time-cli info"
    echo "  time-cli status"
    echo "  time-cli peers"
    echo "  time-cli blocks"
    echo ""
    echo "Wallet Commands:"
    echo "  ⚠️  DO NOT use wallet commands on the masternode!"
    echo "  Generate addresses in wallet-gui on your secure computer."
    echo ""
    echo "  The masternode only needs the address in config, not the keys."
    echo "────────────────────────────────────────────────"
    echo ""
    echo -e "${BLUE}Firewall (if needed):${NC}"
    echo "────────────────────────────────────────────────"
    echo "sudo ufw allow ${P2P_PORT}/tcp comment 'TIME Coin P2P'"
    echo "────────────────────────────────────────────────"
    echo ""
    echo -e "${BLUE}Data Directory Structure:${NC}"
    echo "────────────────────────────────────────────────"
    echo "The node will automatically create:"
    echo "  ${DATA_DIR}/blockchain/    - Blockchain data"
    echo "  ${DATA_DIR}/wallets/       - Node wallet"
    echo "  ${DATA_DIR}/logs/          - Log files"
    echo "  ${DATA_DIR}/mempool.json   - Transaction pool"
    echo "  ${DATA_DIR}/genesis.json   - Genesis block (if downloaded)"
    echo "────────────────────────────────────────────────"
    echo ""
    echo -e "${BLUE}Security Model - Hot Wallet vs Masternode:${NC}"
    echo "────────────────────────────────────────────────"
    echo "⚠️  IMPORTANT: Masternodes should NEVER hold coins!"
    echo ""
    echo "The masternode is a 'cold' server that:"
    echo "  • Validates transactions and blocks"
    echo "  • Participates in consensus"
    echo "  • References a reward address (doesn't store keys)"
    echo ""
    echo "Your 'hot' wallet (wallet-gui) is where you:"
    echo "  • Generate addresses with your mnemonic phrase"
    echo "  • Hold private keys securely"
    echo "  • Receive masternode rewards"
    echo "  • Send transactions"
    echo ""
    echo "Address Prefixes:"
    echo "  • TIME0 - Testnet addresses"
    echo "  • TIME1 - Mainnet addresses"
    echo ""
    echo "Setup Process:"
    echo "  1. Generate address in wallet-gui (hot wallet)"
    echo "  2. Copy the address"
    echo "  3. Add it to masternode config (this server)"
    echo "  4. Rewards go to your hot wallet, NOT this server"
    echo "────────────────────────────────────────────────"
    echo ""
    echo -e "${BLUE}Next Steps:${NC}"
    echo "────────────────────────────────────────────────"
    echo "1. Generate Reward Address in wallet-gui:"
    echo "   a. Run wallet-gui on YOUR SECURE COMPUTER"
    echo "   b. Create or import mnemonic phrase"
    echo "   c. Generate a new receiving address"
    echo "   d. Copy the address (starts with TIME0 for testnet)"
    echo ""
    echo "2. Configure Masternode Reward Address:"
    echo "   Edit: $CONFIG_DIR/testnet.toml"
    echo "   Set:  masternode.address = \"TIME0your_address_from_wallet_gui\""
    echo ""
    echo "3. Restart the masternode service:"
    echo "   sudo systemctl restart ${SERVICE_NAME}"
    echo ""
    echo "4. Monitor the node:"
    echo "   sudo journalctl -u ${SERVICE_NAME} -f"
    echo ""
    echo "🔒 SECURITY: The masternode NEVER stores your private keys!"
    echo "   All rewards go to your hot wallet address."
    echo "────────────────────────────────────────────────"
    echo ""
    echo -e "${GREEN}🎉 Your node is live on TIME Coin testnet.${NC}"
    echo ""
}

#############################
# main flow
#############################

main() {
    print_header "TIME Coin Masternode Installation"

    check_root
    check_repo_dir
    install_dependencies
    install_rust
    build_project
    install_binaries
    setup_masternode_config
    create_systemd_service
    start_masternode
    show_summary
}

main "$@"