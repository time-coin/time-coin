# SyncGate Integration Guide

## Overview
The `SyncGate` prevents nodes from creating blocks when significantly behind the network, eliminating fork creation from out-of-sync nodes.

---

## Integration Points

### 1. Consensus Layer (CRITICAL)

**File:** `consensus/src/lib.rs` (or wherever blocks are created)

```rust
use time_network::SyncGate;

impl ConsensusEngine {
    pub async fn propose_block(&self, data: BlockData) -> Result<Block, String> {
        let next_height = self.current_height + 1;
        
        // CRITICAL: Check with SyncGate before proposing ANY block
        self.peer_manager.sync_gate
            .can_create_block(next_height)
            .await
            .map_err(|e| format!("Cannot create block: {}", e))?;
        
        // Only then create the block
        self.create_block_internal(data).await
    }
}
```

**Why this works:**
- `can_create_block()` enforces 3 rules:
  1. Can't skip blocks (N → N+2)
  2. Can't recreate existing blocks  
  3. Can't create when 5+ blocks behind network
- Returns `Err` if any rule violated
- Consensus must handle the error gracefully

---

### 2. Network Message Handlers

**File:** `network/src/manager.rs` or message handler

#### On Peer Connection
```rust
// In connect_to_peer() after successful handshake
match handshake_result {
    Ok(their_handshake) => {
        // Extract peer's blockchain height
        let peer_height = their_handshake.blockchain_height.unwrap_or(0);
        
        // Update our view of network height
        self.sync_gate.update_network_height(peer_height).await;
        
        // Check if we need to sync before broadcasting
        if self.sync_gate.blocks_behind().await > 5 {
            warn!(
                "Discovered peer at height {}, we're at {} - syncing first",
                peer_height,
                self.sync_gate.local_height().await
            );
            // Trigger sync (existing code)
            self.trigger_blockchain_sync().await?;
        }
    }
}
```

#### On UpdateTip Message
```rust
NetworkMessage::UpdateTip { height, hash } => {
    // Update network height from peer announcement
    self.sync_gate.update_network_height(height).await;
    
    // Log if significantly behind
    let behind = self.sync_gate.blocks_behind().await;
    if behind > 0 {
        info!("Peer announced height {}, we're {} blocks behind", height, behind);
    }
    
    // Trigger sync if needed
    if behind > 10 {
        self.trigger_catch_up_sync(height).await?;
    }
}
```

#### On Peer Announcement (NewPeer message)
```rust
NetworkMessage::NewPeer { address, port, blockchain_height } => {
    // Update network height from broadcast
    if let Some(height) = blockchain_height {
        self.sync_gate.update_network_height(height).await;
    }
    
    // Add peer to connection pool
    self.connect_to_peer(address, port).await?;
}
```

---

### 3. Blockchain Sync Callbacks

**File:** Wherever blocks are added to the chain

```rust
impl Blockchain {
    pub async fn add_block(&mut self, block: Block) -> Result<(), String> {
        // Validate and add block (existing code)
        self.validate_block(&block)?;
        self.blocks.push(block.clone());
        
        // IMPORTANT: Update local height in SyncGate
        let new_height = self.blocks.len() as u64 - 1;
        self.peer_manager.sync_gate
            .update_local_height(new_height)
            .await;
        
        Ok(())
    }
}
```

**Alternative: After sync completes**
```rust
async fn sync_from_peer(&self, target_height: u64) -> Result<(), String> {
    // Mark sync as in progress
    self.sync_gate.start_sync().await;
    
    // Download and add blocks (existing code)
    for height in self.local_height..=target_height {
        let block = self.fetch_block_from_peer(height).await?;
        self.blockchain.add_block(block).await?;
        
        // Update after each block
        self.sync_gate.update_local_height(height).await;
    }
    
    // Mark sync as complete
    self.sync_gate.complete_sync().await;
    
    Ok(())
}
```

---

### 4. Initialization (Daemon Startup)

