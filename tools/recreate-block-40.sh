#!/bin/bash
# Manually trigger block 40 recreation
# This script calls the block producer endpoint to create a new block

NODE_IP="${1:-69.167.168.176}"
API_PORT="24101"

echo "🔧 Triggering block production on node $NODE_IP"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# First, check current blockchain height
echo "📊 Checking current blockchain height..."
CURRENT_HEIGHT=$(curl -s "http://$NODE_IP:$API_PORT/blockchain/info" | jq -r '.height')
echo "   Current height: $CURRENT_HEIGHT"

if [ "$CURRENT_HEIGHT" == "39" ]; then
    echo "✅ At height 39 - ready to create block 40"
else
    echo "⚠️  Warning: Expected height 39, got $CURRENT_HEIGHT"
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Delete block 40 from all nodes first (if it exists)
echo ""
echo "🗑️  Cleaning up invalid block 40 from network..."
for PEER_IP in "161.35.129.70" "178.128.199.144" "165.232.154.150" "134.199.175.106" "69.167.168.176"; do
    echo "   Cleaning $PEER_IP..."
    ssh root@$(echo $PEER_IP | tr '.' '-' | sed 's/^/node-/').time-coin.local "systemctl stop timed && cd /var/lib/time-coin/blockchain && rm -f */block:40 && systemctl start timed" 2>/dev/null &
done

wait
sleep 3

echo ""
echo "🔧 All nodes cleaned - they will now recreate block 40 at midnight"
echo "⏰ Next midnight UTC: $(date -u -d 'tomorrow 00:00' '+%Y-%m-%d %H:%M:%S')"
echo ""
echo "✅ Done! Monitor logs with: journalctl -u timed -f"
