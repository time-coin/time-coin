# Wallet-Masternode Two-Way Communication

## Overview

The TIME Coin wallet GUI has full two-way communication with masternodes using TCP-based protocol. This enables:
- **Blockchain scanning on startup** - Historical transactions are automatically discovered
- **Real-time transaction notifications** - New transactions are pushed to the wallet instantly
- **Balance synchronization** - UTXOs are tracked and wallet balance is kept up-to-date

## Architecture

### Communication Flow

```
┌─────────────────┐                    ┌─────────────────┐
│   GUI Wallet    │◄──────TCP─────────►│   Masternode    │
│                 │                    │                 │
│ 1. Registers    │───RegisterXpub────►│ 2. Scans        │
│    xPub on      │                    │    Blockchain   │
│    Startup      │                    │    Database     │
│                 │                    │                 │
│ 4. Receives &   │◄──UtxoUpdate──────│ 3. Sends Found  │
│    Saves UTXOs  │                    │    UTXOs        │
│                 │                    │                 │
│ 6. Updates UI   │◄──Notifications───│ 5. Monitors New │
│    Balance      │                    │    Blocks       │
└─────────────────┘                    └─────────────────┘
```

## Startup Sequence

### 1. Wallet Initialization (`auto_load_wallet()`)

When the wallet starts:

```rust
// 1. Load wallet from disk
WalletManager::load(network)

// 2. Extract xPub for address monitoring
let wallet_xpub = manager.get_xpub().to_string();

// 3. Initialize network manager and connect to peers
let network_mgr = NetworkManager::new(api_endpoint);
network_mgr.bootstrap().await

// 4. Start TCP listener for real-time updates
TcpProtocolListener::new(peer_addr, wallet_xpub, utxo_tx)
```

### 2. Xpub Registration

The TCP listener automatically registers the wallet's xPub with the masternode:

```rust
// Send RegisterXpub message
let register_msg = NetworkMessage::RegisterXpub { 
    xpub: wallet_xpub 
};
```

### 3. Masternode Blockchain Scan

When a masternode receives `RegisterXpub`:

```rust
// 1. Register xpub in AddressMonitor
address_monitor.register_xpub(xpub).await

// 2. Generate addresses from xpub (external and internal chains)
//    - External chain (receiving): m/0/0 to m/0/99
//    - Internal chain (change): m/1/0 to m/1/99

// 3. Scan entire blockchain for UTXOs
let scanner = BlockchainScanner::new(db, address_monitor, utxo_tracker);
scanner.scan_blockchain().await

// 4. Send all found UTXOs back to wallet
NetworkMessage::UtxoUpdate {
    xpub,
    utxos: found_utxos
}
```

### 4. Wallet UTXO Processing

The wallet receives and processes UTXOs:

```rust
fn check_utxo_updates(&mut self) {
    for utxo in pending_utxos {
        // 1. Add to wallet manager (updates balance)
        wallet_mgr.add_utxo(wallet_utxo);
        
        // 2. Save to database (for transaction history)
        db.save_transaction(&tx_record);
        
        // 3. Update UI
        self.set_success(format!("Received {} TIME!", amount));
    }
}
```

## Two-Way Communication Protocol

### Messages: Wallet → Masternode

| Message | Purpose | Response |
|---------|---------|----------|
| `RegisterXpub` | Register wallet for monitoring | `UtxoUpdate` with historical UTXOs |
| `GetMempool` | Check for pending transactions | `MempoolResponse` |
| `SubmitTransaction` | Broadcast new transaction | Transaction confirmation |
| `Ping` | Check connection health | `Pong` |

### Messages: Masternode → Wallet

| Message | Purpose | Wallet Action |
|---------|---------|---------------|
| `UtxoUpdate` | Send found UTXOs (historical or new) | Add to wallet, save to DB, update UI |
| `NewTransactionNotification` | Real-time new transaction | Same as UtxoUpdate |
| `BlocksData` | Send blockchain data | Process transactions |
| `Pong` | Respond to ping | Update connection status |

## Implementation Details

### File: `wallet-gui/src/tcp_protocol_client.rs`

**TcpProtocolListener**: Maintains persistent TCP connection with masternode

```rust
pub struct TcpProtocolListener {
    peer_addr: String,
    xpub: String,
    utxo_tx: mpsc::UnboundedSender<UtxoInfo>,
}

impl TcpProtocolListener {
    pub async fn start(self) {
        loop {
            // Connect and perform handshake
            // Register xpub
            // Listen for UtxoUpdate messages
            // Reconnect if disconnected
        }
    }
}
```

