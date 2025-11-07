#!/bin/bash

# TIME Coin Masternode Registration Script
# Registers all 4 peer nodes as Free tier masternodes

API="http://localhost:24101"

echo "╔════════════════════════════════════════════════════╗"
echo "║   TIME Coin Masternode Registration               ║"
echo "╚════════════════════════════════════════════════════╝"
echo ""
echo "Date: $(date -u '+%Y-%m-%d %H:%M:%S UTC')"
echo "User: $USER"
echo ""

# Check API connectivity
echo "🔍 Checking API connection..."
if ! curl -s -f "$API/blockchain/info" > /dev/null 2>&1; then
    echo "❌ Cannot connect to API at $API"
    echo "   Make sure your TIME Coin node is running."
    exit 1
fi
echo "✅ API is reachable"
echo ""

# Check current block height
CURRENT_HEIGHT=$(curl -s "$API/blockchain/info" | jq -r '.height')
echo "📊 Current blockchain height: $CURRENT_HEIGHT"
echo ""

# Generate wallet addresses (deterministic for testing)
# In production, use proper wallet generation!
WALLET1="TIME1$(echo -n '134.199.175.106' | sha256sum | cut -c1-40)"
WALLET2="TIME1$(echo -n '178.128.199.144' | sha256sum | cut -c1-40)"
WALLET3="TIME1$(echo -n '50.28.104.50' | sha256sum | cut -c1-40)"
WALLET4="TIME1$(echo -n '165.232.154.150' | sha256sum | cut -c1-40)"

echo "📝 Generated wallet addresses:"
echo "  Node 1 (134.199.175.106): $WALLET1"
echo "  Node 2 (178.128.199.144): $WALLET2"
echo "  Node 3 (50.28.104.50):    $WALLET3"
echo "  Node 4 (165.232.154.150): $WALLET4"
echo ""

# Register each masternode
echo "═══════════════════════════════════════════════════"
echo "  Registering Masternodes"
echo "═══════════════════════════════════════════════════"
echo ""

# Node 1
echo "📝 Registering Node 1: 134.199.175.106"
RESPONSE1=$(curl -s -X POST "$API/masternode/register" \
  -H "Content-Type: application/json" \
  -d "{
    \"node_ip\": \"134.199.175.106\",
    \"wallet_address\": \"$WALLET1\",
    \"tier\": \"Free\"
  }")
echo "$RESPONSE1" | jq
echo ""

# Node 2
echo "📝 Registering Node 2: 178.128.199.144"
RESPONSE2=$(curl -s -X POST "$API/masternode/register" \
  -H "Content-Type: application/json" \
  -d "{
    \"node_ip\": \"178.128.199.144\",
    \"wallet_address\": \"$WALLET2\",
    \"tier\": \"Free\"
  }")
echo "$RESPONSE2" | jq
echo ""

# Node 3
echo "📝 Registering Node 3: 50.28.104.50"
RESPONSE3=$(curl -s -X POST "$API/masternode/register" \
  -H "Content-Type: application/json" \
  -d "{
    \"node_ip\": \"50.28.104.50\",
    \"wallet_address\": \"$WALLET3\",
    \"tier\": \"Free\"
  }")
echo "$RESPONSE3" | jq
echo ""

# Node 4
echo "📝 Registering Node 4: 165.232.154.150"
RESPONSE4=$(curl -s -X POST "$API/masternode/register" \
  -H "Content-Type: application/json" \
  -d "{
    \"node_ip\": \"165.232.154.150\",
    \"wallet_address\": \"$WALLET4\",
    \"tier\": \"Free\"
  }")
echo "$RESPONSE4" | jq
echo ""

# Verification
echo "═══════════════════════════════════════════════════"
echo "  Verification"
echo "═══════════════════════════════════════════════════"
echo ""

echo "📊 Listing all registered masternodes:"
MASTERNODES=$(curl -s "$API/masternodes/list")
echo "$MASTERNODES" | jq
echo ""

MN_COUNT=$(echo "$MASTERNODES" | jq -r '.count')
echo "✅ Total registered masternodes: $MN_COUNT"
echo ""

# Next block calculation
CURRENT_UTC=$(date -u '+%H:%M:%S')
CURRENT_HOUR=$(date -u '+%H')
CURRENT_MIN=$(date -u '+%M')
CURRENT_SEC=$(date -u '+%S')

# Calculate seconds until midnight UTC
SECONDS_IN_DAY=86400
SECONDS_SINCE_MIDNIGHT=$((10#$CURRENT_HOUR * 3600 + 10#$CURRENT_MIN * 60 + 10#$CURRENT_SEC))
SECONDS_UNTIL_MIDNIGHT=$((SECONDS_IN_DAY - SECONDS_SINCE_MIDNIGHT))

HOURS_LEFT=$((SECONDS_UNTIL_MIDNIGHT / 3600))
MINS_LEFT=$(((SECONDS_UNTIL_MIDNIGHT % 3600) / 60))

echo "═══════════════════════════════════════════════════"
echo "  Next Steps"
echo "═══════════════════════════════════════════════════"
echo ""
echo "🕐 Current UTC time: $CURRENT_UTC"
echo "⏰ Next block in: ${HOURS_LEFT}h ${MINS_LEFT}m (at midnight UTC)"
echo ""
echo "💰 Expected rewards per block (4 masternodes):"
echo "   • Treasury: 5.00 TIME"
echo "   • Each masternode: 23.75 TIME"
echo "   • Total per block: 100 TIME"
echo ""
echo "📊 Check rewards after block $((CURRENT_HEIGHT + 1)):"
echo "   curl $API/blockchain/block/$((CURRENT_HEIGHT + 1)) | jq '.block.transactions[0].outputs'"
echo ""
echo "💵 Check individual balances:"
echo "   curl $API/balance/$WALLET1"
echo "   curl $API/balance/$WALLET2"
echo "   curl $API/balance/$WALLET3"
echo "   curl $API/balance/$WALLET4"
echo ""
echo "✅ Registration complete!"