# Time-Coin Sync Fix - Quick Reference Card

## 🚀 Deployment Commands

```bash
# Update and build on each node
ssh node1 'cd ~/time-coin && git pull && cargo build --release && systemctl restart timed'
ssh node2 'cd ~/time-coin && git pull && cargo build --release && systemctl restart timed'
```

## 🔍 Key Log Patterns to Monitor

### ✅ HEALTHY Indicators

```
🔍 VRF Configuration:
   Network: mainnet
   Dev mode: false
   Selector: DefaultVRFSelector (SHA256-based, height-only seed)

🔐 Leader election for block 176:
   Prev hash: abcd1234... (note: NOT used in VRF seed)
   Masternode count: 6
👑 Selected leader: 192.168.1.10

📋 Masternode list synced: 5 → 6 nodes

✅ Network healthy: 4/5 peers responding
```

### ⚠️ WARNING Indicators

```
⚠️  Rejecting masternode list: only 2 nodes (need 3+ for BFT)
   → Wait for more masternodes to join

❌ Network unhealthy: only 1/5 peers responding
   → Check network connectivity

❌ Not enough peers: 2 < 3
   → Wait for network to stabilize
```

## 📊 Verification Commands

### Check Leader Selection Consistency

```bash
# All nodes should show same leader for each block height
for node in node1 node2 node3; do
  echo "=== $node ===" && \
  ssh $node 'grep "Selected leader" ~/.timecoin/logs/node.log | tail -5'
done
```

**Expected:** Same leader for each height across all nodes

### Monitor Block Heights

```bash
# Real-time height monitoring
watch -n 5 'for node in node1 node2 node3; do \
  echo "=== $node ===" && \
  ssh $node "grep \"Block Height\" ~/.timecoin/logs/node.log | tail -1"; \
done'
```

**Expected:** Heights differ by at most 1-2 blocks

### Check Masternode Counts

```bash
# Verify masternode list convergence
for node in node1 node2 node3; do
  echo "=== $node ===" && \
  ssh $node 'grep "Masternode list synced" ~/.timecoin/logs/node.log | tail -3'
done
```

**Expected:** All nodes converge to same count

## 🐛 Troubleshooting

### Problem: Nodes select different leaders

**Symptoms:**
```
Node1: 👑 Selected leader: 192.168.1.10
Node2: 👑 Selected leader: 192.168.1.20
```

**Diagnosis:**
```bash
# Check masternode counts match
grep "Masternode count:" ~/.timecoin/logs/node.log | tail -5
```

**Solution:** Ensure all nodes have same masternode list

---

### Problem: Node stuck at lower block height

**Symptoms:**
```
Node1: Block Height: 175
Node2: Block Height: 180
```

**Diagnosis:**
```bash
# Check if network health checks are failing
grep "Network" ~/.timecoin/logs/node.log | tail -10
```

**Solution:** Fix network connectivity or wait for catch-up (Phase 2 feature)

---

### Problem: Masternode list rejected

**Symptoms:**
```
⚠️  Rejecting masternode list: only 2 nodes (need 3+ for BFT)
```

**Diagnosis:** Not enough masternodes for Byzantine Fault Tolerance

**Solution:** 
- Wait for more masternodes to join
- If testing, use dev mode: `--dev-mode`

---

## 📈 Success Metrics

After deployment, you should see:

1. ✅ **Consistent leaders** - All nodes log same leader for each height
2. ✅ **Stable masternode count** - Count doesn't fluctuate wildly
3. ✅ **Network health passes** - Health checks succeed before block production
4. ✅ **Heights converge** - Block heights stay within 1-2 of each other

## 🔧 Configuration Tips

### Development Mode

```bash
# Allows < 3 masternodes (for testing)
./timed --dev-mode
```

### Log Verbosity

The new logs use emojis for easy scanning:
- 🔍 Configuration
- 🔐 Cryptographic operations (VRF)
- 👑 Leadership decisions
- 📋 List management
- ✅ Success states
- ⚠️ Warnings
- ❌ Errors

Use grep with these to filter:
```bash
tail -f ~/.timecoin/logs/node.log | grep "👑"  # Leader selection
tail -f ~/.timecoin/logs/node.log | grep "⚠️"  # Warnings only
```

## 📞 Getting Help

If issues persist:

1. Collect logs from ALL nodes:
   ```bash
   for node in node1 node2 node3; do
     ssh $node 'cat ~/.timecoin/logs/node.log' > ${node}_logs.txt
   done
   ```

2. Note the issue pattern:
   - Different leaders selected?
   - Network health failing?
   - Heights diverging?

3. Include in bug report:
   - Log excerpts with emojis
   - Masternode count from each node
   - Network topology

## 🎯 Quick Sanity Check

Run this on each node after deployment:

```bash
#!/bin/bash
echo "=== Time-Coin Health Check ==="
echo ""
echo "1. Service Status:"
systemctl status timed | grep Active
echo ""
echo "2. Last 5 Leader Selections:"
grep "Selected leader" ~/.timecoin/logs/node.log | tail -5
echo ""
echo "3. Current Masternode Count:"
grep "Masternode count:" ~/.timecoin/logs/node.log | tail -1
echo ""
echo "4. Network Health:"
grep "Network health" ~/.timecoin/logs/node.log | tail -3
echo ""
echo "5. Recent Errors:"
grep "⚠️\|❌" ~/.timecoin/logs/node.log | tail -5
```

Save as `health_check.sh` and run after deployment.

---

**Remember:** The VRF already uses height-only seed. These fixes add observability and validation.
