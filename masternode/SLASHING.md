# Automated Slashing Implementation

## Overview

This implementation provides a complete automated slashing system for the TIME Coin masternode network. The system automatically calculates penalties, deducts collateral, records events, and transfers slashed funds to the treasury.

## Features

### 1. Violation Types

The system supports multiple violation types with appropriate penalties:

- **Invalid Block** (5%): Masternode validated a block with invalid transactions
- **Double-Signing** (50%): Masternode signed two conflicting blocks at the same height
- **Data Withholding** (25%): Masternode withheld required data from the network
- **Long-Term Abandonment** (10-20%): Masternode offline for extended periods
  - 30-60 days: 10%
  - 60-90 days: 15%
  - 90+ days: 20%
- **Network Attack** (100%): Masternode participated in an attack on the network
- **Consensus Manipulation** (70%): Masternode attempted to manipulate consensus

### 2. Slashing Execution

The slashing execution process follows these steps:

1. **Violation Detection**: Network detects a violation with proof/evidence
2. **Penalty Calculation**: System calculates slash amount based on violation type
3. **Collateral Deduction**: Masternode's collateral is automatically reduced
4. **Tier Adjustment**: Masternode tier is downgraded if necessary based on remaining collateral
5. **Status Update**: Node is marked as slashed if collateral falls below minimum (1,000 COIN)
6. **Event Recording**: Slashing event is recorded in the masternode's history
7. **Treasury Transfer**: Slashed funds are transferred to the treasury

### 3. Components

#### `slashing` module
- Defines violation types
- Implements penalty calculation logic
- Creates slashing records

#### `slashing_executor` module
- Coordinates complete slashing workflow
- Manages treasury transfers
- Publishes slashing events for monitoring
- Tracks all slashing activity

#### `Masternode` extensions
- `execute_slash()`: Executes slashing on the masternode
- `is_slashed`: Flag indicating if node is permanently slashed
- `slashing_history`: Complete history of all slashings

#### `MasternodeNetwork` extensions
- `slash_masternode()`: Network-level slashing interface
- `get_slashing_records()`: Query slashing records
- `is_slashed()`: Check if a node is slashed
- `get_all_slashing_records()`: Get all slashing records across network

## Usage Examples

### Basic Slashing

```rust
use time_masternode::{MasternodeNetwork, slashing::Violation};

let mut network = MasternodeNetwork::new();

// Detect and slash for invalid block
let violation = Violation::InvalidBlock {
    block_height: 1000,
    reason: "Invalid transaction in block".to_string(),
};

let record = network.slash_masternode(
    &masternode_address,
    violation,
    timestamp,
    block_height,
)?;

println!("Slashed {} from masternode", record.amount);
```

### Complete Workflow with Treasury Transfer

```rust
use time_masternode::slashing_executor::SlashingExecutor;

let mut executor = SlashingExecutor::new();

// After executing slash on network and getting record...
let event = executor.execute_slashing(record, timestamp)?;

if event.treasury_transfer_success {
    println!("Treasury transfer successful: {:?}", event.treasury_tx_id);
}
```

### Query Slashing History

```rust
// Get all slashing records for a specific masternode
let records = network.get_slashing_records(&address);

for record in records {
    println!(
        "Slashed {} at block {} for: {}",
        record.amount,
        record.block_height,
        record.violation.description()
    );
}

// Check if a node is slashed
if network.is_slashed(&address) {
    println!("Masternode is permanently slashed");
}
```

### Monitor Slashing Events

```rust
let executor = SlashingExecutor::new();

// Get all events
for event in executor.get_events() {
    println!(
        "Slashing event: {} slashed {} at {}",
        event.record.masternode_id,
        event.record.amount,
        event.event_timestamp
    );
}

// Get total amounts
println!("Total slashed: {}", executor.total_slashed());
println!("Total transferred to treasury: {}", 
    executor.total_transferred_to_treasury());
```

## Slashing Penalties Reference

| Violation Type | Penalty | Description |
|---------------|---------|-------------|
| Invalid Block | 5% | Validated block with invalid transactions |
| Double-Signing | 50% | Signed conflicting blocks at same height |
| Data Withholding | 25% | Withheld required network data |
| Long-Term Abandonment (30-60 days) | 10% | Offline for 30-60 days |
| Long-Term Abandonment (60-90 days) | 15% | Offline for 60-90 days |
| Long-Term Abandonment (90+ days) | 20% | Offline for 90+ days |
| Consensus Manipulation | 70% | Attempted to manipulate consensus |
| Network Attack | 100% | Participated in network attack |

## Tier System Integration

After slashing, masternodes are automatically adjusted to the appropriate tier based on remaining collateral:

- **Professional**: 100,000+ COIN
- **Verified**: 10,000+ COIN  
- **Community**: 1,000+ COIN
- **Slashed**: < 1,000 COIN (permanently disabled)

## Security Considerations

1. **Evidence Required**: All violations (except abandonment) require cryptographic proof
2. **Single Execution**: A slashed node cannot be slashed again (prevents double-jeopardy)
3. **Minimum Collateral**: Nodes below 1,000 COIN are permanently disabled
4. **Audit Trail**: All slashings are permanently recorded with full details
5. **Treasury Integration**: Slashed funds are immediately transferred to prevent loss

## Testing

The implementation includes comprehensive tests:

- Unit tests for penalty calculations
- Unit tests for violation types
- Integration tests for complete slashing workflow
- Integration tests for multiple slashings
- Integration tests for tier adjustments
- Integration tests for event tracking

Run tests with:
```bash
cargo test -p time-masternode
```

## Future Enhancements

Potential improvements for future versions:

1. **Appeal Mechanism**: Allow masternodes to appeal slashing decisions
2. **Gradual Recovery**: Allow slashed nodes to recover after a waiting period
3. **Insurance Pool**: Create an insurance pool for accidental slashings
4. **Slashing Rewards**: Distribute a portion to the reporter of violations
5. **Advanced Evidence Verification**: Implement automated evidence validation
