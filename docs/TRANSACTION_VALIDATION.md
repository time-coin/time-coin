# Transaction Validation and Wallet Notifications

## Overview

This document describes the enhanced transaction validation system and real-time wallet notification features implemented for TIME Coin.

## Key Features

### 1. Invalid Transaction Handling During Consensus

When a masternode broadcasts a transaction during block voting that another node finds invalid (e.g., double-spend), the system now:

1. **Validates the transaction** against the local mempool and UTXO set
2. **Identifies the specific error** (double-spend, insufficient funds, invalid signature, etc.)
3. **Notifies the sender's wallet** via WebSocket connection
4. **Requests the sender to reverse** the invalid transaction from their mempool
5. **Continues consensus** without the invalid transaction

### 2. Transaction Mismatch Resolution

During block proposal validation, if nodes discover transaction mismatches:

1. **Missing transactions are broadcast** to nodes that don't have them
2. **All nodes validate** the newly received transactions
3. **Invalid transactions are rejected** and senders are notified
4. **A new block is created** with only valid, universally-available transactions
5. **Consensus proceeds** with the corrected transaction set

### 3. Real-Time Wallet Notifications

Wallets can subscribe to real-time notifications via WebSocket:

#### Connection

```javascript
const ws = new WebSocket('ws://node-ip:24101/ws/wallet');

// Subscribe to notifications for a specific address
ws.send(JSON.stringify({
    address: "TIME1abc..."
}));
```

#### Notification Types

**Transaction Invalidated**
```json
{
    "type": "tx_invalidated",
    "txid": "abc123...",
    "reason": "DoubleSpend",
    "timestamp": 1700000000
}
```

**Transaction Confirmed**
```json
{
    "type": "tx_confirmed",
    "txid": "abc123...",
    "block_height": 1234,
    "confirmations": 6,
    "timestamp": 1700000000
}
```

**Incoming Payment**
```json
{
    "type": "incoming_payment",
    "txid": "abc123...",
    "amount": 1000000,
    "from_address": "TIME1xyz...",
    "timestamp": 1700000000
}
```

## Implementation Details

### Transaction Validation Flow

```
1. Leader creates block proposal
   ↓
2. Nodes validate all transactions
   ↓
3. If invalid transaction found:
   - Create ValidationResult with error details
   - Send WebSocket notification to affected addresses
   - Vote REJECT on proposal
   ↓
4. If transaction mismatch found:
   - Broadcast missing transactions to peers
   - Peers validate and add to mempool
   - Leader creates new proposal with corrected set
   ↓
5. Consensus proceeds with valid transactions only
```

### API Endpoints

#### Submit Transaction
```
POST /api/v1/transactions
Content-Type: application/json

{
    "txid": "...",
    "inputs": [...],
    "outputs": [...]
}
```

#### Get Transaction
```
GET /api/v1/transactions/{txid}

Response:
{
    "txid": "...",
    "inputs": [...],
    "outputs": [...],
    "status": "pending" | "finalized" | "confirmed"
}
```

#### WebSocket Subscription
```
GET /ws/wallet
Upgrade: websocket

Send: {"address": "TIME1..."}
Receive: Notification messages
```

### Consensus Integration

The `ConsensusValidator` handles validation during consensus:

```rust
// Validate transaction
let result = validator.validate_transaction(&tx, &mempool).await;

if !result.is_valid {
    // Notify affected wallets
    validator.handle_invalid_transaction(result, ws_manager).await;
    
    // Vote REJECT
    return VoteDecision::Reject;
}
```

### Broadcasting Missing Transactions

When a transaction mismatch is detected:

```rust
// Find differences
let (missing_local, missing_remote) = 
    find_transaction_differences(&local_txs, &remote_txs);

// Broadcast our transactions they're missing
for txid in missing_remote {
    if let Some(tx) = mempool.get_transaction(&txid).await {
        validator.broadcast_missing_transaction(&tx, peer_nodes).await;
    }
}

// Request transactions we're missing
for txid in missing_local {
    if let Some(tx) = validator.request_missing_transaction(&txid, peer_nodes).await {
        // Validate and add to mempool
        if mempool.add_transaction(tx).await.is_ok() {
            log::info!("✓ Added missing transaction {}", txid);
        }
    }
}
```

## Error Handling

### Invalid Transaction Reasons

| Error | Description | Action |
|-------|-------------|--------|
| `DoubleSpend` | Input already spent | Notify sender, remove from mempool |
| `InvalidSignature` | Signature verification failed | Notify sender, reject transaction |
| `InsufficientFunds` | Inputs < Outputs | Notify sender, reject transaction |
| `InvalidInput` | Referenced UTXO doesn't exist | Notify sender, reject transaction |

### Wallet Response

When a wallet receives an invalidation notification, it should:

1. **Remove the transaction** from pending transactions
2. **Update the UI** to show the error
3. **Restore UTXOs** that were marked as spent
4. **Notify the user** with a clear error message

Example wallet handler:
```javascript
ws.onmessage = (event) => {
    const notification = JSON.parse(event.data);
    
    if (notification.type === 'tx_invalidated') {
        // Remove from pending
        removePendingTx(notification.txid);
        
        // Show error to user
        showError(`Transaction failed: ${notification.reason}`);
        
        // Refresh balance
        refreshBalance();
    }
};
```

## Security Considerations

1. **No PII in notifications**: Only transaction IDs and public addresses
2. **WebSocket authentication**: Future enhancement for private addresses
3. **Rate limiting**: Prevent notification spam
4. **Quarantine integration**: Votes from quarantined nodes are rejected

## Testing

Test scenarios to verify:

1. ✓ Double-spend detection during consensus
2. ✓ Wallet notification delivery
3. ✓ Transaction broadcasting to missing nodes
4. ✓ Block recreation with corrected transaction set
5. ✓ Consensus proceeds after invalid transaction removal

## Future Enhancements

- [ ] Wallet authentication for WebSocket connections
- [ ] Push notifications for mobile wallets
- [ ] Transaction replacement (RBF)
- [ ] Partial signature support (multisig)
