#!/bin/bash
# Protocol Compatibility Verification Script
# Checks if all masternodes are using compatible TCP connections

echo "════════════════════════════════════════════════════════"
echo "  TIME Coin Protocol Compatibility Check (TCP)"
echo "════════════════════════════════════════════════════════"
echo ""

# List of masternodes to check
NODES=(
    "134.199.175.106"
    "161.35.129.70"
    "165.232.154.150"
    "178.128.199.144"
    "50.28.104.50"
    "69.167.168.176"
)

# Expected values
EXPECTED_PROTOCOL_VERSION=1
EXPECTED_NETWORK="testnet"
EXPECTED_MAGIC_BYTES="7E 57 7E 4D"  # TEST TIME
TCP_PORT=24100

echo "🔍 Expected Protocol Configuration:"
echo "   Protocol Version: $EXPECTED_PROTOCOL_VERSION"
echo "   Network: $EXPECTED_NETWORK"
echo "   Magic Bytes: $EXPECTED_MAGIC_BYTES"
echo "   TCP Port: $TCP_PORT"
echo ""

REACHABLE_COUNT=0
UNREACHABLE_COUNT=0

echo "📊 Checking ${#NODES[@]} masternodes (TCP connectivity)..."
echo ""

for node in "${NODES[@]}"; do
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "🔍 Checking: $node:$TCP_PORT"
    
    # Test TCP connectivity with timeout
    if timeout 3 bash -c "</dev/tcp/$node/$TCP_PORT" 2>/dev/null; then
        echo "   ✅ TCP PORT $TCP_PORT OPEN"
        ((REACHABLE_COUNT++))
        
        # Also check API for version info (optional, for information only)
        api_response=$(curl -s --connect-timeout 2 "http://$node:24101/rpc/getinfo" 2>/dev/null)
        if [ $? -eq 0 ] && [ -n "$api_response" ]; then
            version=$(echo "$api_response" | jq -r '.version' 2>/dev/null)
            if [ -n "$version" ] && [ "$version" != "null" ]; then
                echo "   📦 Version: $version (via HTTP API)"
            fi
        fi
    else
        echo "   ❌ TCP PORT $TCP_PORT CLOSED/FILTERED"
        echo "   🔥 Check firewall: sudo ufw allow $TCP_PORT/tcp"
        ((UNREACHABLE_COUNT++))
    fi
    
    echo ""
done

echo "════════════════════════════════════════════════════════"
echo "📊 Summary:"
echo "   ✅ TCP Reachable: $REACHABLE_COUNT"
echo "   ❌ TCP Unreachable: $UNREACHABLE_COUNT"
echo ""

if [ $REACHABLE_COUNT -eq ${#NODES[@]} ]; then
    echo "✅ ALL NODES TCP REACHABLE!"
    echo "   All masternodes can communicate via TCP port $TCP_PORT."
    echo ""
    echo "   Next steps:"
    echo "   1. Ensure all nodes are updated to latest version"
    echo "   2. Run: sudo ./scripts/update-node.sh on each node"
    echo "   3. Monitor consensus with: sudo journalctl -u timed -f"
    echo ""
    exit 0
elif [ $UNREACHABLE_COUNT -gt 0 ]; then
    echo "⚠️  TCP CONNECTIVITY ISSUES DETECTED!"
    echo ""
    echo "   $UNREACHABLE_COUNT node(s) cannot be reached on TCP port $TCP_PORT."
    echo "   Consensus requires TCP communication, NOT HTTP API."
    echo ""
    echo "   Action Required (on each unreachable node):"
    echo "   1. Open firewall: sudo ufw allow $TCP_PORT/tcp"
    echo "   2. Check service: sudo systemctl status timed"
    echo "   3. Verify listening: sudo netstat -tlnp | grep $TCP_PORT"
    echo "   4. Restart if needed: sudo systemctl restart timed"
    echo ""
    exit 1
else
    echo "❌ NO NODES REACHABLE"
    echo ""
    echo "   Could not reach any nodes on TCP port $TCP_PORT."
    echo "   Check your network configuration and firewall rules."
    echo ""
    exit 2
fi
