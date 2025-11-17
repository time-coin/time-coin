# Wallet Notification System

## Overview

The TIME Coin wallet uses a **WebSocket-based notification system** for real-time updates about transactions and balance changes. This provides instant feedback to users without requiring constant polling.

## How It Works

### 1. **Initial Wallet Sync (Polling)**
When the wallet starts:
- Wallet connects to masternode via HTTP API
- Sends its addresses to `/wallet/sync` endpoint
- Masternode returns all UTXOs for those addresses
- Wallet calculates current balance

### 2. **Real-Time Notifications (WebSocket)**
After initial sync:
- Wallet opens WebSocket connection to `/ws/wallet`
- Subscribes to its addresses for real-time updates
- Receives instant notifications for:
  - Incoming payments
  - Transaction confirmations
  - Transaction invalidations (double-spend, etc.)

### 3. **Transaction States**

```
SENT → MEMPOOL → FINALIZED → IN_BLOCK → CONFIRMED
  ↓        ↓          ↓          ↓           ↓
  0        0          0          1        2+ conf
```

- **Sent**: Transaction broadcast to network
- **Mempool**: Accepted by masternodes (0 confirmations)
- **Finalized**: Instant finality via BFT consensus (usable)
- **In Block**: Included in midnight block (1 confirmation)
- **Confirmed**: Multiple block confirmations (2+)

## API Endpoints

### HTTP Endpoints (Synchronous)

#### Sync Wallet
```http
POST /wallet/sync
Content-Type: application/json

{
  "addresses": ["TIME1...", "TIME2..."]
}
```

Response:
```json
{
  "utxos": [
    {
      "txid": "abc123...",
      "vout": 0,
      "amount": 100000,
      "address": "TIME1...",
      "confirmations": 5
    }
  ],
  "balance": 100000,
  "unconfirmed_balance": 50000
}
```

#### Send Transaction
```http
POST /transactions
Content-Type: application/json

{
  "inputs": [...],
  "outputs": [...],
  "signature": "..."
}
```

Response:
```json
{
  "success": true,
  "txid": "def456...",
  "status": "pending"
}
```

### WebSocket Endpoint (Real-Time)

#### Connect and Subscribe
```javascript
// Connect to WebSocket
const ws = new WebSocket('ws://masternode-ip:24101/ws/wallet');

// Subscribe to address
ws.send(JSON.stringify({
  address: "TIME1mkLKMVtAbyBefLRcn8cLqjaxbVuBYheeYB"
}));

// Listen for notifications
ws.onmessage = (event) => {
  const notification = JSON.parse(event.data);
  handleNotification(notification);
};
```

## Notification Types

### 1. Incoming Payment
Triggered when your address receives coins:

```json
{
  "type": "incoming_payment",
  "txid": "abc123...",
  "amount": 50000,
  "from_address": "TIME1sender...",
  "timestamp": 1700000000
}
```

**When to show**: Immediately when received (0 confirmations)

### 2. Transaction Confirmed
Triggered when transaction is included in a block:

```json
{
  "type": "tx_confirmed",
  "txid": "abc123...",
  "block_height": 1234,
  "confirmations": 1,
  "timestamp": 1700000000
}
```

**When to show**: 
- 1 confirmation: "Confirmed in block"
- 6+ confirmations: "Fully confirmed"

### 3. Transaction Invalidated
Triggered if transaction is rejected (double-spend, etc.):

```json
{
  "type": "tx_invalidated",
  "txid": "abc123...",
  "reason": {
    "type": "double_spend",
    "details": "Input already spent in tx def456..."
  },
  "timestamp": 1700000000
}
```

**When to show**: Immediately with error message to user

## Implementation in GUI Wallet

### Rust Example (using tokio-tungstenite)

```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};

pub struct WalletNotificationClient {
    ws_url: String,
    addresses: Vec<String>,
}

impl WalletNotificationClient {
    pub async fn connect(&self) -> Result<()> {
        let (ws_stream, _) = connect_async(&self.ws_url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Subscribe to addresses
        for address in &self.addresses {
            let subscribe_msg = json!({
                "address": address
            });
            write.send(Message::Text(subscribe_msg.to_string())).await?;
        }

        // Listen for notifications
        while let Some(msg) = read.next().await {
            match msg? {
                Message::Text(text) => {
                    let notification: WalletNotification = 
                        serde_json::from_str(&text)?;
                    self.handle_notification(notification).await;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn handle_notification(&self, notification: WalletNotification) {
        match notification {
            WalletNotification::IncomingPayment { amount, .. } => {
                println!("💰 Received {} TIME", amount);
                // Update UI, play sound, show notification
            }
            WalletNotification::TxConfirmed { confirmations, .. } => {
                println!("✅ Transaction confirmed ({} blocks)", confirmations);
            }
            WalletNotification::TxInvalidated { reason, .. } => {
                eprintln!("❌ Transaction rejected: {:?}", reason);
                // Alert user
            }
        }
    }
}
```

## Transaction Flow Examples

### Example 1: Sending Coins

