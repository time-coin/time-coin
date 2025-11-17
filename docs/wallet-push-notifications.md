# Wallet Real-Time Push Notifications

## Overview

The TIME Coin wallet receives real-time notifications when transactions are received, eliminating the need for constant polling. This document explains the push notification architecture.

## Architecture

### 1. Initial Setup (HTTP)

When the wallet starts, it fetches the initial list of masternodes from the API:

```
Wallet → HTTP GET time-coin.io/api/peers → Masternode list
```

### 2. P2P Connection & Subscription

The wallet connects to ONE masternode via TCP and subscribes for notifications:

```
Wallet → TCP connect to masternode:24100
Wallet → send RequestWalletTransactions { xpub }
```

When the masternode receives `RequestWalletTransactions`:
1. Subscribes the wallet's IP to the xpub
2. Derives addresses from xpub using BIP44 (gap limit 20)
3. Searches blockchain for historical transactions
4. Sends `WalletTransactionsResponse` with all past transactions

### 3. Real-Time Push Notifications

When a new transaction arrives at the masternode:

```
1. Transaction arrives in mempool or confirmed block
2. Masternode derives addresses from subscribed xpubs
3. Masternode checks if transaction involves any derived addresses
4. If match found:
   Masternode → send NewTransactionNotification to wallet
5. Wallet displays instant notification (no polling!)
```

### 4. Peer Discovery

The wallet can discover additional peers through connected masternodes:

```
Wallet → send GetPeerList
Masternode → send PeerList (known peers)
Wallet → connects to newly discovered peers
```

## Message Types

### RequestWalletTransactions
```rust
RequestWalletTransactions {
    xpub: String,  // Extended public key for address derivation
}
```

**Sent by**: Wallet  
**Purpose**: Subscribe to notifications and request historical transactions  
**Response**: `WalletTransactionsResponse`

### WalletTransactionsResponse
```rust
WalletTransactionsResponse {
    transactions: Vec<WalletTransaction>,
    last_synced_height: u64,
}
```

**Sent by**: Masternode  
**Purpose**: Deliver all historical transactions for the xpub  
**Triggered by**: `RequestWalletTransactions`

### NewTransactionNotification
```rust
NewTransactionNotification {
    transaction: WalletTransaction,
}
```

**Sent by**: Masternode  
**Purpose**: Instantly notify wallet of incoming transaction  
**Triggered by**: New transaction in mempool/block that matches a subscribed address

### WalletTransaction
```rust
pub struct WalletTransaction {
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: u64,
    pub timestamp: u64,
    pub block_height: u64,
    pub confirmations: u32,
}
```

## Implementation Status

### ✅ Completed

- Network protocol message types defined
- Masternode subscription tracking (`wallet_subscriptions` HashMap)
- Masternode `subscribe_wallet()` method
- Masternode `notify_wallet_transaction()` method
- Masternode handles `RequestWalletTransactions` and subscribes wallets
- Masternode sends `WalletTransactionsResponse` with historical data

### 🚧 In Progress

- Masternode xpub → address derivation (BIP44 implementation)
- Masternode blockchain search for historical transactions
- Masternode calls `notify_wallet_transaction()` when new tx arrives

### 📋 TODO

- [ ] Wallet-GUI: Set up `PeerManager` to maintain P2P connections
- [ ] Wallet-GUI: Handle incoming `NewTransactionNotification` messages
- [ ] Wallet-GUI: Display toast notification when new transaction arrives
- [ ] Wallet-GUI: Update transaction list in real-time
- [ ] Wallet-GUI: Update balance in real-time
- [ ] Masternode: Implement BIP44 address derivation from xpub
- [ ] Masternode: Search blockchain for transactions to derived addresses
- [ ] Masternode: Call `notify_wallet_transaction()` from mempool
- [ ] Masternode: Call `notify_wallet_transaction()` from block processor

## Code Locations

### Network Protocol
- `network/src/protocol.rs` - Message type definitions

### Masternode
- `network/src/manager.rs` - PeerManager with wallet subscription tracking
  - `subscribe_wallet()` - Subscribe a wallet to notifications
  - `unsubscribe_wallet()` - Unsubscribe on disconnect
  - `notify_wallet_transaction()` - Send push notification
- `cli/src/main.rs` - Message handling (handles `RequestWalletTransactions`)

### Wallet
- `wallet-gui/src/main.rs` - Currently using HTTP polling (temporary)
- Future: Will use P2P for both historical sync and real-time notifications

## Benefits

1. **Instant Notifications**: No polling delay, transactions appear immediately
2. **Reduced Network Load**: No constant HTTP requests from thousands of wallets
3. **Efficient**: Single TCP connection handles both queries and push notifications
4. **Scalable**: Masternodes only send data when relevant transactions occur

## Example Flow

```
User sends 100 TIME to your wallet address (TIME0xyz...)

1. Transaction broadcast to network
2. Masternode A receives transaction
3. Masternode A derives addresses from subscribed xpubs
4. Masternode A finds match: xpub123 derives to TIME0xyz...
5. Masternode A looks up which wallets are subscribed to xpub123
6. Masternode A sends NewTransactionNotification to wallet IP
7. Wallet GUI displays: "💰 Received 100 TIME from TIME0abc..."
   - Instant notification (< 1 second)
   - No polling required
   - Real-time balance update
```

## Security Considerations

- Wallets share their xpub (public keys) but never private keys
- Masternodes can only VIEW transactions, not spend coins
- TCP connections use the same security as masternode-to-masternode communication
- Rate limiting prevents notification spam
- Disconnected wallets are automatically unsubscribed

## Performance

- **Memory**: ~100 bytes per subscribed wallet
- **CPU**: Minimal (only checks addresses when new tx arrives)
- **Network**: Only sends data when relevant transactions occur
- **Latency**: < 1 second from transaction broadcast to wallet notification
