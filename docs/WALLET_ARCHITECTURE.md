# Wallet Architecture

## Overview

The TIME wallet uses a two-tier storage system to separate cryptographic keys from metadata and transaction history.

## Storage

### 1. time-wallet.dat (Cryptographic Keys Only)

**Location**: `~/.time-coin/{network}/time-wallet.dat`

**Purpose**: Store ONLY the cryptographic keys needed for signing and address derivation.

**Contents**:
- `xpub`: Extended public key for deterministic address derivation (BIP32)
- `encrypted_mnemonic`: AES-256-GCM encrypted BIP-39 mnemonic phrase
- `encrypted_master_key`: AES-256-GCM encrypted master private key
- `network`: Network type (Mainnet or Testnet)

**Size**: ~1KB (fixed, regardless of number of addresses)

**Security**: Encrypted with user password. Never contains plaintext keys.

### 2. wallet.db (Metadata and Transactions)

**Location**: `~/.time-coin/{network}/wallet.db/`

**Purpose**: Store all metadata, derived addresses, and transaction history.

**Contents**:
- **Derived Addresses**:
  - Address string
  - Derivation index (for owned addresses)
  - Is owned (true = receive address, false = send address)
  - Contact name, email, phone
  - Label
  - Is default flag

- **Transactions** (future):
  - Transaction hash
  - Block height
  - Timestamp
  - Inputs/outputs
  - Amount
  - Fee
  - Status

- **UTXO Set** (future):
  - Available unspent outputs
  - Balances per address

**Size**: Grows with usage (addresses + transactions)

**Security**: Unencrypted (contains no private keys)

## Address Derivation

### BIP32 Hierarchical Deterministic (HD) Wallets

Path: `m/44'/0'/0'/0/{index}`

- Account: 0
- Change: 0 (receive addresses)
- Index: 0, 1, 2, ... (sequential address generation)

### Address Types

**Owned Addresses** (`is_owned = true`):
- Generated from wallet's xpub
- Used for receiving coins
- Shown in "Receive" tab
- Have derivation index

**External Addresses** (`is_owned = false`):
- Addresses of other users/contacts
- Used for sending coins
- Shown in "Send" tab
- No derivation index

## Workflow

### 1. Wallet Creation

```
User enters mnemonic phrase + password
  ↓
Generate xpub from mnemonic
  ↓
Encrypt mnemonic with password
  ↓
Encrypt master key with password
  ↓
Save to time-wallet.dat
  ↓
Initialize empty wallet.db
```

### 2. Generating Receive Address

```
User clicks "Generate New Address"
  ↓
Get next index from wallet.db
  ↓
Derive address from xpub + index
  ↓
Store address in wallet.db with index, is_owned=true
  ↓
Show address in UI
```

### 3. Adding Send Address

```
User scans QR code or enters address manually
  ↓
Validate address format
  ↓
Store address in wallet.db with is_owned=false
  ↓
User can add contact info (name, email, phone)
```

### 4. Sending Transaction

```
User selects send address and amount
  ↓
Prompt for password
  ↓
Decrypt master key from time-wallet.dat
  ↓
Derive signing key for source address
  ↓
Sign transaction
  ↓
Broadcast to network
  ↓
Store transaction in wallet.db
```

### 5. Transaction Sync

```
Wallet connects to masternode
  ↓
Send xpub to masternode
  ↓
Masternode derives all addresses (gap limit: 20)
  ↓
Masternode searches blockchain for transactions
  ↓
Masternode returns transactions
  ↓
Wallet stores transactions in wallet.db
  ↓
Update UTXO set and balances
```

## Benefits

1. **Security**:
   - Private keys never leave time-wallet.dat
   - Metadata can be backed up separately without exposing keys
   - Password required only for signing

2. **Efficiency**:
   - No need to store every address in wallet file
   - Derive addresses on-demand from xpub
   - Minimal wallet file size

3. **Privacy**:
   - Generate new address for each transaction
   - No address reuse
   - Contact info separate from keys

4. **Backup**:
   - Mnemonic phrase = full wallet recovery
   - Metadata can be regenerated or imported separately

## Address Prefixes

- **Mainnet**: `TIME1...` 
- **Testnet**: `TIME0...`

This makes it immediately clear which network an address belongs to.

## Future Enhancements

1. **Multi-account support**: m/44'/0'/{account}'/0/{index}
2. **Change addresses**: m/44'/0'/0'/1/{index}
3. **Hardware wallet support**: Sign with external device
4. **Watch-only wallets**: Import xpub without private keys
5. **Transaction labels and notes**
6. **Address book sharing** (encrypted)
7. **HD wallet standards**: BIP44, BIP49 (SegWit), BIP84 (Native SegWit)

## Gap Limit

The masternode uses a gap limit of 20 when scanning for addresses:
- Derive addresses from xpub sequentially
- Stop when 20 consecutive unused addresses are found
- This ensures all used addresses are discovered while limiting unnecessary derivations

## Contact Information

Both receive and send addresses can have associated contact information:
- **Receive addresses**: Who is sending TO you
- **Send addresses**: Who you are sending TO

This allows tracking both incoming and outgoing transaction participants.
