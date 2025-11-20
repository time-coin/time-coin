# Wallet Balance Rescan - Quick Reference

## Problem
Masternode rewards disappear when the node is reset because wallet balance was not persisted or recalculated from the blockchain.

## Solution
Implemented automatic and manual blockchain rescanning to ensure wallet balances are always accurate.

## New Features

### 1. Automatic Balance Sync on Startup
- Node automatically syncs wallet balance from UTXO set on startup
- Displays balance and UTXO count
- No user action required

### 2. Manual Rescan Command
```bash
# Rescan node's wallet
time-cli wallet rescan

# Rescan specific address
time-cli wallet rescan --address TIME1abc...xyz

# JSON output
time-cli --json wallet rescan
```

## How It Works

1. **UTXO-Based** - Balance is calculated from blockchain's UTXO (Unspent Transaction Output) set
2. **Indexed Lookup** - UTXOs are indexed by address for fast queries
3. **Always Accurate** - Balance reflects the actual spendable coins on the blockchain

## Example Output

```
🔄 Syncing wallet balance...
💰 Wallet balance synced: 15.5 TIME (3 UTXOs)
```

## Files Changed

- `cli/src/bin/time-cli.rs` - Added `Rescan` command
- `cli/src/main.rs` - Added automatic balance sync on startup

## Benefits

✅ **Persistent Balance** - Never lose track of rewards after restart  
✅ **Automatic** - Syncs on every startup  
✅ **Manual Control** - Rescan anytime with CLI command  
✅ **Fast** - UTXO lookups are indexed and efficient  
✅ **Transparent** - See exactly which UTXOs contribute to balance  

## See Also

- Full documentation: `docs/WALLET_BALANCE_RESCAN.md`
- API documentation: `api/src/wallet_sync_handlers.rs`
