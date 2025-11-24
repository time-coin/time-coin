#!/bin/bash
#
# mint.sh - Mint testnet coins using node.json wallet
#
# Usage: ./mint.sh <amount_in_TIME>
# Example: ./mint.sh 1000    # Mints 1000 TIME coins
#

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Check if amount provided
if [ $# -lt 1 ]; then
    echo -e "${RED}Error: Amount required${NC}"
    echo "Usage: $0 <amount_in_TIME>"
    echo "Example: $0 1000"
    exit 1
fi

AMOUNT_TIME=$1

# Convert TIME to smallest unit (1 TIME = 1,000,000 units)
AMOUNT_UNITS=$((AMOUNT_TIME * 1000000))

# Default paths
NODE_JSON="/var/lib/time-coin/wallets/node.json"
API_URL="http://localhost:24101"
NETWORK="testnet"

# Allow override via environment variables
NODE_JSON=${NODE_JSON:-"/var/lib/time-coin/wallets/node.json"}
API_URL=${API_URL:-"http://localhost:24101"}

echo -e "${GREEN}TIME Coin Testnet Minting Tool${NC}"
echo "================================"
echo ""

# Check if node.json exists
if [ ! -f "$NODE_JSON" ]; then
    echo -e "${RED}Error: node.json not found at $NODE_JSON${NC}"
    echo "Please specify the path with: NODE_JSON=/path/to/node.json $0 $AMOUNT_TIME"
    exit 1
fi

echo "Reading wallet from: $NODE_JSON"

# Extract private key bytes and convert to hex
PRIVATE_KEY_HEX=$(python3 -c "
import json
with open('$NODE_JSON', 'r') as f:
    data = json.load(f)
    signing_key = data['keypair']['signing_key']
    print(''.join(f'{b:02x}' for b in signing_key))
")

# Extract address bytes and convert to hex  
ADDRESS_HEX=$(python3 -c "
import json
with open('$NODE_JSON', 'r') as f:
    data = json.load(f)
    addr_bytes = data['address']['bytes']
    print(''.join(f'{b:02x}' for b in addr_bytes))
")

# Create TIME address with proper encoding
# For testnet, addresses start with TIME0
# We'll use the hex address for now - the node should handle it
RECIPIENT_ADDRESS="TIME0$ADDRESS_HEX"

echo "Private Key: ${PRIVATE_KEY_HEX:0:16}...${PRIVATE_KEY_HEX: -16}"
echo "Address: $RECIPIENT_ADDRESS"
echo "Amount: $AMOUNT_TIME TIME ($AMOUNT_UNITS units)"
echo "Network: $NETWORK"
echo "API URL: $API_URL"
echo ""

# Check if in time-coin repo
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Must run from time-coin repository root${NC}"
    exit 1
fi

echo -e "${YELLOW}Building tx-perf-test tool...${NC}"
cargo build --release -p tx-perf-test 2>&1 | grep -E "(Compiling|Finished)" || true
echo ""

echo -e "${GREEN}Minting $AMOUNT_TIME TIME coins...${NC}"
echo ""

# Run the mint command
./target/release/tx-perf-test \
    --network "$NETWORK" \
    --api-url "$API_URL" \
    --private-key "$PRIVATE_KEY_HEX" \
    --recipient "$RECIPIENT_ADDRESS" \
    --mint-coins "$AMOUNT_UNITS" \
    --tx-count 0

echo ""
echo -e "${GREEN}✅ Mint transaction submitted!${NC}"
echo ""
echo "Check your balance with:"
echo "  time-cli balance"
echo ""
echo "Or view transaction in logs:"
echo "  journalctl -u timed -f"
