# BIP-39 Mnemonic Wallet Implementation

## Overview
This implementation adds BIP-39 mnemonic phrase support to the TIME Coin hot wallet, addressing issue #89. Users can now create and restore wallets using industry-standard word phrases (12 or 24 words).

## Files Added/Modified

### New Files
- `wallet/src/mnemonic.rs` - Core BIP-39 implementation
- `wallet/examples/mnemonic_wallet_demo.rs` - Comprehensive usage example

### Modified Files
- `wallet/Cargo.toml` - Added bip39 dependency
- `wallet/src/lib.rs` - Exported mnemonic module
- `wallet/src/wallet.rs` - Added `from_mnemonic()` method and tests

## API Reference

### Generating a New Mnemonic

```rust
use wallet::generate_mnemonic;

// Generate a 12-word mnemonic (128 bits of entropy)
let mnemonic_12 = generate_mnemonic(12)?;

// Generate a 24-word mnemonic (256 bits of entropy - more secure)
let mnemonic_24 = generate_mnemonic(24)?;
```

Supported word counts: 12, 15, 18, 21, or 24 words.

### Creating a Wallet from Mnemonic

```rust
use wallet::{Wallet, NetworkType};

// Basic usage (no passphrase)
let wallet = Wallet::from_mnemonic(
    "your twelve word mnemonic phrase here goes like this example phrase",
    "",  // Empty string for no passphrase
    NetworkType::Mainnet
)?;

// With passphrase for additional security
let wallet_secure = Wallet::from_mnemonic(
    "your twelve word mnemonic phrase here goes like this example phrase",
    "my-secure-passphrase",
    NetworkType::Mainnet
)?;
```

### Validating a Mnemonic

```rust
use wallet::validate_mnemonic;

let result = validate_mnemonic("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about");
assert!(result.is_ok());
```

### Using the MnemonicPhrase Wrapper

```rust
use wallet::MnemonicPhrase;

// Generate
let phrase = MnemonicPhrase::generate(12)?;
println!("Mnemonic: {}", phrase);

// Create from string
let phrase = MnemonicPhrase::from_phrase("your mnemonic here")?;

// Derive keypair
let keypair = phrase.to_keypair("")?;
```

## Key Features

1. **Deterministic Wallet Generation**: Same mnemonic always produces the same wallet
2. **Optional Passphrase**: Add an extra layer of security (BIP-39 extension)
3. **Multiple Word Counts**: Support for 12, 15, 18, 21, and 24-word phrases
4. **Full Recovery**: Complete wallet restoration from just the mnemonic
5. **Industry Standard**: Uses well-tested BIP-39 implementation

## Security Considerations

### Best Practices
- ✅ Always store mnemonic phrases offline (paper backup)
- ✅ Use 24-word phrases for maximum security
- ✅ Consider using a passphrase for additional protection
- ✅ Never share your mnemonic phrase with anyone
- ❌ Never store mnemonic digitally (no photos, no cloud storage)
- ❌ Never send mnemonic over network or messaging apps

### Implementation Details
- Uses `bip39` crate v2.2.0 (no known vulnerabilities)
- Ed25519 keys derived from BIP-39 seed using SHA-256
- Passphrase support via BIP-39 standard
- All cryptographic operations use secure random number generation

## Testing

Run all wallet tests including mnemonic tests:
```bash
cargo test -p wallet
```

Run only mnemonic-related tests:
```bash
cargo test -p wallet mnemonic
```

Run the demo example:
```bash
cargo run --example mnemonic_wallet_demo
```

## Code Examples

### Complete Workflow Example

```rust
use wallet::{Wallet, NetworkType, generate_mnemonic, UTXO};

// 1. Generate mnemonic
let mnemonic = generate_mnemonic(12)?;
println!("SAVE THIS: {}", mnemonic);

// 2. Create wallet from mnemonic
let mut wallet = Wallet::from_mnemonic(&mnemonic, "", NetworkType::Mainnet)?;
println!("Address: {}", wallet.address());

// 3. Add funds (UTXOs)
let utxo = UTXO {
    tx_hash: [1u8; 32],
    output_index: 0,
    amount: 100000,
    address: wallet.address_string(),
};
wallet.add_utxo(utxo);

// 4. Create transaction
let recipient = Wallet::new(NetworkType::Mainnet)?;
let tx = wallet.create_transaction(&recipient.address_string(), 1000, 50)?;

// 5. Later, recover wallet from mnemonic
let recovered = Wallet::from_mnemonic(&mnemonic, "", NetworkType::Mainnet)?;
assert_eq!(wallet.address_string(), recovered.address_string());
```

### Wallet Recovery Scenario

```rust
// User loses device but has mnemonic written down
let saved_mnemonic = "wash task faith transfer trumpet horror coach dismiss just humble oppose pony";

// Restore on new device
let wallet = Wallet::from_mnemonic(saved_mnemonic, "", NetworkType::Mainnet)?;
// All addresses and keys are recovered!
```

## Compatibility

- ✅ Compatible with TimeCoin masternode consensus
- ✅ Works with 24-hour time blocks
- ✅ Integrates with existing wallet infrastructure
- ✅ No breaking changes to existing wallet API

## Dependencies

- `bip39 = "2.2"` with features `["serde", "rand"]`
- Uses existing ed25519-dalek for key generation
- Uses SHA-256 for seed-to-key derivation

## Future Enhancements (Not Implemented)

These were considered but not implemented to keep changes minimal:

- BIP-32 HD wallet (hierarchical derivation paths)
- BIP-44 multi-account support
- Multiple address generation from single mnemonic
- Derivation path customization

The current implementation provides a solid foundation for these features if needed in the future.

## Issue Resolution

This implementation fully addresses issue #89:
- ✅ Mnemonic phrase input for private key generation
- ✅ BIP-39 standard implementation
- ✅ Deterministic wallet address generation
- ✅ Wallet recovery from mnemonic
- ✅ Secure handling of mnemonic (never logged)
- ✅ Compatible with TimeCoin architecture
- ✅ User-friendly API with clear documentation

## Questions or Issues?

See the example demo for comprehensive usage: `cargo run --example mnemonic_wallet_demo`

For more information, refer to:
- [BIP-39 Specification](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki)
- TimeCoin documentation
- Test suite in `wallet/src/mnemonic.rs` and `wallet/src/wallet.rs`
