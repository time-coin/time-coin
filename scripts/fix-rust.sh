#!/bin/bash

#############################################################
# Fix Rust Installation
# 
# This script removes apt-installed cargo and installs
# proper Rust toolchain using rustup.
#
# Usage: 
#   sudo ./fix-rust.sh
#############################################################

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

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

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

check_root() {
    if [[ $EUID -ne 0 ]]; then
        print_error "This script must be run as root or with sudo"
        exit 1
    fi
}

main() {
    print_header "Fixing Rust Installation"
    
    check_root
    
    # Remove apt-installed cargo/rust if present
    print_info "Removing apt-installed cargo/rust packages..."
    apt remove -y cargo rustc || true
    apt autoremove -y || true
    
    print_success "Old packages removed"
    
    # Install rustup properly
    print_header "Installing Rust via rustup"
    
    # Install for root user (since timed runs as root)
    if command -v rustup &> /dev/null; then
        print_info "Rustup already installed, updating..."
        rustup update stable
    else
        print_info "Installing rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    fi
    
    # Source cargo environment
    export PATH="$HOME/.cargo/bin:$PATH"
    [ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"
    
    # Install components
    print_info "Installing Rust components..."
    rustup component add rustfmt clippy
    
    # Verify installation
    print_header "Verification"
    
    if command -v cargo &> /dev/null && command -v rustc &> /dev/null; then
        print_success "Rust installed successfully!"
        print_info "Rustc version: $(rustc --version)"
        print_info "Cargo version: $(cargo --version)"
        print_info "Rustup version: $(rustup --version)"
    else
        print_error "Installation verification failed"
        exit 1
    fi
    
    print_header "Complete!"
    echo ""
    echo -e "${GREEN}✅ Rust has been properly installed via rustup${NC}"
    echo ""
    echo -e "${BLUE}Next steps:${NC}"
    echo "1. Run: source ~/.cargo/env"
    echo "2. Run: ./update.sh"
    echo ""
}

main "$@"
