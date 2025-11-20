#!/bin/bash
# Quarantine and Reset Script for Controlled Nodes
# This script blocks corrupted nodes and resets blockchain on clean nodes

set -e

CONTROLLED_NODES=(
    "69.167.168.176"
    "134.199.175.106"
    "161.35.129.70"
    "50.28.104.50"
)

BAD_NODES=(
    "178.128.199.144"
    "165.232.154.150"
)

echo "==================================================================="
echo "TIME COIN NETWORK QUARANTINE & RESET"
echo "==================================================================="
echo ""
echo "This will:"
echo "  1. Block corrupted nodes via firewall (${BAD_NODES[*]})"
echo "  2. Reset blockchain to genesis on clean nodes"
echo "  3. Prepare network for midnight block 40 creation"
echo ""
read -p "Continue? (yes/no): " -r
if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
    echo "Aborted."
    exit 0
fi

# Step 1: Quarantine bad nodes via firewall
echo ""
echo "STEP 1: Implementing firewall quarantine..."
for node in "${CONTROLLED_NODES[@]}"; do
    echo "  → Quarantining on $node..."
    for bad_node in "${BAD_NODES[@]}"; do
        ssh root@$node "iptables -A INPUT -s $bad_node -j DROP 2>/dev/null || true"
        ssh root@$node "iptables -A OUTPUT -d $bad_node -j DROP 2>/dev/null || true"
    done
    ssh root@$node "netfilter-persistent save 2>/dev/null || iptables-save > /etc/iptables/rules.v4 || true"
    echo "  ✓ $node quarantine complete"
done

echo ""
echo "STEP 2: Waiting 10 seconds for quarantine to take effect..."
sleep 10

# Step 2: Reset blockchain on controlled nodes
echo ""
echo "STEP 3: Resetting blockchain on controlled nodes..."
for node in "${CONTROLLED_NODES[@]}"; do
    echo "  → Resetting $node..."
    ssh root@$node "cd /root/time-coin && ./scripts/reset-blockchain.sh --yes" || echo "    ⚠️  Reset failed on $node"
    echo "  ✓ $node reset complete"
    sleep 2
done

echo ""
echo "==================================================================="
echo "✅ QUARANTINE & RESET COMPLETE"
echo "==================================================================="
echo ""
echo "Your 4 nodes are now isolated from corrupted nodes."
echo "Clean blockchain will start at midnight UTC (block 40)."
echo ""
echo "Next steps:"
echo "1. Monitor: ssh 69.167.168.176 'journalctl -u timed -f'"
echo "2. Wait for midnight: $(date -u -d '2025-11-21 00:00:00' '+%Y-%m-%d %H:%M:%S %Z' 2>/dev/null || echo '2025-11-21 00:00:00 UTC')"
echo "3. Verify block 40 creation with proper BFT consensus"
echo ""
echo "To verify quarantine is working:"
echo "  ssh 69.167.168.176 'iptables -L -n | grep -E \"178.128.199.144|165.232.154.150\"'"
echo ""
