# Testnet Reset Instructions - Consensus Migration

## What Happened
The testnet has blocks created by the **old leader-based BFT consensus**. We just deployed **new deterministic consensus** code. The two systems create blocks with different hashes, causing incompatibility.

## What You Need To Do

### Step 1: Coordinate with All Node Operators
Since testnet blocks need to be recreated, all operators must reset at the same time.

### Step 2: On EACH Node, Run:
```bash
# Stop the node
sudo systemctl stop timed

# Clear blockchain data (keeps wallet & masternode key)
sudo rm -rf /var/lib/time-coin/blockchain/*
sudo rm -rf /var/lib/time-coin/block_height.txt
sudo rm -rf /var/lib/time-coin/quarantine.json

# Pull latest code
cd /root/time-coin-node
git pull

# Build new version
./build.sh

# Restart node
sudo systemctl start timed
```

Or use the automated script:
```bash
cd /root/time-coin-node
./reset-testnet.sh
```

### Step 3: Monitor First Block Creation
With 10-minute block interval:
```bash
journalctl -u timed -f
```

Look for:
```
🔷 Deterministic Consensus - Block #1
   ✓ Created local block: ...
   📡 Requesting block 1 from X peers...
   ✓ Received X peer blocks
   ✅ CONSENSUS: X/X nodes agree (100%)
   ✅ Block finalized and accepted
```

### Step 4: Verify Consensus
After 20 minutes (2 blocks), check all nodes have same chain:

```bash
# On each node:
time-cli blockchain info
```

All nodes should show:
- Same block height
- Same latest block hash
- No quarantined peers

## Why This Fixed The Problem

### Old System (Broken):
- Node A (leader): Creates block with hash `abc123...`
- Node B: Waits for leader, times out
- Node C: Waits for leader, downloads wrong block
- **Result**: Consensus fails, nodes quarantine each other

### New System (Working):
- All nodes: Create block deterministically at :00 seconds
- All nodes: Compare blocks → all identical!
- All nodes: Accept block immediately
- **Result**: Instant consensus, no leader needed

## Expected Behavior After Reset

1. **Block 1 (10 minutes after start)**:
   - All nodes create identical block
   - Consensus achieved in <5 seconds
   - Block finalized

2. **Block 2 (20 minutes after start)**:
   - Same process
   - Previous hash correctly points to Block 1
   - Chain builds properly

3. **Ongoing**:
   - Every 10 minutes, new block
   - 100% consensus rate
   - No timeouts or failures

## Reverting to 24-Hour Blocks Later

Once testing is complete:
1. Change `BLOCK_INTERVAL` back to 24 hours in code
2. Rebuild and restart all nodes
3. No reset needed - existing blocks are valid

## Questions?

- Discord: [your channel]
- Or check logs: `journalctl -u timed -f`
