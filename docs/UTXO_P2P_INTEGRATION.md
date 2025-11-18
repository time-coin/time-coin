# UTXO Protocol P2P Integration

This document describes the P2P integration of the UTXO State Protocol for TIME Coin's instant finality system.

## Overview

The UTXO Protocol P2P Integration connects three major components:

1. **UTXO State Protocol** (consensus layer) - Tracks UTXO states in real-time
2. **P2P Network Layer** (network layer) - Handles masternode communication
3. **Instant Finality** (voting layer) - Achieves transaction consensus

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Wallet Layer                             │
│  (Subscribe to UTXOs, receive notifications via WebSocket)      │
└────────────────────────┬────────────────────────────────────────┘
                         │ WebSocket (Port 24002)
                         │
┌────────────────────────▼────────────────────────────────────────┐
│                    Masternode Integration                        │
│                  (MasternodeUTXOIntegration)                     │
│                                                                   │
│  ┌──────────────────┐  ┌──────────────────┐  ┌───────────────┐ │
│  │ UTXO State       │  │ P2P Message      │  │ Instant       │ │
│  │ Manager          │  │ Handler          │  │ Finality      │ │
│  │                  │  │                  │  │               │ │
│  │ - Track states   │  │ - Route msgs     │  │ - Vote on TX  │ │
│  │ - Lock UTXOs     │  │ - Subscriptions  │  │ - Achieve     │ │
│  │ - Notify changes │  │ - Broadcast      │  │   consensus   │ │
│  └──────────────────┘  └──────────────────┘  └───────────────┘ │
└────────────────────────┬────────────────────────────────────────┘
                         │ TCP P2P (Port 24000)
                         │
┌────────────────────────▼────────────────────────────────────────┐
│                   P2P Network Layer                              │
│  (Masternode-to-Masternode Communication)                        │
└──────────────────────────────────────────────────────────────────┘
```

## Components

### 1. UTXOProtocolHandler (network/src/utxo_handler.rs)

**Purpose**: Handle UTXO-related messages in the P2P network

**Key Features**:
- Process UTXO state queries
- Manage wallet subscriptions
- Handle transaction broadcasts
- Route state notifications

**Message Types**:
```rust
UTXOStateQuery       // Query current UTXO states
UTXOStateResponse    // Response with UTXO states
UTXOStateNotification // Push state change notification
UTXOSubscribe        // Subscribe to address/UTXO updates
UTXOUnsubscribe      // Unsubscribe from updates
TransactionBroadcast // Lock UTXOs and trigger voting
```

### 2. MasternodeUTXOIntegration (masternode/src/utxo_integration.rs)

**Purpose**: Integrate UTXO protocol with masternode operations

**Key Features**:
- Initialize notification handlers
- Process incoming transactions
- Handle instant finality votes
- Broadcast state changes
- Manage subscriptions

**Usage**:
```rust
// 1. Create integration
let utxo_manager = Arc::new(UTXOStateManager::new(node_id));
let peer_manager = Arc::new(PeerManager::new(...));
let integration = MasternodeUTXOIntegration::new(
    node_id,
    utxo_manager,
    peer_manager,
);

// 2. Initialize
integration.initialize().await?;

// 3. Process transactions
integration.process_transaction(&tx).await?;

// 4. Handle network messages
let response = integration.handle_network_message(&msg, peer_ip).await?;
```

### 3. PeerManager Extensions (network/src/manager.rs)

**New Methods**:
```rust
// Broadcast UTXO notification to all masternodes
pub async fn broadcast_utxo_notification(
    &self,
    notification: &UTXOStateNotification,
)

// Send UTXO notification to specific peer (wallet)
pub async fn send_utxo_notification_to_peer(
    &self,
    peer_ip: IpAddr,
    notification: &UTXOStateNotification,
) -> Result<(), String>

