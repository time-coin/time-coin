# Next Steps for Network Stability
**Date:** 2025-11-29  
**Current Status:** TCP keep-alive fix deployed, awaiting node updates

## 🎯 Immediate Actions (CRITICAL)

### 1. Update All Testnet Nodes
**Priority:** HIGHEST  
**Time Required:** 15 minutes per node

Each node needs to:
```bash
cd /root/time-coin-node
git pull
cargo build --release
systemctl restart timed
```

**Monitor during restart:**
```bash
journalctl -u timed -f | grep -E "TCP keep-alive|connection|vote"
```

**Expected log output:**
- NO "Broken pipe" errors
- Connections stay at 3-4 peers
- Votes get "✓ Vote sent and ACKed by" messages

### 2. Monitor Connection Stability
**Priority:** HIGH  
**Duration:** 30 minutes after all nodes update

Watch for:
```bash
# Count active connections over time
watch -n 10 'journalctl -u timed --since "5 minutes ago" | grep "Available TCP connections"'

# Check for broken pipes
journalctl -u timed --since "10 minutes ago" | grep "Broken pipe"
# Should return NOTHING

# Monitor vote success rate
journalctl -u timed --since "10 minutes ago" | grep "Vote broadcast"
# Should show 3-4 successful votes
```

### 3. Verify Consensus Works
**Priority:** HIGH  
**Time:** Check at next 10-minute block interval

```bash
# Watch for consensus reaching 3/4 votes
journalctl -u timed -f | grep -E "CONSENSUS|votes"
```

**Success criteria:**
- ✅ Blocks reach consensus with 3/4 votes
- ✅ No "Insufficient votes" messages
- ✅ No "Vote stalled" messages
- ✅ Blocks finalize in <5 seconds

---

## 🔧 If Issues Persist

### Scenario 1: Still Seeing "Broken Pipe"

**Diagnosis:**
```bash
# Check socket settings
ss -tnp | grep :24100 | head -5
# Look for "keepalive" in output
```

**Fix:**
- Verify all nodes pulled latest code (commit 59078cd)
- Check systemd service file doesn't override socket settings
- Ensure no firewall is killing connections aggressively

### Scenario 2: Votes Still Not Getting Through

**Diagnosis:**
```bash
# Check for ACK timeouts
journalctl -u timed --since "5 minutes ago" | grep "ACK"
```

**If seeing "Timeout waiting for ACK":**
- Check network latency: `ping <peer_ip>`
- Verify peer is processing votes (check their logs)
- Increase ACK timeout in code (currently 5 seconds)

### Scenario 3: Connections Stable but No Votes

**Diagnosis:**
```bash
# Check consensus mode
journalctl -u timed --since "1 minute ago" | grep "Consensus Status"
```

**If in BOOTSTRAP mode:**
- Need 3+ masternodes for BFT
- Check masternode registration: `time-cli network peers`
- Verify wallet addresses in handshakes

---

## 📊 Success Metrics

After all nodes update, within 30 minutes:

### Connection Stability
```
✓ All nodes maintain 3-4 connections
✓ Zero "Broken pipe" errors
✓ Connections last >10 minutes without drops
```

### Vote Delivery
```
✓ 75%+ vote success rate (3/4 nodes)
✓ All votes get ACKed within 1 second
✓ Zero "Timeout waiting for ACK"
```

### Consensus Performance
```
✓ Blocks reach consensus in <5 seconds
✓ Zero "Insufficient votes" errors
✓ 100% block finalization rate
```

---

## 🚀 Once Stable

### Short Term (Next Session)

1. **Enable block recreation** on all nodes
   - Set `block_recreation = true` in config
   - Allows catch-up for missed blocks

2. **Monitor for 24 hours**
   - Check logs daily
   - Verify no regressions
   - Track vote success rates

3. **Document baseline metrics**
   - Connection uptime
   - Vote success rate
   - Consensus latency
   - Block finalization time

### Medium Term (This Week)

1. **Add connection health monitoring**
   - Periodic ping/pong messages
   - Detect dead connections proactively
   - Auto-reconnect on failure

2. **Implement connection pool management**
   - Max connections per peer
   - Connection reuse strategy
   - Graceful connection shutdown

3. **Add metrics and monitoring**
   - Prometheus metrics
   - Connection state tracking
   - Vote delivery statistics

### Long Term (Next Sprint)

1. **Implement true deterministic consensus**
   - Fix masternode ordering
   - Consistent transaction selection
   - Reproducible block hashes

2. **Add fork resolution improvements**
   - Don't replace identical blocks
   - Better longest chain detection
   - Smarter rollback logic

3. **Network resilience features**
   - Gossip protocol for votes
   - Redundant vote paths
   - Network partition recovery

---

## 🆘 Emergency Rollback

If the fix causes worse issues:

```bash
# On each node:
cd /root/time-coin-node
git checkout 11cb51b  # Previous commit
cargo build --release
systemctl restart timed
```

Then investigate logs to understand what went wrong.

---

## 📝 Update Log

Update this section after each deployment:

### 2025-11-29 02:48 UTC
- ✅ TCP keep-alive fix committed (59078cd)
- ⏳ Waiting for node updates
- ⏳ Monitoring for stability

### [Add timestamp when nodes update]
- Node 1 (134.199.175.106): [status]
- Node 2 (161.35.129.70): [status]
- Node 3 (165.232.154.150): [status]
- Node 4 (69.167.168.176): [status]
- Node 5 (50.28.104.50): [status]

### [Add timestamp after monitoring]
- Connection stability: [metric]
- Vote success rate: [metric]
- Consensus working: [yes/no]

---

**Next Review:** After all nodes update + 30 minutes monitoring
