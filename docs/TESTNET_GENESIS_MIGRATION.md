# Testnet Genesis Migration: Adding Proof-of-Time

**Date:** December 1, 2025  
**Commit:** `a1fbea1`  
**Impact:** 🔴 **BREAKING CHANGE** - All masternodes must migrate

---

## What Changed

**Old Genesis (Incompatible):**
```json
{
  "version": 1,
  "message": "TIME Coin Testnet Launch - October 12, 2025",
  "block": {
    "header": {
      "block_number": 0,
      "timestamp": "2025-10-12T00:00:00Z",
      // No proof_of_time field
    }
  }
}
```

**New Genesis (Required):**
```json
{
  "version": 2,
  "message": "TIME Coin Testnet Relaunch - December 1, 2025 - Proof of Time Enabled",
  "block": {
    "header": {
      "block_number": 0,
      "timestamp": "2025-12-01T00:00:00Z",
      "proof_of_time": {
        "output": "0000...",
        "proof": "0000...",
        "iterations": 100000,
        "input_hash": "genesis_bootstrap_2025_12_01"
      }
    }
  }
}
```

---

## Why This Migration is Needed

**Problem:**
- Old blocks don't have `proof_of_time` field
- Code now requires `proof_of_time` for all blocks
- Deserialization fails: `missing field proof_of_time`
- Network cannot sync with mixed genesis versions

**Solution:**
- Create new genesis block with Proof-of-Time
- All masternodes reset to new genesis
- Establish PoT baseline for entire chain

---

## Migration Steps

### ⚠️ CRITICAL: All Masternodes Must Migrate Together

**Coordinate with other node operators before starting!**

### Step 1: Stop Your Masternode

```bash
sudo systemctl stop timed
```

### Step 2: Backup Old Data (Optional)

```bash
# Backup old blockchain (optional - will be deleted)
cd ~/time-coin-node
sudo tar -czf backup-old-chain-$(date +%Y%m%d).tar.gz data/

# Move backup to safe location
sudo mv backup-old-chain-*.tar.gz ~/backups/
```

### Step 3: Delete Old Blockchain

```bash
cd ~/time-coin-node

# Delete old blockchain database
sudo rm -rf data/blockchain/

# Keep other data (config, peer database, etc.)
# Only blockchain/ directory is deleted
```

### Step 4: Pull Latest Code

```bash
cd ~/time-coin
git fetch origin
git reset --hard origin/main
git pull origin main

# Should see commit a1fbea1 or later
git log --oneline -3
```

### Step 5: Verify Genesis File

```bash
# Check that genesis file has proof_of_time
cat ~/time-coin-node/config/genesis-testnet.json | grep -A 5 proof_of_time

# Should see:
#   "proof_of_time": {
#     "output": "0000000000...",
#     "proof": "0000000000...",
#     "iterations": 100000,
#     "input_hash": "genesis_bootstrap_2025_12_01"
#   }
```

### Step 6: Verify Config Has Genesis Loading Enabled

```bash
# Check config
cat ~/time-coin-node/config/testnet.toml | grep -A 2 load_genesis_from_file

# Should see:
# load_genesis_from_file = true  <-- MUST BE true

# If false, edit config:
nano ~/time-coin-node/config/testnet.toml

# Change:
# load_genesis_from_file = false
# To:
# load_genesis_from_file = true

# Save and exit (Ctrl+O, Enter, Ctrl+X)
```

### Step 7: Rebuild

```bash
cd ~/time-coin
cargo build --release

# This will take 5-10 minutes
# Wait for: Finished `release` profile
```

### Step 8: Restart Masternode

```bash
sudo systemctl start timed
```

### Step 9: Verify Genesis Loaded

```bash
# Watch logs
journalctl -u timed -f

# Should see:
# 🔍 Genesis loading is enabled
#    Genesis file path: /root/time-coin-node/config/genesis-testnet.json
#    Genesis block on disk: false
# 📥 Loading genesis block from file...
# ✅ Genesis block loaded: <hash>
# ✅ Blockchain state loaded successfully
```

