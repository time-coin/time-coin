#!/bin/bash
# Reset Testnet Blockchain on All Nodes
# This clears corrupted blockchain data and forces all nodes to start fresh with genesis

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Testnet nodes
NODES=(
    "root@50.28.104.50"
    "root@161.35.129.70"
    "root@165.232.154.150"
    "root@178.128.199.144"
    "root@216.198.79.65"
    "root@64.29.17.65"
    "root@69.167.168.176"
)

echo -e "${CYAN}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║        TIME Coin Testnet Blockchain Reset             ║${NC}"
echo -e "${CYAN}╚════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}⚠️  WARNING: This will delete all blockchain data!${NC}"
echo -e "${YELLOW}   All nodes will restart from genesis block.${NC}"
echo ""
echo -e "Nodes to reset:"
for node in "${NODES[@]}"; do
    echo -e "  • ${node}"
done
echo ""
read -p "Continue? (yes/no): " confirm

if [ "$confirm" != "yes" ]; then
    echo -e "${RED}Aborted.${NC}"
    exit 1
fi

echo ""
echo -e "${CYAN}Starting testnet reset...${NC}"
echo ""

# Track success/failure
SUCCESS=()
FAILED=()

for node in "${NODES[@]}"; do
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}Resetting: ${node}${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    
    if ssh -o ConnectTimeout=5 "$node" "
        echo '  Stopping timed service...' &&
        systemctl stop timed &&
        echo '  Removing blockchain data...' &&
        rm -rf /var/lib/time-coin/blockchain/* &&
        echo '  Starting timed service...' &&
        systemctl start timed &&
        sleep 2 &&
        echo '  Checking status...' &&
        systemctl is-active timed --quiet
    " 2>/dev/null; then
        echo -e "${GREEN}✓ ${node} reset successfully${NC}"
        SUCCESS+=("$node")
    else
        echo -e "${RED}✗ ${node} reset failed${NC}"
        FAILED+=("$node")
    fi
    echo ""
done

echo -e "${CYAN}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║                    Reset Summary                       ║${NC}"
echo -e "${CYAN}╚════════════════════════════════════════════════════════╝${NC}"
echo ""

if [ ${#SUCCESS[@]} -gt 0 ]; then
    echo -e "${GREEN}✓ Successfully reset (${#SUCCESS[@]}/${#NODES[@]}):${NC}"
    for node in "${SUCCESS[@]}"; do
        echo -e "  • ${node}"
    done
    echo ""
fi

if [ ${#FAILED[@]} -gt 0 ]; then
    echo -e "${RED}✗ Failed to reset (${#FAILED[@]}/${#NODES[@]}):${NC}"
    for node in "${FAILED[@]}"; do
        echo -e "  • ${node}"
    done
    echo ""
    echo -e "${YELLOW}⚠️  Manual intervention required for failed nodes${NC}"
    echo -e "${YELLOW}   Run on each failed node:${NC}"
    echo -e "     systemctl stop timed"
    echo -e "     rm -rf /var/lib/time-coin/blockchain/*"
    echo -e "     systemctl start timed"
    echo ""
fi

echo -e "${CYAN}Next steps:${NC}"
echo -e "  1. Wait 30 seconds for all nodes to connect"
echo -e "  2. Check node status with: ${YELLOW}journalctl -u timed -f${NC}"
echo -e "  3. Verify blockchain sync across nodes"
echo ""
echo -e "${GREEN}Testnet reset complete!${NC}"