### File: `masternode/src/blockchain_scanner.rs`

**BlockchainScanner**: Scans blockchain for wallet addresses

```rust
pub struct BlockchainScanner {
    db: Arc<BlockchainDB>,
    address_monitor: Arc<AddressMonitor>,
    utxo_tracker: Arc<UtxoTracker>,
}

impl BlockchainScanner {
    pub async fn scan_blockchain(&self) -> Result<usize> {
        // Load all blocks from database
        // For each transaction output:
        //   - Check if address is monitored
        //   - If yes, add to UTXO tracker
        // Return count of UTXOs found
    }
}
```

### File: `masternode/src/utxo_integration.rs`

**UTXO Integration**: Handles wallet messages

```rust
async fn handle_wallet_message(
    &mut self,
    message: NetworkMessage,
) -> Result<Option<NetworkMessage>> {
    match message {
        NetworkMessage::RegisterXpub { xpub } => {
            // 1. Register with address monitor
            self.address_monitor.register_xpub(xpub).await?;
            
            // 2. Subscribe to UTXO tracker
            self.utxo_tracker.subscribe_xpub(xpub).await?;
            
            // 3. Scan blockchain
            let scanner = BlockchainScanner::new(...);
            scanner.scan_blockchain().await?;
            
            // 4. Get UTXOs and send to wallet
            let utxos = self.utxo_tracker.get_utxos_for_xpub(xpub).await?;
            Ok(Some(NetworkMessage::UtxoUpdate { xpub, utxos }))
        }
        // ... other messages
    }
}
```

## Database Integration

### Wallet Database (`wallet.db`)

**Tables**:
- `transactions` - All received/sent transactions
- `contacts` - Address book (including owned addresses)
- `owned_addresses` - Wallet's receiving addresses with derivation indices

**Transaction Records**:
```rust
pub struct TransactionRecord {
    pub tx_hash: String,
    pub timestamp: i64,
    pub from_address: Option<String>,
    pub to_address: String,
    pub amount: u64,
    pub status: TransactionStatus, // Pending, Confirmed, Approved, Failed
    pub block_height: Option<u64>,
    pub notes: Option<String>,
}
```

When UTXOs are received from blockchain scan:
```rust
let tx_record = TransactionRecord {
    tx_hash: utxo.txid.clone(),
    timestamp: chrono::Utc::now().timestamp(),
    from_address: None,
    to_address: utxo.address.clone(),
    amount: utxo.amount,
    status: TransactionStatus::Confirmed,
    block_height: utxo.block_height,
    notes: Some(format!("Scanned from blockchain (height: {})", height)),
};
db.save_transaction(&tx_record)?;
```

## Testing the Flow

### 1. Start a Masternode

```bash
cd masternode
cargo run --release -- --testnet
```

### 2. Start the Wallet GUI

```bash
cd wallet-gui
cargo run --release
```

### 3. Check Logs

**Wallet logs should show**:
```
✅ Connected to masternode
📤 Registering xpub: tpub...
📥 Received UTXO update: 5 UTXOs
💰 Processing new UTXO: 1.5 TIME to TIME1...
💾 Saved transaction to database
```

**Masternode logs should show**:
```
Received xpub registration request
Scanning blockchain for existing transactions
Found 5 UTXOs for xpub
Blockchain scan complete, found 5 UTXOs
```

## Ongoing Monitoring

After initial scan, the wallet continues to receive updates:

1. **Mempool monitoring** - Checks for pending transactions every 30 seconds
2. **Block updates** - New blocks trigger UTXO updates for monitored addresses
3. **Real-time notifications** - Masternodes push new transactions instantly

## Benefits

✅ **No manual refresh needed** - Transactions appear automatically
✅ **Historical transactions recovered** - Blockchain scan finds all past UTXOs
✅ **Real-time balance updates** - New transactions update balance instantly
✅ **Persistent connection** - TCP listener reconnects if disconnected
✅ **Database persistence** - All transactions saved for history view

## Security Considerations

- **xPub only** - Private keys never leave the wallet
- **Address derivation** - Masternode only monitors derived addresses
- **Encrypted wallet** - Wallet file contains encrypted private keys
- **TLS optional** - Can add TLS for encrypted communication (future)

## Future Enhancements

- [ ] Multiple masternode connections for redundancy
- [ ] Encrypted communication (TLS/SSL)
- [ ] Advanced filtering (date ranges, amounts)
- [ ] Transaction labeling and notes
- [ ] Multi-signature wallet support
- [ ] Hardware wallet integration