// Handle UTXO protocol messages
pub async fn handle_utxo_message(
    &self,
    message: &NetworkMessage,
    peer_ip: IpAddr,
    utxo_handler: &UTXOProtocolHandler,
) -> Result<Option<NetworkMessage>, String>
```

## Transaction Flow

### Step 1: Transaction Broadcast

```
Wallet → Masternode: TransactionBroadcast(tx)
```

1. Masternode receives transaction
2. UTXOProtocolHandler locks all input UTXOs
3. Transaction forwarded to consensus for voting
4. State notifications sent to subscribers

### Step 2: UTXO Locking

```rust
// For each input UTXO:
UTXOState::Unspent → UTXOState::Locked { txid, locked_at }
```

**Purpose**: Prevent double-spending during voting period

**Notification**: All subscribers receive state change

### Step 3: Masternode Voting

```
Masternodes validate and vote on transaction
Vote messages exchanged via P2P network
```

### Step 4: Instant Finality

```rust
// When 2/3 + 1 votes achieved:
UTXOState::Locked → UTXOState::SpentFinalized { txid, votes }
```

**Result**: Transaction has instant finality (irreversible)

**Notification**: All wallets immediately see finalized state

### Step 5: Block Confirmation

```rust
// When included in block:
UTXOState::SpentFinalized → UTXOState::Confirmed { txid, block_height }
```

**Purpose**: On-chain confirmation for historical records

## Subscription Management

### Wallet Subscribe

```rust
// Wallet subscribes to addresses and specific UTXOs
NetworkMessage::UTXOSubscribe {
    outpoints: vec!["txid:vout".to_string()],
    addresses: vec!["time1_address".to_string()],
    subscriber_id: "wallet_1".to_string(),
}
```

**Stored**:
- In UTXOProtocolHandler (P2P layer)
- In UTXOStateManager (consensus layer)

### State Change Notification

```rust
// When UTXO state changes:
NetworkMessage::UTXOStateNotification {
    notification: UTXOStateNotification {
        outpoint: OutPoint { txid, vout },
        old_state: UTXOState::Unspent,
        new_state: UTXOState::Locked { ... },
        timestamp: ...,
        originator: "masternode_1",
    }
}
```

**Routing**:
1. UTXOStateManager detects state change
2. Notification handler called
3. UTXOProtocolHandler finds interested subscribers
4. PeerManager sends to subscribed wallets

### Wallet Unsubscribe

```rust
NetworkMessage::UTXOUnsubscribe {
    subscriber_id: "wallet_1".to_string(),
}
```

## Network Messages

### Message Format

All messages are JSON-encoded and sent over TCP with length prefix:

```
[4 bytes: length][JSON payload]
```

### UTXO State Query

**Request**:
```json
{
  "type": "UTXOStateQuery",
  "outpoints": ["txid:vout", "txid2:vout2"]
}
```

**Response**:
```json
{
  "type": "UTXOStateResponse",
  "states": [
    {
      "outpoint": {"txid": "...", "vout": 0},
      "state": "Unspent"
    },
    {
      "outpoint": {"txid": "...", "vout": 1},
      "state": {"Locked": {"txid": "...", "locked_at": 1234567}}
    }
  ]
}
```

## Example Integration

See `examples/masternode_utxo_integration.rs` for complete example.

## Testing

```bash
# Run network tests
cargo test --package time-network

# Run consensus tests
cargo test --package time-consensus

# Run integration tests
cargo test --package time-masternode
```

## Security Considerations

1. **UTXO Locking**: Prevents double-spending during voting
2. **Subscription Validation**: Only valid addresses can subscribe
3. **Notification Authentication**: Verify notification originators
4. **Rate Limiting**: Prevent spam of state queries
5. **Byzantine Tolerance**: Requires 2/3+1 honest masternodes

## Performance

- **Transaction Processing**: < 100ms to lock UTXOs
- **State Notification**: < 50ms to all subscribers
- **Query Response**: < 10ms for UTXO state lookup
- **Instant Finality**: < 1 second with 7 masternodes

## Future Enhancements

1. **State Checkpointing**: Periodic UTXO state snapshots
2. **Optimized Queries**: Batch queries for multiple UTXOs
3. **Compression**: Compress state notifications
4. **Pruning**: Remove old confirmed UTXOs
5. **Metrics**: Detailed performance monitoring

## References

- [UTXO State Protocol](../consensus/src/utxo_state_protocol.rs)
- [Network Protocol](../network/src/protocol.rs)
- [Instant Finality](../consensus/src/instant_finality.rs)
- [TIME Coin Protocol Documentation](./TIME_COIN_PROTOCOL.md)
