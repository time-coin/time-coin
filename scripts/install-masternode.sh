#!/bin/bash

#############################################################
# TIME Coin Masternode Installation Script
# 
# This script assumes you have already run setup-system.sh
# and the TIME Coin repository is cloned.
#
# This script will:
# - Build the TIME coin project
# - Install the masternode binary
# - Set up the masternode configuration
# - Create and start the systemd service
#
# Usage: 
#   cd ~/time-coin
#   sudo ./scripts/install-masternode.sh
#############################################################

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NODE_DIR="$HOME/time-coin-node"
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
    if ! su - $SUDO_USER -c "command -v cargo" &> /dev/null; then
        print_error "Rust is not installed!"
        print_info "Please run setup-system.sh first"
        exit 1
    fi
    
    print_success "Prerequisites check passed"
    print_info "Repository: $REPO_DIR"
    print_info "Rust: $(su - $SUDO_USER -c 'source $HOME/.cargo/env && rustc --version')"
}

#############################################################
# Installation Steps
#############################################################

build_project() {
    print_header "Building TIME Coin Masternode"
    
    print_info "This may take 5-10 minutes on first build..."
    
    cd "$REPO_DIR"
    
    # Build the project as the non-root user
    su - $SUDO_USER -c "cd $REPO_DIR && source \$HOME/.cargo/env && cargo build --release --bin time-node"
    
    print_success "TIME Coin built successfully"
    
    # Show the binary info
    if [ -f "$REPO_DIR/target/release/time-node" ]; then
        BINARY_SIZE=$(du -h "$REPO_DIR/target/release/time-node" | cut -f1)
        print_info "Binary size: $BINARY_SIZE"
    fi
}

install_binary() {
    print_header "Installing TIME Node Binary"
    
    # Stop service if running
    if systemctl is-active --quiet ${SERVICE_NAME}; then
        print_info "Stopping existing service..."
        systemctl stop ${SERVICE_NAME}
    fi
    
    # Copy binary to system path
    cp "$REPO_DIR/target/release/time-node" /usr/local/bin/
    chmod +x /usr/local/bin/time-node
    
    # Verify installation
    if command -v time-node &> /dev/null; then
        print_success "time-node installed to /usr/local/bin/"
    else
        print_error "Failed to install time-node binary"
        exit 1
    fi
}

setup_masternode_config() {
    print_header "Setting Up Masternode Configuration"
    
    # Create node directory structure as non-root user
    su - $SUDO_USER -c "mkdir -p $CONFIG_DIR"
    su - $SUDO_USER -c "mkdir -p $NODE_DIR/data"
    su - $SUDO_USER -c "mkdir -p $NODE_DIR/logs"
    
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
    
    su - $SUDO_USER -c "cat > $CONFIG_DIR/testnet.toml" << 'EOF'
# TIME Coin Testnet Configuration

[network]
listen_addr = "0.0.0.0:24100"
external_addr = ""
bootstrap_nodes = []

[masternode]
enabled = true
# Keys will be generated during first run or via CLI

[rpc]
enabled = true
listen_addr = "127.0.0.1:24101"

[storage]
data_dir = "./data"

[logging]
level = "info"
EOF
    
    print_success "Masternode configuration created"
    print_info "Config location: $CONFIG_DIR/testnet.toml"
    print_info "Ports: P2P=24100, RPC=24101 (testnet)"
}

create_systemd_service() {
    print_header "Creating Systemd Service"
    
    # Create service file
    cat > /etc/systemd/system/${SERVICE_NAME}.service << EOF
[Unit]
Description=TIME Coin Testnet Masternode
After=network.target

[Service]
Type=simple
User=$SUDO_USER
WorkingDirectory=$NODE_DIR
ExecStart=/usr/local/bin/time-node --config $CONFIG_DIR/testnet.toml
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=full
ProtectHome=read-only

[Install]
WantedBy=multi-user.target
EOF
    
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
    sleep 2
    
    # Check status
    if systemctl is-active --quiet ${SERVICE_NAME}; then
        print_success "Masternode is running!"
        
        echo ""
        print_info "Service Status:"
        systemctl status ${SERVICE_NAME} --no-pager -l | head -15
    else
        print_error "Failed to start masternode"
        print_info "Check logs with: journalctl -u ${SERVICE_NAME} -f"
        exit 1
    fi
}

show_summary() {
    print_header "Masternode Installation Complete!"
    
    cat << EOF

${GREEN}✅ TIME Coin Masternode Successfully Installed!${NC}

${BLUE}Installation Details:${NC}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Repository:        $REPO_DIR
Node Directory:    $NODE_DIR
Configuration:     $CONFIG_DIR/testnet.toml
Binary:            /usr/local/bin/time-node
Service:           ${SERVICE_NAME}.service
Network Ports:     24100 (P2P), 24101 (RPC)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

${BLUE}Useful Commands:${NC}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Check status:      sudo systemctl status ${SERVICE_NAME}
View logs:         sudo journalctl -u ${SERVICE_NAME} -f
Stop node:         sudo systemctl stop ${SERVICE_NAME}
Start node:        sudo systemctl start ${SERVICE_NAME}
Restart node:      sudo systemctl restart ${SERVICE_NAME}
Disable service:   sudo systemctl disable ${SERVICE_NAME}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

${BLUE}Firewall (if needed):${NC}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
sudo ufw allow 24100/tcp comment 'TIME Coin Testnet P2P'
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

${BLUE}To Update Masternode:${NC}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
cd $REPO_DIR
git pull origin main
sudo ./scripts/install-masternode.sh
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

${GREEN}Happy Mining! 🚀${NC}

EOF
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
    install_binary
    setup_masternode_config
    create_systemd_service
    start_masternode
    
    # Show summary
    show_summary
}

# Run main installation
main "$@"
