# UTXO Synchronization Fix Plan

## Current Issues

### 1. Mempool Not Syncing Between Nodes ❌
- One masternode has 3 transactions, another has 1
- Mempool sync handler exists but not being called properly
- Periodic sync not running

### 2. Wallet Not Receiving Transactions ❌
- Wallet shows no transactions despite activity
- UTXO data not being sent to wallet
- WebSocket notifications not working properly

### 3. Block Rewards Not Visible ❌
- Block rewards (coinbase transactions) go directly into blocks
- They're NOT in mempool (this is correct)
- But wallet doesn't see them because UTXO index is not properly populated

### 4. UTXO Index Not Populated ❌
- UTXOs tracked in blockchain state
- But not indexed by address for fast lookup
- wallet_sync_handlers can't find UTXOs efficiently

## Root Cause Analysis

The `sync_wallet_addresses` function calls:
```rust
let address_utxos = Vec::new();  // ← Empty! Never populated
let balance = blockchain.get_balance(address);  // ← Works
```

But it never actually queries the UTXO set to populate `address_utxos`!

## Implementation Plan

### Phase 1: Fix UTXO Retrieval in Wallet Sync
- Modify `sync_wallet_addresses` to actually query UTXO set
- Use `blockchain.utxo_set().get_utxos_for_address(address)`
- Populate UtxoInfo with proper data

### Phase 2: Fix Mempool Synchronization
- Ensure mempool periodic sync is running
- Fix request_mempool_sync to work properly
- Verify gossip protocol shares transactions

### Phase 3: Fix WebSocket Transaction Notifications
- When transaction added to mempool → notify subscribed wallets
- When transaction confirmed in block → notify subscribed wallets
- Use xpub subscription to filter relevant transactions

### Phase 4: Add Address Index for Efficient Lookups
- Create address → UTXO index in blockchain state
- Update index when blocks are added
- Update index when transactions spent