1. User clicks "Send"
2. Wallet creates and signs transaction
3. POST to `/transactions`
4. **Immediate response**: `{"txid": "abc...", "status": "pending"}`
5. Wait 2-5 seconds for instant finality
6. **WebSocket notification**: `{"type": "tx_confirmed", "confirmations": 0}`
7. Show "Finalized" in UI (instant, usable)
8. At midnight, block is created
9. **WebSocket notification**: `{"type": "tx_confirmed", "confirmations": 1}`
10. Show "1 confirmation" in UI

### Example 2: Receiving Coins

1. Someone sends you coins
2. **WebSocket notification**: `{"type": "incoming_payment", "amount": 5000}`
3. UI shows: "💰 Incoming: 5000 TIME (0 confirmations)"
4. After instant finality (2-5 sec)
5. UI updates: "💰 Received: 5000 TIME (Finalized)"
6. At midnight
7. **WebSocket notification**: `{"type": "tx_confirmed", "confirmations": 1}`
8. UI shows: "✅ 5000 TIME (1 confirmation)"

### Example 3: Double Spend Detection

1. Attacker sends you coins
2. **WebSocket notification**: `{"type": "incoming_payment", "amount": 1000}`
3. UI shows pending transaction
4. Attacker tries to double-spend same coins
5. **WebSocket notification**: `{"type": "tx_invalidated", "reason": "double_spend"}`
6. UI shows: "⚠️ Transaction REJECTED - Double spend detected"
7. Balance reverted

## Best Practices

### For GUI Wallet Developers

1. **Connection Management**
   - Auto-reconnect on disconnect (exponential backoff)
   - Subscribe to addresses after reconnect
   - Handle connection errors gracefully

2. **UI Updates**
   - Show pending status immediately after sending
   - Update to "Finalized" after instant finality (~5 seconds)
   - Show confirmation count after block inclusion
   - Use animations/sounds for incoming payments

3. **Error Handling**
   - Display clear error messages for invalidated transactions
   - Revert balance changes if transaction rejected
   - Log WebSocket errors for debugging

4. **Security**
   - Validate all incoming notifications
   - Verify transaction details match expected values
   - Don't trust unconfirmed transactions for large amounts

### For Masternode Operators

1. **WebSocket Server**
   - Already implemented in `/api/src/websocket.rs`
   - Automatically broadcasts notifications
   - Handles address subscriptions

2. **Triggering Notifications**
   ```rust
   // When transaction enters mempool
   ws_manager.notify_incoming_payment(IncomingPaymentEvent {
       txid: tx.txid.clone(),
       amount: output.amount,
       from_address: input_address,
       to_address: output.address.clone(),
       timestamp: Utc::now().timestamp(),
   }).await;

   // When transaction confirmed
   ws_manager.notify_tx_confirmed(TxConfirmationEvent {
       txid: tx.txid.clone(),
       block_height: block.height,
       confirmations: 1,
       timestamp: Utc::now().timestamp(),
   }, &address).await;

   // When transaction invalidated
   ws_manager.notify_tx_invalidated(TxInvalidationEvent {
       txid: tx.txid.clone(),
       reason: InvalidationReason::DoubleSpend,
       affected_addresses: vec![address],
       timestamp: Utc::now().timestamp(),
   }).await;
   ```

## Testing WebSocket Notifications

### Using wscat (command line)
```bash
# Install wscat
npm install -g wscat

# Connect to masternode
wscat -c ws://localhost:24101/ws/wallet

# Subscribe to address
> {"address": "TIME1mkLKMVtAbyBefLRcn8cLqjaxbVuBYheeYB"}

# Wait for notifications...
< {"type":"incoming_payment","txid":"abc...","amount":1000,...}
```

### Using JavaScript (browser/Node.js)
```javascript
const ws = new WebSocket('ws://localhost:24101/ws/wallet');

ws.onopen = () => {
  console.log('Connected');
  ws.send(JSON.stringify({
    address: 'TIME1mkLKMVtAbyBefLRcn8cLqjaxbVuBYheeYB'
  }));
};

ws.onmessage = (event) => {
  const notification = JSON.parse(event.data);
  console.log('Notification:', notification);
};

ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

ws.onclose = () => {
  console.log('Disconnected - reconnecting in 5s...');
  setTimeout(connect, 5000);
};
```

## Fallback to Polling

If WebSocket connection fails, wallet can fall back to HTTP polling:

```rust
// Poll every 10 seconds
loop {
    let response = http_client
        .get(&format!("{}/wallet/sync", masternode_url))
        .json(&json!({ "addresses": wallet_addresses }))
        .send()
        .await?;
    
    let sync_data: WalletSyncResponse = response.json().await?;
    update_wallet_state(sync_data);
    
    tokio::time::sleep(Duration::from_secs(10)).await;
}
```

## Summary

| Method | Use Case | Latency | Bandwidth |
|--------|----------|---------|-----------|
| **WebSocket** | Real-time notifications | Instant | Low |
| **HTTP Sync** | Initial sync, fallback | 10-30s | Medium |
| **Polling** | Backup only | 10-30s | High |

**Recommendation**: Use WebSocket for real-time updates with HTTP polling as fallback.
