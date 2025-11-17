# Wallet WebSocket API

## Overview

The TIME Coin network provides real-time WebSocket notifications for wallet applications. This allows wallets to receive instant updates about transactions without polling the API.

## Connection

Connect to a masternode's WebSocket endpoint:

```
ws://masternode-ip:24101/ws/wallet
```

### Example (JavaScript)
```javascript
const ws = new WebSocket('ws://localhost:24101/ws/wallet');

ws.onopen = () => {
    // Subscribe to receive notifications for your address
    ws.send(JSON.stringify({
        address: 'TIME1mo1GozVUqbcvdAivuYFu1aRDaB57CCarWe'
    }));
};

ws.onmessage = (event) => {
    const notification = JSON.parse(event.data);
    handleNotification(notification);
};
```

## Notification Types

### 1. Incoming Payment

Sent when a transaction is received in the mempool.

```json
{
    "type": "incoming_payment",
    "txid": "abc123...",
    "amount": 1000000,
    "from_address": "TIME1xyz...",
    "timestamp": 1700000000
}
```

### 2. Transaction Confirmed

Sent when a transaction reaches consensus and is finalized.

```json
{
    "type": "tx_confirmed",
    "txid": "abc123...",
    "block_height": 1234,
    "confirmations": 1,
    "timestamp": 1700000000
}
```

### 3. Transaction Invalidated

Sent when a transaction is rejected (double-spend, invalid signature, etc.).

```json
{
    "type": "tx_invalidated",
    "txid": "abc123...",
    "reason": "DoubleSpend",
    "timestamp": 1700000000
}
```

#### Invalidation Reasons:
- `DoubleSpend` - Transaction attempts to spend already-spent outputs
- `InvalidSignature` - Cryptographic signature is invalid
- `InsufficientFunds` - Not enough balance to complete transaction
- `InvalidInput` - Referenced input doesn't exist
- `Other` - Other validation errors

## Wallet Sync API

In addition to WebSocket notifications, wallets can use HTTP endpoints to sync their state.

### Sync Wallet Addresses

**POST** `/wallet/sync`

Returns all UTXOs and recent transactions for the provided addresses.

```json
{
    "addresses": ["TIME1abc...", "TIME1xyz..."]
}
```

**Response:**
```json
{
    "utxos": {
        "TIME1abc...": [
            {
                "tx_hash": "abc123...",
                "output_index": 0,
                "amount": 1000000,
                "address": "TIME1abc...",
                "block_height": 1234,
                "confirmations": 10
            }
        ]
    },
    "total_balance": 1000000,
    "recent_transactions": [
        {
            "tx_hash": "abc123...",
            "from_address": "TIME1xyz...",
            "to_address": "TIME1abc...",
            "amount": 1000000,
            "block_height": 1234,
            "timestamp": 1700000000,
            "confirmations": 10
        }
    ],
    "current_height": 1244
}
```

### Get Pending Transactions

**POST** `/wallet/pending`

Returns unconfirmed transactions from mempool for the provided addresses.

```json
["TIME1abc...", "TIME1xyz..."]
```

### Validate Transaction

**POST** `/wallet/validate`

Validates a transaction before broadcasting.

```json
{
    "transaction_hex": "0123456789abcdef..."
}
```

## Best Practices

### 1. HD Wallet Sync
For hierarchical deterministic wallets, sync all derived addresses:
```javascript
const addresses = deriveAddresses(mnemonic, 0, 20); // First 20 addresses
await syncWallet(addresses);
```

### 2. Reconnection Logic
Always implement reconnection with exponential backoff:
```javascript
let reconnectDelay = 1000;
ws.onclose = () => {
    setTimeout(() => {
        reconnect();
        reconnectDelay = Math.min(reconnectDelay * 2, 60000);
    }, reconnectDelay);
};
```

### 3. Balance Updates
Update balance immediately on notifications:
```javascript
function handleNotification(notification) {
    if (notification.type === 'incoming_payment') {
        balance += notification.amount;
        showNotification(`Received ${notification.amount} TIME`);
    } else if (notification.type === 'tx_confirmed') {
        updateConfirmations(notification.txid, notification.confirmations);
    } else if (notification.type === 'tx_invalidated') {
        revertTransaction(notification.txid);
        showError(`Transaction failed: ${notification.reason}`);
    }
}
```

### 4. Multiple Address Subscription
You can change subscribed address at any time:
```javascript
// Switch to different address
ws.send(JSON.stringify({
    address: 'TIME1new...'
}));
```

## Security Considerations

1. **Connection Security**: Use WSS (WebSocket Secure) in production
2. **No Authentication**: WebSocket only receives public blockchain data
3. **Rate Limiting**: Masternodes may limit WebSocket connections
4. **Validation**: Always validate notification data before displaying

## Error Handling

```javascript
ws.onerror = (error) => {
    console.error('WebSocket error:', error);
    // Fall back to polling API
    startPolling();
};

ws.onclose = (event) => {
    if (!event.wasClean) {
        console.warn('Connection closed unexpectedly:', event.reason);
    }
};
```
