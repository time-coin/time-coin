# Transaction Synchronization Fixes

## Issues Found

### 1. ❌ Wallet Not Receiving Transactions
**Root Cause:** The `sync_wallet_addresses` function in `api/src/wallet_sync_handlers.rs` was creating an empty vector for UTXOs and never populating it.

```rust
// OLD CODE (line 66):
let address_utxos = Vec::new();  // ← Created but never filled!
```

**Fix Applied:** Now properly queries the UTXO set and populates address_utxos with actual data:
```rust
let utxo_entries = blockchain.utxo_set().get_utxos_for_address(address);
for (outpoint, output) in utxo_entries {
    // Find block height and confirmations
    // Create UtxoInfo and add to address_utxos
}
```

### 2. ❌ Wallet GUI Not Syncing
**Root Cause:** The `trigger_transaction_sync()` function in `wallet-gui/src/main.rs` does NOTHING. It just logs a message saying "automatic" but doesn't actually pull data.

```rust
fn trigger_transaction_sync(&mut self) {
    // Transaction sync now happens automatically via WebSocket protocol_client
    // The xpub subscription is already active from the connection established on startup
    log::info!("✅ Wallet sync via TCP WebSocket (automatic - already connected)");
    
    // ← NO ACTUAL CODE TO SYNC!
}
```

**Issue:** The protocol_client has a TODO comment saying subscription isn't actually sent:
```rust
// wallet-gui/src/protocol_client.rs:564
// TODO: Send subscription if already connected
log::info!("xpub subscription queued");
```

### 3. ✅ Mempool Synchronization (Working)
The mempool sync IS implemented and working:
- Periodic sync runs every 30 seconds
- Sends `MempoolQuery` to peers
- Receives `MempoolResponse` and adds missing transactions
- Saves mempool to disk

**Location:** `masternode/src/utxo_integration.rs:686-739`

### 4. ❌ Block Rewards Not in Mempool (By Design)
**This is CORRECT behavior!**
- Block rewards are coinbase transactions
- They go directly into blocks when mined
- They do NOT go through mempool
- Wallet should see them via UTXO index (now fixed)

### 5. ❌ xpub Sync Empty UTXOs
**Root Cause:** Same as issue #1 - the `sync_wallet_xpub` function had:
```rust
utxos_by_address.insert(address, Vec::new());  // ← Empty!
```

**Fix Applied:** Now populates UTXOs properly before inserting.

## What Still Needs Implementation

### Priority 1: Wallet Must Actually Request Sync
The wallet GUI needs to actually call the API to get transactions:

**Option A: HTTP REST API Call**
```rust
fn trigger_transaction_sync(&mut self) {
    if let Some(network_mgr) = &self.network_manager {
        // Get wallet addresses or xpub
        // Make HTTP call to /api/wallet/sync or /api/wallet/sync_xpub
        // Store returned transactions in database
    }
}
```

**Option B: TCP Protocol Message**
```rust
// Send WalletSyncRequest via TCP connection
// Receive WalletSyncResponse with UTXOs and transactions
// Update local database
```

### Priority 2: Real-time Transaction Notifications
When a transaction is added to mempool or confirmed in a block:
1. Masternode checks if any subscribed xpubs match
2. Derive addresses from xpub (up to gap limit)
3. Check if transaction involves those addresses
4. Send WebSocket notification to that wallet

**Implementation needed in:** `masternode/src/utxo_integration.rs`

### Priority 3: Mempool Persistence on Restart
Currently mempool is saved to disk and loaded on startup (✅ Working).
But verify it's properly restoring state after masternode restart.

## Testing Checklist

### Test 1: UTXO Retrieval
```bash
# On masternode
curl http://localhost:24001/api/blocks/height
curl http://localhost:24001/api/blockchain/balance/ADDRESS

# Should show block rewards and any transactions
```

### Test 2: Wallet Sync via API
```bash
# Test API endpoint directly
curl -X POST http://localhost:24001/api/wallet/sync \
  -H "Content-Type: application/json" \
  -d '{"addresses": ["YOUR_ADDRESS"]}'
  
# Should return UTXOs and transactions
```

### Test 3: Mempool Synchronization
1. Send transaction from wallet to masternode A
2. Check masternode A mempool: `curl http://localhost:24001/api/mempool`
3. Wait 30 seconds
4. Check masternode B mempool: `curl http://localhost:24002/api/mempool`
5. Both should have same transactions

### Test 4: Block Reward Visibility
1. Mine or wait for a block
2. Check blockchain state on masternode
3. Query balance of block reward recipient
4. Should show the coinbase transaction value

## Summary

**Fixed:**
- ✅ UTXO retrieval in wallet sync endpoints
- ✅ Proper field names (vout instead of index)

**Already Working:**
- ✅ Mempool synchronization between masternodes
- ✅ Mempool persistence to disk
- ✅ Transaction broadcasting and voting

**Still Needs Work:**
- ❌ Wallet GUI must actually call sync endpoint
- ❌ WebSocket real-time notifications for wallet
- ❌ xpub subscription protocol implementation
- ❌ Efficient address indexing for large blockchains

