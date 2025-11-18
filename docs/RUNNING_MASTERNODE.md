# Running TIME Coin Masternode with WebSocket Support

## 🚀 Quick Start

### Build the Masternode

```bash
cargo build --release -p time-masternode
```

### Run the Masternode

```bash
./target/release/time-masternode
```

By default, the WebSocket server listens on `0.0.0.0:8765`.

### Custom WebSocket Port

```bash
WS_ADDR="0.0.0.0:9000" ./target/release/time-masternode
```

## 🌐 WebSocket Protocol

The masternode exposes a WebSocket server that wallets connect to for real-time updates.

### Connection

```
ws://masternode-ip:8765
```

### Message Types

#### Client → Masternode

**Subscribe to address updates**:
```json
{
  "type": "Subscribe",
  "data": {
    "addresses": ["time1abc...", "time1xyz..."]
  }
}
```

**Unsubscribe**:
```json
{
  "type": "Unsubscribe",
  "data": {
    "addresses": ["time1abc..."]
  }
}
```

**Ping** (heartbeat):
```json
{
  "type": "Ping"
}
```

#### Masternode → Client

**UTXO State Change**:
```json
{
  "type": "UtxoStateChange",
  "data": {
    "outpoint": {
      "txid": "abc123...",
      "vout": 0
    },
    "state": "SpentFinalized",
    "transaction": {...}
  }
}
```

**Transaction Pending**:
```json
{
  "type": "TransactionPending",
  "data": {
    "tx": {...}
  }
}
```

**Transaction Finalized** (Instant Finality achieved):
```json
{
  "type": "TransactionFinalized",
  "data": {
    "txid": "abc123...",
    "block_height": 12345
  }
}
```

**New Block**:
```json
{
  "type": "NewBlock",
  "data": {
    "height": 12346,
    "hash": "def456..."
  }
}
```

**Pong** (response to Ping):
```json
{
  "type": "Pong"
}
```

## 🔧 Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `WS_ADDR` | `0.0.0.0:8765` | WebSocket server bind address |

### Firewall Rules

Ensure port 8765 (or your custom port) is open for incoming WebSocket connections:

```bash
# UFW (Ubuntu/Debian)
sudo ufw allow 8765/tcp

# firewalld (RHEL/CentOS)
sudo firewall-cmd --permanent --add-port=8765/tcp
sudo firewall-cmd --reload

# iptables
sudo iptables -A INPUT -p tcp --dport 8765 -j ACCEPT
```

## 🏗️ Architecture

```
┌─────────────┐          WebSocket           ┌────────────────┐
│   Wallet    │ ◄───────────────────────────► │   Masternode   │
│  (Client)   │      ws://mn-ip:8765          │  (WS Server)   │
└─────────────┘                               └────────────────┘
                                                      │
                                                      │ Broadcasts
                                                      ▼
                                              ┌───────────────┐
                                              │  UTXO State   │
                                              │   Protocol    │
                                              │  (Consensus)  │
                                              └───────────────┘
```

## 🧪 Testing WebSocket Connection

### Using `wscat` (Node.js)

```bash
npm install -g wscat
wscat -c ws://localhost:8765
```

Then send messages:
```json
{"type":"Subscribe","data":{"addresses":["time1test"]}}
{"type":"Ping"}
```

### Using Python

```python
import websocket
import json

ws = websocket.create_connection("ws://localhost:8765")

# Subscribe
ws.send(json.dumps({
    "type": "Subscribe",
    "data": {"addresses": ["time1test"]}
}))

# Listen for messages
while True:
    msg = ws.recv()
    print(f"Received: {msg}")
```

### Using JavaScript (Browser)

```javascript
const ws = new WebSocket('ws://localhost:8765');

ws.onopen = () => {
    console.log('Connected to masternode');
    
    // Subscribe to addresses
    ws.send(JSON.stringify({
        type: 'Subscribe',
        data: { addresses: ['time1test'] }
    }));
};

ws.onmessage = (event) => {
    const msg = JSON.parse(event.data);
    console.log('Received:', msg);
    
    if (msg.type === 'TransactionFinalized') {
        console.log('✅ Transaction finalized instantly!');
    }
};
```

## 📊 Monitoring

### Check Connected Clients

The masternode logs each connection:
```
📱 New WebSocket connection from: 192.168.1.100:54321
📢 Client abc-123 subscribed to 5 addresses
👋 Client abc-123 disconnected
```

### Health Check

The wallet sends periodic Ping messages to check connection health.

## 🔒 Production Deployment

### Using systemd (Linux)

Create `/etc/systemd/system/time-masternode.service`:

```ini
[Unit]
Description=TIME Coin Masternode
After=network.target

[Service]
Type=simple
User=timecoin
WorkingDirectory=/opt/timecoin
Environment="WS_ADDR=0.0.0.0:8765"
ExecStart=/opt/timecoin/time-masternode
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable time-masternode
sudo systemctl start time-masternode
sudo systemctl status time-masternode
```

View logs:
```bash
sudo journalctl -u time-masternode -f
```

### Using Docker

```dockerfile
FROM debian:bookworm-slim
COPY target/release/time-masternode /usr/local/bin/
EXPOSE 8765
CMD ["time-masternode"]
```

Build and run:
```bash
docker build -t time-masternode .
docker run -p 8765:8765 -e WS_ADDR=0.0.0.0:8765 time-masternode
```

### Using Docker Compose

```yaml
version: '3.8'
services:
  masternode:
    image: time-masternode
    ports:
      - "8765:8765"
    environment:
      - WS_ADDR=0.0.0.0:8765
    restart: unless-stopped
```

## 🐛 Troubleshooting

### Port Already in Use

```bash
# Find what's using port 8765
sudo lsof -i :8765
# or
sudo netstat -tulpn | grep 8765
```

### Wallet Not Receiving Updates

1. Check firewall allows port 8765
2. Verify masternode is running: `ps aux | grep time-masternode`
3. Test WebSocket connection with `wscat`
4. Check wallet is connected to correct IP:port

### Connection Drops

- Check network stability
- Verify firewall/NAT isn't timing out connections
- Enable TCP keepalive in production

## 📚 See Also

- [TIME Coin Protocol Specification](../docs/TIME_COIN_PROTOCOL.md)
- [UTXO State Protocol](../consensus/src/utxo_state_protocol.rs)
- [Wallet WebSocket Client](../wallet-gui/README.md)

---

**Status**: ✅ Ready for testing  
**Protocol Version**: 1.0  
**Default Port**: 8765
