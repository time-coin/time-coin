# HD Wallet Implementation

## Overview

TIME Coin supports **Hierarchical Deterministic (HD) wallets** using BIP-32/BIP-44 standards with **xpub-based address discovery**. This allows you to generate unlimited receiving addresses from a single mnemonic phrase, with efficient blockchain synchronization.

## Key Features

### 1. **BIP-39 Mnemonic Phrases**
- Generate 12 or 24-word recovery phrases
- Industry-standard word list
- Optional passphrase for additional security
- Phrases are **never stored** - must be written down and kept safe

### 2. **Extended Public Key (xpub) Generation**
- Automatically generated from mnemonic during wallet creation
- Stored securely in `time-wallet.dat`
- Enables masternode to discover all wallet addresses
- Privacy-preserving: read-only access, no spending capability

### 3. **HD Address Derivation**
- Uses BIP-44 derivation path: `m/44'/0'/account'`
- All addresses can be recovered from the mnemonic
- No need to backup individual keys
- Sequential address generation

### 4. **Network-Specific Address Prefixes**
- **TIME1** = Mainnet addresses (version byte 0x00)
- **TIME0** = Testnet addresses (version byte 0x6F)
- Instant visual identification prevents mixing networks
- Better than Bitcoin's approach - same structure, clear difference

### 5. **Efficient Blockchain Sync**
- Wallet sends xpub to masternode (not all addresses!)
- Masternode derives addresses and scans blockchain automatically
- Gap limit of 20 consecutive empty addresses
- Discovers all used addresses without manual address generation

### 6. **Wallet GUI Integration**
- Create wallet from mnemonic phrase
- Generate unlimited receiving addresses
- Address book with name/email/phone (stored in database)
- QR codes for easy sharing
- Separate send/receive contact management
- Print recovery phrase (no digital copy)

## Address Format

### TIME0 vs TIME1

**Testnet (TIME0)**:
```
TIME0xYz789qWe4rT5uI6oP7aS8dF9gH0jK
```
- Version byte: `0x6F` (decimal 111)
- For testing and development
- No real value

**Mainnet (TIME1)**:
```
TIME1aBc123DeF456GhI789JkL012MnO345
```
- Version byte: `0x00` (decimal 0)
- Real coins with real value
- Production use only

**Benefits**:
- Same private key generates different addresses on each network
- Prevents accidental mixing of testnet and mainnet coins
- Clear visual distinction: 0 = test, 1 = real
- Checksum includes network version for validation

## How It Works

### Creating a Wallet

1. **Generate Mnemonic**: 
   ```
   12 or 24 random words from BIP-39 word list
   ```

2. **Derive Master Key**:
   ```
   Mnemonic → Seed (512 bits) → Master Extended Private Key
   ```

3. **Generate xpub**:
   ```
   Master Key → Extended Public Key (xpub) at m/44'/0'/account'
   Stored in time-wallet.dat for sync
   ```

4. **Generate Addresses**:
   ```
   Master Key → m/44'/0'/0' → TIME1abc... (address 1)
   Master Key → m/44'/0'/1' → TIME1def... (address 2)
   Master Key → m/44'/0'/2' → TIME1ghi... (address 3)
   ...and so on
   ```

### Wallet Sync with Masternode

**Traditional Method** (Legacy):
```
Wallet → Send 100 addresses → Masternode
← Scan blockchain for each address ←
```

**xpub Method** (New & Efficient):
```
Wallet → Send xpub only → Masternode
                         Masternode derives addresses automatically
                         Scans blockchain with gap limit
← Returns all transactions & UTXOs ←
```

**Gap Limit Algorithm**:
1. Masternode derives address at index 0, 1, 2, ...
2. If address has transactions: reset gap counter
3. If address is empty: increment gap counter
4. Stop when gap counter reaches 20
5. Maximum safety limit: 1000 addresses

**Benefits**:
- **Privacy**: Only one key sent instead of many addresses
- **Efficiency**: No need to pre-generate addresses in wallet
- **Auto-discovery**: Finds all used addresses automatically
- **Future-proof**: Works with unlimited addresses

### Wallet Recovery

With your mnemonic phrase, you can:
- Recover all addresses (via xpub derivation)
- Restore your entire wallet
- Access funds on any device
- Automatically find all used addresses (via gap limit scan)

## Usage

### In GUI Wallet

1. **First Time Setup**:
   - Launch wallet
   - Select "Generate New Phrase" or "Enter Existing Phrase"
   - Write down your 12/24 words
   - Print backup (optional, but recommended)
   - Create wallet
   - xpub automatically generated and stored

2. **Generate New Address**:
   - Go to "Receive" screen
   - Click "➕ New Address"
   - Enter label/name (stored in database, not wallet file)
   - Address is automatically derived from your mnemonic
   - Contact info (name/email/phone) saved separately

3. **Send Coins**:
   - Go to "Send" screen
   - Add recipient address manually or scan QR code
   - Enter amount and contact info
   - Send addresses stored separately from receive addresses

