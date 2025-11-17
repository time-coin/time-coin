# Wallet Sync API Documentation

## Overview

The Wallet Sync API provides secure synchronization between TIME Coin wallets and masternodes using two methods:

1. **xpub-based sync** (Recommended) - Efficient HD wallet synchronization
2. **Address-based sync** (Legacy) - Direct address list synchronization

This approach ensures wallets can:
- Discover all UTXOs for their addresses
- Receive notifications of incoming transactions
- Validate transactions before broadcasting
- Maintain accurate balance information

## Security Model

**xpub-Based Sync** (Recommended):
- Wallet sends **extended public key (xpub)** only
- Masternode derives addresses automatically using BIP-44
- Gap limit algorithm discovers all used addresses
- Read-only key: no spending capability
- Maximum 1000 addresses scanned per request

**Address-Based Sync** (Legacy):
- Wallet sends **list of addresses** (not private keys)
- Masternode scans blockchain for UTXOs matching those addresses
- Wallet receives authoritative UTXO state from blockchain
- Works with non-HD wallets

**Benefits**:
- Privacy: Only xpub or addresses shared, never private keys
- Accuracy: Masternode has authoritative blockchain state
- Auto-discovery: Wallet learns about new incoming transactions
- Efficiency: xpub method reduces bandwidth and complexity
- Verification: Wallet can cryptographically verify responses

## Address Format

TIME Coin addresses use network-specific prefixes:

- **TIME1**: Mainnet addresses (version byte 0x00)
- **TIME0**: Testnet addresses (version byte 0x6F)

**Format**: `PREFIX` + Base58(`version` + `hash160` + `checksum`)

**Examples**:
```
TIME1aBc123DeF456GhI789JkL012MnO345  // Mainnet
TIME0xYz789qWe4rT5uI6oP7aS8dF9gH0jK  // Testnet
```

This prevents mixing testnet and mainnet coins, and provides instant visual identification.

## API Endpoints

### 1. Sync Wallet with xpub (Recommended)

**Endpoint**: `POST /wallet/sync-xpub`

**Description**: Synchronizes HD wallet using extended public key. Masternode automatically derives and scans addresses using BIP-44 gap limit algorithm.

**Request**:
```json
{
  "xpub": "xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8",
  "start_index": 0
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
  "current_height": 1240,
  "addresses_scanned": 42,
  "addresses_with_activity": 15
}
```

**Algorithm** (Gap Limit):
```
1. Start at index 0 (or start_index)
2. Derive address from xpub at current index
3. Check blockchain for transactions at that address
4. If transactions found:
   - Add to results
   - Reset gap_count to 0
5. If no transactions:
   - Increment gap_count
6. If gap_count >= 20: STOP (gap limit reached)
7. If index >= 1000: STOP (safety limit)
8. Otherwise: increment index and goto step 2
```

**Usage Example** (Rust):
```rust
use reqwest::Client;
use serde_json::json;

async fn sync_wallet_xpub(xpub: String) -> Result<WalletSyncResponse, Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client
        .post("http://masternode:24101/wallet/sync-xpub")
        .json(&json!({ 
            "xpub": xpub,
            "start_index": 0
        }))
        .timeout(Duration::from_secs(60)) // Longer timeout for address scanning
        .send()
        .await?
        .json::<WalletSyncResponse>()
        .await?;
    
    Ok(response)
}
```

**Benefits**:
- Only one key transmitted (privacy)
- Automatic address discovery (no pre-generation needed)
- Efficient gap limit algorithm
- Standard BIP-44 compliance
- Future-proof for unlimited addresses

---

### 2. Sync Wallet Addresses (Legacy)

**Endpoint**: `POST /wallet/sync`

**Description**: Synchronizes wallet state with blockchain by providing a list of addresses. Used for non-HD wallets or when xpub is not available.

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

---

### 3. Validate Transaction

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

---

### 4. Get Pending Transactions

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

For HD wallets using BIP-39 mnemonic phrases, TIME Coin supports two sync methods:

### Method 1: xpub-Based Sync (Recommended)

**How it works**:
1. Wallet generates xpub from mnemonic during creation
2. Wallet sends xpub to masternode
3. Masternode derives addresses using BIP-44 and scans blockchain
4. Masternode uses gap limit (20) to find all used addresses
5. Returns all UTXOs and transactions

**Advantages**:
- ✅ Single request
- ✅ Privacy-preserving (one key vs many addresses)
- ✅ Automatic address discovery
- ✅ No client-side address generation needed
- ✅ Efficient bandwidth usage

