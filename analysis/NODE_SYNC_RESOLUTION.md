# Node Sync Discrepancy Resolution Guide

**Date:** 2025-11-30  
**Issue:** One node substantially ahead of others

## Step 1: Diagnose the Situation

### Check All Node Heights

On each node, run:
```bash
time-cli blockchain-info
```

Record:
- Node IP
- Current height
- Chain tip hash
- Genesis hash

Example:
```
Node A: 165.232.154.150 - Height 150, Hash abc123...
Node B: 192.168.1.100 - Height 128, Hash def456...
Node C: 192.168.1.101 - Height 128, Hash def456...
Node D: 192.168.1.102 - Height 127, Hash ghi789...
```

### Determine Network Consensus

**Network consensus = what the majority of nodes agree on**

In the example above:
- Node A is ahead (150) - **SUSPICIOUS**
- Nodes B, C are at 128 with same hash - **LIKELY CONSENSUS**
- Node D is at 127 - catching up

**Rule:** If one node is significantly ahead of all others, it's likely on a fork.

---

## Step 2: Verify Which Chain is Correct

### Option A: Check Block Hashes at Same Height

On all nodes, check hash at the same height (use the lowest common height):

```bash
# On each node
time-cli get-block-hash 127
```

**If hashes match:** Chains are consistent up to that point
**If hashes differ:** Fork occurred before that height

### Option B: Check Genesis Hash

```bash
# On each node
time-cli blockchain-info | grep genesis
```

**All nodes MUST have the same genesis hash** or they're on completely different chains.

---

## Step 3: Resolution Strategy

### Scenario 1: One Node Ahead, Others Agree (Most Common)

**Symptoms:**
- Node A: Height 150
- Nodes B, C, D: Height 128 with same hash
- Node A started accepting invalid blocks or forked

**Solution: Rollback the ahead node to consensus height**

On Node A:
```bash
# Stop the node
sudo systemctl stop timed

# Rollback to consensus height (128)
time-cli rollback --height 128

# Start the node
sudo systemctl start timed

# Watch it sync
sudo journalctl -u timed -f
```

The node will:
1. Rollback to height 128
2. Connect to peers
3. Download blocks 129+ from network consensus
4. Catch up with correct chain

---

### Scenario 2: Majority Behind, One Node Ahead (Check First!)

**Symptoms:**
- Node A: Height 150
- Nodes B, C, D: Height 128

**Before rolling back, verify:**

1. **Is Node A producing valid blocks?**
   ```bash
   # On Node A
   time-cli get-block 149
   # Check if block looks valid (has transactions, valid masternode, etc)
   ```

2. **Can other nodes see Node A?**
   ```bash
   # On Node B
   time-cli peers
   # Is 165.232.154.150 listed?
   ```

3. **Are other nodes stuck or progressing?**
   ```bash
   # On Node B
   sudo journalctl -u timed -n 50
   # Are they trying to sync? Any errors?
   ```

**If Node A is correct and others are stuck:**

On Nodes B, C, D:
```bash
# Stop node
sudo systemctl stop timed

# Force sync from Node A
time-cli sync-from-peer 165.232.154.150

# Start node
sudo systemctl start timed
```

---

### Scenario 3: Complete Chain Divergence (Nuclear Option)

**Symptoms:**
- Different genesis hashes
- Completely incompatible chains
- Network split

**Solution: Reset to network consensus**

**Step 1: Identify Authoritative Node**
- Which node has the "correct" chain?
- Usually the one that's been running longest
- Or the one masternodes are using

**Step 2: Reset Divergent Nodes**

On each divergent node:
```bash
# Stop node
sudo systemctl stop timed

# Backup current chain (just in case)
sudo cp -r /var/lib/time-coin/blockchain /var/lib/time-coin/blockchain.backup

# Remove blockchain data
sudo rm -rf /var/lib/time-coin/blockchain/*

# Start node (will sync from genesis)
sudo systemctl start timed

# Watch sync
sudo journalctl -u timed -f
```

---

## Step 4: Verify Sync Progress

After applying fix, monitor all nodes:

