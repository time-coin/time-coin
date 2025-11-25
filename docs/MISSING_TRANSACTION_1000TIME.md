# Missing 1000 Coin Transaction - Investigation

## Transaction Details

**To Address**: `TIME0mqL8BerCYvUFNyWDaLWKg24AG7BUKhpvNa`  
**Amount**: 1000 TIME  
**Status**: Not showing in GUI wallet

## Investigation Steps

### 1. Check if Transaction is in a Block

```bash
# On masternode
cd ~/time-coin

# Search all blocks for the address
for i in {0..50}; do
  echo "Checking block $i..."
  time-cli block $i 2>/dev/null | grep -i "TIME0mqL8BerCYvUFNyWDaLWKg24AG7BUKhpvNa" && echo "Found in block $i!"
done
```

### 2. Check Mempool

```bash
curl http://localhost:24101/mempool | jq '.transactions[] | select(.outputs[].address == "TIME0mqL8BerCYvUFNyWDaLWKg24AG7BUKhpvNa")'
```

### 3. Check Transaction Hash

If you have the transaction hash from when you sent it:

```bash
time-cli transaction <TXID>
```

### 4. Verify Address Belongs to GUI Wallet

The critical question: **Does the GUI wallet own this address?**

Check in the GUI:
- Look for a "Receive" tab or "Addresses" section
- See if `TIME0mqL8BerCYvUFNyWDaLWKg24AG7BUKhpvNa` is listed

**If NOT listed** → The address doesn't belong to this wallet!

### 5. Check Address Derivation

The GUI wallet uses xpub to derive addresses. Let's verify:

```bash
# On your Windows machine where wallet-gui runs
cd ~/projects/time-coin

# Check what addresses the wallet derives
# Look for "Derived addresses" or similar output when wallet starts
RUST_LOG=debug ./target/release/wallet-gui.exe 2>&1 | grep -i address
```

## Common Issues

### Issue 1: Address Doesn't Belong to This Wallet

**Symptoms**: 
- GUI doesn't show the address in its address list
- 1000 coins "missing"

**Cause**: You sent to an address from a different wallet (maybe CLI wallet's change address?)

**Solution**: 
```bash
# Check which wallet owns this address
time-cli wallet address

# If it shows TIME0mqL8... then the CLI wallet owns it!
time-cli wallet balance  # Should show the 1000 coins
```

### Issue 2: Transaction Not in a Block

**Symptoms**:
- Transaction in mempool but not in blockchain
- Instant finality didn't work (voting failed)

**Cause**: Transaction stuck in mempool (like we saw earlier)

**Solution**:
```bash
# Check mempool
curl http://localhost:24101/mempool | jq

# If stuck, restart masternode
sudo systemctl restart timed
```

### Issue 3: GUI Not Scanning Right Address Range

**Symptoms**:
- Address IS in blockchain
- GUI scans but doesn't find it

**Cause**: GUI only scans first 20 addresses (index 0-19)

**Solution**: Check which index the address is at. If > 19, need to increase scan range.

## Quick Diagnostic

Run this on masternode to find the transaction:

```bash
#!/bin/bash
TARGET_ADDR="TIME0mqL8BerCYvUFNyWDaLWKg24AG7BUKhpvNa"

echo "Searching for address: $TARGET_ADDR"
echo ""

# Check mempool
echo "=== MEMPOOL CHECK ==="
curl -s http://localhost:24101/mempool | jq -r '.transactions[]? | select(.outputs[]?.address == "'$TARGET_ADDR'") | "FOUND IN MEMPOOL: TX " + .txid'

# Check blockchain
echo ""
echo "=== BLOCKCHAIN CHECK ==="
for height in {0..50}; do
  result=$(time-cli block $height 2>/dev/null | grep -c "$TARGET_ADDR")
  if [ "$result" -gt 0 ]; then
    echo "FOUND IN BLOCK $height"
    time-cli block $height 2>/dev/null | grep -A5 -B5 "$TARGET_ADDR"
  fi
done

echo ""
echo "=== SCAN COMPLETE ==="
```

Save as `find_transaction.sh`, make executable, and run:
```bash
chmod +x find_transaction.sh
./find_transaction.sh
```

## Expected Outcomes

### Outcome A: Found in Block
```
FOUND IN BLOCK 45
  "outputs": [
    {
      "address": "TIME0mqL8BerCYvUFNyWDaLWKg24AG7BUKhpvNa",
      "amount": 1000
    }
  ]
```
→ **Transaction succeeded, but GUI isn't recognizing the address**

### Outcome B: Found in Mempool
```
FOUND IN MEMPOOL: TX a8b0c781d9c74e83
```
→ **Transaction pending, not finalized**

### Outcome C: Not Found
```
=== SCAN COMPLETE ===
(no results)
```
→ **Transaction never made it to blockchain or mempool**

## Next Steps Based on Results

**If in block**: 
1. Verify address belongs to GUI wallet
2. Check GUI is scanning correct address range
3. Reconnect GUI to trigger rescan

**If in mempool**:
1. Wait for next block OR
2. Restart masternode to mine it

**If not found**:
1. Check CLI wallet - coins might have returned to sender
2. Check transaction actually sent (look for TXID in CLI wallet history)

---

**Date**: 2025-11-25  
**Time**: ~06:05 UTC  
**Status**: Investigating