4. **Transaction Sync**:
   - Automatic on wallet startup
   - Click "Sync" button to manually refresh
   - Wallet sends xpub to masternode
   - Masternode discovers all addresses and returns transactions

### In Code

```rust
use wallet::{Wallet, NetworkType};

// Create wallet from mnemonic with xpub generation
let mnemonic = "word1 word2 word3 ... word12";
let wallet = Wallet::from_mnemonic(
    mnemonic, 
    "", // passphrase (optional)
    NetworkType::Mainnet
)?;

// xpub is automatically generated
let xpub = wallet.get_xpub()?;

// Sync with masternode using xpub
let response = network.sync_wallet_transactions_xpub(xpub).await?;

// Process discovered transactions
for tx in response.recent_transactions {
    println!("Found transaction: {} TIME", tx.amount as f64 / 100_000_000.0);
}

// Generate new addresses (HD derivation)
let address1 = wallet.generate_new_address()?;
let address2 = wallet.generate_new_address()?;

// All addresses can be recovered with the same mnemonic
```

## Security

### ✅ **Good Practices**
- Write down your mnemonic on paper
- Store it in a safe place (fireproof safe, safe deposit box)
- Never share your mnemonic with anyone
- Consider using a passphrase for additional security
- Print your recovery phrase when creating wallet
- **Do not display private keys** (wallet GUI removed this feature)
- Keep `time-wallet.dat` file secure (contains xpub but no mnemonic)
- Contact info stored in database is separate from keys

### ❌ **Bad Practices**
- Don't store mnemonic digitally (screenshots, text files, etc.)
- Don't save to cloud storage
- Don't send via email/messaging apps
- Don't take photos with your phone
- Don't share with support staff (we'll never ask)
- Don't display private keys on screen
- Don't mix testnet (TIME0) and mainnet (TIME1) addresses

### 🔒 **Data Separation**
- **time-wallet.dat**: Stores keys and xpub (encrypted, critical)
- **wallet.db** (sled): Stores contact info, labels, transaction cache (can be rebuilt)
- **Mnemonic**: Never stored anywhere (user must backup)

If `wallet.db` is lost: Wallet can resync from blockchain using xpub
If `time-wallet.dat` is lost: Wallet can be recovered from mnemonic
If mnemonic is lost: **Funds are irrecoverable!**

## Technical Details

### Address Format

**Structure**: `PREFIX` + Base58(`version_byte` + `hash160` + `checksum`)

Where:
- **PREFIX**: `TIME1` (mainnet) or `TIME0` (testnet)
- **version_byte**: `0x00` (mainnet) or `0x6F` (testnet)
- **hash160**: SHA256(public_key) → RIPEMD160 = 20 bytes
- **checksum**: First 4 bytes of SHA256(SHA256(version + hash160))

**Example**:
```
Public Key (32 bytes)
   ↓ SHA256
   ↓ RIPEMD160
Hash160 (20 bytes)
   ↓ Prepend version byte
   ↓ Calculate checksum
   ↓ Base58 encode
   ↓ Prepend network prefix
TIME1aBc123DeF456... (mainnet)
or
TIME0xYz789qWe012... (testnet)
```

### Derivation Path

TIME Coin uses BIP-44 standard derivation:
```
m / purpose' / coin_type' / account' / change / address_index
m / 44'      / 0'         / account' / 0     / index
```

