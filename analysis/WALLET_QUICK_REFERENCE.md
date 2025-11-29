# Quick Reference: Wallet-Masternode Communication

## Overview

✅ **Two-way communication is working**  
✅ **Blockchain scanning happens on startup**  
✅ **All transactions saved to database**

## How to Test

### 1. Start Masternode

```bash
cd masternode
cargo run --release -- --testnet
```

### 2. Start Wallet GUI

```bash
cd wallet-gui
cargo run --release
```

### 3. Watch the Logs

**Wallet should show:**
```
📂 Found X peers in database
🔗 Connecting NetworkManager to discovered peers...
✅ NetworkManager connected to peers successfully
🔌 Initializing TCP listener for xpub monitoring
🔗 Starting TCP listener for <peer>
📤 Registering xpub: tpub...
📥 Received UTXO update: X UTXOs
💰 Processing new UTXO: 1.5 TIME
💾 Saved transaction to database
💼 Updated balance: 1.5 TIME
```

**Masternode should show:**
```
Received xpub registration request
Xpub registered successfully, now scanning for UTXOs...
Scanning blockchain for existing transactions
Found X UTXOs for xpub
Blockchain scan completed
```

## Architecture

### Startup Flow
1. Wallet connects to masternode via TCP (port 24100 testnet / 24101 mainnet)
2. Wallet sends `RegisterXpub` with public key
3. Masternode generates 200 addresses (100 external + 100 internal)
4. Masternode scans blockchain database for those addresses
5. Masternode sends `UtxoUpdate` with all found UTXOs
6. Wallet saves to database and updates balance

### Real-time Updates
1. New block created
2. Masternode validates and checks outputs
3. If output matches monitored address → sends `UtxoUpdate`
4. Wallet receives and processes immediately

## Key Files

### Wallet GUI
- `wallet-gui/src/main.rs` - Main application logic
  - `auto_load_wallet()` - Initializes connection on startup
  - `check_utxo_updates()` - Processes incoming UTXOs and saves to DB
  
- `wallet-gui/src/tcp_protocol_client.rs` - TCP communication
  - `TcpProtocolListener` - Maintains persistent connection
  - Handles xpub registration and UTXO updates

### Masternode
- `masternode/src/utxo_integration.rs` - Message handling
  - Processes `RegisterXpub` messages
  - Triggers blockchain scan
  - Returns `UtxoUpdate` response

- `masternode/src/blockchain_scanner.rs` - Blockchain scanning
  - `scan_blockchain()` - Scans entire chain for addresses
  - `scan_for_xpub()` - Scans for specific xpub

- `masternode/src/address_monitor.rs` - Address derivation
  - Generates addresses from xpub
  - Tracks monitored addresses

- `masternode/src/utxo_tracker.rs` - UTXO management
  - Tracks UTXOs per xpub
  - Provides UTXO queries

## Protocol Messages

### Wallet → Masternode
| Message | Purpose |
|---------|---------|
| `RegisterXpub { xpub }` | Register wallet for monitoring |
| `GetMempool` | Check pending transactions |
| `Ping` | Health check |

### Masternode → Wallet
| Message | Purpose |
|---------|---------|
| `UtxoUpdate { xpub, utxos }` | Send UTXOs (historical or new) |
| `NewTransactionNotification { transaction }` | Real-time update |
| `Pong` | Health check response |

## Database

### Location
- **Linux/Mac:** `~/.time-coin/wallet.db`
- **Windows:** `%USERPROFILE%\.time-coin\wallet.db`

### Tables
- `transactions` - All sent/received transactions
- `contacts` - Address book (owned and external addresses)

### Query Examples
```sql
-- View all transactions
SELECT * FROM transactions ORDER BY timestamp DESC;

-- View owned addresses
SELECT * FROM contacts WHERE is_owned = 1;

-- Check balance (sum of confirmed UTXOs)
SELECT SUM(amount) FROM transactions 
WHERE status = 'Confirmed' AND to_address IN 
  (SELECT address FROM contacts WHERE is_owned = 1);
```

## Troubleshooting

### Wallet not connecting?
1. Check masternode is running and accessible
2. Check firewall allows TCP port 24100/24101
3. Check config.toml has correct bootstrap nodes

### No transactions showing?
1. Check wallet logs for UTXO updates
2. Verify xpub was registered (look for "Registering xpub" log)
3. Check masternode logs for blockchain scan completion
4. Query database directly to see if transactions were saved

### Balance not updating?
1. Check `check_utxo_updates()` is being called (should be automatic)
2. Verify transactions are in database
3. Restart wallet to force full sync

## Security Notes

- ✅ Only xPub is shared (not private keys)
- ✅ Private keys never leave the wallet
- ✅ Wallet file is encrypted on disk
- ✅ Addresses are derived deterministically from xPub
- ⚠️ TCP communication is not encrypted (TLS optional future enhancement)

## Next Steps

Want to enhance the system? Consider:
- [ ] Multiple masternode connections for redundancy
- [ ] TLS/SSL encrypted communication
- [ ] Progress indicator during initial blockchain scan
- [ ] Transaction filtering (date ranges, amounts)
- [ ] Export transaction history to CSV
- [ ] Hardware wallet support

## More Documentation

- **Full Technical Docs:** `docs/WALLET_MASTERNODE_COMMUNICATION.md`
- **Implementation Summary:** `docs/WALLET_SYNC_SUMMARY.md`
