#!/bin/bash
# Protocol Compatibility Verification Script
# Checks if all masternodes are using compatible protocol versions

echo "════════════════════════════════════════════════════════"
echo "  TIME Coin Protocol Compatibility Check"
echo "════════════════════════════════════════════════════════"
echo ""

# List of masternodes to check
NODES=(
    "134.199.175.106:24101"
    "161.35.129.70:24101"
    "165.232.154.150:24101"
    "178.128.199.144:24101"
    "50.28.104.50:24101"
    "69.167.168.176:24101"
)

# Expected values
EXPECTED_PROTOCOL_VERSION=1
EXPECTED_NETWORK="testnet"
EXPECTED_MAGIC_BYTES="7E 57 7E 4D"  # TEST TIME

echo "🔍 Expected Protocol Configuration:"
echo "   Protocol Version: $EXPECTED_PROTOCOL_VERSION"
echo "   Network: $EXPECTED_NETWORK"
echo "   Magic Bytes: $EXPECTED_MAGIC_BYTES"
echo ""

INCOMPATIBLE_COUNT=0
UNREACHABLE_COUNT=0
COMPATIBLE_COUNT=0

echo "📊 Checking ${#NODES[@]} masternodes..."
echo ""

for node in "${NODES[@]}"; do
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "🔍 Checking: $node"
    
    # Get node info via RPC
    response=$(curl -s --connect-timeout 5 "http://$node/rpc/getinfo" 2>/dev/null)
    
    if [ $? -ne 0 ] || [ -z "$response" ]; then
        echo "   ❌ UNREACHABLE - Cannot connect to node"
        ((UNREACHABLE_COUNT++))
        continue
    fi
    
    # Extract version info
    version=$(echo "$response" | jq -r '.version' 2>/dev/null)
    network=$(echo "$response" | jq -r '.network' 2>/dev/null)
    commit=$(echo "$response" | jq -r '.commit' 2>/dev/null)
    
    if [ -z "$version" ] || [ "$version" == "null" ]; then
        echo "   ❌ INCOMPATIBLE - Cannot parse version info"
        echo "   Response: $response"
        ((INCOMPATIBLE_COUNT++))
        continue
    fi
    
    echo "   📦 Version: $version"
    echo "   🌐 Network: $network"
    echo "   📝 Commit: $commit"
    
    # Check network
    if [ "$network" != "$EXPECTED_NETWORK" ]; then
        echo "   ❌ NETWORK MISMATCH - Expected $EXPECTED_NETWORK, got $network"
        ((INCOMPATIBLE_COUNT++))
        continue
    fi
    
    # Extract commit hash from version (format: 0.1.0-COMMIT)
    node_commit=$(echo "$version" | cut -d'-' -f2)
    
    # Check if commit is recent (should be 345b616 or later for all fixes)
    REQUIRED_COMMIT="345b616"
    
    if [ "$commit" != "$REQUIRED_COMMIT" ] && [[ ! "$version" =~ "$REQUIRED_COMMIT" ]]; then
        echo "   ⚠️  OLD VERSION - Expected $REQUIRED_COMMIT or later"
        echo "   ℹ️  This node may not have latest protocol fixes"
        ((INCOMPATIBLE_COUNT++))
    else
        echo "   ✅ COMPATIBLE - Protocol version and network match"
        ((COMPATIBLE_COUNT++))
    fi
    
    echo ""
done

echo "════════════════════════════════════════════════════════"
echo "📊 Summary:"
echo "   ✅ Compatible: $COMPATIBLE_COUNT"
echo "   ⚠️  Incompatible/Old: $INCOMPATIBLE_COUNT"
echo "   ❌ Unreachable: $UNREACHABLE_COUNT"
echo ""

if [ $COMPATIBLE_COUNT -eq ${#NODES[@]} ]; then
    echo "✅ ALL NODES COMPATIBLE!"
    echo "   All masternodes are using compatible protocol versions."
    exit 0
elif [ $INCOMPATIBLE_COUNT -gt 0 ]; then
    echo "⚠️  PROTOCOL MISMATCH DETECTED!"
    echo ""
    echo "   $INCOMPATIBLE_COUNT node(s) have incompatible versions."
    echo "   This will prevent consensus from working properly."
    echo ""
    echo "   Action Required:"
    echo "   1. Update all nodes to latest version (345b616 or later)"
    echo "   2. Run: sudo ./scripts/update-node.sh on each node"
    echo "   3. Verify with: ./scripts/verify-protocol.sh"
    echo ""
    exit 1
else
    echo "❌ CONNECTIVITY ISSUES"
    echo ""
    echo "   $UNREACHABLE_COUNT node(s) could not be reached."
    echo "   Check network connectivity and firewall rules."
    echo ""
    exit 2
fi
