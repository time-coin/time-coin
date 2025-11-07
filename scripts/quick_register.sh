#!/bin/bash

# Quick Masternode Registration for TIME Coin
# Run this after your node is running

API="http://localhost:24101"

echo "╔════════════════════════════════════════════════════╗"
echo "║   TIME Coin - Quick Masternode Registration       ║"
echo "╚════════════════════════════════════════════════════╝"
echo ""

# Check API
if ! curl -s -f "$API/blockchain/info" > /dev/null 2>&1; then
    echo "❌ Cannot connect to API at $API"
    echo "   Make sure your node is running"
    exit 1
fi

echo "✅ API is reachable"
echo ""

# Get current peers
PEERS=$(curl -s "$API/peers" | jq -r '.peers[].address' | cut -d':' -f1)

if [ -z "$PEERS" ]; then
    echo "⚠️  No peers connected yet"
    echo "   Wait for peers to connect, then run this script again"
    exit 0
fi

echo "📋 Connected peers:"
echo "$PEERS" | while read peer; do echo "  • $peer"; done
echo ""

# Generate wallets for each peer (for testing)
echo "🔑 Generating wallet addresses (FOR TESTING ONLY)..."
echo ""

for PEER_IP in $PEERS; do
    WALLET="TIME1$(echo -n "$PEER_IP" | sha256sum | cut -c1-40)"
    
    echo "Registering: $PEER_IP"
    echo "  Wallet: $WALLET"
    
    RESPONSE=$(curl -s -X POST "$API/masternode/register" \
        -H "Content-Type: application/json" \
        -d "{\"node_ip\":\"$PEER_IP\",\"wallet_address\":\"$WALLET\",\"tier\":\"Free\"}")
    
    if echo "$RESPONSE" | jq -e '.success' > /dev/null 2>&1; then
        echo "  ✅ Registered"
    else
        echo "  ❌ Failed: $(echo $RESPONSE | jq -r '.message // .error')"
    fi
    echo ""
done

echo "═══════════════════════════════════════════════════"
echo "📊 Current Masternodes:"
curl -s "$API/masternodes/list" | jq -r '.masternodes[] | "  • \(.node_ip) → \(.wallet_address)"'
echo ""

MN_COUNT=$(curl -s "$API/masternodes/list" | jq -r '.count')
echo "✅ Total registered: $MN_COUNT masternodes"
echo ""
echo "⏰ Rewards will start with the next block (midnight UTC)"
echo "   Current UTC: $(date -u '+%Y-%m-%d %H:%M:%S')"
echo ""