#!/bin/bash

#############################################################
# TIME Coin Blockchain Reset Script
# 
# This script resets the blockchain to a fresh state with the
# new genesis block dated October 12, 2024.
#
# WARNING: This will erase all blockchain data!
# Wallet data will be preserved.
#
# Usage: 
#   sudo ./scripts/reset-blockchain.sh
#   sudo ./scripts/reset-blockchain.sh --yes
#############################################################

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SERVICE_NAME="timed"

# Detect data directory (Bitcoin-style: ~/.timecoin)
if [ -n "$TIME_COIN_DATA_DIR" ]; then
    DATA_DIR="$TIME_COIN_DATA_DIR"
elif [ -d "$HOME/.timecoin" ]; then
    DATA_DIR="$HOME/.timecoin"
elif [ -d "/var/lib/time-coin" ]; then
    # Legacy path support
    DATA_DIR="/var/lib/time-coin"
    echo -e "${YELLOW}⚠️  Using legacy path: /var/lib/time-coin${NC}"
    echo -e "${YELLOW}⚠️  Consider migrating to: ~/.timecoin${NC}"
else
    DATA_DIR="$HOME/.timecoin"
fi

BLOCKCHAIN_DIR="$DATA_DIR/data/blockchain"
GENESIS_FILE="$DATA_DIR/data/genesis.json"
WALLET_DIR="$DATA_DIR/data/wallets"
LOGS_DIR="$DATA_DIR/logs"
BACKUP_BASE_DIR="/var/backups"
CONFIG_GENESIS="$REPO_DIR/config/genesis-testnet.json"

# Parse command line arguments
SKIP_CONFIRM=false
for arg in "$@"; do
    case $arg in
        --yes|-y)
            SKIP_CONFIRM=true
            shift
            ;;
    esac
done

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

confirm_reset() {
    if [ "$SKIP_CONFIRM" = true ]; then
        print_info "Confirmation skipped (--yes flag provided)"
        return 0
    fi
    
    print_warning "THIS WILL ERASE ALL BLOCKCHAIN DATA!"
    echo -e "${YELLOW}The following will be removed:${NC}"
    echo "  - Blockchain database: $BLOCKCHAIN_DIR"
    echo "  - Genesis file: $GENESIS_FILE"
    echo ""
    echo -e "${GREEN}The following will be preserved:${NC}"
    echo "  - Wallet data: $WALLET_DIR"
    echo "  - Logs: $LOGS_DIR"
    echo ""
    read -p "Are you sure you want to continue? (yes/no): " -r
    echo
    if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
        print_info "Reset cancelled by user"
        exit 0
    fi
}

#############################################################
# Reset Steps
#############################################################

stop_node_service() {
    print_header "Stopping TIME Coin Node Service"
    
    if systemctl is-active --quiet ${SERVICE_NAME}; then
        print_info "Stopping ${SERVICE_NAME} service..."
        systemctl stop ${SERVICE_NAME}
        
        # Wait for service to stop
        local count=0
        while systemctl is-active --quiet ${SERVICE_NAME} && [ $count -lt 10 ]; do
            sleep 1
            count=$((count + 1))
        done
        
        if systemctl is-active --quiet ${SERVICE_NAME}; then
            print_error "Failed to stop service"
            exit 1
        fi
        
        print_success "Service stopped"
    else
        print_info "Service is not running"
    fi
}

create_backup() {
    print_header "Creating Backup"
    
    if [ ! -d "$DATA_DIR" ]; then
        print_info "No data directory exists, skipping backup"
        return
    fi
    
    local timestamp
    timestamp=$(date +%Y%m%d-%H%M%S)
    local backup_dir="$BACKUP_BASE_DIR/time-coin-$timestamp"
    
    print_info "Creating backup at: $backup_dir"
    mkdir -p "$backup_dir"
    
    # Backup blockchain data if it exists
    if [ -d "$BLOCKCHAIN_DIR" ]; then
        print_info "Backing up blockchain data..."
        cp -r "$BLOCKCHAIN_DIR" "$backup_dir/" 2>/dev/null || print_warning "Blockchain directory empty or inaccessible"
    fi
    
    # Backup genesis file if it exists
    if [ -f "$GENESIS_FILE" ]; then
        print_info "Backing up genesis file..."
        cp "$GENESIS_FILE" "$backup_dir/"
    fi
    
    print_success "Backup created: $backup_dir"
}

