# Wallet GUI Refactoring Plan

## Problem
The current wallet GUI has fundamental architectural issues:
- Uses `std::sync::Mutex` with async operations (causes deadlocks)
- Holds locks across `.await` points (blocking)
- Complex peer management with TCP connection pooling
- Multiple layers of abstraction (NetworkManager, PeerManager, ProtocolClient)
- Timeouts everywhere to work around blocking issues

## Root Cause
**A wallet GUI should be a simple client, not a P2P node.**

The current design treats the wallet like a mini-node that:
- Manages peer connections
- Does peer discovery
- Maintains connection pools
- Handles complex networking state

This is unnecessary complexity. A wallet just needs to:
1. Connect to ONE masternode
2. Send requests
3. Get responses
4. Display data

## Proposed Solution

### Phase 1: Use SimpleClient for All Network Operations

Replace all network calls with the new `SimpleClient`:

**Before (complex):**
```rust
let network_mgr = Arc<Mutex<NetworkManager>>::new(...);
tokio::spawn_blocking(move || {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let mut mgr = network_mgr.lock().unwrap(); // Holding lock...
        mgr.connect_to_peers(peers).await; // ...across await (DEADLOCK!)
    });
});
```

**After (simple):**
```rust
let client = SimpleClient::new("masternode:24100", NetworkType::Testnet);
let txs = client.get_transactions(xpub).await?; // Just works!
```

### Phase 2: Refactor WalletApp State

**Remove:**
- `network_manager: Option<Arc<Mutex<NetworkManager>>>`
- `peer_manager: Option<Arc<PeerManager>>`
- `protocol_client: Option<Arc<ProtocolClient>>`
- All the complex refresh/sync logic

**Add:**
- `client: Option<SimpleClient>` - Single async client
- `last_refresh: Option<Instant>` - Simple refresh tracking

### Phase 3: Simplify Refresh Logic

**Current (90 lines, complex):**
- Spawns multiple tasks
- Manages mutex locks
- Has nested timeouts
- Uses spawn_blocking with nested runtimes

**New (10 lines, simple):**
```rust
async fn refresh_transactions(&self, xpub: &str) {
    if let Some(client) = &self.client {
        match client.get_transactions(xpub).await {
            Ok(txs) => {
                // Update UI state
                log::info!("Got {} transactions", txs.len());
            }
            Err(e) => log::warn!("Refresh failed: {}", e),
        }
    }
}
```

### Phase 4: Handle Async in egui

egui callbacks are synchronous, but we can spawn async tasks:

```rust
// In egui UI callback
if ui.button("Refresh").clicked() {
    let client = self.client.clone();
    let xpub = self.wallet_manager.get_xpub();
    
    // Spawn async task - don't block UI
    tokio::spawn(async move {
        let txs = client.get_transactions(&xpub).await?;
        // Send result back via channel
        tx_channel.send(txs).ok();
    });
}

// Later in update() - check for results
if let Ok(txs) = self.rx_channel.try_recv() {
    self.transactions = txs;
}
```

## Implementation Steps

1. ✅ **Created `simple_client.rs`** - Pure async TCP client
2. ⏳ **Replace transaction fetching** - Use SimpleClient instead of protocol_client
3. ⏳ **Remove NetworkManager** - No longer needed
4. ⏳ **Remove PeerManager** - Wallet doesn't need peer discovery
5. ⏳ **Simplify state management** - Use channels for async results
6. ⏳ **Remove all mutexes** - Use message passing instead

## Benefits

- **No blocking** - Pure async, no spawn_blocking
- **No deadlocks** - No mutexes held across awaits
- **No timeouts** - Operations complete or fail cleanly
- **Simpler code** - ~1000 lines removed
- **Faster** - No connection pooling overhead
- **More reliable** - Less state to manage

## Migration Strategy

### Option A: Gradual (safer, slower)
1. Add SimpleClient alongside existing code
2. Migrate one feature at a time
3. Remove old code when feature works

### Option B: Clean Rewrite (faster, riskier)
1. Create new `simple_wallet_gui` binary
2. Copy only UI code (mnemonic, password, display)
3. Use SimpleClient for all network
4. Test and replace when stable

## Next Steps

I recommend **Option B** - the current codebase has too much technical debt.
A clean rewrite will be faster and result in better code quality.

Would you like me to:
1. Start the clean rewrite now?
2. Or gradually migrate the existing code?
