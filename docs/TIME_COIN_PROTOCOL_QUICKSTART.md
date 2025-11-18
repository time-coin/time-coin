# TIME Coin Protocol - Quick Start Guide

## What is it?

The **TIME Coin Protocol** is a real-time UTXO state tracking system that enables **instant transaction finality** (sub-3 seconds) through masternode consensus, while preventing double-spend attacks.

This protocol is TIME Coin's unique innovation: Bitcoin's UTXO model + instant finality = real-world usability.

## Quick Start

### 1. Run the Demo

```bash
cd tools/utxo-protocol-demo
cargo run
```

This interactive demo shows the complete UTXO lifecycle from creation to finality.

### 2. Basic Usage

```rust
use time_consensus::utxo_state_protocol::UTXOStateManager;
use time_core::{OutPoint, TxOutput};

// Initialize
let manager = UTXOStateManager::new("my_node".to_string());

// Add UTXO
let outpoint = OutPoint::new("tx123".to_string(), 0);
let output = TxOutput::new(1000, "address".to_string());
manager.add_utxo(outpoint.clone(), output).await?;

// Lock UTXO (prevents double-spend)
manager.lock_utxo(&outpoint, "spending_tx_456".to_string()).await?;

// Mark as finalized (after consensus)
manager.mark_spent_finalized(&outpoint, "spending_tx_456".to_string(), 2).await?;
```

### 3. Subscribe to State Changes

```rust
use std::collections::HashSet;
use time_consensus::utxo_state_protocol::UTXOSubscription;

// Subscribe to addresses
let mut addresses = HashSet::new();
addresses.insert("TIME1myaddress".to_string());

let subscription = UTXOSubscription {
    outpoints: HashSet::new(),
    addresses,
    subscriber_id: "my_wallet".to_string(),
};

manager.subscribe(subscription).await;

// Set notification handler
manager.set_notification_handler(|notification| async move {
    println!("UTXO state changed: {:?}", notification.new_state);
}).await;
```

## UTXO State Lifecycle

```
Unspent → Locked → SpentPending → SpentFinalized → Confirmed
   ↑         ↓
   └─────────┘ (if transaction fails)
```

## Key Features

✅ **Instant Finality**: <3 second transaction confirmation  
✅ **Double-Spend Prevention**: Lock-based protection  
✅ **Real-Time Notifications**: Push updates to subscribers  
✅ **Byzantine Fault Tolerance**: 67%+ consensus required  
✅ **UTXO Accounting**: Bitcoin-compatible model  

## Network Messages

```rust
// Query UTXO states
NetworkMessage::UTXOStateQuery { outpoints }

// Subscribe to updates
NetworkMessage::UTXOSubscribe {
    outpoints,
    addresses,
    subscriber_id,
}

// Receive notifications
NetworkMessage::UTXOStateNotification { notification }
```

## Integration Points

### With Instant Finality Manager

```rust
// 1. Submit transaction
let txid = finality_manager.submit_transaction(tx).await?;

// 2. Lock UTXOs
for input in &tx.inputs {
    utxo_manager.lock_utxo(&input.previous_output, txid.clone()).await?;
}

// 3. Check consensus
if finality_manager.has_transaction_consensus(&txid).await {
    // 4. Mark finalized
    utxo_manager.finalize_transaction(&tx, votes).await?;
}
```

### With Block Production

```rust
// Get finalized transactions for block
let approved_txs = get_approved_transactions(&utxo_manager).await;

// Include in block
block.add_transactions(approved_txs);

// After block confirmation
for tx in &block.transactions {
    mark_utxos_confirmed(&utxo_manager, &tx, block.height).await?;
}
```

## Security

- **Quorum**: 67%+ masternode approval required
- **Tolerance**: Up to 33% malicious nodes
- **Lock Propagation**: Immediate across network
- **Vote Validation**: Only registered masternodes

## Performance

- **Finality Time**: <3 seconds typical
- **Throughput**: 1000+ TPS
- **UTXOs Tracked**: Millions (in-memory)
- **Subscriptions**: Thousands per node

## Common Patterns

### Wallet Integration

```rust
// Subscribe to user addresses
let subscription = create_subscription(&user_addresses);
manager.subscribe(subscription).await;

// Update balance on notifications
manager.set_notification_handler(|notification| async move {
    if notification.new_state == UTXOState::SpentFinalized {
        update_balance().await;
    }
}).await;
```

### Exchange Deposit Detection

```rust
// Subscribe to deposit addresses
for address in deposit_addresses {
    subscribe_to_address(&manager, &address).await;
}

// Credit account on instant finality
manager.set_notification_handler(|notification| async move {
    if let UTXOState::SpentFinalized { .. } = notification.new_state {
        credit_user_account(&notification).await;
    }
}).await;
```

### Point of Sale

```rust
// Customer scans QR code, sends payment
// POS subscribes to merchant address
subscribe_to_merchant_address(&manager).await;

// Wait for instant finality
let notification = wait_for_notification(&manager).await;

if matches!(notification.new_state, UTXOState::SpentFinalized { .. }) {
    println!("✅ Payment received! (under 3 seconds)");
    complete_sale();
}
```

## Testing

```bash
# Run protocol tests
cd consensus
cargo test utxo_state_protocol

# Run demo
cd tools/utxo-protocol-demo
cargo run
```

## Documentation

- **Full Documentation**: [docs/time-coin-protocol.md](docs/time-coin-protocol.md)
- **Implementation**: [consensus/src/utxo_state_protocol.rs](consensus/src/utxo_state_protocol.rs)
- **Summary**: [TIME_COIN_PROTOCOL_SUMMARY.md](TIME_COIN_PROTOCOL_SUMMARY.md)
- **Demo**: [tools/utxo-protocol-demo/](tools/utxo-protocol-demo/)

## Support

For questions or issues:
- Check the [full documentation](docs/time-coin-protocol.md)
- Run the [demo](tools/utxo-protocol-demo/)
- Review the [implementation](consensus/src/utxo_state_protocol.rs)

---

**Quick Reference Card** - Keep this handy for rapid integration!
