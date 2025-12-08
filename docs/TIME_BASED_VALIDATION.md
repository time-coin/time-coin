# TIME Coin Time-Based Validation System

**Status**: ✅ Implemented  
**Module**: `core/src/time_validator.rs`  
**Date**: December 8, 2025

---

## Overview

TIME Coin's time-based validation system ensures that nodes operate on correct time and validates blocks based on time constraints. This prevents time manipulation attacks where malicious actors try to create fake "longest chains" by manipulating their system clock.

## Key Features

### 1. **Expected Block Height Calculation**
- Calculates how many blocks *should* exist based on elapsed time since genesis
- Formula: `expected_height = (current_time - genesis_time) / block_time`
- **Testnet**: Block every 10 minutes (600 seconds)
- **Mainnet**: Block every 1 hour (3600 seconds)

### 2. **Catch-Up Mode Detection**
- Automatically detects when local node is behind
- Triggers synchronization with network
- Provides estimated catch-up time

### 3. **Future Block Prevention**
- Rejects blocks with timestamps >30 seconds in the future
- Prevents nodes from claiming future blocks by advancing system clock
- Validates peer height claims against time-based maximum

### 4. **Time Drift Detection**
- Validates system clock against network time
- Maximum allowed drift: 5 minutes
- Auto-corrects or alerts if drift exceeds tolerance

### 5. **Block Interval Validation**
- Ensures minimum time between blocks (9 minutes for testnet)
- Prevents spam/rapid block creation
- Complements VDF time-lock mechanism

---

## Configuration

### Genesis Block
```rust
pub const GENESIS_TIMESTAMP: i64 = 1733011200;
// December 1, 2025 00:00:00 UTC
```

### Block Times
```rust
// Testnet (Current)
pub const TESTNET_BLOCK_TIME_SECONDS: i64 = 600; // 10 minutes

// Mainnet (Future)
pub const MAINNET_BLOCK_TIME_SECONDS: i64 = 3600; // 1 hour
```

### Validation Tolerances
```rust
pub const MAX_TIME_DRIFT_SECONDS: i64 = 300; // 5 minutes
pub const MAX_FUTURE_BLOCK_SECONDS: i64 = 30; // 30 seconds
pub const MIN_BLOCK_INTERVAL_SECONDS: i64 = 540; // 9 minutes (testnet)
```

---

## Usage

### Initialize Time Validator

```rust
use time_core::time_validator::TimeValidator;

// For testnet
let validator = TimeValidator::new_testnet();

// For mainnet (when launched)
let validator = TimeValidator::new_mainnet(genesis_timestamp);
```

### Check Expected Block Height

```rust
use time_core::time_validator::current_timestamp;

let current_time = current_timestamp();
let expected_height = validator.calculate_expected_height(current_time)?;

println!("Expected block height: {}", expected_height);
```

### Detect Catch-Up Mode

```rust
let local_height = state.get_height();
let current_time = current_timestamp();

if validator.should_catch_up(local_height, current_time)? {
    // Enter catch-up mode
    let info = validator.get_catch_up_info(local_height, current_time)?;
    println!("{}", info.status_message());
    // Output: "Node is 50 blocks behind (expected: 100, current: 50). 
    //          Estimated catch-up time: 0h 8m"
}
```

### Validate Block Timestamp

```rust
let block_timestamp = block.header.timestamp.timestamp();
let current_time = current_timestamp();

// Reject future blocks
validator.validate_block_timestamp(block_timestamp, current_time)?;
```

### Validate Block Height

```rust
// Ensure block height is reasonable for its timestamp
validator.validate_block_height(
    block.header.block_number,
    block.header.timestamp.timestamp()
)?;
```

### Validate Block Interval

```rust
// Ensure blocks aren't created too quickly
validator.validate_block_interval(
    previous_block.header.timestamp.timestamp(),
    current_block.header.timestamp.timestamp()
)?;
```

### Validate Peer Height

```rust
// Reject peers claiming impossible block heights
validator.validate_peer_height(peer_height, current_time)?;
```

### Validate System Time

