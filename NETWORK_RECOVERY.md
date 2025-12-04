# Emergency Network Recovery Procedure

## Current Status
The network has experienced a **chain split** due to the VRF consensus bug. Nodes are at different heights with incompatible blocks:
- LW-Michigan: Block 262
- reitools.us: Block 260  
- ubuntu: Block 257
- NewYork: Block 257

## Why the Fix Alone Isn't Enough

The VRF fix prevents **new** splits, but doesn't resolve **existing** divergence. Each node thinks it's on the correct chain and won't accept blocks from other nodes because the hashes don't match.

## Recovery Options

### Option 1: Full Network Reset (RECOMMENDED - Cleanest)

This starts fresh with all nodes synchronized from genesis:

```bash
# On ALL nodes simultaneously:

# 1. Stop the daemon
sudo systemctl stop timed

# 2. Backup current blockchain (optional)
mkdir -p ~/timecoin-backup
cp -r ~/.timecoin/blockchain ~/timecoin-backup/blockchain-$(date +%Y%m%d-%H%M%S)

# 3. Remove divergent blockchain data
rm -rf ~/.timecoin/blockchain/*

# 4. Update to fixed code
cd /path/to/time-coin
git pull
cargo build --release --bin timed

# 5. Start one "seed node" first (e.g., reitools.us)
sudo systemctl start timed
# Wait 2 minutes for it to create genesis

# 6. Start other nodes
sudo systemctl start timed

# 7. Monitor synchronization
tail -f ~/.timecoin/logs/node.log | grep "Block Height"
```

###Option 2: Designate Canonical Chain (Faster - If one chain is clearly ahead)

Choose the node with the highest valid height as the source of truth:

```bash
# On the CANONICAL node (e.g., LW-Michigan at height 262):
# 1. Keep it running - this is your source of truth
systemctl status timed

# On ALL OTHER nodes:
# 1. Stop the daemon
sudo systemctl stop timed

# 2. Backup and clear blockchain
mkdir -p ~/timecoin-backup
cp -r ~/.timecoin/blockchain ~/timecoin-backup/blockchain-diverged
rm -rf ~/.timecoin/blockchain/*

# 3. Update code
cd /path/to/time-coin
git pull
cargo build --release --bin timed

# 4. Start and let it sync from LW-Michigan
sudo systemctl start timed

# 5. Monitor catch-up
tail -f ~/.timecoin/logs/node.log
```

### Option 3: Hard Fork to New Genesis (Clean Slate - Best for testnet)

If you want to start completely fresh with a new genesis:

```bash
# On ALL nodes:
sudo systemctl stop timed
rm -rf ~/.timecoin/blockchain/*
rm -rf ~/.timecoin/storage/*

# Update config to change genesis parameters (optional)
# Edit config files to increment network version

cd /path/to/time-coin
git pull  
cargo build --release --bin timed

# Start all nodes simultaneously
sudo systemctl start timed
```

## Verification After Recovery

On each node, run:

```bash
# Check block height
curl -s localhost:24101/api/blockchain/tip | jq '.height'

# Check block hash
curl -s localhost:24101/api/blockchain/tip | jq '.hash'

# Check masternodes
curl -s localhost:24101/api/masternodes | jq 'length'
```

**All nodes MUST report:**
- ✅ Same height
- ✅ Same block hash at that height  
- ✅ Same masternode count

## Monitoring After Recovery

Watch logs for consensus:

```bash
# Should see same heights across all nodes
watch -n 5 '
  echo "=== reitools ===" && ssh reitools.us "curl -s localhost:24101/api/blockchain/tip | jq .height" &&
  echo "=== Michigan ===" && ssh michigan-node "curl -s localhost:24101/api/blockchain/tip | jq .height" &&
  echo "=== NewYork ===" && ssh newyork-node "curl -s localhost:24101/api/blockchain/tip | jq .height"
'
```

## Why This Happened

1. **Old VRF Bug**: Used `previous_hash` in leader selection
2. **Initial Divergence**: Nodes at different heights selected different leaders
3. **Chain Split**: Each node created its own incompatible blocks
4. **Sync Failure**: Nodes reject each other's blocks (different hashes)
5. **Continued Divergence**: Each node keeps building on its own fork

## Prevention (Already Fixed)

The VRF fix (commit 3c9542e) ensures this won't happen again:
- ✅ Leader selection uses ONLY block height
- ✅ All nodes agree on leader regardless of sync state
- ✅ No more chain splits from VRF disagreement

## Decision Guide

**Choose Option 1 (Full Reset) if:**
- This is a testnet
- You don't have critical data on the chain
- You want the cleanest recovery

**Choose Option 2 (Canonical Chain) if:**
- This is production
- One node has significantly more blocks than others
- You want to preserve transaction history

**Choose Option 3 (Hard Fork) if:**
- You want a completely fresh start
- Testing major protocol changes
- Chain data is corrupted/unusable

## Need Help?

If you need assistance choosing or executing recovery:
1. Check node logs for specific errors
2. Verify network connectivity between nodes
3. Ensure all nodes have the updated VRF fix
4. Test with 2 nodes first before adding more
