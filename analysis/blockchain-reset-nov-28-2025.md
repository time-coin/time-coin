# Blockchain Reset - November 28, 2025

## Issue Discovered

**Problem**: Existing blockchain has invalid block hashes that don't chain correctly.

### Symptoms
- Block 2's `previous_hash` points to genesis (9a81c7...) instead of block 1 (8a5817...)
- All peer nodes get quarantined during sync due to "invalid chain"
- Fork detection continuously triggers false positives
- New nodes cannot sync from existing nodes

### Root Cause
The old **leader-based BFT consensus** system had bugs in block hash calculation that caused:
1. Truncated hash strings (16 chars instead of full 64-char hex)
2. Incorrect `previous_hash` references in block headers
3. Blocks that appear valid individually but don't form a valid chain

## Solution

**Complete blockchain reset** using the new **deterministic consensus** system.

### Why This Works
The new consensus system:
- All nodes generate identical blocks deterministically
- Proper 64-character SHA3-256 hashes
- Correct `previous_hash` chain linking
- No leader election failures or timeouts

### Reset Process

1. **Stop all nodes**:
   ```bash
   sudo systemctl stop timed
   ```

2. **Run reset script** (keeps only genesis):
   ```bash
   ./reset-blockchain-keep-genesis.sh
   ```

3. **Restart nodes**:
   ```bash
   sudo systemctl start timed
   ```

4. **Nodes will recreate blocks** via deterministic consensus every 10 minutes (testnet)

### What Gets Reset
- ✅ Deleted: Blocks 1-47 (invalid hashes)
- ✅ Kept: Block 0 (genesis - still valid)
- ✅ Kept: Wallet balances (stored separately)
- ✅ Kept: Peer connections
- ✅ Kept: Configuration

### Expected Behavior After Reset

1. All nodes start at height 0 (genesis only)
2. At next 10-minute interval (03:40, 03:50, etc.):
   - Each node generates deterministic block #1
   - Nodes compare blocks with peers
   - If 2/3+ match → block finalized
   - If differences → reconciliation process
3. Process repeats for historical blocks 2-6789
4. Once caught up, normal block production continues

### Timeline

- **Genesis**: October 12, 2025 00:00:00 UTC (timestamp: 1760227200)
- **Reset Date**: November 28, 2025 03:40:00 UTC
- **Blocks to Recreate**: ~6789 blocks (47 days × 1 block/day, but using 10min intervals for testing)

### Monitoring

Watch the consensus process:
```bash
journalctl -u timed -f | grep -E "(Consensus|BLOCK|consensus_block)"
```

Expected log output:
```
🔷 Deterministic Consensus - Block #1
   ✓ Created local block: consensus_block_1
   📡 Requesting block 1 from 4 peers...
   ✓ Received 4 peer blocks
   📊 Consensus check: 5/5 matching
   ✅ CONSENSUS REACHED - Block finalized!
```

### Verification

After reset, verify chain integrity:
```bash
# Check block height
curl http://localhost:24101/blockchain/height

# Check specific blocks chain correctly
curl http://localhost:24101/blockchain/block/1
# Verify previous_hash matches genesis hash

curl http://localhost:24101/blockchain/block/2  
# Verify previous_hash matches block 1 hash
```

## Prevention

Future updates will include:
1. ✅ Automated chain validation on startup
2. ✅ Quarantine system for invalid chains
3. ✅ Deterministic block recreation
4. 🔄 Checkpoint system (planned)
5. 🔄 Block hash verification in consensus (planned)

## Notes

- This is a **testnet reset** - no mainnet impact
- **Wallet balances preserved** - they're in the UTXO database, not blocks
- **Genesis unchanged** - all nodes must have matching genesis (9a81c7599d8eed97...)
- **10-minute blocks** are temporary for testing
- **Production will use 24-hour blocks** as originally designed
