# HD Wallet Implementation

## Overview

TIME Coin now supports **Hierarchical Deterministic (HD) wallets** using BIP-32/BIP-44 standards. This allows you to generate unlimited receiving addresses from a single mnemonic phrase, all of which can be recovered using that same phrase.

## Features

### 1. **BIP-39 Mnemonic Phrases**
- Generate 12 or 24-word recovery phrases
- Industry-standard word list
- Optional passphrase for additional security
- Phrases are **never stored** - must be written down and kept safe

### 2. **HD Address Derivation**
- Uses BIP-44 derivation path: `m/44'/0'/account'`
- All addresses can be recovered from the mnemonic
- No need to backup individual keys
- Sequential address generation

### 3. **Wallet GUI Integration**
- Create wallet from mnemonic phrase
- Generate unlimited receiving addresses
- Address book with name/email/phone
- QR codes for easy sharing
- Print recovery phrase (no digital copy)

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

3. **Generate Addresses**:
   ```
   Master Key → m/44'/0'/0' → Address 1
   Master Key → m/44'/0'/1' → Address 2
   Master Key → m/44'/0'/2' → Address 3
   ...and so on
   ```

### Wallet Recovery

With your mnemonic phrase, you can:
- Recover all addresses
- Restore your entire wallet
- Access funds on any device

## Usage

### In GUI Wallet

1. **First Time Setup**:
   - Launch wallet
   - Select "Generate New Phrase" or "Enter Existing Phrase"
   - Write down your 12/24 words
   - Print backup (optional, but recommended)
   - Create wallet

2. **Generate New Address**:
   - Go to "Receive" screen
   - Click "➕ New Address"
   - Enter label/name
   - Address is automatically derived from your mnemonic

3. **Address Book**:
   - Add name, email, or phone to addresses
   - Helps identify payments
   - Makes the wallet more user-friendly

### In Code

```rust
use wallet::{Wallet, NetworkType};

// Create wallet from mnemonic
let mnemonic = "word1 word2 word3 ... word12";
let wallet = Wallet::from_mnemonic(
    mnemonic, 
    "", // passphrase (optional)
    NetworkType::Mainnet
)?;

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

### ❌ **Bad Practices**
- Don't store mnemonic digitally (screenshots, text files, etc.)
- Don't save to cloud storage
- Don't send via email/messaging apps
- Don't take photos with your phone
- Don't share with support staff (we'll never ask)

## Technical Details

### Derivation Path

TIME Coin uses BIP-44 standard derivation:
```
m / purpose' / coin_type' / account' / change / address_index
m / 44'      / 0'         / account' / 0     / index
```

- **Purpose**: 44' (BIP-44)
- **Coin Type**: 0' (Bitcoin-compatible, will register custom later)
- **Account**: Increments for each new address (0', 1', 2', ...)
- **Change**: 0 for receiving addresses
- **Address Index**: Reserved for future use

### Cryptography

- **Mnemonic**: BIP-39 (2048-word English list)
- **Seed Derivation**: PBKDF2-HMAC-SHA512
- **Key Derivation**: BIP-32 (HMAC-SHA512)
- **Signing**: Ed25519 (curve25519-dalek)

### Dependencies

- `bip39 = "2.2"` - Mnemonic generation and validation
- `bip32 = "0.5"` - HD key derivation
- `ed25519-dalek = "2.1"` - Digital signatures

## Migration

### Existing Wallets

Old wallets without mnemonics will continue to work:
- Random address generation (non-HD)
- All existing addresses preserved
- Can't recover from mnemonic (backup wallet file instead)

### New Wallets

All new wallets created from mnemonics:
- Full HD wallet support
- Recoverable from 12/24 words
- Unlimited addresses

## Future Enhancements

1. **Encryption**: Encrypt mnemonic in wallet file
2. **Hardware Wallet**: Support for Ledger/Trezor
3. **Multi-Account**: Support multiple accounts from one seed
4. **Change Addresses**: Implement change address derivation
5. **Gap Limit**: Standard BIP-44 gap limit for address scanning

## See Also

- [BIP-39 Specification](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki)
- [BIP-32 Specification](https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki)
- [BIP-44 Specification](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki)
- [WALLET.md](WALLET.md) - General wallet documentation
- [INSTALLATION.md](INSTALLATION.md) - Installation guide
