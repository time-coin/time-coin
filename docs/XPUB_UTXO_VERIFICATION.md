# XPUB UTXO Scanning - Verification Guide

## Current Status

✅ **Masternode received xpub registration**: `xpub6CZosSrjGTSQi9Hb...`  
✅ **Scanned blockchain successfully**  
ℹ️ **Result**: No UTXOs found

## Why "No UTXOs Found"?

This is **NORMAL** if:
1. ✅ **New wallet** - No transactions sent to this wallet yet
2. ✅ **All UTXOs spent** - Previous transactions consumed all coins
3. ✅ **Waiting for first transaction**

This is **UNEXPECTED** if:
1. ❌ You see coins in CLI wallet but not GUI wallet
2. ❌ Address derivation mismatch between wallet types

## Verification Steps

### 1. Check CLI Wallet Balance

```bash
time-cli wallet balance
```

**Expected**: Shows available UTXOs and balance

### 2. Check CLI Wallet Address

```bash
time-cli wallet address
```

**Expected**: Shows your receiving address (e.g., `TIME0m...`)

### 3. Send Test Transaction to GUI Wallet

From CLI wallet, send to GUI wallet's address:

**First, get GUI wallet address from the GUI**, then:

```bash
# Send 10 coins to test
time-cli wallet send TIME0<GUI_WALLET_ADDRESS> 10
```

### 4. Verify Transaction in Logs

Watch masternode logs:
```bash
journalctl -u timed -f
```

You should see:
```
📝 Added transaction XXX to mempool
🚀 Triggering instant finality...
✅ Transaction finalized
```

### 5. Check GUI Wallet Again

The GUI should receive a `UtxoUpdate` message with the new UTXO.

## Understanding the Flow

**When you send TO the GUI wallet:**

1. Transaction gets mined into a block
2. Block contains output to GUI wallet's address
3. When GUI connects: masternode scans all blocks
4. Finds outputs matching GUI's addresses (derived from xpub)
5. Sends `UtxoUpdate` to GUI with found UTXOs

**Current situation:**
- ✅ GUI wallet registered successfully
- ✅ Masternode scanned blockchain
- ℹ️ No outputs found for GUI's addresses (expected for new wallet)

## Test Scenario

### Generate a UTXO for the GUI Wallet:

1. **Get GUI receiving address** (from GUI interface)

2. **Send from CLI**:
   ```bash
   time-cli wallet send <GUI_ADDRESS> 5
   ```

3. **Wait for block** (or instant finality)

4. **Reconnect GUI** (or it gets notified automatically)

5. **GUI shows balance**: 5 TIME

## Debugging Address Derivation

If you want to verify address derivation is working:

### Check what addresses masternode derives:

Add debug logging to line 1703 in `cli/src/main.rs`:

```rust
if let Ok(address) = wallet::xpub_to_address(&xpub, 0, i, WalletNetworkType::Testnet) {
    println!("  Derived address {}: {}", i, address);  // ADD THIS
    // ... rest of code
```

Then rebuild and restart:
```bash
cargo build --release --bin timed
sudo systemctl restart timed
```

When GUI connects, you'll see all 20 derived addresses in logs.

### Compare with GUI wallet addresses:

The GUI should show the same addresses (in the receive tab or address list).

If they match → **System working correctly, just no coins yet**  
If they don't match → **Bug in address derivation**

## Summary

**Most likely**: Your GUI wallet is brand new and hasn't received any coins yet!

**Next step**: Send a test transaction from CLI wallet to GUI wallet to verify the system works.

**Expected result**: 
- Transaction appears in GUI
- Balance updates
- UTXO shows in GUI wallet

---

**Date**: 2025-11-25  
**Status**: Normal operation - new wallet with no transactions yet