remove_blockchain_data() {
    print_header "Removing Blockchain Data"
    
    # Remove blockchain directory
    if [ -d "$BLOCKCHAIN_DIR" ]; then
        print_info "Removing blockchain database..."
        rm -rf "$BLOCKCHAIN_DIR"
        print_success "Blockchain database removed"
    else
        print_info "No blockchain database to remove"
    fi
    
    # Remove old genesis file
    if [ -f "$GENESIS_FILE" ]; then
        print_info "Removing old genesis file..."
        rm -f "$GENESIS_FILE"
        print_success "Old genesis file removed"
    else
        print_info "No genesis file to remove"
    fi
}

install_new_genesis() {
    print_header "Installing New Genesis Block"
    
    # Ensure data directory exists
    mkdir -p "$(dirname "$GENESIS_FILE")"
    
    # Check if config genesis file exists
    if [ ! -f "$CONFIG_GENESIS" ]; then
        print_error "Genesis config file not found: $CONFIG_GENESIS"
        exit 1
    fi
    
    # Copy new genesis file
    print_info "Installing genesis block (October 12, 2024)..."
    cp "$CONFIG_GENESIS" "$GENESIS_FILE"
    
    # Verify the file was copied
    if [ -f "$GENESIS_FILE" ]; then
        print_success "New genesis block installed"
        
        # Show genesis info
        if command -v jq &> /dev/null; then
            echo ""
            print_info "Genesis Block Details:"
            jq . "$GENESIS_FILE"
        fi
    else
        print_error "Failed to install genesis file"
        exit 1
    fi
}

update_configuration() {
    print_header "Updating Node Configuration"
    
    # Ensure directories exist
    mkdir -p "$BLOCKCHAIN_DIR"
    mkdir -p "$WALLET_DIR"
    mkdir -p "$LOGS_DIR"
    
    # Set proper permissions
    chown -R "$SUDO_USER":"$SUDO_USER" "$DATA_DIR" 2>/dev/null || true
    
    print_success "Configuration updated"
    print_info "Data directory: $DATA_DIR"
    print_info "Blockchain: $BLOCKCHAIN_DIR"
    print_info "Wallets: $WALLET_DIR"
    print_info "Logs: $LOGS_DIR"
}

start_node_service() {
    print_header "Starting TIME Coin Node Service"
    
    if systemctl list-unit-files | grep -q "^${SERVICE_NAME}.service"; then
        print_info "Starting ${SERVICE_NAME} service..."
        systemctl start ${SERVICE_NAME}
        
        # Wait a moment for it to start
        sleep 2
        
        if systemctl is-active --quiet ${SERVICE_NAME}; then
            print_success "Service started successfully"
            
            echo ""
            print_info "Service Status:"
            systemctl status ${SERVICE_NAME} --no-pager -l | head -15
        else
            print_warning "Service failed to start"
            print_info "Check logs with: journalctl -u ${SERVICE_NAME} -f"
        fi
    else
        print_info "Service ${SERVICE_NAME} is not installed"
        print_info "You will need to start the node manually"
    fi
}

show_summary() {
    print_header "Blockchain Reset Complete!"
    
    cat << EOF

${GREEN}✅ Blockchain Successfully Reset!${NC}

${BLUE}What Was Done:${NC}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ TIME Coin node service stopped
✅ Backup created in $BACKUP_BASE_DIR
✅ Blockchain database cleared
✅ Old genesis file removed
✅ New genesis block installed (October 12, 2024)
✅ Node configuration updated
✅ TIME Coin node service started
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

${BLUE}Data Preserved:${NC}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ Wallet data: $WALLET_DIR
✅ Logs: $LOGS_DIR
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

${BLUE}Useful Commands:${NC}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Check status:      sudo systemctl status ${SERVICE_NAME}
View logs:         sudo journalctl -u ${SERVICE_NAME} -f
Restart node:      sudo systemctl restart ${SERVICE_NAME}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

${GREEN}The blockchain will now rebuild from the October 12, 2024 genesis block!${NC}
${GREEN}Masternode dividends will be paid to registered masternodes after the reset.${NC}

EOF
}

#############################################################
# Main Reset Flow
#############################################################

main() {
    print_header "TIME Coin Blockchain Reset"
    
    # Check if running as root
    check_root
    
    # Confirm reset with user
    confirm_reset
    
    # Run reset steps
    stop_node_service
    create_backup
    remove_blockchain_data
    install_new_genesis
    update_configuration
    start_node_service
    
    # Show summary
    show_summary
}

# Run main reset
main "$@"
