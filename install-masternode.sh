#!/bin/bash

#############################################################
# TIME Coin Masternode Installation Script
#
# This script will:
# - Install build dependencies (apt)
# - Install Rust toolchain if missing
# - Build time-node and time-cli
# - Install binaries to /usr/local/bin/
# - Create config in /root/time-coin-node/config/testnet.toml
# - Create+enable+start systemd service "time-node"
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

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NODE_DIR="/root/time-coin-node"
CONFIG_DIR="$NODE_DIR/config"
SERVICE_NAME="time-node"

# These values are now generic (no per-server branching)
P2P_PORT=24100
API_PORT=24101
DATA_DIR="/var/lib/time-coin"

# We'll leave masternode address empty for now and let operator configure
MASTERNODE_ADDRESS=""

# We'll prefer peer discovery instead of hardcoding other machines.
# The node should hit this to find peers.
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

    print_info "Building time-node and time-cli (release mode)..."
    cd "$REPO_DIR"
    cargo build --release

    print_success "Binaries built successfully"

    # Size info
    if [ -f "$REPO_DIR/target/release/time-node" ]; then
        NODE_SIZE=$(du -h "$REPO_DIR/target/release/time-node" | cut -f1)
        print_info "time-node size: $NODE_SIZE"
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

    cp "$REPO_DIR/target/release/time-node" /usr/local/bin/
    cp "$REPO_DIR/target/release/time-cli" /usr/local/bin/
    chmod +x /usr/local/bin/time-node
    chmod +x /usr/local/bin/time-cli

    # Verify
    if command -v time-node >/dev/null 2>&1 && command -v time-cli >/dev/null 2>&1; then
        print_success "Binaries installed to /usr/local/bin"
        print_info "time-node version: $(time-node --version 2>&1 | head -1 || echo 'unknown')"
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

    mkdir -p "$CONFIG_DIR"
    mkdir -p "$NODE_DIR/data"
    mkdir -p "$NODE_DIR/logs"
    mkdir -p "$DATA_DIR"
    mkdir -p "$DATA_DIR/wallets"

    local CONFIG_FILE="$CONFIG_DIR/testnet.toml"

    # We no longer prompt interactively; we just overwrite.
    print_info "Writing config to $CONFIG_FILE"

    cat > "$CONFIG_FILE" <<CONFIGEOF
# TIME Coin Testnet Node Configuration

[network]
listen_addr = "0.0.0.0:${P2P_PORT}"
api_port = ${API_PORT}
testnet = true

# Node will fetch peers from this discovery service. This removes the need
# to hardcode per-server peers.
peer_discovery_url = "${PEER_DISCOVERY_URL}"

[masternode]
enabled = true
# If this node is collateralized / staked, put its registered address here.
# Leaving blank is fine; the node can still run/relay.
address = "${MASTERNODE_ADDRESS}"

[storage]
data_dir = "${DATA_DIR}"
CONFIGEOF

    print_success "Configuration created"
    print_info "Config location: $CONFIG_FILE"
    print_info "Data dir:       $DATA_DIR"
    print_info "Wallet dir:     $DATA_DIR/wallets"
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
ExecStart=/usr/local/bin/time-node --config $CONFIG_DIR/testnet.toml
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
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Node Directory:    $NODE_DIR"
    echo "Configuration:     $CONFIG_DIR/testnet.toml"
    echo "Binaries:          /usr/local/bin/time-{node,cli}"
    echo "Service:           ${SERVICE_NAME}.service"
    echo "Network Ports:     ${P2P_PORT} (P2P), ${API_PORT} (API, testnet)"
    echo "Wallet Storage:    ${DATA_DIR}/wallets"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo -e "${BLUE}Useful Commands:${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
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
    echo "  time-cli wallet generate-address <PUBKEY>"
    echo "  time-cli wallet create <PUBKEY>"
    echo "  time-cli wallet balance <ADDRESS>"
    echo "  time-cli wallet info <ADDRESS>"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo -e "${BLUE}Firewall (if needed):${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "sudo ufw allow ${P2P_PORT}/tcp comment 'TIME Coin P2P'"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
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
