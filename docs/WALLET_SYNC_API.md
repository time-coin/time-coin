# Wallet Sync API Documentation

## Overview

The Wallet Sync API provides secure, address-based synchronization between TIME Coin wallets and masternodes. This approach ensures wallets can:
- Discover all UTXOs for their addresses
- Receive notifications of incoming transactions
- Validate transactions before broadcasting
- Maintain accurate balance information

## Security Model

**Address-Based Sync** (Implemented):
- Wallet sends **addresses** (not private keys or full UTXOs)
- Masternode scans blockchain for UTXOs matching those addresses
- Wallet receives authoritative UTXO state from blockchain
- Supports HD wallet address discovery

**Benefits**:
- Privacy: Only addresses are shared, never private keys
- Accuracy: Masternode has authoritative blockchain state
- Auto-discovery: Wallet learns about new incoming transactions
- Verification: Wallet can cryptographically verify responses

## API Endpoints

### 1. Sync Wallet Addresses

**Endpoint**: `POST /wallet/sync`

**Description**: Synchronizes wallet state with blockchain by providing a list of addresses.

**Request**:
```json
{
  "addresses": [
    "TIME1abc123...",
    "TIME1def456...",
    "TIME1ghi789..."
  ]
}
```

**Response**:
```json
{
  "utxos": {
    "TIME1abc123...": [
      {
        "tx_hash": "a1b2c3d4...",
        "output_index": 0,
        "amount": 100000,
        "address": "TIME1abc123...",
        "block_height": 1234,
        "confirmations": 6
      }
    ],
    "TIME1def456...": []
  },
  "total_balance": 100000,
  "recent_transactions": [
    {
      "tx_hash": "a1b2c3d4...",
      "from_address": "TIME1xyz...",
      "to_address": "TIME1abc123...",
      "amount": 100000,
      "block_height": 1234,
      "timestamp": 1700000000,
      "confirmations": 6
    }
  ],
  "current_height": 1240
}
```

**Usage Example** (Rust):
```rust
use reqwest::Client;
use serde_json::json;

async fn sync_wallet(addresses: Vec<String>) -> Result<WalletSyncResponse, Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client
        .post("http://masternode:24101/wallet/sync")
        .json(&json!({ "addresses": addresses }))
        .send()
        .await?
        .json::<WalletSyncResponse>()
        .await?;
    
    Ok(response)
}
```

### 2. Validate Transaction

**Endpoint**: `POST /wallet/validate`

**Description**: Validates a transaction before broadcasting to the network.

**Request**:
```json
{
  "transaction_hex": "01000000..."
}
```

**Response**:
```json
{
  "valid": true,
  "error": null,
  "tx_hash": "a1b2c3d4e5f6..."
}
```

**Error Response**:
```json
{
  "valid": false,
  "error": "Insufficient funds",
  "tx_hash": null
}
```

**Usage Example**:
```rust
async fn validate_tx(tx_hex: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client
        .post("http://masternode:24101/wallet/validate")
        .json(&json!({ "transaction_hex": tx_hex }))
        .send()
        .await?
        .json::<ValidateTransactionResponse>()
        .await?;
    
    Ok(response.valid)
}
```

### 3. Get Pending Transactions

**Endpoint**: `POST /wallet/pending`

**Description**: Gets pending transactions from mempool for specified addresses.

**Request**:
```json
[
  "TIME1abc123...",
  "TIME1def456..."
]
```

**Response**:
```json
[
  {
    "tx_hash": "a1b2c3d4...",
    "from_address": "TIME1xyz...",
    "to_addresses": ["TIME1abc123..."],
    "amount": 50000,
    "timestamp": 1700000123,
    "pending": true
  }
]
```

**Usage Example**:
```rust
async fn get_pending(addresses: Vec<String>) -> Result<Vec<IncomingTransactionNotification>, Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client
        .post("http://masternode:24101/wallet/pending")
        .json(&addresses)
        .send()
        .await?
        .json::<Vec<IncomingTransactionNotification>>()
        .await?;
    
    Ok(response)
}
```

## WebSocket Notifications

**Endpoint**: `WS /ws/wallet`

**Description**: Real-time notifications for wallet events.

**Connection**:
```javascript
const ws = new WebSocket('ws://masternode:24101/ws/wallet');

ws.onmessage = (event) => {
    const notification = JSON.parse(event.data);
    console.log('Incoming transaction:', notification);
};
```

**Notification Format**:
```json
{
  "type": "incoming_transaction",
  "tx_hash": "a1b2c3d4...",
  "to_address": "TIME1abc...",
  "amount": 50000,
  "pending": true
}
```

## HD Wallet Integration

For HD wallets using BIP-39 mnemonic phrases, the sync process works as follows:

1. **Wallet generates addresses** from mnemonic using derivation paths
2. **Wallet requests sync** for all generated addresses (including gap limit)
3. **Masternode returns UTXOs** for all addresses
4. **Wallet discovers used addresses** and generates more if needed
5. **Repeat** until gap limit of unused addresses is reached

**Example Sync Flow**:
```rust
// Generate first 20 addresses from mnemonic
let mut addresses = Vec::new();
for i in 0..20 {
    let address = wallet.derive_address(i)?;
    addresses.push(address);
}

// Sync with masternode
let response = sync_wallet(addresses).await?;

// Check if we need to generate more addresses
let used_addresses = response.utxos.len();
if used_addresses > 15 {
    // Generate next batch (gap limit not reached)
    for i in 20..40 {
        let address = wallet.derive_address(i)?;
        // Add to next sync batch
    }
}
```

## Security Considerations

1. **Address Privacy**: While addresses are public on blockchain, avoid reusing them
2. **HTTPS Required**: Always use HTTPS in production to prevent MITM attacks
3. **Verify Responses**: Wallet should verify UTXO data against blockchain if possible
4. **Rate Limiting**: Masternodes may rate-limit sync requests
5. **Trusted Masternodes**: Consider syncing with multiple masternodes and comparing results

## Error Handling

**Common Errors**:
- `400 Bad Request`: Invalid address format or malformed request
- `429 Too Many Requests`: Rate limit exceeded
- `500 Internal Server Error`: Blockchain sync issues
- `503 Service Unavailable`: Masternode not synchronized

**Retry Strategy**:
```rust
async fn sync_with_retry(addresses: Vec<String>, max_retries: u32) -> Result<WalletSyncResponse, Box<dyn std::error::Error>> {
    for attempt in 0..max_retries {
        match sync_wallet(addresses.clone()).await {
            Ok(response) => return Ok(response),
            Err(e) if attempt < max_retries - 1 => {
                tokio::time::sleep(Duration::from_secs(2u64.pow(attempt))).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

## Performance

- **Sync frequency**: Recommended every 30-60 seconds for active wallets
- **Batch size**: Up to 100 addresses per sync request
- **Response time**: Typically <100ms for small address sets
- **WebSocket**: Preferred for real-time updates with lower overhead

## Future Enhancements

1. **Bloom Filters**: Reduce bandwidth for SPV clients
2. **Compact Block Filters**: BIP-157/158 style filters
3. **Merkle Proof**: Cryptographic proof of UTXO inclusion
4. **Transaction History**: Full transaction history pagination
5. **Address Labels**: Server-side encrypted address labeling
