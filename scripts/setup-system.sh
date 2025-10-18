#!/bin/bash

#############################################################
# TIME Coin System Setup Script
# 
# This script prepares your Ubuntu server for TIME Coin by:
# - Installing system dependencies
# - Installing Rust
# - Cloning the TIME Coin repository
#
# Run this ONCE on a fresh server, then use install-masternode.sh
#
# Usage: 
#   curl -O https://raw.githubusercontent.com/time-coin/time-coin/main/scripts/setup-system.sh
#   chmod +x setup-system.sh
#   sudo ./setup-system.sh
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
INSTALL_DIR="$HOME/time-coin"

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
    if su - $SUDO_USER -c "command -v rustc" &> /dev/null; then
        RUST_VERSION=$(su - $SUDO_USER -c "rustc --version")
        print_info "Rust is already installed: $RUST_VERSION"
        read -p "Do you want to update it? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            su - $SUDO_USER -c "rustup update"
            print_success "Rust updated"
        fi
    else
        # Install Rust as the non-root user
        print_info "Installing Rust (this may take a minute)..."
        su - $SUDO_USER -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
        
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
    
    # Check if directory already exists
    if [ -d "$INSTALL_DIR" ]; then
        print_warning "Directory $INSTALL_DIR already exists"
        print_info "Skipping clone. Use 'git pull' to update or remove the directory to clone fresh."
        return
    fi
    
    # Clone the repository
    print_info "Cloning repository..."
    su - $SUDO_USER -c "git clone $REPO_URL $INSTALL_DIR"
    
    print_success "Repository cloned to $INSTALL_DIR"
}

show_summary() {
    print_header "System Setup Complete!"
    
    cat << EOF

${GREEN}✅ System Successfully Prepared for TIME Coin!${NC}

${BLUE}What Was Installed:${NC}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ System dependencies (gcc, git, curl, etc.)
✅ Rust toolchain ($(su - $SUDO_USER -c "source \$HOME/.cargo/env && rustc --version | cut -d' ' -f2"))
✅ TIME Coin repository → $INSTALL_DIR
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

${BLUE}Next Steps:${NC}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
1. Run the masternode installation script:
   
   cd $INSTALL_DIR
   sudo ./scripts/install-masternode.sh

2. Or build manually:
   
   cd $INSTALL_DIR
   source \$HOME/.cargo/env
   cargo build --all --release

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

${GREEN}Ready to install your masternode! 🚀${NC}

EOF
}

#############################################################
# Main Installation Flow
#############################################################

main() {
    print_header "TIME Coin System Setup"
    
    # Check if running as root
    check_root
    
    # Run installation steps
    install_system_dependencies
    install_rust
    clone_repository
    
    # Show summary
    show_summary
}

# Run main installation
main "$@"
