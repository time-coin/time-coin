# CLI Wallet Database Access Issue

## Problem

The `time-cli wallet balance` command shows:
```
This wallet command requires local database access
```

This means `time-cli` needs to be run from the directory where the wallet database is located, or you need to specify the data directory.

## Solution

### Option 1: Find and Use the Wallet Database

```bash
# Find where wallet.db is located
find /root -name "wallet.db" 2>/dev/null
find /home -name "wallet.db" 2>/dev/null

# Common locations:
ls -la ~/.time-coin/testnet/wallet.db
ls -la ~/.time-coin/mainnet/wallet.db
ls -la /root/.time-coin/testnet/wallet.db
```

Once found, run from that directory:
```bash
cd ~/.time-coin/testnet  # or wherever wallet.db is
time-cli wallet balance
```

### Option 2: Check Via API

If the masternode has API access to the wallet:

```bash
# Check wallet info via API
curl -s http://localhost:24101/wallet/info

# Or check UTXOs
curl -s http://localhost:24101/wallet/utxos
```

### Option 3: Check Transaction in Logs

Look for the full transaction details from when it was created:

```bash
# Get the transaction details from logs
journalctl -u timed --since "3 hours ago" | grep -B10 -A10 "TIME0mqL8BerCYvUFNyWDaLWKg24AG7BUKhpvNa"
```

## What We Know

From your logs:
```
Nov 25 04:47:18 reitools.us timed[1072033]:    To:     TIME0mqL8BerCYvUFNyWDaLWKg24AG7BUKhpvNa
```

This shows the transaction was **created** but we need to know:
1. What was the amount?
2. What was the TXID?
3. Did it get broadcast successfully?
4. Is it in mempool or was it rejected?

## Get More Details

```bash
# Get the full transaction creation log entry
journalctl -u timed --since "3 hours ago" | grep -B20 "TIME0mqL8BerCYvUFNyWDaLWKg24AG7BUKhpvNa" | head -30

# Check if there were any errors after the transaction
journalctl -u timed --since "3 hours ago" | grep -A5 "TIME0mqL8BerCYvUFNyWDaLWKg24AG7BUKhpvNa"

# Check current mempool
curl -s http://localhost:24101/mempool | grep -i "TIME0mqL8"
```

## Most Likely Scenario

Based on the double-spend errors we saw earlier:

1. ✅ Transaction was created (04:47:18)
2. ❌ Transaction was rejected (double-spend - previous txs stuck in mempool)
3. ⚠️ Coins were NOT deducted from CLI wallet
4. ✅ Mempool was cleared by restart
5. 💡 **Coins should still be in your CLI wallet**

## Next Steps

1. **Find wallet.db location**
2. **Check actual balance**
3. **Try sending again** (now that address derivation is fixed and mempool is clear)

---

**Date**: 2025-11-25 06:30 UTC
