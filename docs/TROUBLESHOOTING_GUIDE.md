# TIME Coin Troubleshooting Complete Guide

**Table of Contents**
- [Overview](#overview)
- [Data Directory Migration](#data-directory-migration)
- [Testnet Reset and Genesis Migration](#testnet-reset-and-genesis-migration)
- [Common Issues](#common-issues)
- [Balance and Transaction Issues](#balance-and-transaction-issues)
- [Fork Detection and Recovery](#fork-detection-and-recovery)
- [Double Spend Prevention](#double-spend-prevention)
- [Selective Block Resync](#selective-block-resync)
- [Quick Migration Checklist](#quick-migration-checklist)

---

## Overview

This comprehensive troubleshooting guide covers common issues, migrations, and recovery procedures for TIME Coin nodes. Follow the appropriate section based on your issue.

---

## Data Directory Migration

### Overview

TIME Coin has migrated to industry-standard data directory locations, following Bitcoin's convention.

### New Directory Structure

#### Linux/Mac
```
~/.timecoin/
├── config/
│   └── testnet.toml
├── data/
│   ├── blockchain/
│   ├── wallets/
│   └── genesis.json
└── logs/
    └── node.log
```

#### Windows
```
%APPDATA%\timecoin\
├── config\
│   └── testnet.toml
├── data\
│   ├── blockchain\
│   ├── wallets\
│   └── genesis.json
└── logs\
    └── node.log
```

### Old Locations (Legacy)

**Linux**:
- `/var/lib/time-coin/` (system-wide)
- `~/time-coin-node/` (user-specific)

**Windows**:
- `%LOCALAPPDATA%\time-coin\`

### Migration Instructions

#### Linux/Mac

```bash
# Stop the node
sudo systemctl stop timed

# Option 1: Move from /var/lib/time-coin
sudo mv /var/lib/time-coin ~/.timecoin
sudo chown -R $USER:$USER ~/.timecoin

# Option 2: Move from ~/time-coin-node
mv ~/time-coin-node ~/.timecoin

# Restart the node
sudo systemctl start timed
```

#### Windows

```powershell
# Stop the node
Stop-Service timed

# Move from old location
Move-Item "$env:LOCALAPPDATA\time-coin" "$env:APPDATA\timecoin"

# Restart the node
Start-Service timed
```

### Environment Variable Override

You can use a custom location by setting the `TIME_COIN_DATA_DIR` environment variable:

```bash
# Linux/Mac
export TIME_COIN_DATA_DIR=/custom/path/.timecoin

# Windows (PowerShell)
$env:TIME_COIN_DATA_DIR = "C:\custom\path\timecoin"
```

### Backward Compatibility

The node automatically detects and uses legacy paths if they exist:

1. First checks: `$TIME_COIN_DATA_DIR` (if set)
2. Then checks: `~/.timecoin` (new standard)
3. Falls back to: `/var/lib/time-coin` (legacy)

A warning will be displayed if using legacy paths.

---

## Testnet Reset and Genesis Migration

### When to Reset

- New genesis block release
- Network consensus issues
- Major protocol changes
- Blockchain corruption

### Prerequisites

- All nodes running compatible version
- Access to one "seed node" that will load genesis from file
- `genesis-testnet.json` file in the `config/` directory

### Step-by-Step Reset Procedure

#### 1. Stop All Nodes

On **all** testnet nodes:

```bash
sudo systemctl stop timed
```

#### 2. Backup Important Data

```bash
# Create backup directory
BACKUP_DIR="/var/backups/time-coin-$(date +%Y%m%d-%H%M%S)"
sudo mkdir -p $BACKUP_DIR

# Backup wallet data
sudo cp -r ~/.timecoin/data/wallets $BACKUP_DIR/

# Backup configuration
sudo cp ~/.timecoin/config/testnet.toml $BACKUP_DIR/

# Backup logs (optional)
sudo cp -r ~/.timecoin/logs $BACKUP_DIR/
```

#### 3. Clear Blockchain Data

On **all** nodes:

```bash
# Clear blockchain data (preserves wallet)
sudo rm -rf ~/.timecoin/data/blockchain/*
sudo rm -f ~/.timecoin/data/genesis.json
```

#### 4. Configure ONE Seed Node

On **ONE** node (most stable/reliable):

```bash
# Edit testnet.toml
nano ~/.timecoin/config/testnet.toml
```

Enable genesis loading:

```toml
[blockchain]
genesis_file = "$HOME/.timecoin/data/genesis-testnet.json"
load_genesis_from_file = true  # CHANGE THIS TO true (normally false)
```

**IMPORTANT:** After reset completes, change it back to `false` to prevent independent chains!

#### 5. Copy New Genesis File

```bash
# Copy new genesis to data directory
cp ~/time-coin/config/genesis-testnet.json ~/.timecoin/data/
```

#### 6. Start the Seed Node First

```bash
sudo systemctl start timed

# Watch logs to confirm genesis loads
sudo journalctl -u timed -f
```

You should see:
```
✓ Genesis block loaded from file
✓ Blockchain initialized: height=0
```

#### 7. Start Other Nodes

After seed node is running (wait 30 seconds):

```bash
# On all other nodes
sudo systemctl start timed

# Verify they sync from seed node
sudo journalctl -u timed -f
```

#### 8. Disable Genesis Loading on Seed Node

After all nodes are synced:

```bash
# On seed node, edit config
nano ~/.timecoin/config/testnet.toml

# Change back to false
[blockchain]
load_genesis_from_file = false  # IMPORTANT: Change back to false

# Restart seed node
sudo systemctl restart timed
```

### Verification

```bash
# Check block height on all nodes
time-cli info

# Check they all have same genesis hash
time-cli blocks | head -5

# Verify peer connections
time-cli peers
```

---

## Common Issues

### Node Won't Start

**Symptoms**: Service fails to start, crashes immediately

**Diagnosis**:
```bash
# Check status
sudo systemctl status timed

# Check logs
sudo journalctl -u timed -n 50

# Check for port conflicts
sudo netstat -tlnp | grep 24101
sudo netstat -tlnp | grep 24100
```

**Solutions**:

1. **Port already in use**:
```bash
# Find process using port
sudo lsof -i :24101

# Kill old process
sudo pkill timed

# Restart service
sudo systemctl restart timed
```

2. **Configuration error**:
```bash
# Validate TOML syntax
cat ~/.timecoin/config/testnet.toml

# Check for common issues
grep -E "enabled|bind|port" ~/.timecoin/config/testnet.toml
```

3. **Permission issues**:
```bash
# Fix ownership
sudo chown -R $USER:$USER ~/.timecoin

# Fix permissions
chmod 755 ~/.timecoin
chmod 755 ~/.timecoin/data
chmod 755 ~/.timecoin/logs
chmod 644 ~/.timecoin/config/testnet.toml
```

### No Peer Connections

**Symptoms**: `time-cli peers` shows 0 peers

**Diagnosis**:
```bash
# Check network interface
ip addr show

# Check firewall
sudo ufw status

# Test P2P port
telnet localhost 24100
```

**Solutions**:

1. **Firewall blocking**:
```bash
# Allow P2P port
sudo ufw allow 24100/tcp

# Allow API port (if needed)
sudo ufw allow 24101/tcp
```

2. **No bootstrap peers**:
```bash
# Add bootstrap peers to config
nano ~/.timecoin/config/testnet.toml

[network]
bootstrap_nodes = [
    "192.168.1.100:24100",
    "192.168.1.101:24100"
]
```

3. **Network interface issue**:
```bash
# Bind to all interfaces
nano ~/.timecoin/config/testnet.toml

[network]
listen_addr = "0.0.0.0:24100"  # Not just 127.0.0.1
```

### Blockchain Won't Sync

**Symptoms**: Block height stuck, not advancing

**Diagnosis**:
```bash
# Check current height
time-cli info

# Check logs for sync messages
sudo journalctl -u timed -f | grep -i sync

# Check peer connections
time-cli peers
```

**Solutions**:

1. **Connected to wrong network**:
```bash
# Verify network in config
grep "network" ~/.timecoin/config/testnet.toml

# Should be "testnet" not "mainnet"
```

2. **Peers on different chain**:
```bash
# Check genesis hash matches peers
time-cli blocks | head -1

# If different, need to reset and resync
```

3. **Corrupted blockchain data**:
```bash
# Reset blockchain (keeps wallet)
sudo systemctl stop timed
rm -rf ~/.timecoin/data/blockchain/*
sudo systemctl start timed
```

---

## Balance and Transaction Issues

### Balance Shows Zero

**Symptoms**: `time-cli wallet balance` shows 0 but you have funds

**Diagnosis**:
```bash
# Check UTXO set
time-cli utxos YOUR_ADDRESS

# Check blockchain height
time-cli info

# Check logs for errors
sudo journalctl -u timed | grep -i balance
```

**Solutions**:

1. **Wallet not synced**:
```bash
# Rescan blockchain
time-cli wallet rescan

# Or force full sync
time-cli wallet sync --force
```

2. **Wrong address**:
```bash
# Verify your address
time-cli wallet address

# Check if funds are on different address
time-cli wallet addresses
```

3. **Database corruption**:
```bash
# Rebuild balance from UTXO set
sudo systemctl stop timed
rm -f ~/.timecoin/data/balances.db
sudo systemctl start timed
```

### Missing 1000 TIME Transaction

**Symptoms**: Grant application approved but 1000 TIME not showing

**Diagnosis**:
```bash
# Check transaction in blockchain
time-cli transactions YOUR_ADDRESS

# Check block height when grant was approved
time-cli info

# Check mempool
curl http://localhost:24101/mempool/status
```

**Solutions**:

1. **Transaction in mempool (not confirmed yet)**:
```bash
# Wait for next block
# Check again in 5-10 minutes

# Or check mempool directly
time-cli mempool
```

2. **Transaction never broadcast**:
```bash
# Check grant status
curl http://localhost:24101/grants/status/YOUR_EMAIL

# If approved but not sent, restart node
sudo systemctl restart timed
```

3. **Block containing transaction orphaned**:
```bash
# Check for fork
time-cli blocks | head -10

# Verify you're on main chain
time-cli peers
```

---

## Fork Detection and Recovery

### Detecting Forks

**Symptoms**:
- Different block heights across nodes
- Different block hashes at same height
- Transactions disappear and reappear

**Detection Commands**:
```bash
# Check your block height and hash
time-cli info

# Check specific block
time-cli blocks | grep "Height: 100"

# Compare with peers
for peer in $(time-cli peers | grep -oE '[0-9.]+:24100'); do
    echo "=== $peer ==="
    curl -s http://${peer%:*}:24101/blockchain/info | jq
done
```

### Fork Recovery

#### Automatic Recovery (Recommended)

The node automatically handles most forks:

```bash
# Just restart the node
sudo systemctl restart timed

# Monitor logs
sudo journalctl -u timed -f

# Look for:
# "Fork detected at height X"
# "Switching to longer chain"
# "Orphaned block: ..."
```

#### Manual Recovery

If automatic recovery fails:

```bash
# Stop node
sudo systemctl stop timed

# Backup current chain
cp -r ~/.timecoin/data/blockchain ~/.timecoin/data/blockchain.backup

# Remove last N blocks (adjust based on fork point)
rm ~/.timecoin/data/blockchain/block_*.json | tail -10

# Restart and resync
sudo systemctl start timed
```

#### Nuclear Option (Full Resync)

Last resort if blockchain is completely corrupted:

```bash
# Stop node
sudo systemctl stop timed

# Backup wallet
cp -r ~/.timecoin/data/wallets ~/.timecoin/wallets.backup

# Remove all blockchain data
rm -rf ~/.timecoin/data/blockchain/*

# Restart and resync from genesis
sudo systemctl start timed
```

---

## Double Spend Prevention

### How TIME Coin Prevents Double Spends

1. **UTXO Validation**: Every transaction input must reference a valid unspent output
2. **Mempool Conflict Detection**: Rejects transactions spending same UTXO
3. **Consensus Verification**: Masternodes validate all transactions before block creation
4. **Instant Finality**: Transactions are final once included in a block

### Detecting Double Spend Attempts

**In Logs**:
```bash
# Search for double spend attempts
sudo journalctl -u timed | grep -i "double\|conflict\|already spent"
```

**In Mempool**:
```bash
# Check for conflicting transactions
time-cli mempool | grep -A 5 "conflict"
```

### If You Receive a Double Spend Warning

```bash
# Check transaction status
time-cli transaction TXID

# Verify UTXO is still unspent
time-cli utxos ADDRESS

# Wait for confirmation
# Transaction with more fees/earlier timestamp wins
```

---

## Selective Block Resync

### When to Use Selective Resync

- Specific blocks appear corrupted
- Block data missing but not entire chain
- Faster than full resync

### Resync Specific Block Range

```bash
# Stop node
sudo systemctl stop timed

# Remove blocks in range (e.g., 100-110)
cd ~/.timecoin/data/blockchain
for i in {100..110}; do
    rm -f block_$i.json
done

# Restart node (will request missing blocks from peers)
sudo systemctl start timed

# Monitor resync
sudo journalctl -u timed -f | grep -i "block\|sync"
```

### Resync Last N Blocks

```bash
# Stop node
sudo systemctl stop timed

# Remove last 20 blocks
cd ~/.timecoin/data/blockchain
ls -t block_*.json | head -20 | xargs rm

# Restart
sudo systemctl start timed
```

### Verify Resync Success

```bash
# Check height recovered
time-cli info

# Verify block hashes match peers
time-cli blocks | head -20

# Check for gaps
ls -1 ~/.timecoin/data/blockchain/block_*.json | sort -n | awk -F'[_.]' '{print $2}' | awk 'NR>1{if($1!=p+1)print "Gap: "p+1" to "$1-1;p=$1}BEGIN{p=0}'
```

---

## Quick Migration Checklist

Use this checklist when migrating to new version or resetting testnet:

### Pre-Migration

- [ ] Backup wallet: `cp -r ~/.timecoin/data/wallets ~/wallets.backup`
- [ ] Backup config: `cp ~/.timecoin/config/testnet.toml ~/testnet.toml.backup`
- [ ] Note current block height: `time-cli info`
- [ ] Note connected peers: `time-cli peers > ~/peers.txt`
- [ ] Stop node: `sudo systemctl stop timed`

### Migration

- [ ] Pull latest code: `cd ~/time-coin && git pull`
- [ ] Build new binaries: `cargo build --release`
- [ ] Update binaries: `sudo cp target/release/timed /usr/local/bin/`
- [ ] Update config if needed
- [ ] Update genesis if provided: `cp config/genesis-testnet.json ~/.timecoin/data/`

### Post-Migration

- [ ] Start node: `sudo systemctl start timed`
- [ ] Check logs: `sudo journalctl -u timed -f`
- [ ] Verify sync: `time-cli info`
- [ ] Check balance: `time-cli wallet balance`
- [ ] Check peers: `time-cli peers`
- [ ] Test transaction (small amount)
- [ ] Monitor for 24 hours

### Rollback (If Issues)

- [ ] Stop node: `sudo systemctl stop timed`
- [ ] Restore old binaries
- [ ] Restore config: `cp ~/testnet.toml.backup ~/.timecoin/config/testnet.toml`
- [ ] Restore wallet: `cp -r ~/wallets.backup ~/.timecoin/data/wallets`
- [ ] Restart: `sudo systemctl start timed`

---

## Emergency Contacts

### Community Support

- **GitHub Issues**: https://github.com/time-coin/time-coin/issues
- **Discord**: [Join Server]
- **Telegram**: [Join Group]

### Reporting Bugs

When reporting issues, include:

```bash
# System info
uname -a
cat /etc/os-release

# Node version
timed --version

# Configuration (redact private info)
cat ~/.timecoin/config/testnet.toml

# Recent logs
sudo journalctl -u timed -n 100 --no-pager

# Blockchain status
time-cli info
time-cli peers
```

---

## Related Documentation

- [Build and Install Guide](BUILD_AND_INSTALL.md)
- [Masternode Guide](MASTERNODE_GUIDE.md)
- [Dashboard Guide](DASHBOARD_GUIDE.md)
- [Wallet Guide](WALLET_GUIDE.md)

---

**Consolidated from**:
- DATA_DIRECTORY_MIGRATION.md
- TESTNET_GENESIS_MIGRATION.md
- TESTNET_RESET_GUIDE.md
- QUICK_MIGRATION.md
- MISSING_TRANSACTION_1000TIME.md
- fork-detection-and-recovery.md
- DOUBLE_SPEND_PREVENTION.md
- SELECTIVE_BLOCK_RESYNC.md