**Example**:
```rust
// Wallet side
let xpub = wallet.get_xpub()?;
let response = sync_wallet_xpub(xpub).await?;

// Process all discovered transactions
for tx in response.recent_transactions {
    println!("Found: {} TIME from {}", 
        tx.amount as f64 / 100_000_000.0,
        tx.from_address
    );
}
```

### Method 2: Address-Based Sync (Legacy)

**How it works**:
1. Wallet generates addresses from mnemonic using derivation paths
2. Wallet sends list of addresses (including gap limit)
3. Masternode scans blockchain for each address
4. Wallet discovers used addresses and generates more if needed
5. Repeat until gap limit of unused addresses is reached

**Advantages**:
- ✅ Works with non-HD wallets
- ✅ Explicit control over addresses
- ✅ Compatible with old masternodes

**Example**:
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

### Comparison

| Feature | xpub-Based | Address-Based |
|---------|-----------|---------------|
| Privacy | ✅ High (one key) | ⚠️ Lower (many addresses) |
| Efficiency | ✅ Single request | ❌ Multiple requests |
| Complexity | ✅ Simple (wallet) | ⚠️ Complex (wallet) |
| Auto-discovery | ✅ Yes | ⚠️ Manual |
| HD Wallet Support | ✅ Native | ⚠️ Requires logic |
| Non-HD Support | ❌ No | ✅ Yes |
| Bandwidth | ✅ Low | ⚠️ Higher |

**Recommendation**: Use xpub-based sync for all new HD wallets. Use address-based sync only for legacy non-HD wallets.

## Security Considerations

1. **xpub Privacy**: 
   - xpub allows deriving all public addresses (not private keys)
   - Anyone with xpub can see all wallet addresses and balances
   - xpub cannot spend funds (read-only)
   - Share xpub only with trusted services

2. **Address Privacy**: 
   - While addresses are public on blockchain, avoid reusing them
   - TIME0/TIME1 prefixes clearly distinguish network

3. **Network Validation**:
   - Verify TIME0 addresses are used on testnet only
   - Verify TIME1 addresses are used on mainnet only
   - Wallets should reject cross-network transactions

4. **HTTPS Required**: 
   - Always use HTTPS in production to prevent MITM attacks

5. **Verify Responses**: 
   - Wallet should verify UTXO data against blockchain if possible
   - Cross-check with multiple masternodes for critical operations

6. **Rate Limiting**: 
   - Masternodes may rate-limit sync requests
   - Implement exponential backoff

7. **Trusted Masternodes**: 
   - Consider syncing with multiple masternodes and comparing results
   - Validate masternode SSL certificates

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

### xpub-Based Sync
- **Initial sync**: ~1-2 seconds (depends on address usage)
- **Address derivation**: <1ms per address
- **Gap limit scan**: Stops at 20 empty addresses
- **Maximum addresses**: 1000 (safety limit)
- **Recommended frequency**: Every 60 seconds for active wallets

### Address-Based Sync
- **Sync frequency**: Recommended every 30-60 seconds for active wallets
- **Batch size**: Up to 100 addresses per sync request
- **Response time**: Typically <100ms for small address sets
- **Recommended for**: Non-HD wallets, legacy systems

### WebSocket
- **Real-time updates**: Instant notifications
- **Lower overhead**: Reduced polling
- **Preferred for**: Active wallets, mobile apps

## Future Enhancements

1. ~~**xpub-based Sync**~~: ✅ **IMPLEMENTED** - Efficient HD wallet sync
2. ~~**Network Prefixes**~~: ✅ **IMPLEMENTED** - TIME0/TIME1 distinction
3. ~~**Gap Limit**~~: ✅ **IMPLEMENTED** - BIP-44 standard gap limit
4. **Bloom Filters**: Reduce bandwidth for SPV clients
5. **Compact Block Filters**: BIP-157/158 style filters
6. **Merkle Proof**: Cryptographic proof of UTXO inclusion
7. **Transaction History**: Full transaction history pagination
8. **Address Labels**: Server-side encrypted address labeling
9. **Multi-Account Support**: BIP-44 account level derivation

## See Also

- [HD-WALLET.md](HD-WALLET.md) - HD wallet implementation details
- [API.md](API.md) - General API documentation
- [BIP-44](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki) - Multi-account hierarchy standard
- [wallet-websocket-api.md](wallet-websocket-api.md) - WebSocket notifications
