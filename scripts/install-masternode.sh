#!/bin/bash

#############################################################
# TIME Coin Masternode Installation Script
# 
# This script will:
# - Install all required dependencies
# - Clone and build the TIME coin repository
# - Set up the masternode configuration
# - Create and start the systemd service
#
# Usage: 
#   curl -O https://raw.githubusercontent.com/time-coin/time-coin/main/install-masternode.sh
#   chmod +x install-masternode.sh
#   sudo ./install-masternode.sh
#############################################################

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
REPO_URL="https://github.com/time-coin/time-coin.git"
INSTALL_DIR="$HOME/projects/time-coin"
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

#############################################################
# Installation Steps
#############################################################

install_system_dependencies() {
    print_header "Installing System Dependencies"
    
    apt update
    apt upgrade -y
    
    apt install -y \
        build-essential \
        curl \
        git \
        pkg-config \
        libssl-dev \
        ca-certificates
    
    print_success "System dependencies installed"
}

install_rust() {
    print_header "Installing Rust"
    
    # Check if Rust is already installed
    if command -v rustc &> /dev/null; then
        print_info "Rust is already installed: $(rustc --version)"
        read -p "Do you want to update it? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            su - $SUDO_USER -c "rustup update"
            print_success "Rust updated"
        fi
    else
        # Install Rust as the non-root user
        su - $SUDO_USER -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
        su - $SUDO_USER -c "source \$HOME/.cargo/env"
        
        print_success "Rust installed"
    fi
    
    # Add Rust components
    su - $SUDO_USER -c "source \$HOME/.cargo/env && rustup component add rustfmt clippy"
    
    # Verify installation
    RUST_VERSION=$(su - $SUDO_USER -c "source \$HOME/.cargo/env && rustc --version")
    CARGO_VERSION=$(su - $SUDO_USER -c "source \$HOME/.cargo/env && cargo --version")
    
    print_info "Rust: $RUST_VERSION"
    print_info "Cargo: $CARGO_VERSION"
}

clone_repository() {
    print_header "Cloning TIME Coin Repository"
    
    # Create projects directory
    su - $SUDO_USER -c "mkdir -p $(dirname $INSTALL_DIR)"
    
    # Remove existing directory if it exists
    if [ -d "$INSTALL_DIR" ]; then
        print_warning "Directory $INSTALL_DIR already exists"
        read -p "Do you want to remove it and clone fresh? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            rm -rf "$INSTALL_DIR"
            print_info "Removed existing directory"
        else
            print_info "Using existing directory"
            return
        fi
    fi
    
    # Clone the repository
    su - $SUDO_USER -c "git clone $REPO_URL $INSTALL_DIR"
    
    print_success "Repository cloned to $INSTALL_DIR"
}

build_project() {
    print_header "Building TIME Coin"
    
    print_info "This may take 5-10 minutes..."
    
    # Build the project
    su - $SUDO_USER -c "cd $INSTALL_DIR && source \$HOME/.cargo/env && cargo build --all --release"
    
    print_success "TIME Coin built successfully"
    
    # Show the binary
    if [ -f "$INSTALL_DIR/target/release/time-node" ]; then
        BINARY_SIZE=$(du -h "$INSTALL_DIR/target/release/time-node" | cut -f1)
        print_info "Binary size: $BINARY_SIZE"
    fi
}

install_binary() {
    print_header "Installing TIME Node Binary"
    
    # Copy binary to system path
    cp "$INSTALL_DIR/target/release/time-node" /usr/local/bin/
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
    
    # Create node directory structure
    su - $SUDO_USER -c "mkdir -p $CONFIG_DIR"
    su - $SUDO_USER -c "mkdir -p $NODE_DIR/data"
    su - $SUDO_USER -c "mkdir -p $NODE_DIR/logs"
    
    # Generate keypair
    print_info "Generating masternode keypair..."
    
    # Create a simple config generator script
    cat > /tmp/generate_config.sh << 'CONFIGEOF'
#!/bin/bash
source $HOME/.cargo/env
cd $HOME/projects/time-coin

# Generate keypair using the crypto library
# This is a placeholder - you may need to create a CLI tool to generate keys
cat > $HOME/time-coin-node/config/testnet.toml << 'EOF'
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

echo "✅ Configuration file created"
CONFIGEOF
    
    chmod +x /tmp/generate_config.sh
    su - $SUDO_USER -c "/tmp/generate_config.sh"
    rm /tmp/generate_config.sh
    
    print_success "Masternode configuration created"
    print_info "Config location: $CONFIG_DIR/testnet.toml"
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
ExecStart=/usr/local/bin/time-node start --config $CONFIG_DIR/testnet.toml
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
        systemctl status ${SERVICE_NAME} --no-pager | head -15
    else
        print_error "Failed to start masternode"
        print_info "Check logs with: journalctl -u ${SERVICE_NAME} -f"
        exit 1
    fi
}

show_summary() {
    print_header "Installation Complete!"
    
    cat << EOF

${GREEN}✅ TIME Coin Masternode Successfully Installed!${NC}

${BLUE}Installation Details:${NC}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Repository:        $INSTALL_DIR
Node Directory:    $NODE_DIR
Configuration:     $CONFIG_DIR/testnet.toml
Binary:            /usr/local/bin/time-node
Service:           ${SERVICE_NAME}.service
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

${BLUE}Next Steps:${NC}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
1. Monitor logs to ensure the node is syncing
2. Generate or import your masternode keys
3. Register your masternode on the network
4. Verify your masternode is active
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
    
    # Run installation steps
    install_system_dependencies
    install_rust
    clone_repository
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
