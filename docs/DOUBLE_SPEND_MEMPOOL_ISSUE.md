# Double-Spend Detection - Transaction Stuck in Mempool

## Issue
```
✗ Failed to send transaction: {"error":"internal_error","message":"Failed to add to mempool: Double-spend attempt detected"}
```

## Root Cause

The masternode has transactions in the mempool from previous send attempts (txid: `a8b0c781d9c74e83`, `d75bba45c96f5f3e`). These transactions:

1. ✅ Were created and signed successfully
2. ✅ Were added to mempool
3. ❌ **Never got instant finality votes** (voting protocol was broken)
4. ❌ **Never got included in blocks** (no consensus)
5. ⚠️ Are **still in mempool** consuming the UTXOs

When you try to send again, it sees the same UTXOs are already spent in pending transactions = double-spend detection.

## Why Voting Failed

The transactions show they were broadcast to peers, but the logs don't show any vote responses. This is because:

1. **Old protocol** - Some masternodes may not have the protocol fixes yet
2. **Handshake mismatch** - Peers with old code can't respond to vote requests
3. **Timeout** - Vote requests timeout waiting for responses that never come

## Solution

### **Option 1: Clear Mempool** (Quick Fix)

Restart the masternode to clear the mempool:

```bash
sudo systemctl restart timed
```

Then try sending again:
```bash
time-cli wallet send TIME0n28FubuMU12kqojgrHbvj99xQzmENjmt45 1
```

### **Option 2: Wait for Block** (Automatic)

The transactions will eventually get mined into a block (when the next block is produced), then you can send again.

### **Option 3: Check Mempool** (Diagnostic)

See what's in the mempool:

```bash
curl http://localhost:24101/mempool | jq
```

You should see the pending transactions with their vote status.

## Verification

After clearing mempool or waiting for block, check wallet balance:

```bash
time-cli wallet balance
```

The UTXOs should be available again.

## Long-Term Fix

**Update ALL masternodes** to have the protocol fixes:

```bash
# On each masternode
cd ~/time-coin
git pull
cargo build --release --bin timed
sudo systemctl restart timed
```

Once all masternodes have the latest code with magic byte protocol fixes:
- ✅ Vote requests will work
- ✅ Instant finality will happen
- ✅ Transactions will finalize in <1 second
- ✅ No more stuck transactions

## Status

**Current state:**
- Your masternode: ✅ Latest code
- Other masternodes: ❌ May have old code (some show `0.1.0-10b7cd5`, others `0.1.0-9cd6375`)
- Voting: ❌ Not working (protocol mismatch)
- Mempool: ⚠️ Has stuck transactions

**Need to:**
1. Restart your masternode to clear mempool (immediate)
2. Update other masternodes to latest code (for voting to work)

---

**Date**: 2025-11-25  
**Status**: Pending mempool clear and peer updates