### Step 10: Confirm Network Sync

```bash
# Check logs for peer connections
journalctl -u timed -f | grep -i "peer\|genesis\|block"

# Should see:
# ✅ Connected to X peers
# 📊 Chain height: 0 (genesis)
# 🔄 Syncing with network...

# As other nodes come online, blocks will sync
```

---

## Verification Checklist

**Before Migration:**
- [ ] Coordinate with other node operators
- [ ] Backup old data (optional)
- [ ] Read full migration guide

**During Migration:**
- [ ] `sudo systemctl stop timed` - Service stopped
- [ ] `sudo rm -rf data/blockchain/` - Old chain deleted
- [ ] `git pull origin main` - Latest code pulled
- [ ] `cargo build --release` - Build successful
- [ ] `load_genesis_from_file = true` in config
- [ ] `genesis-testnet.json` has `proof_of_time`

**After Migration:**
- [ ] `sudo systemctl start timed` - Service started
- [ ] `journalctl -u timed -f` - No errors in logs
- [ ] Genesis block loaded from JSON file
- [ ] Blockchain state initialized
- [ ] Connected to peer masternodes
- [ ] Network syncing properly

---

## Troubleshooting

### Issue: "Genesis loading from file is disabled"

**Logs show:**
```
ℹ️  Genesis loading from file is disabled - will sync from peers
```

**Solution:**
```bash
nano ~/time-coin-node/config/testnet.toml

# Set:
load_genesis_from_file = true

# Restart:
sudo systemctl restart timed
```

---

### Issue: "Could not read genesis file"

**Logs show:**
```
⚠️  Could not read genesis file /root/time-coin-node/config/genesis-testnet.json: No such file
```

**Solution:**
```bash
# Copy genesis file from repo
cd ~/time-coin
cp config/genesis-testnet.json ~/time-coin-node/config/

# Or update path in config:
nano ~/time-coin-node/config/testnet.toml

# Set:
genesis_file = "/root/time-coin/config/genesis-testnet.json"

# Restart:
sudo systemctl restart timed
```

---

### Issue: "Failed to load genesis block: missing field proof_of_time"

**Logs show:**
```
❌ Failed to load genesis block: missing field `proof_of_time`
```

**Solution:**
```bash
# Old genesis file - need to pull latest
cd ~/time-coin
git pull origin main

# Copy new genesis:
cp config/genesis-testnet.json ~/time-coin-node/config/

# Restart:
sudo systemctl restart timed
```

---

### Issue: "Genesis already exists on disk"

**Logs show:**
```
✅ Genesis block already exists on disk
```

**Means:** Old genesis is still in database

**Solution:**
```bash
# Stop service
sudo systemctl stop timed

# Delete old blockchain
sudo rm -rf ~/time-coin-node/data/blockchain/

# Start service
sudo systemctl start timed

# Should now load new genesis from JSON
```

---

### Issue: Peers timing out during genesis download

**Logs show:**
```
⚠️  Could not query peer X.X.X.X - timeout after 10s
```

**Means:** Other nodes haven't migrated yet or aren't online

**Solution:**
- Wait for other masternodes to migrate
- Ensure `load_genesis_from_file = true` so you don't need to download
- Your node will load genesis from JSON file instead

---

## Network-Wide Coordination

### Migration Timeline

**Phase 1: Prepare (15 minutes)**
- All operators read migration guide
- Confirm ready to migrate
- Agree on start time

**Phase 2: Migrate (30 minutes)**
- All nodes stop simultaneously
- Delete old blockchain
- Pull latest code
- Rebuild
- Restart with new genesis

**Phase 3: Verify (15 minutes)**
- All nodes confirm genesis loaded
- Nodes connect to each other
- Network starts producing blocks

**Total Downtime:** ~1 hour

---

## Expected Log Output (Successful Migration)

