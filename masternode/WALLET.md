# Masternode Wallet (Optional Advanced Feature)

⚠️ **IMPORTANT**: This is an OPTIONAL advanced feature. Most masternode operators do NOT need this.

## Simple Setup (Recommended)

For most masternode operators, you only need:
1. A **hot wallet** (wallet-gui) on your secure computer to hold funds
2. A simple **private key** in `masternode.conf` for signing masternode messages
3. The masternode references your hot wallet address for rewards

**This is the recommended setup** - your masternode never holds your funds or rewards.

### Simple Private Key Setup

The `masternode.conf` file contains just what you need:

```
# Format: alias IP:port masternodeprivkey collateral_txid collateral_output_index
mn1 192.168.1.100:24000 93HaYBVUCYjEMeeH1Y4sBGLALQZE1Yc1K64xiqgX37tGBDQL8Xg 2bcd3c84c84f87eaa86e4e56834c92927a07f9e18718810b92e0d0324456a67c 0
```

This simple private key is only used for:
- Signing masternode broadcast messages
- Authenticating with the network
- Proving ownership of the collateral

**It does NOT hold funds or rewards** - those go to your hot wallet address.

## Full Wallet (Optional, Advanced Users Only)

The full HD wallet feature documented below is only needed if you want to:
- Manage multiple addresses on the masternode itself
- Store and spend funds directly from the masternode
- Use advanced HD wallet features

**⚠️ Security Warning**: Storing funds on a public-facing masternode server is NOT recommended. Use the simple setup above instead.

---

## Full Wallet Features

If you decide to use the optional full wallet, it provides:

- **BIP-39 Mnemonic Support**: Generate and restore wallets using standard 12-word mnemonic phrases
- **HD Wallet**: Hierarchical Deterministic wallet with address derivation
- **Secure Storage**: Wallet data stored in `time-wallet.dat` with future encryption support
- **Network Support**: Separate wallets for mainnet and testnet
- **Transaction Signing**: Built-in transaction creation and signing

## File Structure

The wallet stores data in the following locations:

### Linux/macOS
- Mainnet: `~/.local/share/time-coin/mainnet/masternode/time-wallet.dat`
- Testnet: `~/.local/share/time-coin/testnet/masternode/time-wallet.dat`

### Windows
- Mainnet: `%APPDATA%\time-coin\mainnet\masternode\time-wallet.dat`
- Testnet: `%APPDATA%\time-coin\testnet\masternode\time-wallet.dat`

## Usage Examples

### Creating a New Wallet

```rust
use time_masternode::WalletManager;
use wallet::NetworkType;

// Generate a new mnemonic phrase
let mnemonic = WalletManager::generate_mnemonic()?;
println!("Save this mnemonic phrase: {}", mnemonic);

// Create wallet from mnemonic
let manager = WalletManager::create_from_mnemonic(
    NetworkType::Testnet,
    &mnemonic,
)?;

println!("Wallet created with xpub: {}", manager.get_xpub());
println!("Primary address: {}", manager.primary_address()?);
```

### Loading an Existing Wallet

```rust
use time_masternode::WalletManager;
use wallet::NetworkType;

// Load existing wallet
let manager = WalletManager::load(NetworkType::Testnet)?;

println!("Wallet loaded successfully");
println!("Primary address: {}", manager.primary_address()?);
println!("Balance: {} satoshis", manager.balance());
```

### Generating New Addresses

```rust
// Generate a new receiving address
let (address, index) = manager.generate_new_address_with_index()?;
println!("New address #{}: {}", index, address);

// Or simply get the next address
let address = manager.get_next_address()?;
println!("Next address: {}", address);
```

### Creating and Signing Transactions

```rust
// Create a transaction
let mut tx = manager.create_transaction(
    "tTIME1abcd...",  // recipient address
    100_000_000,       // 1 TIME (in satoshis)
    10_000,           // 0.0001 TIME fee
)?;

// Transaction is automatically signed by create_transaction
println!("Transaction created: {}", hex::encode(tx.txid));
```

### Restoring a Wallet from Mnemonic

```rust
use time_masternode::WalletManager;
use wallet::NetworkType;

let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

// Validate mnemonic first
WalletManager::validate_mnemonic(mnemonic)?;

// Replace existing wallet (creates backup)
let manager = WalletManager::replace_from_mnemonic(
    NetworkType::Testnet,
    mnemonic,
)?;

println!("Wallet restored successfully");
```

## Security Considerations

1. **Mnemonic Phrase**: The 12-word mnemonic phrase is the master key to your wallet. Store it securely offline.

2. **Backup**: Always backup your mnemonic phrase before deleting or replacing a wallet. The wallet file alone cannot be restored without it.

3. **File Permissions**: The `time-wallet.dat` file contains sensitive key material. On Unix systems, it's automatically set to read/write for owner only (0600).

4. **Encryption**: Future versions will support password-based encryption of the wallet file. Currently, the mnemonic is base64-encoded but not encrypted.

## Wallet Data Structure

The `time-wallet.dat` file stores:

- **Extended Public Key (xpub)**: For address derivation
- **Encrypted Mnemonic**: Base64-encoded (encryption planned)
- **Master Private Key**: For transaction signing
- **Network Type**: Mainnet or Testnet
- **Timestamps**: Creation and modification times

## Integration with Masternode (Optional)

The full wallet can optionally be integrated with the masternode if you choose to use it:
1. Loads the wallet on startup (if configured)
2. Uses the xpub to monitor all derived addresses
3. Signs transactions for masternode operations
4. Manages collateral and rewards

**Note**: By default, masternodes use the simple private key approach and don't load this wallet.

## API Reference

### WalletManager

```rust
// Create new wallet
pub fn create_from_mnemonic(network: NetworkType, mnemonic: &str) -> Result<Self, WalletDatError>

// Load existing wallet
pub fn load(network: NetworkType) -> Result<Self, WalletDatError>

// Generate mnemonic
pub fn generate_mnemonic() -> Result<String, WalletDatError>

// Validate mnemonic
pub fn validate_mnemonic(phrase: &str) -> Result<(), WalletDatError>

// Get xpub
pub fn get_xpub(&self) -> &str

// Derive address at index
pub fn derive_address(&self, index: u32) -> Result<String, WalletDatError>

// Get next address
pub fn get_next_address(&mut self) -> Result<String, WalletDatError>

// Create transaction
pub fn create_transaction(&mut self, to_address: &str, amount: u64, fee: u64) -> Result<Transaction, WalletDatError>

// Get balance
pub fn balance(&self) -> u64

// Add UTXO
pub fn add_utxo(&mut self, utxo: UTXO)
```

## Examples Directory

See the `examples/` directory for complete working examples:

- `wallet_create.rs` - Creating a new wallet
- `wallet_restore.rs` - Restoring from mnemonic
- `wallet_transaction.rs` - Creating transactions

## Troubleshooting

### Wallet Not Found
If you get a "Wallet file not found" error, create a new wallet using `create_from_mnemonic`.

### Address Not Recognized
Make sure you're using the correct network type (Mainnet vs Testnet). Testnet addresses start with "tTIME".

### Permission Denied
On Unix systems, ensure your user has read/write access to the TIME Coin data directory.