**File:** `cli/src/bin/timed.rs` or daemon initialization

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ... existing initialization ...
    
    // Load blockchain from disk
    let blockchain = Blockchain::load_from_disk(&data_dir).await?;
    let current_height = blockchain.height();
    
    // Initialize network manager
    let peer_manager = PeerManager::new(network, listen_addr, public_addr);
    
    // IMPORTANT: Initialize SyncGate with current blockchain height
    peer_manager.sync_gate.update_local_height(current_height).await;
    
    info!("Initialized at height {}", current_height);
    
    // Start consensus
    let consensus = ConsensusEngine::new(peer_manager.clone(), blockchain);
    consensus.start().await?;
    
    Ok(())
}
```

---

## Usage Examples

### Check if Can Create Block
```rust
match self.sync_gate.can_create_block(next_height).await {
    Ok(()) => {
        // Safe to create block
        let block = self.create_block(...).await?;
    }
    Err(e) => {
        // Can't create - either sync or wait
        warn!("Cannot create block: {}", e);
        
        // Optionally wait for sync
        if self.sync_gate.is_syncing().await {
            info!("Waiting for sync to complete...");
            self.sync_gate.wait_for_sync().await?;
        }
    }
}
```

### Query Sync Status
```rust
// Check if behind
if self.sync_gate.is_behind().await {
    let behind = self.sync_gate.blocks_behind().await;
    info!("Node is {} blocks behind network", behind);
    
    // Trigger aggressive sync
    self.parallel_sync_to_height(
        self.sync_gate.network_height().await
    ).await?;
}
```

### Safe Block Creation Loop
```rust
loop {
    tokio::time::sleep(block_interval).await;
    
    let next_height = self.blockchain.height() + 1;
    
    // Gate: Don't create if behind
    if let Err(e) = self.sync_gate.can_create_block(next_height).await {
        warn!("Skipping block creation: {}", e);
        continue;
    }
    
    // Safe to create
    match self.propose_block(next_height).await {
        Ok(block) => {
            self.blockchain.add_block(block).await?;
            self.sync_gate.update_local_height(next_height).await;
        }
        Err(e) => error!("Block creation failed: {}", e),
    }
}
```

---

## Testing

### Unit Test: Fork Prevention
```rust
#[tokio::test]
async fn test_prevents_fork_when_behind() {
    let manager = PeerManager::new(...);
    
    // Simulate local height at 100
    manager.sync_gate.update_local_height(100).await;
    
    // Peer announces height 110
    manager.sync_gate.update_network_height(110).await;
    
    // Try to create block 101 - should be blocked (10 blocks behind, max is 5)
    let result = manager.sync_gate.can_create_block(101).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("too far behind"));
}
```

### Integration Test: Sync Then Create
```rust
#[tokio::test]
async fn test_sync_then_create_block() {
    let manager = PeerManager::new(...);
    manager.sync_gate.update_local_height(95).await;
    manager.sync_gate.update_network_height(100).await;
    
    // Start sync
    manager.sync_gate.start_sync().await;
    
    // Simulate syncing to 99
    for height in 96..=99 {
        manager.sync_gate.update_local_height(height).await;
    }
    
    manager.sync_gate.complete_sync().await;
    
    // Now should be able to create block 100
    assert!(manager.sync_gate.can_create_block(100).await.is_ok());
}
```

---

## Monitoring

### Add Metrics/Logging
```rust
// In your heartbeat or status logging
let local = self.sync_gate.local_height().await;
let network = self.sync_gate.network_height().await;
let behind = self.sync_gate.blocks_behind().await;
let syncing = self.sync_gate.is_syncing().await;

info!(
    "Height: local={}, network={}, behind={}, syncing={}",
    local, network, behind, syncing
);

// Prometheus metrics (if using metrics crate)
metrics::gauge!("blockchain.local_height", local as f64);
metrics::gauge!("blockchain.network_height", network as f64);
metrics::gauge!("blockchain.blocks_behind", behind as f64);
```

---

## Troubleshooting

### Issue: "Cannot create block: too far behind network"
**Cause:** Node is 5+ blocks behind  
**Solution:** Wait for sync to complete or trigger manual sync

```rust
if self.sync_gate.blocks_behind().await > 5 {
    self.trigger_aggressive_sync().await?;
}
```

### Issue: "Sync stalled: local=X, network=Y"
**Cause:** Sync started but not progressing  
**Solution:** Check peer connections, verify block requests succeeding

```rust
// Check if peers responding
let connected_peers = self.peer_manager.connected_peer_count().await;
if connected_peers == 0 {
    warn!("No peers connected - can't sync");
    self.discover_and_connect_peers().await?;
}
```

### Issue: Fork still happening
**Cause:** SyncGate not integrated with consensus  
**Solution:** Ensure `can_create_block()` called BEFORE every block creation

---

## Performance

**Overhead per block creation check:**
- 3 RwLock reads (~1μs total)
- 3 comparisons (negligible)
- **Total: < 2μs per check**

**Memory overhead:**
- 3 × Arc<RwLock<u64>> = ~72 bytes
- 1 × Arc<RwLock<bool>> = 24 bytes
- **Total: ~96 bytes per PeerManager**

**Negligible impact when nodes are synced.**

---

## Summary

1. ✅ Add `can_create_block()` before ALL block creation
2. ✅ Call `update_network_height()` on peer announcements  
3. ✅ Call `update_local_height()` after adding blocks
4. ✅ Initialize with current height at startup
5. ✅ Handle errors gracefully (wait/sync)

**Result:** Zero forks from out-of-sync nodes! 🎉