```bash
# Watch logs
sudo journalctl -u timed -f

# Check height every 30 seconds
watch -n 30 'time-cli blockchain-info'

# Check peers
time-cli peers
```

**Expected behavior:**
- Rolled back node re-syncs from consensus
- All nodes converge on same height
- Chain tip hashes match
- Block production continues normally

---

## Step 5: Prevent Future Divergence

### Enable Fork Detection

Ensure nodes have proper fork detection:
```bash
# Should be enabled by default in latest version
# Check logs for fork detection messages
sudo journalctl -u timed | grep -i "fork"
```

### Monitor Node Sync

Set up alerting:
```bash
# Check if nodes are more than 5 blocks apart
#!/bin/bash
HEIGHTS=$(for node in node1 node2 node3; do
  ssh $node 'time-cli blockchain-info | grep height'
done | awk '{print $2}')

MAX=$(echo "$HEIGHTS" | sort -n | tail -1)
MIN=$(echo "$HEIGHTS" | sort -n | head -1)
DIFF=$((MAX - MIN))

if [ $DIFF -gt 5 ]; then
  echo "WARNING: Nodes diverged by $DIFF blocks!"
fi
```

### Keep Nodes Updated

```bash
# On each node
cd /path/to/time-coin
git pull origin main
cargo build --release
sudo systemctl restart timed
```

---

## Quick Decision Tree

```
Is one node significantly ahead?
├─ YES
│  ├─ Do other nodes agree with each other? (same height & hash)
│  │  ├─ YES → Rollback the ahead node to consensus height
│  │  └─ NO → All nodes diverged, need full analysis
│  └─ Check genesis hashes
│     ├─ All same → Rollback ahead node
│     └─ Different → Complete chain reset needed
└─ NO
   └─ Normal sync lag (< 5 blocks) → Wait, will catch up
```

---

## Common Mistakes to Avoid

❌ **Don't assume the ahead node is correct**
   - Usually ahead = on a fork
   
❌ **Don't rollback without checking consensus**
   - Verify what the majority agrees on first
   
❌ **Don't delete blockchain data as first resort**
   - Try rollback first, reset only if necessary
   
❌ **Don't quarantine peers during investigation**
   - Fork detection might trigger quarantine
   - Clear quarantine: `time-cli clear-quarantine <ip>`

---

## Example: Real Resolution

**Scenario:** Node A at 150, Nodes B/C/D at 128

**Step 1: Check consensus**
```bash
# Node B
$ time-cli blockchain-info
Height: 128
Hash: 9a81c7599d8eed97...

# Node C  
$ time-cli blockchain-info
Height: 128
Hash: 9a81c7599d8eed97...  # Same as B ✓

# Node D
$ time-cli blockchain-info  
Height: 128
Hash: 9a81c7599d8eed97...  # Same as B ✓

# Node A
$ time-cli blockchain-info
Height: 150
Hash: abc123def456...      # Different! ✗
```

**Consensus: Height 128, Hash 9a81c7599d8eed97...**

**Step 2: Rollback Node A**
```bash
# On Node A
sudo systemctl stop timed
time-cli rollback --height 128
sudo systemctl start timed
```

**Step 3: Verify**
```bash
# Wait 2 minutes, then check Node A
$ time-cli blockchain-info
Height: 129
Hash: <matching network>
# Syncing from correct chain ✓
```

**Step 4: Monitor convergence**
```bash
# After 10 minutes
$ time-cli blockchain-info
Height: 135
# All nodes should be at same height ±1-2 blocks
```

---

## Need Help?

1. **Check logs:** `sudo journalctl -u timed -f`
2. **Check peers:** `time-cli peers`
3. **Check quarantine:** `time-cli quarantine-status`
4. **Clear quarantine if needed:** `time-cli clear-quarantine <ip>`

## Summary

**Default action when one node is ahead:**
1. Verify network consensus (majority of nodes)
2. Rollback the ahead node to consensus height
3. Let it re-sync from network
4. Monitor convergence

**Only reset blockchain data if:**
- Genesis hashes differ
- Rollback fails
- Chain is completely corrupted
