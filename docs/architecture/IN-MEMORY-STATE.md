# In-Memory State Architecture

## Overview

TIME Coin uses a unique in-memory state system optimized for 24-hour blocks.

## Architecture

```
┌──────────────────────────────────────────┐
│       IN-MEMORY STATE (Today)            │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  • All transactions since 00:00 UTC      │
│  • Current balances                      │
│  • Masternode states                     │
│  • Mempool (pending transactions)        │
│  ⚡ Instant verification                 │
│  💾 Cleared every 24 hours               │
└──────────────────────────────────────────┘
              ↓ At Midnight UTC
┌──────────────────────────────────────────┐
│     FINALIZED BLOCKS (Historical)        │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  Block #0: Genesis                       │
│  Block #1: Oct 17, 2025                  │
│  Block #2: Oct 18, 2025                  │
│  💾 Immutable, stored on disk            │
└──────────────────────────────────────────┘
```

## Daily Cycle

### 00:00 UTC - Day Start
1. Load latest block from disk
2. Initialize fresh in-memory state
3. Begin accepting transactions

### Throughout the Day
- Transactions stay in memory
- Instant balance lookups
- Fast validation
- No disk I/O for current operations

### 23:59 UTC - Day End
1. Stop accepting new transactions
2. Finalize current state
3. Create Block #N with:
   - All today's transactions
   - Final balance snapshot
   - Merkle root
   - Block hash
4. Write block to disk
5. Clear memory

### 00:00 UTC - Next Day
1. New day begins
2. Fresh state
3. Repeat cycle

## Benefits

### ⚡ Performance
- All current operations in RAM
- No disk reads for validation
- Sub-millisecond transaction verification

### 📦 Efficiency
- Only ~1 day of data in memory
- Predictable memory usage
- Natural garbage collection

### 🔒 Security
- Daily immutable snapshots
- Full audit trail in blocks
- State can be reconstructed from any block

### 🚀 Scalability
- Memory usage doesn't grow with chain length
- Fast node startup
- Easy pruning/archival

## Implementation

### DailyState
Manages current day's state in memory:
```rust
pub struct DailyState {
    day_start: DateTime<Utc>,
    transactions: Vec<Transaction>,
    balances: HashMap<Address, u64>,
    mempool: Vec<Transaction>,
}
```

### BlockFinalizer
Converts state to finalized block at midnight:
```rust
let block = BlockFinalizer::finalize(&state, previous_hash);
storage.save_block(&block)?;
```

### BlockStorage
Persistent storage for finalized blocks:
```rust
let storage = BlockStorage::open("data/blockchain")?;
let block = storage.get_block(height)?;
```

## Node Startup

1. Open block storage
2. Load latest block (if exists)
3. Initialize DailyState from block
4. Start processing transactions

## Recovery

If node crashes:
- Load last finalized block
- Current day's transactions may be lost
- But all previous days are safe on disk
- Rebuild current state from mempool rebroadcasts

## Future Enhancements

- State checkpointing (hourly snapshots)
- Mempool persistence
- Partial state recovery
- Distributed state verification
