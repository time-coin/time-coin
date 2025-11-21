#!/bin/bash
# Force immediate block creation (simulates midnight trigger)
# Run this on the block producer node

echo "🚀 Forcing immediate block creation"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Get node info
NODE_IP=$(curl -s https://api.ipify.org)
echo "Node IP: $NODE_IP"

# Check if we're the leader
API_PORT="24101"
HEIGHT=$(curl -s "http://localhost:$API_PORT/blockchain/info" | jq -r '.height')
echo "Current height: $HEIGHT"

NEXT_HEIGHT=$((HEIGHT + 1))
echo "Next block: $NEXT_HEIGHT"

# The block producer runs automatically at midnight UTC
# To force immediate creation, we would need to either:
# 1. Restart the service at midnight
# 2. Add an API endpoint to trigger manual block creation
# 3. Wait for the next scheduled midnight

echo ""
echo "⏰ Waiting for next midnight UTC to create block $NEXT_HEIGHT"
echo ""
echo "To manually trigger block creation NOW:"
echo "1. Stop the service: systemctl stop timed"  
echo "2. Set system time to midnight: timedatectl set-time '00:00:00'"
echo "3. Start service: systemctl start timed"
echo "4. Wait 30 seconds for block creation"
echo "5. Restore time: systemctl set-ntp true"
echo ""
echo "⚠️  WARNING: Time manipulation can cause issues!"
echo ""
