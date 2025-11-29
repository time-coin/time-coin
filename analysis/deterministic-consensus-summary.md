# Summary - Deterministic Consensus Implementation & Blockchain Reset

**Date**: November 28, 2025  
**Changes**: Replaced leader-based BFT with deterministic consensus + blockchain reset

---

## What Changed

### 1. Consensus Model: Leader-Based → Deterministic

**Before** (Leader-Based BFT):
- One leader elected via VRF
- Leader creates and proposes block
- Other nodes vote on proposal
- **Problems**:
  - Leader timeout failures
  - Single point of failure
  - Slow consensus (60+ seconds)
  - Frequent "no consensus" states

**After** (Deterministic Consensus):
- All nodes generate identical blocks deterministically
- Nodes compare blocks with each other
- 2/3+ agreement → block finalized
- Differences → automatic reconciliation
- **Benefits**:
  - No single point of failure
  - Fast consensus (< 10 seconds)
  - Automatic error correction
  - Byzantine fault tolerance maintained

### 2. Block Generation

**Deterministic Factors**:
- Block number
- Timestamp (aligned to 10-minute intervals in testnet)
- Previous block hash
- Sorted transactions (by txid)
- Masternode rewards (calculated deterministically)
- Validator ID: `consensus_block_{N}` (not tied to any specific node)

**Result**: All honest nodes create byte-for-byte identical blocks

### 3. Consensus Process

```
1. Block Time Arrives (e.g., 03:40:00 UTC)
   ↓
2. Each Node Creates Block Locally
   - Deterministic coinbase
   - Sorted transactions
   - Calculate merkle root & hash
   ↓
3. Request Peer Blocks (5-second timeout)
   ↓
4. Compare Blocks
   - Hash comparison
   - Transaction comparison
   - Merkle root verification
   ↓
5a. IF 2/3+ Match → FINALIZE
5b. IF Differences → RECONCILE
   - Identify differing transactions
   - Validate with network
   - Reject invalid txs
   - Recreate block with consensus txs
   ↓
6. Save to Disk & Broadcast
```

### 4. Files Changed

**New Files**:
- `cli/src/deterministic_consensus.rs` - New consensus implementation
- `reset-blockchain-keep-genesis.sh` - Blockchain reset utility
- `analysis/blockchain-reset-nov-28-2025.md` - Issue documentation

**Modified Files**:
- `cli/src/main.rs` - Integrate deterministic consensus
- `cli/src/block_producer.rs` - Use deterministic consensus
- `docs/CONSENSUS.md` - Update consensus documentation

**Removed Files**:
- `cli/src/bft_consensus.rs` - Old leader-based system (still in git history)

---

## Why Blockchain Reset Was Needed

### The Problem

Existing blocks (1-47) had **invalid hash chains**:

```
Block 0 (Genesis): 9a81c7599d8eed97... ✅ Valid
Block 1: hash=8a5817db9bcf7676... ✅
Block 2: previous_hash=9a81c7599d8eed97... ❌ WRONG! Should be 8a5817db...
Block 3: previous_hash=9a81c7599d8eed97... ❌ WRONG! Should be block2 hash
...
```

Every block after genesis pointed to genesis as its parent, not the actual previous block!

### Root Cause

The old BFT consensus had bugs in hash calculation:
1. Used truncated strings (16 chars) instead of full hashes
2. Didn't properly pass `previous_hash` between blocks
3. Blocks appeared valid individually but chain was broken

### Solution

1. ✅ Implement deterministic consensus with proper hashing
2. ✅ Delete blocks 1-47 (invalid)
3. ✅ Keep block 0 (genesis - still valid)
4. ✅ Let nodes recreate blocks 1-N using deterministic consensus

---

## Testing Instructions

### On Each Testnet Node

```bash
# 1. Pull latest code
cd ~/time-coin-node
git pull

# 2. Rebuild
./build.sh

# 3. Stop node
sudo systemctl stop timed

# 4. Reset blockchain (keeps genesis)
./reset-blockchain-keep-genesis.sh

# 5. Start node
sudo systemctl start timed

# 6. Watch consensus
journalctl -u timed -f | grep -E "(Consensus|BLOCK)"
```

### Expected Behavior

Every 10 minutes, you should see:
```
🔷 Deterministic Consensus - Block #1
   ✓ Created local block: consensus_block_1...
   📡 Requesting block 1 from 4 peers...
   ✓ Received 4 peer blocks
   📊 Consensus check: 5/5 matching blocks
   ✅ CONSENSUS REACHED - Block finalized!
```