```rust
// Check if local system clock is accurate
let local_time = current_timestamp();
let network_time = get_network_consensus_time()?; // From peers

validator.validate_system_time(local_time, network_time)?;
```

---

## Security Model

### Attack Prevention

#### 1. **Future Block Attack**
**Attack**: Node advances system clock to create blocks with future timestamps

**Prevention**:
```rust
// Block timestamps must be within 30 seconds of current time
validator.validate_block_timestamp(block_time, current_time)?;

// Peer heights must not exceed time-based maximum
validator.validate_peer_height(peer_height, current_time)?;
```

**Result**: Attack fails - future blocks rejected by network

#### 2. **Rollback Attack**
**Attack**: Attacker creates alternative chain from earlier block

**Prevention**:
```rust
// Each block height must match time-based expectation
validator.validate_block_height(block_height, block_timestamp)?;

// Combined with VDF (2-5 minute sequential computation per block)
// Attacker must spend real time creating each block
```

**Result**: Attack fails - cannot create blocks faster than real time

#### 3. **Time Manipulation Attack**
**Attack**: Node sets incorrect system time to sync with wrong chain

**Prevention**:
```rust
// System time validated against network consensus
validator.validate_system_time(local_time, network_time)?;

// Maximum drift: 5 minutes
// Nodes with excessive drift rejected
```

**Result**: Attack fails - node rejected by network

---

## Time Calculation Examples

### Testnet (10-minute blocks)

```
Genesis: December 1, 2025 00:00:00 UTC (1733011200)

After 1 hour (3600 seconds):
  expected_height = 3600 / 600 = 6 blocks

After 1 day (86400 seconds):
  expected_height = 86400 / 600 = 144 blocks

After 1 week (604800 seconds):
  expected_height = 604800 / 600 = 1008 blocks
```

### Mainnet (1-hour blocks) - Future

```
Genesis: TBD

After 1 day (86400 seconds):
  expected_height = 86400 / 3600 = 24 blocks

After 1 week (604800 seconds):
  expected_height = 604800 / 3600 = 168 blocks

After 1 year (31536000 seconds):
  expected_height = 31536000 / 3600 = 8760 blocks
```

---

## Integration Points

### Consensus Layer

```rust
// In block validation
pub fn validate_block(&self, block: &Block) -> Result<(), BlockError> {
    let validator = TimeValidator::new_testnet();
    let current_time = current_timestamp();
    
    // Check block timestamp is not in future
    validator.validate_block_timestamp(
        block.header.timestamp.timestamp(),
        current_time
    )?;
    
    // Check block height matches time expectation
    validator.validate_block_height(
        block.header.block_number,
        block.header.timestamp.timestamp()
    )?;
    
    // Check block interval
    if let Some(prev_block) = self.get_previous_block(&block.header.previous_hash) {
        validator.validate_block_interval(
            prev_block.header.timestamp.timestamp(),
            block.header.timestamp.timestamp()
        )?;
    }
    
    // ... other validations
    Ok(())
}
```

### Network Layer

```rust
// In peer connection handling
pub fn handle_peer_handshake(&mut self, peer: &Peer) -> Result<(), NetworkError> {
    let validator = TimeValidator::new_testnet();
    let current_time = current_timestamp();
    
    // Validate peer's claimed height
    validator.validate_peer_height(peer.height, current_time)?;
    
    // Validate peer's time
    validator.validate_system_time(current_time, peer.timestamp)?;
    
    Ok(())
}
```

### Sync Manager

```rust
// In synchronization logic
pub fn check_sync_status(&self) -> Result<SyncStatus, SyncError> {
    let validator = TimeValidator::new_testnet();
    let current_time = current_timestamp();
    let local_height = self.state.get_height();
    
    if validator.should_catch_up(local_height, current_time)? {
        let info = validator.get_catch_up_info(local_height, current_time)?;
        
        return Ok(SyncStatus::CatchingUp {
            blocks_behind: info.blocks_behind,
            estimated_time: info.estimated_catch_up_time_seconds,
        });
    }
    
    Ok(SyncStatus::InSync)
}
```

