# TIME Coin Dashboard Complete Guide

**Table of Contents**
- [Overview](#overview)
- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
- [Interface Layout](#interface-layout)
- [Wallet Balance Detection](#wallet-balance-detection)
- [Troubleshooting](#troubleshooting)
- [Development](#development)

---

## Overview

A real-time monitoring dashboard for TIME Coin nodes that displays blockchain status, network peers, mempool information, and wallet balance. The dashboard provides an easy-to-read interface with automatic updates every 5 seconds.

---

## Features

### Core Features
- **Real-time Updates**: Auto-refreshes every 5 seconds
- **Cross-Platform**: Works on Windows (CMD, PowerShell, Git Bash), Linux, and macOS
- **Blockchain Status**: Current block height and best block hash
- **Network Monitoring**: Connected peers with connection details and ping times
- **Mempool Status**: Pending transactions count
- **Wallet Balance**: Automatic detection and display of node wallet balance
- **Colorful Interface**: Easy-to-read colored output using Unicode box-drawing characters

### Visual Elements
- **Header**: TIME COIN NODE DASHBOARD with timestamp
- **Blockchain Section**: Height and hash
- **Wallet Section**: Address and balance (if detected)
- **Network Section**: List of connected peers with ping times
- **Mempool Section**: Transaction count
- **Footer**: Exit instructions and refresh interval

---

## Installation

### Build from Source

```bash
# Navigate to repository
cd time-coin

# Build dashboard binary
cargo build --release --bin time-dashboard

# Binary location:
# Windows: target/release/time-dashboard.exe
# Linux/Mac: target/release/time-dashboard
```

### Quick Install Script (Linux)

```bash
# Create helper script
cat > ~/dashboard.sh << 'EOF'
#!/bin/bash
cd ~/time-coin
cargo build --release --bin time-dashboard 2>&1 | grep -v "Compiling\|Finished" || true
./target/release/time-dashboard
EOF

chmod +x ~/dashboard.sh

# Run dashboard
~/dashboard.sh
```

---

## Usage

### Basic Usage

Connect to the default local node (http://localhost:24101):

```bash
# Linux/Mac
./target/release/time-dashboard

# Windows
.\target\release\time-dashboard.exe
```

### Custom API Endpoint

Connect to a remote or custom node:

```bash
time-dashboard http://192.168.1.100:24101
```

### Running as Background Monitor

#### Linux/macOS (tmux)
```bash
# Create new tmux session
tmux new -s dashboard

# Run dashboard
time-dashboard

# Detach: Ctrl+B, then D
# Reattach: tmux attach -t dashboard
```

#### Linux/macOS (screen)
```bash
# Create new screen session
screen -S dashboard

# Run dashboard
time-dashboard

# Detach: Ctrl+A, then D
# Reattach: screen -r dashboard
```

#### Windows (background)
```powershell
# Run minimized
Start-Process -FilePath ".\target\release\time-dashboard.exe" -WindowStyle Minimized
```

---

## Interface Layout

### When Node is Running

```
╔══════════════════════════════════════════════════════════════╗
║         TIME COIN NODE DASHBOARD                            ║
╚══════════════════════════════════════════════════════════════╝
  Current Time: 2025-12-04 21:30:00 UTC
  API Endpoint: http://localhost:24101

┌─ Blockchain Status ──────────────────────────────────────────┐
│ Block Height:        216
│ Best Block Hash:     b0931265cff15219...
│ Wallet Address:      TIME0mkLKMVtAbyBefLRcn8cLqjaxbVuBYheeYB
│ Confirmed Balance:   2423.84158152 TIME
└──────────────────────────────────────────────────────────────┘

┌─ Network Peers ──────────────────────────────────────────────┐
│ Connected Peers:     3
│  1. 192.168.1.100:24100 (45ms)
│  2. 192.168.1.101:24100 (62ms)
│  3. 10.0.0.50:24100 (120ms)
└──────────────────────────────────────────────────────────────┘

┌─ Mempool Status ─────────────────────────────────────────────┐
│ Pending Transactions: 5
└──────────────────────────────────────────────────────────────┘

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Press Ctrl+C to exit | Auto-refresh every 5 seconds
```

### When Node is Not Reachable

```
╔══════════════════════════════════════════════════════════════╗
║         TIME COIN NODE DASHBOARD                            ║
╚══════════════════════════════════════════════════════════════╝
  Current Time: 2025-12-04 21:30:00 UTC
  API Endpoint: http://localhost:24101

┌─ Error ──────────────────────────────────────────────────────┐
│ Cannot connect to node: Request failed: connection refused
└──────────────────────────────────────────────────────────────┘

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Press Ctrl+C to exit | Auto-refresh every 5 seconds
```

---

## Wallet Balance Detection

The dashboard automatically detects and displays the node's wallet balance using multiple methods.

### Detection Methods

#### 1. Environment Variable (Highest Priority)
```bash
# Linux/Mac
export WALLET_ADDRESS="TIME0your_wallet_address"
./target/release/time-dashboard

# Windows PowerShell
$env:WALLET_ADDRESS = "TIME0your_wallet_address"
.\target\release\time-dashboard.exe
```

#### 2. API Endpoint (Automatic)
Dashboard queries:
- `/masternode/wallet` (primary)
- `/node/wallet` (fallback)

#### 3. Startup Message
When wallet is detected:
```
✓ Found masternode wallet: TIME0mkLKMVtAbyBefLRcn8cLqjaxbVuBYheeYB
```

When wallet is not found:
```
⚠️  No wallet address found. Checking:
   1. WALLET_ADDRESS environment variable
   2. http://localhost:24101/masternode/wallet endpoint
   3. http://localhost:24101/node/wallet endpoint

ℹ️  Wallet balance will not be displayed.
```

### Wallet Display Section

When wallet is detected:
```
┌─ Blockchain Status ──────────────────────────────────────────┐
│ Block Height:        216
│ Best Block Hash:     b0931265cff15219...
│ Wallet Address:      TIME0mkLKMVtAbyBefLRcn8cLqjaxbVuBYheeYB
│ Confirmed Balance:   2423.84158152 TIME
└──────────────────────────────────────────────────────────────┘
```

When wallet is not found:
```
┌─ Blockchain Status ──────────────────────────────────────────┐
│ Block Height:        216
│ Best Block Hash:     b0931265cff15219...
└──────────────────────────────────────────────────────────────┘

💰 Wallet Balance:
   ⚠️  Unable to fetch wallet balance
```

---

## Troubleshooting

### Connection Issues

#### Problem: "Cannot connect to node"

**Causes**:
- Node is not running
- Node API is not enabled
- Firewall blocking connection
- Wrong API endpoint

**Solutions**:
```bash
# 1. Check if node is running
ps aux | grep timed
sudo systemctl status timed

# 2. Test API endpoint
curl http://localhost:24101/health

# 3. Check node configuration (~/.timecoin/config/testnet.toml)
[rpc]
enabled = true
bind = "0.0.0.0"
port = 24101

# 4. Restart node
sudo systemctl restart timed
```

### Wallet Balance Issues

#### Problem: "Unable to fetch wallet balance"

**Causes**:
- Node wallet not initialized
- API endpoint not responding
- Empty wallet address in API response

**Solutions**:
```bash
# 1. Set environment variable manually
export WALLET_ADDRESS="TIME0your_address"

# 2. Test wallet endpoint
curl http://localhost:24101/masternode/wallet

# 3. Check node logs
sudo journalctl -u timed -n 50

# 4. Rebuild node with latest code
cd ~/time-coin
git pull origin main
cargo build --release --bin timed
sudo systemctl restart timed
```

### Peers Not Showing

#### Problem: Peers section shows error

**Causes**:
- Slow peer endpoint (timeout)
- Network connectivity issues
- No peers connected

**Solutions**:
```bash
# 1. Check peer connections directly
curl http://localhost:24101/peers

# 2. Check node peer count
time-cli info

# 3. Add bootstrap peers to config
[network]
bootstrap_nodes = [
    "192.168.1.100:24100",
    "192.168.1.101:24100"
]
```

### Mempool Errors

#### Problem: "Mempool info error: Parse failed"

**Causes**:
- API response format mismatch
- Mempool not initialized

**Solutions**:
```bash
# 1. Test mempool endpoint
curl http://localhost:24101/mempool/status

# 2. Rebuild dashboard with latest code
cd ~/time-coin
git pull origin main
cargo build --release --bin time-dashboard

# 3. Restart node if needed
sudo systemctl restart timed
```

### Display Issues

#### Problem: Broken characters or formatting

**Causes**:
- Terminal doesn't support Unicode
- Wrong character encoding

**Solutions**:
```bash
# Linux: Set UTF-8 locale
export LANG=en_US.UTF-8
export LC_ALL=en_US.UTF-8

# Windows: Use Windows Terminal or PowerShell 7+
# Git Bash: Should work by default

# Test Unicode support
echo "┌─┐└─┘│━"
```

---

## Development

### Building

```bash
# Debug build (faster compilation)
cargo build --bin time-dashboard

# Release build (optimized)
cargo build --release --bin time-dashboard

# With specific features
cargo build --release --bin time-dashboard --features "extended-metrics"
```

### Code Location

```
tools/
└── masternode-dashboard/
    ├── Cargo.toml
    └── src/
        └── main.rs          # Dashboard implementation
```

### API Endpoints Used

| Endpoint | Purpose | Response |
|----------|---------|----------|
| `/health` | Node health check | `{"status":"ok"}` |
| `/blockchain/info` | Blockchain status | Height, hash, network |
| `/masternode/wallet` | Node wallet address | `{"wallet_address":"TIME0..."}` |
| `/balance/{address}` | Wallet balance | `{"balance":123,"balance_time":"1.23"}` |
| `/peers` | Connected peers | `{"peers":[...],"count":5}` |
| `/mempool/status` | Mempool status | `{"size":12,"transactions":[...]}` |

### Customization

#### Change Refresh Interval

Edit `src/main.rs`:
```rust
// Default: 5 seconds
thread::sleep(Duration::from_secs(5));

// Change to 10 seconds
thread::sleep(Duration::from_secs(10));
```

#### Change API Timeout

```rust
let client = Client::builder()
    .timeout(Duration::from_secs(5))    // Request timeout
    .connect_timeout(Duration::from_secs(3))  // Connect timeout
    .build()?;
```

#### Add Custom Metrics

```rust
// Add to display section
println!("Custom Metric: {}", custom_value);
```

---

## Performance Considerations

### Resource Usage
- **CPU**: Very low (~0.1% on modern systems)
- **Memory**: ~10-20 MB
- **Network**: ~1 KB per refresh (5 KB/s at 5-second refresh)

### Optimization Tips

1. **Increase refresh interval** if node is remote
2. **Use tmux/screen** instead of keeping terminal open
3. **Monitor multiple nodes** with multiple dashboard instances
4. **Adjust timeouts** for slow networks

---

## Related Documentation

- [API Reference](API.md)
- [Masternode Guide](MASTERNODE_GUIDE.md)
- [Troubleshooting Guide](TROUBLESHOOTING_GUIDE.md)
- [Network Guide](NETWORK_GUIDE.md)

---

**Consolidated from**:
- DASHBOARD.md
- DASHBOARD_EXAMPLE.md
- DASHBOARD_WALLET_TROUBLESHOOTING.md
