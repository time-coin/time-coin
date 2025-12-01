# Quick Genesis Migration Guide

## For nodes using `/var/lib/time-coin/` data directory

This is the **quick version** for experienced operators who know their setup.

---

## ⚡ One-Line Migration (Fast)

```bash
sudo systemctl stop timed && \
sudo rm -rf /var/lib/time-coin/blockchain/* && \
cd ~/time-coin && git pull origin main && \
cargo build --release && \
sudo systemctl start timed && \
journalctl -u timed -f
```

**What it does:**
1. Stops service
2. Deletes old blockchain (keeps wallets/keys)
3. Pulls latest code with PoT genesis
4. Rebuilds binary
5. Starts service
6. Shows live logs

**Time:** ~10 minutes (mostly cargo build)

---

## 📋 Step-by-Step (Safer)

### 1. Stop Service
```bash
sudo systemctl stop timed
```

### 2. Delete Old Blockchain
```bash
# Option A: Delete everything in blockchain directory
sudo rm -rf /var/lib/time-coin/blockchain/*

# Option B: Delete just the database files (keeps directory structure)
sudo rm -rf /var/lib/time-coin/blockchain/blocks_*
sudo rm -rf /var/lib/time-coin/blockchain/LOCK
sudo rm -rf /var/lib/time-coin/blockchain/MANIFEST-*
```

### 3. Update Code
```bash
cd ~/time-coin
git pull origin main
```

### 4. Verify Genesis Has PoT
```bash
cat config/genesis-testnet.json | grep -A 5 proof_of_time
```

**Expected output:**
```json
"proof_of_time": {
  "output": "0000000000000000000000000000000000000000000000000000000000000000",
  "proof": "0000000000000000000000000000000000000000000000000000000000000000",
  "iterations": 100000,
  "input_hash": "genesis_bootstrap_2025_12_01"
}
```

### 5. Check Config
```bash
# Find your config file
sudo find /root -name "testnet.toml" 2>/dev/null
sudo find /etc -name "testnet.toml" 2>/dev/null

# Verify genesis loading is enabled
grep "load_genesis_from_file" /path/to/your/testnet.toml
```

**Should show:**
```toml
load_genesis_from_file = true
```

**If it shows `false`, enable it:**
```bash
sudo nano /path/to/your/testnet.toml
# Change: load_genesis_from_file = false
# To:     load_genesis_from_file = true
```

### 6. Rebuild
```bash
cd ~/time-coin
cargo build --release
```

### 7. Start Service
```bash
sudo systemctl start timed
```

### 8. Verify Success
```bash
# Check status
sudo systemctl status timed

# Watch logs
journalctl -u timed -f
```

---

## ✅ Success Indicators

**You'll see these logs:**
```
🔍 Genesis loading is enabled
   Genesis file path: /root/time-coin/config/genesis-testnet.json
   Genesis block on disk: false
📥 Loading genesis block from file...
✅ Genesis block loaded: <hash>
✅ Blockchain state loaded successfully
   Chain height: 0
Starting block producer...
```

**You should NOT see:**
```
⏳ Waiting for genesis block to be downloaded...  ← BAD
```

---

## 🔍 Verify Genesis Block

```bash
# Check via API (if running)
curl -s http://localhost:24101/api/blockchain/block/0 | jq .

# Check database directly
ls -lh /var/lib/time-coin/blockchain/
```

---

## 🚨 Troubleshooting

### "Still waiting for genesis"

**Cause:** Old blockchain database still present, conflicting with new format

**Fix:**
```bash
sudo systemctl stop timed
sudo rm -rf /var/lib/time-coin/blockchain/*
sudo systemctl start timed
journalctl -u timed -f
```

### "Failed to deserialize block"

**Cause:** Mixing old and new genesis formats

**Fix:** Delete blockchain database (see above)

### "No genesis file found"

**Cause:** Genesis file path wrong in config

**Fix:**
```bash
# Check where genesis file actually is
find ~/time-coin -name "genesis-testnet.json"

# Update config to point to it
sudo nano /path/to/testnet.toml
# Set: genesis_file = "/root/time-coin/config/genesis-testnet.json"
```

### "Build failed"

**Cause:** Code not up to date or dependencies missing

**Fix:**
```bash
cd ~/time-coin
git fetch origin
git reset --hard origin/main
cargo clean
cargo build --release
```

---

## 📊 Data Locations Reference

### Common Paths

| Purpose | Location |
|---------|----------|
| Blockchain data | `/var/lib/time-coin/blockchain/` |
| Wallet data | `/var/lib/time-coin/wallet/` |
| Config file | `/root/time-coin-node/config/testnet.toml` |
| Genesis file | `/root/time-coin/config/genesis-testnet.json` |
| Binary | `/root/time-coin/target/release/timed` |
| Service logs | `journalctl -u timed` |

### Your Specific Setup

Edit this section for your nodes:

```bash
DATA_DIR=/var/lib/time-coin
CONFIG_FILE=/path/to/your/testnet.toml
GENESIS_FILE=/root/time-coin/config/genesis-testnet.json
REPO_DIR=/root/time-coin
```

---

## 🎯 Quick Commands

```bash
# View service status
sudo systemctl status timed

# View live logs
journalctl -u timed -f

# View last 50 lines
journalctl -u timed -n 50

# Restart service
sudo systemctl restart timed

# Check disk usage
du -sh /var/lib/time-coin/*

# Check genesis block
curl -s http://localhost:24101/api/blockchain/block/0 | jq .height

# Check chain height
curl -s http://localhost:24101/api/blockchain/info | jq .height

# Check peer connections
curl -s http://localhost:24101/api/network/peers | jq length
```

---

## 🔐 Important Notes

- **Wallets are NOT deleted** - only blockchain data
- **Private keys remain safe** - stored separately
- **Network downtime required** - coordinate with other operators
- **All nodes must migrate together** - incompatible with old genesis
- **Takes ~10 minutes per node** - mostly compile time
- **No backup needed** - genesis can be regenerated from JSON

---

## 📞 Support

If you encounter issues:

1. Check logs: `journalctl -u timed -n 100`
2. Verify paths: All directories exist and writable
3. Check permissions: Service has access to data directory
4. Verify config: `load_genesis_from_file = true`
5. Confirm code is latest: `git log --oneline -1`

---

**Last updated:** December 1, 2025  
**Genesis version:** v2 (Proof-of-Time enabled)  
**Migration required by:** All nodes before next block production
