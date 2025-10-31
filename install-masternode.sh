#!/bin/bash

#############################################################
# TIME Coin Masternode Installation Script
# 
# This script will:
# - Build both time-node and time-cli binaries
# - Install the binaries to /usr/local/bin/
# - Set up the masternode configuration
# - Create and start the systemd service
#
# Usage: 
#   cd ~/time-coin
#   sudo ./install-masternode.sh <server-name>
#   
#   server-name: ubuntu | newyork | reitools
#############################################################

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get server name from argument
if [ -z "$1" ]; then
    echo -e "${RED}❌ Error: Server name required${NC}"
    echo "Usage: sudo ./install-masternode.sh <server-name>"
    echo "  server-name: ubuntu | newyork | reitools"
    exit 1
fi

SERVER_NAME="$1"

# Configuration based on server
case "$SERVER_NAME" in
    ubuntu)
        MASTERNODE_ADDRESS="TIME1ubuntu"
        PEER1="reitools.us:24100"
        PEER2="newyork-server:24100"
        ;;
    newyork)
        MASTERNODE_ADDRESS="TIME1newyork"
        PEER1="ubuntu-server:24100"
        PEER2="reitools.us:24100"
        ;;
    reitools)
        MASTERNODE_ADDRESS="TIME1reitools"
        PEER1="ubuntu-server:24100"
        PEER2="newyork-server:24100"
        ;;
    *)
        echo -e "${RED}❌ Invalid server name: $SERVER_NAME${NC}"
        echo "Valid options: ubuntu | newyork | reitools"
        exit 1
        ;;
esac

# Directories
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NODE_DIR="/root/time-coin-node"
CONFIG_DIR="$NODE_DIR/config"
SERVICE_NAME="time-node"

#############################################################
# Helper Functions
#############################################################

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

check_root() {
    if [[ $EUID -ne 0 ]]; then
        print_error "This script must be run as root or with sudo"
        exit 1
    fi
}

check_prerequisites() {
    print_header "Checking Prerequisites"
    
    # Check if in correct directory
    if [ ! -f "$REPO_DIR/Cargo.toml" ]; then
        print_error "Not in TIME Coin repository directory!"
        print_info "Please run this script from: ~/time-coin/"
        exit 1
    fi
    
    # Check if Rust is installed
    if ! command -v cargo &> /dev/null; then
        print_error "Rust is not installed!"
        print_info "Install with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
    
    print_success "Prerequisites check passed"
    print_info "Repository: $REPO_DIR"
    print_info "Rust: $(rustc --version)"
    print_info "Server: $SERVER_NAME"
    print_info "Masternode Address: $MASTERNODE_ADDRESS"
}

#############################################################
# Installation Steps
#############################################################

build_project() {
    print_header "Building TIME Coin Binaries"
    
    print_info "Building time-node and time-cli..."
    print_info "This may take 5-10 minutes on first build..."
    
    cd "$REPO_DIR"
    
    # Build both binaries
    cargo build --release
    
    print_success "Binaries built successfully"
    
    # Show binary info
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
    print_header "Installing Binaries"
    
    # Stop service if running
    if systemctl is-active --quiet ${SERVICE_NAME}; then
        print_info "Stopping existing service..."
        systemctl stop ${SERVICE_NAME}
    fi
    
    # Copy binaries to system path
    cp "$REPO_DIR/target/release/time-node" /usr/local/bin/
    cp "$REPO_DIR/target/release/time-cli" /usr/local/bin/
    chmod +x /usr/local/bin/time-node
    chmod +x /usr/local/bin/time-cli
    
    # Verify installation
    if command -v time-node &> /dev/null && command -v time-cli &> /dev/null; then
        print_success "Binaries installed to /usr/local/bin/"
        print_info "time-node version: $(time-node --version 2>&1 | head -1 || echo 'unknown')"
        print_info "time-cli version: $(time-cli --version 2>&1 | head -1 || echo 'unknown')"
    else
        print_error "Failed to install binaries"
        exit 1
    fi
}

setup_masternode_config() {
    print_header "Setting Up Masternode Configuration"
    
    # Create node directory structure
    mkdir -p "$CONFIG_DIR"
    mkdir -p "$NODE_DIR/data"
    mkdir -p "$NODE_DIR/logs"
    mkdir -p "/var/lib/time-coin"
    mkdir -p "/var/lib/time-coin/wallets"
    
    # Check if config already exists
    if [ -f "$CONFIG_DIR/testnet.toml" ]; then
        print_warning "Configuration file already exists"
        read -p "Do you want to overwrite it? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_info "Keeping existing configuration"
            return
        fi
    fi
    
    # Create configuration file
    print_info "Creating testnet configuration..."
    
    cat > "$CONFIG_DIR/testnet.toml" << CONFIGEOF
[network]
listen_addr = "0.0.0.0:24100"
api_port = 24101
testnet = true

[masternode]
enabled = true
address = "$MASTERNODE_ADDRESS"

[peers]
bootstrap = [
    "$PEER1",
    "$PEER2"
]

[storage]
data_dir = "/var/lib/time-coin"
CONFIGEOF
    
    print_success "Masternode configuration created"
    print_info "Config location: $CONFIG_DIR/testnet.toml"
    print_info "Masternode: $MASTERNODE_ADDRESS"
    print_info "Peers: $PEER1, $PEER2"
    print_info "Ports: P2P=24100, API=24101 (testnet)"
}

create_systemd_service() {
    print_header "Creating Systemd Service"
    
    # Create service file
    cat > /etc/systemd/system/${SERVICE_NAME}.service << SERVICEEOF
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
    
    # Reload systemd
    systemctl daemon-reload
    
    print_success "Systemd service created"
}

start_masternode() {
    print_header "Starting Masternode"
    
    # Enable service
    systemctl enable ${SERVICE_NAME}
    
    # Start service
    systemctl start ${SERVICE_NAME}
    
    # Wait a moment for it to start
    sleep 3
    
    # Check status
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

show_summary() {
    print_header "Masternode Installation Complete!"
    
    echo ""
    echo -e "${GREEN}✅ TIME Coin Masternode Successfully Installed!${NC}"
    echo ""
    echo -e "${BLUE}Installation Details:${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Server:            $SERVER_NAME"
    echo "Masternode:        $MASTERNODE_ADDRESS"
    echo "Repository:        $REPO_DIR"
    echo "Node Directory:    $NODE_DIR"
    echo "Configuration:     $CONFIG_DIR/testnet.toml"
    echo "Binaries:          /usr/local/bin/time-{node,cli}"
    echo "Service:           ${SERVICE_NAME}.service"
    echo "Network Ports:     24100 (P2P), 24101 (API)"
    echo "Wallet Storage:    /var/lib/time-coin/wallets"
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
    echo "sudo ufw allow 24100/tcp comment 'TIME Coin P2P'"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo -e "${BLUE}To Update Masternode:${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "cd ~/time-coin"
    echo "git pull"
    echo "sudo ./install-masternode.sh $SERVER_NAME"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo -e "${GREEN}Happy Mining! 🚀${NC}"
    echo ""
}

#############################################################
# Main Installation Flow
#############################################################

main() {
    print_header "TIME Coin Masternode Installation"
    
    # Check if running as root
    check_root
    
    # Check prerequisites
    check_prerequisites
    
    # Run installation steps
    build_project
    install_binaries
    setup_masternode_config
    create_systemd_service
    start_masternode
    
    # Show summary
    show_summary
}

# Run main installation
main "$@"
