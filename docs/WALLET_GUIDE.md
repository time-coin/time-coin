# TIME Coin Wallet Complete Guide

**Table of Contents**
- [Overview](#overview)
- [HD Wallet Implementation](#hd-wallet-implementation)
- [Wallet Architecture](#wallet-architecture)
- [Wallet Sync API](#wallet-sync-api)
- [Balance Management](#balance-management)
- [Notifications](#notifications)
- [Mobile Integration](#mobile-integration)
- [Security](#security)
- [Troubleshooting](#troubleshooting)
- [Development Guide](#development-guide)

---

## Overview

TIME Coin wallets support **Hierarchical Deterministic (HD) wallets** using BIP-32/BIP-44 standards with xpub-based address discovery. This comprehensive guide covers all aspects of wallet implementation, synchronization, and security.

---

## HD Wallet Implementation

### Key Features

#### 1. BIP-39 Mnemonic Phrases
- Generate 12 or 24-word recovery phrases
- Industry-standard word list
- Optional passphrase for additional security
- Phrases are **never stored** - must be written down and kept safe

#### 2. Extended Public Key (xpub) Generation
- Automatically generated from mnemonic during wallet creation
- Stored securely in `time-wallet.dat`
- Enables masternode to discover all wallet addresses
- Privacy-preserving: read-only access, no spending capability

#### 3. HD Address Derivation
- Uses BIP-44 derivation path: `m/44'/0'/account'`
- All addresses can be recovered from the mnemonic
- No need to backup individual keys
- Sequential address generation

#### 4. Network-Specific Address Prefixes
- **TIME1** = Mainnet addresses (version byte 0x00)
- **TIME0** = Testnet addresses (version byte 0x6F)
- Instant visual identification prevents mixing networks

#### 5. Efficient Blockchain Sync
- Wallet sends xpub to masternode (not all addresses!)
- Masternode derives addresses and scans blockchain automatically
- Gap limit of 20 consecutive empty addresses
- Discovers all used addresses without manual address generation

---

## Wallet Architecture

### Storage System

The TIME wallet uses a two-tier storage system to separate cryptographic keys from metadata and transaction history.

#### 1. time-wallet.dat (Cryptographic Keys Only)

**Location**: `~/.timecoin/{network}/time-wallet.dat`

**Purpose**: Store ONLY the cryptographic keys needed for signing and address derivation.

**Contents**:
- `xpub`: Extended public key for deterministic address derivation (BIP32)
- `encrypted_mnemonic`: AES-256-GCM encrypted BIP-39 mnemonic phrase
- `encrypted_master_key`: AES-256-GCM encrypted master private key
- `network`: Network type (Mainnet or Testnet)

**Size**: ~1KB (fixed, regardless of number of addresses)

**Security**: Encrypted with user password. Never contains plaintext keys.

#### 2. wallet.db (Metadata and Transactions)

**Location**: `~/.timecoin/{network}/wallet.db/`

**Purpose**: Store all metadata, derived addresses, and transaction history.

**Contents**:
- **Derived Addresses**: Address string, derivation index, contact info, labels
- **Transactions**: Transaction history, confirmations, amounts
- **UTXO Set**: Available unspent outputs

---

## Wallet Sync API

### Synchronization Methods

#### xpub-Based Sync (Recommended)
- Wallet sends **extended public key (xpub)** only
- Masternode derives addresses automatically using BIP-44
- Gap limit algorithm discovers all used addresses
- Read-only key: no spending capability
- Maximum 1000 addresses scanned per request

#### Address-Based Sync (Legacy)
- Wallet sends **list of addresses** (not private keys)
- Masternode scans blockchain for UTXOs matching those addresses
- Works with non-HD wallets

### API Endpoints

#### POST /wallet/sync/xpub
Register xpub and sync wallet state.

**Request**:
```json
{
  "xpub": "xpub6D4BDPcP2GT577Vvch3R8wDkScZWzQzMMUm3PWbmWvVJrZwQY4VUNgqFJPMM3No2dFDFGTsxxpG5uJh7n7epu4trkrX7x7DogT5Uv6fcLW5",
  "start_index": 0
}
```

**Response**:
```json
{
  "addresses": ["TIME0...", "TIME0..."],
  "utxos": {
    "TIME0abc...": [
      {
        "txid": "abc123...",
        "vout": 0,
        "amount": 100000000,
        "confirmations": 10
      }
    ]
  },
  "next_index": 5,
  "total_balance": 100000000
}
```

#### POST /wallet/register/xpub
Register xpub for real-time notifications.

**Request**:
```json
{
  "xpub": "xpub6D4BDPcP2GT577...",
  "notification_endpoint": "https://wallet.example.com/notify"
}
```

---

## Balance Management

### Balance Persistence

Wallet balances are persisted in the blockchain database to ensure consistency across node restarts.

#### Features:
- **Automatic Saving**: Balance saved after every block update
- **UTXO Snapshot**: Full UTXO set backed up on disk
- **Fast Startup**: Load balance from database on node restart
- **Rescan Available**: Manual rescan if balance seems incorrect

#### Rescan Command:
```bash
time-cli wallet rescan
```

This will:
1. Clear cached balance
2. Recalculate from UTXO set
3. Save new balance to database
4. Display updated balance

---

## Notifications

### Real-Time Transaction Notifications

Wallets can register for real-time notifications of incoming transactions.

#### WebSocket API

**Connect**: `ws://masternode:24101/wallet/notifications`

**Register**:
```json
{
  "action": "register",
  "xpub": "xpub6D4BDPcP2GT577..."
}
```

**Receive Notifications**:
```json
{
  "type": "incoming_transaction",
  "txid": "abc123...",
  "address": "TIME0...",
  "amount": 100000000,
  "confirmations": 0,
  "timestamp": 1234567890
}
```

### Push Notifications (Mobile)

Mobile wallets can register for push notifications via FCM/APNS.

**Register Device**:
```bash
POST /wallet/register/device
```

**Request**:
```json
{
  "xpub": "xpub6D4BDPcP2GT577...",
  "device_token": "fcm_token_here",
  "platform": "ios" // or "android"
}
```

---

## Mobile Integration

### Mobile Wallet Protocol

Mobile wallets use a thin client architecture communicating with masternodes via API.

#### Key Features:
- **No blockchain download**: Mobile app doesn't store blockchain
- **Fast sync**: Query masternode for UTXO state
- **Battery efficient**: No continuous syncing required
- **Privacy preserved**: Only xpub shared, not private keys

#### Mobile Workflow:
1. User creates/imports wallet (generates mnemonic)
2. App derives xpub from mnemonic
3. App registers xpub with masternode
4. Masternode scans blockchain for addresses
5. App receives UTXO state and balance
6. App can create and broadcast transactions

---

## Security

### Key Security Features

#### 1. Encryption
- All private keys encrypted with AES-256-GCM
- Password-protected wallet file
- Mnemonic never stored in plaintext

#### 2. File Protection
- Wallet files have 600 permissions (owner read/write only)
- Automatic backup before modifications
- Safe write: write to temp, then atomic rename

#### 3. Test Safety
- Tests use isolated directories
- Real wallet files never touched during tests
- Cleanup after test runs

#### 4. Communication Security
- TLS for all masternode communication (production)
- xpub provides read-only access
- Private keys never leave the device

### Best Practices

✅ **DO**:
- Write down mnemonic phrase on paper
- Store mnemonic in safe/vault
- Use strong wallet password
- Test recovery process
- Keep software updated

❌ **DON'T**:
- Store mnemonic digitally (no screenshots!)
- Share mnemonic with anyone
- Use weak passwords
- Connect to untrusted masternodes
- Ignore security warnings

---

## Troubleshooting

### Balance Not Showing

**Problem**: Wallet balance shows 0 but you have funds.

**Solution**:
```bash
# Rescan blockchain
time-cli wallet rescan

# Or force full sync
time-cli wallet sync --force
```

### Wallet File Corrupted

**Problem**: Can't open wallet file.

**Solution**:
1. Check backup: `~/.timecoin/backups/time-wallet.dat.backup`
2. Restore from backup
3. If no backup, recover from mnemonic phrase

### Address Not Found

**Problem**: Transaction sent to your address but not detected.

**Solution**:
1. Ensure wallet is synced: `time-cli wallet sync`
2. Check address derivation index increased
3. Verify network (testnet vs mainnet)

### Can't Connect to Masternode

**Problem**: Wallet can't sync with masternode.

**Solution**:
1. Check masternode URL configuration
2. Verify masternode is online: `curl http://masternode:24101/health`
3. Check firewall settings
4. Try different masternode

---

## Development Guide

### Creating a New Wallet

```rust
use time_wallet::Wallet;

// Create new HD wallet
let wallet = Wallet::new(WalletNetworkType::Testnet)?;

// Get mnemonic (show to user ONCE)
let mnemonic = wallet.mnemonic();
println!("Write this down: {}", mnemonic);

// Save wallet (encrypted)
wallet.save_to_file("~/.timecoin/testnet/time-wallet.dat")?;
```

### Importing from Mnemonic

```rust
// Import existing wallet
let mnemonic = "word1 word2 word3 ... word12";
let wallet = Wallet::from_mnemonic(mnemonic, WalletNetworkType::Testnet)?;

// Derive addresses
for i in 0..5 {
    let address = wallet.derive_address(i)?;
    println!("Address {}: {}", i, address);
}
```

### Syncing with Masternode

```rust
// Get xpub
let xpub = wallet.xpub();

// Send to masternode
let response = client
    .post("http://masternode:24101/wallet/sync/xpub")
    .json(&json!({ "xpub": xpub, "start_index": 0 }))
    .send()?;

// Process UTXOs
let sync_data: WalletSyncResponse = response.json()?;
for (address, utxos) in sync_data.utxos {
    println!("Address {}: {} UTXOs", address, utxos.len());
}
```

### Creating Transaction

```rust
// Get UTXOs from sync
let utxos = wallet_sync_response.utxos;

// Create transaction
let tx = wallet.create_transaction(
    "TIME0recipient_address",
    100_000_000, // 1 TIME
    1_000,       // fee
    &utxos
)?;

// Sign transaction
let signed_tx = wallet.sign_transaction(tx)?;

// Broadcast
client
    .post("http://masternode:24101/transaction/broadcast")
    .json(&signed_tx)
    .send()?;
```

---

## Related Documentation

- [API Reference](API.md)
- [Network Protocol](NETWORK_GUIDE.md)
- [Masternode Guide](MASTERNODE_GUIDE.md)
- [Security Hardening](SECURITY_HARDENING.md)

---

**Consolidated from**:
- HD-WALLET.md
- WALLET_ARCHITECTURE.md
- WALLET_BALANCE_PERSISTENCE.md
- WALLET_BALANCE_RESCAN.md
- WALLET_MASTERNODE_COMMUNICATION.md
- WALLET_P2P_COMMUNICATION.md
- WALLET_PROTOCOL_INTEGRATION.md
- WALLET_SYNC_API.md
- WALLET-FILE-PROTECTION.md
- WALLET-TEST-SAFETY.md
- CLI_WALLET_DATABASE_ACCESS.md
- wallet-notifications.md
- wallet-push-notifications.md
- wallet-websocket-api.md
- WALLET_NOTIFICATIONS.md
- XPUB-SYNC-PHASE-2.md
- XPUB-SYNC-PHASE-3.md
- XPUB_UTXO_VERIFICATION.md