- **Purpose**: 44' (BIP-44)
- **Coin Type**: 0' (Bitcoin-compatible, will register custom later)
- **Account**: Increments for each new address (0', 1', 2', ...)
- **Change**: 0 for receiving addresses, 1 for change addresses
- **Address Index**: Reserved for future use

### xpub Derivation

```
Mnemonic (12/24 words)
   ↓ BIP-39 (PBKDF2-HMAC-SHA512)
Seed (512 bits)
   ↓ BIP-32
Master Extended Private Key
   ↓ Derive m/44'/0'/0'
Account Extended Private Key
   ↓ Get public key
Account Extended Public Key (xpub)
   ↓ Store in wallet
xpub661MyMwAqRbcF...
```

The xpub can then derive child addresses without needing private keys:
```
xpub → m/0 → Address 0
xpub → m/1 → Address 1
xpub → m/2 → Address 2
...
```

### Cryptography

- **Mnemonic**: BIP-39 (2048-word English list)
- **Seed Derivation**: PBKDF2-HMAC-SHA512 (2048 iterations)
- **Key Derivation**: BIP-32 (HMAC-SHA512)
- **Address Hashing**: SHA256 + RIPEMD160
- **Encoding**: Base58Check with checksum
- **Signing**: Ed25519 (curve25519-dalek)

### Dependencies

- `bip39 = "2.2"` - Mnemonic generation and validation
- `bip32 = "0.5"` - HD key derivation and xpub generation
- `ed25519-dalek = "2.1"` - Digital signatures
- `sha2` - SHA256 hashing for addresses
- `ripemd` - RIPEMD160 hashing for addresses
- `bs58` - Base58 encoding for addresses

## Data Storage

### time-wallet.dat
**Binary format**, stores:
- Network type (mainnet/testnet)
- KeyEntry list (address + encrypted private key)
- **xpub** (extended public key for sync)
- Checksum for integrity

**Security**: Should be encrypted and backed up regularly

### wallet.db (sled database)
Stores wallet metadata:
- Contact information (name, email, phone)
- Address labels
- Transaction cache
- UTXOs
- `is_owned` flag (true = receive address, false = send address)

**Recovery**: Can be rebuilt from blockchain if lost

## Wallet Sync Protocol

### 1. Legacy Address-Based Sync

**Endpoint**: `POST /wallet/sync`

```json
{
  "addresses": [
    "TIME1abc...",
    "TIME1def...",
    "TIME1ghi..."
  ]
}
```

**Issues**:
- Must send all addresses (privacy concern)
- Wallet must pre-generate addresses
- Inefficient for HD wallets with many addresses

### 2. xpub-Based Sync (Recommended)

**Endpoint**: `POST /wallet/sync-xpub`

```json
{
  "xpub": "xpub661MyMwAqRbcF...",
  "start_index": 0
}
```

**Response**:
```json
{
  "utxos": {
    "TIME1abc...": [{ "tx_hash": "...", "amount": 100000, ... }]
  },
  "total_balance": 500000,
  "recent_transactions": [...],
  "current_height": 1240,
  "addresses_scanned": 42,
  "addresses_with_activity": 15
}
```

**Algorithm** (Masternode side):
```rust
let mut gap_count = 0;
let mut index = start_index;

while gap_count < 20 && index < 1000 {
    let address = derive_address_from_xpub(xpub, 0, index, network)?;
    
    if has_transactions(address) {
        collect_utxos(address);
        gap_count = 0; // Reset on activity
    } else {
        gap_count += 1;
    }
    
    index += 1;
}
```

**Benefits**:
- Privacy: Only one xpub sent
- Efficiency: Automatic address discovery
- Simple: Wallet doesn't manage address list
- Standard: BIP-44 gap limit (20)

## Migration

### Existing Wallets (Pre-xpub)

Old wallets without mnemonics will continue to work:
- Random address generation (non-HD)
- All existing addresses preserved
- Can't recover from mnemonic (backup wallet file instead)
- Must use legacy address-based sync
- **Recommendation**: Create new HD wallet and transfer funds

### New Wallets (Post-xpub)

All new wallets created from mnemonics:
- Full HD wallet support
- xpub automatically generated and stored
- Recoverable from 12/24 words
- Unlimited addresses with efficient sync
- TIME0/TIME1 address prefixes based on network

### Upgrading to xpub

If you have an old wallet without xpub:
1. **Backup** your current `time-wallet.dat` and mnemonic
2. **Create new wallet** from your existing mnemonic
3. xpub will be generated automatically
4. Wallet will use efficient xpub-based sync
5. All addresses and funds will be recovered

## Future Enhancements

1. ~~**xpub-based Sync**~~: ✅ **IMPLEMENTED** - Efficient address discovery
2. ~~**Network Prefixes**~~: ✅ **IMPLEMENTED** - TIME0/TIME1 distinction
3. ~~**Gap Limit**~~: ✅ **IMPLEMENTED** - Standard BIP-44 gap limit for address scanning
4. **Hardware Wallet**: Support for Ledger/Trezor
5. **Multi-Account**: Support multiple accounts from one seed
6. **Change Addresses**: Implement BIP-44 change address derivation (m/44'/0'/0'/1/index)
7. **Encrypted Storage**: Encrypt time-wallet.dat with user password
8. **Address Validation**: Client-side validation of TIME0/TIME1 format
9. **QR Code Scanning**: Camera-based QR code address input for Send tab

## Troubleshooting

### "xpub not available" error
- Old wallet created before xpub feature
- **Solution**: Recreate wallet from mnemonic to generate xpub

### Addresses not discovered
- Gap limit may have been exceeded (>20 empty addresses)
- **Solution**: Use legacy sync with specific addresses, or check masternode logs

### Wrong network addresses (TIME0 vs TIME1)
- Wallet created on different network
- **Solution**: Create new wallet on correct network, transfer funds

### Transaction sync fails
- Masternode may not support `/wallet/sync-xpub` endpoint
- **Solution**: Update masternode or use legacy `/wallet/sync` endpoint

## See Also

- [BIP-39 Specification](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki) - Mnemonic phrases
- [BIP-32 Specification](https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki) - HD key derivation
- [BIP-44 Specification](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki) - Multi-account hierarchy
- [WALLET_SYNC_API.md](WALLET_SYNC_API.md) - Wallet sync API documentation
- [API.md](API.md) - General API documentation
- [INSTALLATION.md](INSTALLATION.md) - Installation guide