```bash
Dec 01 20:00:00 node1 timed: 🚀 Starting TIME Coin masternode...
Dec 01 20:00:00 node1 timed: 📂 Data directory: /root/time-coin-node/data
Dec 01 20:00:00 node1 timed: 🔍 Genesis loading is enabled
Dec 01 20:00:00 node1 timed:    Genesis file path: /root/time-coin-node/config/genesis-testnet.json
Dec 01 20:00:00 node1 timed:    Genesis block on disk: false
Dec 01 20:00:00 node1 timed: 📥 Loading genesis block from file...
Dec 01 20:00:01 node1 timed: ✅ Genesis block loaded: a1b2c3d4e5f6...
Dec 01 20:00:01 node1 timed: ✅ Blockchain state loaded successfully
Dec 01 20:00:01 node1 timed:    Chain height: 0
Dec 01 20:00:01 node1 timed:    Genesis hash: a1b2c3d4e5f6...
Dec 01 20:00:02 node1 timed: 🌐 Connecting to peers...
Dec 01 20:00:03 node1 timed: ✅ Connected to peer 161.35.129.70:24100
Dec 01 20:00:03 node1 timed: ✅ Connected to peer 134.199.175.106:24100
Dec 01 20:00:03 node1 timed: 📊 Network status:
Dec 01 20:00:03 node1 timed:    Connected peers: 2
Dec 01 20:00:03 node1 timed:    Chain height: 0 (genesis)
Dec 01 20:00:03 node1 timed: ✅ Masternode ready!
```

---

## Post-Migration

### What Happens Next?

1. **Genesis Block:** All nodes have identical genesis with PoT
2. **Block Production:** Next block (height 1) will be produced at midnight UTC
3. **VDF Computation:** Block producer computes VDF for 10 minutes (testnet)
4. **Consensus:** Masternodes validate the VDF proof
5. **Finality:** Block 1 becomes immutable via Proof-of-Time

### New Block Timeline (10-minute blocks on testnet)

```
Block 0 (Genesis): 2025-12-01 00:00:00 UTC
  ↓ (10 minutes)
Block 1: 2025-12-01 00:10:00 UTC
  ↓ (10 minutes)
Block 2: 2025-12-01 00:20:00 UTC
  ↓ ...
```

### Monitoring Commands

```bash
# Watch logs
journalctl -u timed -f

# Check chain height
curl -s http://localhost:24101/api/blockchain/info | jq '.height'

# Check peer count
curl -s http://localhost:24101/api/network/peers | jq 'length'

# Check latest block
curl -s http://localhost:24101/api/blockchain/tip | jq '.'
```

---

## Key Differences: Old vs New Genesis

| Field | Old Genesis | New Genesis |
|-------|-------------|-------------|
| **Version** | 1 | 2 |
| **Timestamp** | 2025-10-12T00:00:00Z | 2025-12-01T00:00:00Z |
| **Coinbase Timestamp** | 1760227200 | 1733011200 |
| **proof_of_time** | ❌ Missing | ✅ Present |
| **PoT Iterations** | N/A | 100,000 |
| **Message** | "Testnet Launch" | "Proof of Time Enabled" |

---

## Questions?

**Stuck during migration?**
- Check logs: `journalctl -u timed -f`
- See troubleshooting section above
- Contact other node operators

**Migration successful?**
- Verify genesis loaded: ✅
- Verify peers connected: ✅
- Wait for block 1 production: ⏳

---

## Summary

✅ **What We Accomplished:**
- Created new genesis with Proof-of-Time
- Established VDF baseline for the chain
- Enabled block finality through time-based proofs
- Reset testnet with PoT-compatible genesis

✅ **What This Enables:**
- Blocks cannot be rolled back (PoT proves time elapsed)
- No need to trust majority consensus for finality
- Cryptographic proof of time order
- Resistance to long-range attacks

✅ **Next Steps:**
1. All nodes migrate to new genesis
2. Network starts producing PoT blocks
3. Monitor for 24 hours
4. Verify PoT proofs are valid
5. Celebrate secure, time-proven blockchain! 🎉

---

**Happy Mining with Proof-of-Time!** ⏰🔗✨