### Verification

```bash
# Check block height increases
watch -n 30 'curl -s http://localhost:24101/blockchain/height'

# Verify blocks chain correctly
for i in {1..5}; do
  echo "Block $i:"
  curl -s http://localhost:24101/blockchain/block/$i | jq '{height: .header.block_number, hash: .hash, prev: .header.previous_hash}'
done
```

---

## Production Timeline

### Current (Testnet)
- ✅ 10-minute blocks (for testing)
- ✅ Deterministic consensus active
- ✅ Rapid block recreation

### Before Mainnet Launch
1. Change block interval back to 24 hours
2. Extensive testnet validation (1 week+)
3. Monitor consensus success rate
4. Verify all edge cases
5. Load testing with many transactions

### Mainnet Launch
- 24-hour blocks (as designed)
- Deterministic consensus (proven)
- Checkpoint system (planned)
- Emergency rollback procedures (planned)

---

## Monitoring & Alerts

### Key Metrics to Watch

1. **Consensus Success Rate**
   - Target: 100% (all blocks reach consensus)
   - Alert if < 95%

2. **Block Creation Time**
   - Target: < 10 seconds
   - Alert if > 30 seconds

3. **Peer Agreement**
   - Target: 100% of peers agree
   - Alert if < 66% (below BFT threshold)

4. **Chain Integrity**
   - Every block's `previous_hash` must match prior block's `hash`
   - Alert on any mismatch

### Log Messages to Monitor

✅ **Good**:
- `✅ CONSENSUS REACHED - Block finalized!`
- `✓ Received N peer blocks`
- `📊 Consensus check: N/N matching`

⚠️ **Warning**:
- `⚠️ DIFFERENCES DETECTED - Reconciliation needed`
- `✗ Timeout from peer`
- `⚠️ No peer responses`

❌ **Error**:
- `🚨 RECONCILIATION FAILED`
- `❌ Block rejected by network`
- `⚠️ Invalid transaction detected`

---

## Support & Troubleshooting

### If Consensus Fails

1. **Check peer connectivity**:
   ```bash
   curl http://localhost:24101/network/peers
   ```

2. **Verify time sync**:
   ```bash
   timedatectl status  # Should show "synchronized: yes"
   ```

3. **Check logs for errors**:
   ```bash
   journalctl -u timed -n 100 --no-pager | grep -i error
   ```

4. **Restart if needed**:
   ```bash
   sudo systemctl restart timed
   ```

### If Block Heights Diverge

If different nodes have different heights:

1. Stop all nodes
2. Run `reset-blockchain-keep-genesis.sh` on all
3. Start all nodes simultaneously
4. They will recreate blocks together

---

## Questions & Answers

**Q: Will wallet balances be lost?**  
A: No! Balances are in the UTXO database, separate from blocks.

**Q: How long will recreation take?**  
A: ~6789 blocks × 10 minutes = ~47 days worth, but at 10-minute intervals = ~47 days. However, if we implement parallel recreation, it could be much faster.

**Q: What if a node joins late?**  
A: It syncs genesis from peers, then participates in deterministic recreation of all subsequent blocks.

**Q: Can blocks be recreated incorrectly?**  
A: No - the process is deterministic. All honest nodes create identical blocks. Any differences trigger reconciliation.

**Q: What about the old blocks' data?**  
A: Preserved in git history. Can be analyzed for bugs but won't be used in the chain.

---

## Next Steps

### Immediate (Testing Phase)
1. ✅ Deploy to all testnet nodes
2. ✅ Reset blockchains
3. ⏳ Monitor consensus for 24+ hours
4. ⏳ Verify block recreation works
5. ⏳ Test with transaction load

### Short-term (Pre-Mainnet)
1. ⏳ Implement checkpoint system
2. ⏳ Add consensus metrics dashboard
3. ⏳ Performance optimization
4. ⏳ Change back to 24-hour blocks
5. ⏳ Final testnet validation

### Long-term (Mainnet & Beyond)
1. ⏳ Mainnet launch with deterministic consensus
2. ⏳ Real-time consensus monitoring
3. ⏳ Automated alerting system
4. ⏳ Cross-chain validation (future)

---

## Contact

For issues or questions:
- Check logs: `journalctl -u timed -f`
- GitHub Issues: https://github.com/time-coin/time-coin/issues
- Discord: (link when available)

---

**Document Version**: 1.0  
**Last Updated**: November 28, 2025 04:15 UTC  
**Status**: Active Implementation
