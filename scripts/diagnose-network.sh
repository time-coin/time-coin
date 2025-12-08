#!/bin/bash
# Diagnose network connectivity and peer discovery issues

echo "=== TIME Coin Network Diagnostics ==="
echo ""

# Check if timed is running
echo "1. Service Status:"
systemctl is-active timed && echo "✅ Service is running" || echo "❌ Service is not running"
echo ""

# Check listening ports
echo "2. Listening Ports:"
echo "Expected ports: 24100 (P2P), 24101 (RPC)"
netstat -tlnp | grep -E "24100|24101" || echo "⚠️  No TIME Coin ports listening"
echo ""

# Check firewall
echo "3. Firewall Status (ufw):"
if command -v ufw >/dev/null 2>&1; then
    ufw status | grep -E "24100|24101" || echo "⚠️  Ports may not be open in firewall"
else
    echo "ℹ️  ufw not installed"
fi
echo ""

# Check iptables
echo "4. iptables Rules:"
iptables -L -n | grep -E "24100|24101" || echo "ℹ️  No specific iptables rules for TIME Coin ports"
echo ""

# Test peer discovery URL
echo "5. Peer Discovery URL:"
DISCOVERY_URL="https://time-coin.io/api/peers"
echo "Testing: $DISCOVERY_URL"
if command -v curl >/dev/null 2>&1; then
    curl -s -m 10 "$DISCOVERY_URL" | head -20 || echo "❌ Failed to fetch peers"
else
    echo "⚠️  curl not installed"
fi
echo ""

# Check RPC locally
echo "6. Local RPC Test:"
echo "Testing: http://localhost:24101/api/blockchain/info"
if command -v curl >/dev/null 2>&1; then
    curl -s -m 5 http://localhost:24101/api/blockchain/info || echo "❌ RPC not responding"
else
    echo "⚠️  curl not installed"
fi
echo ""

# Check recent logs
echo "7. Recent Network Logs (last 30 lines):"
journalctl -u timed -n 30 --no-pager | grep -E "peer|sync|network|connect|bootstrap" || echo "No network-related logs found"
echo ""

# Check config
echo "8. Network Configuration:"
CONFIG_FILE="/root/.timecoin/config/testnet.toml"
if [ -f "$CONFIG_FILE" ]; then
    echo "Config file: $CONFIG_FILE"
    grep -A 5 "\[network\]" "$CONFIG_FILE" || echo "⚠️  No [network] section found"
else
    echo "❌ Config file not found: $CONFIG_FILE"
fi
echo ""

# Check external connectivity
echo "9. External IP:"
if command -v curl >/dev/null 2>&1; then
    EXTERNAL_IP=$(curl -s -m 5 ifconfig.me || curl -s -m 5 icanhazip.com || echo "unknown")
    echo "External IP: $EXTERNAL_IP"
else
    echo "⚠️  curl not installed"
fi
echo ""

# Test connection to known peer (if any)
echo "10. Testing Connection to time-coin.io:"
if command -v nc >/dev/null 2>&1; then
    timeout 5 nc -zv time-coin.io 443 2>&1 || echo "⚠️  Cannot reach time-coin.io"
else
    ping -c 3 time-coin.io || echo "⚠️  Cannot ping time-coin.io"
fi
echo ""

echo "=== Diagnostic Complete ==="
echo ""
echo "Common issues:"
echo "  • Firewall blocking ports 24100/24101"
echo "  • Peer discovery URL unreachable"
echo "  • No bootstrap nodes configured"
echo "  • External address not set correctly"