---

## Error Handling

### TimeValidationError Types

```rust
pub enum TimeValidationError {
    /// Node's clock is too far off
    ClockDrift { 
        local_time: i64, 
        network_time: i64, 
        drift_seconds: i64 
    },
    
    /// Block is from the future
    FutureBlock { 
        block_time: i64, 
        current_time: i64 
    },
    
    /// Too many blocks for elapsed time
    TooManyBlocks { 
        block_height: u64, 
        max_allowed: u64 
    },
    
    /// Node is behind (catch-up needed)
    InsufficientBlocks { 
        block_height: u64, 
        expected_min: u64 
    },
    
    /// Block created too quickly
    BlockTooFast { 
        time_since_previous: i64, 
        minimum_required: i64 
    },
    
    /// Genesis block timestamp invalid
    InvalidGenesis,
    
    /// Calculation error
    CalculationError(String),
}
```

### Example Error Handling

```rust
match validator.validate_block_height(block_height, block_time) {
    Ok(()) => {
        // Block height is valid
        println!("✓ Block height valid");
    }
    Err(TimeValidationError::TooManyBlocks { block_height, max_allowed }) => {
        // Reject block - peer is lying or clock is wrong
        println!("✗ Block height {} exceeds maximum {}", block_height, max_allowed);
        reject_block();
    }
    Err(TimeValidationError::FutureBlock { block_time, current_time }) => {
        // Reject block - timestamp in future
        println!("✗ Block from future: {} vs {}", block_time, current_time);
        reject_block();
    }
    Err(e) => {
        println!("✗ Validation error: {}", e);
    }
}
```

---

## Testing

Run the time validator tests:

```bash
cargo test --package time-core time_validator
```

### Test Coverage

- ✅ Expected height calculation
- ✅ Future block detection
- ✅ Block interval validation
- ✅ Catch-up detection
- ✅ Too many blocks detection
- ✅ Minimum height calculation
- ✅ Time drift validation

---

## Relationship to VDF (Proof-of-Time)

The time validator works **in conjunction** with VDF, not as a replacement:

| Feature | Time Validator | VDF (Proof-of-Time) |
|---------|---------------|---------------------|
| **Purpose** | Validate blocks match real time | Prove computational time elapsed |
| **Prevents** | Future blocks, time manipulation | Instant rollbacks, parallel computation |
| **Mechanism** | Timestamp checking | Sequential hashing |
| **Speed** | Instant validation | 2-5 min computation, 1 sec verification |
| **Scope** | Block height vs time | Individual block creation |

**Together they provide**:
- VDF prevents instant block creation (must spend 2-5 min per block)
- Time validator ensures block count matches elapsed time
- Result: Attack must spend real time AND cannot exceed time-based maximum

---

## Future Enhancements

### Planned Features

1. **NTP Integration**
   - Automatic time synchronization with NTP servers
   - Reduce reliance on system clock

2. **Network Time Consensus**
   - Calculate median time from connected peers
   - More robust than single NTP server

3. **Adaptive Block Times**
   - Adjust block time based on network conditions
   - Maintain target block production rate

4. **Time-Locked Transactions**
   - Enable transactions that can only be spent after specific time
   - Useful for escrow, vesting, etc.

---

## References

- **VDF Documentation**: `docs/PROOF_OF_TIME.md`
- **Block Structure**: `core/src/block.rs`
- **Consensus Rules**: `docs/TIME_COIN_PROTOCOL_SPECIFICATION.md`
- **Genesis Configuration**: `config/genesis-testnet.json`

---

**Implementation Status**: ✅ Complete  
**Tests**: ✅ Passing  
**Integration**: 🔄 Next Step

---

## Next Steps for Integration

1. **Update Block Validation** - Add time validator checks to block validation logic
2. **Update Peer Handshake** - Validate peer heights and timestamps
3. **Add Sync Manager** - Implement catch-up mode logic
4. **Add Monitoring** - Dashboard showing sync status and time drift
5. **Add Alerts** - Warn if system clock is incorrect

