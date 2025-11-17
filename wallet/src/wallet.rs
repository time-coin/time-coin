use crate::address::{Address, AddressError, NetworkType};
use crate::keypair::{Keypair, KeypairError};
use crate::mnemonic::{mnemonic_to_keypair, mnemonic_to_keypair_hd, MnemonicError};
use crate::transaction::{Transaction, TransactionError, TxInput, TxOutput};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WalletError {
    #[error("Keypair error: {0}")]
    KeypairError(#[from] KeypairError),

    #[error("Address error: {0}")]
    AddressError(#[from] AddressError),

    #[error("Transaction error: {0}")]
    TransactionError(#[from] TransactionError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error")]
    SerializationError,

    #[error("Insufficient funds: have {have}, need {need}")]
    InsufficientFunds { have: u64, need: u64 },

    #[error("Invalid address")]
    InvalidAddress,

    #[error("Mnemonic error: {0}")]
    MnemonicError(#[from] MnemonicError),
}

/// UTXO (Unspent Transaction Output)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UTXO {
    pub tx_hash: [u8; 32],
    pub output_index: u32,
    pub amount: u64,
    pub address: String,
}

/// Address metadata for contact information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AddressMetadata {
    pub label: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub is_default: bool,
}

/// Wallet for managing keys, addresses, and transactions
#[derive(Serialize, Deserialize)]
pub struct Wallet {
    keypair: Keypair,
    address: Address,
    network: NetworkType,
    #[serde(default)]
    balance: u64,
    #[serde(default)]
    nonce: u64,
    #[serde(default)]
    utxos: Vec<UTXO>,
    #[serde(default)]
    additional_addresses: Vec<(Keypair, Address)>,
    #[serde(default)]
    mnemonic_phrase: Option<String>,
    #[serde(default)]
    next_address_index: u32,
    #[serde(default)]
    xpub: Option<String>,
}

impl Wallet {
    /// Create a new wallet with a random keypair
    pub fn new(network: NetworkType) -> Result<Self, WalletError> {
        let keypair = Keypair::generate()?;
        let public_key = keypair.public_key_bytes();
        let address = Address::from_public_key(&public_key, network)?;

        Ok(Self {
            keypair,
            address,
            network,
            balance: 0,
            nonce: 0,
            utxos: Vec::new(),
            additional_addresses: Vec::new(),
            mnemonic_phrase: None,
            next_address_index: 0,
            xpub: None,
        })
    }

    /// Create a wallet from an existing secret key
    pub fn from_secret_key(secret_key: &[u8], network: NetworkType) -> Result<Self, WalletError> {
        let keypair = Keypair::from_secret_key(secret_key)?;
        let public_key = keypair.public_key_bytes();
        let address = Address::from_public_key(&public_key, network)?;

        Ok(Self {
            keypair,
            address,
            network,
            balance: 0,
            nonce: 0,
            utxos: Vec::new(),
            additional_addresses: Vec::new(),
            mnemonic_phrase: None,
            next_address_index: 0,
            xpub: None,
        })
    }

    /// Create a wallet from hex-encoded secret key
    pub fn from_private_key_hex(hex_key: &str, network: NetworkType) -> Result<Self, WalletError> {
        let keypair = Keypair::from_hex(hex_key)?;
        let public_key = keypair.public_key_bytes();
        let address = Address::from_public_key(&public_key, network)?;

        Ok(Self {
            keypair,
            address,
            network,
            balance: 0,
            nonce: 0,
            utxos: Vec::new(),
            additional_addresses: Vec::new(),
            mnemonic_phrase: None,
            next_address_index: 0,
            xpub: None,
        })
    }

    /// Create a wallet from a BIP-39 mnemonic phrase
    ///
    /// # Arguments
    /// * `mnemonic` - The mnemonic phrase (space-separated words)
    /// * `passphrase` - Optional passphrase for additional security (use "" for none)
    /// * `network` - Network type (Mainnet or Testnet)
    ///
    /// # Returns
    /// * `Result<Self, WalletError>` - The created wallet
    ///
    /// # Example
    /// ```
    /// use wallet::{Wallet, NetworkType, generate_mnemonic};
    ///
    /// let mnemonic = generate_mnemonic(12).unwrap();
    /// let wallet = Wallet::from_mnemonic(&mnemonic, "", NetworkType::Mainnet).unwrap();
    /// ```
    pub fn from_mnemonic(
        mnemonic: &str,
        passphrase: &str,
        network: NetworkType,
    ) -> Result<Self, WalletError> {
        let keypair = mnemonic_to_keypair(mnemonic, passphrase)?;
        let public_key = keypair.public_key_bytes();
        let address = Address::from_public_key(&public_key, network)?;

        Ok(Self {
            keypair,
            address,
            network,
            balance: 0,
            nonce: 0,
            utxos: Vec::new(),
            additional_addresses: Vec::new(),
            mnemonic_phrase: Some(mnemonic.to_string()),
            next_address_index: 0,
            xpub: None,
        })
    }

    /// Get the wallet address
    pub fn address(&self) -> &Address {
        &self.address
    }

    /// Get the wallet address as a string
    pub fn address_string(&self) -> String {
        self.address.to_string()
    }

    /// Get the public key
    pub fn public_key(&self) -> [u8; 32] {
        self.keypair.public_key_bytes()
    }

    /// Get the public key as hex string
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public_key())
    }

    /// Get the secret key (be careful with this!)
    pub fn secret_key(&self) -> [u8; 32] {
        self.keypair.secret_key_bytes()
    }

    /// Export private key as hex string (⚠️ Keep secret!)
    pub fn export_private_key(&self) -> String {
        self.keypair.secret_key_hex()
    }

    /// Get current balance
    pub fn balance(&self) -> u64 {
        self.balance
    }

    /// Set balance (called when syncing with blockchain)
    pub fn set_balance(&mut self, balance: u64) {
        self.balance = balance;
    }

    /// Get current nonce
    pub fn nonce(&self) -> u64 {
        self.nonce
    }

    /// Set nonce (called when syncing with blockchain)
    pub fn set_nonce(&mut self, nonce: u64) {
        self.nonce = nonce;
    }

    /// Increment nonce
    pub fn increment_nonce(&mut self) {
        self.nonce += 1;
    }

    /// Get network type
    pub fn network(&self) -> NetworkType {
        self.network
    }

    /// Add UTXO
    pub fn add_utxo(&mut self, utxo: UTXO) {
        let amount = utxo.amount;
        self.utxos.push(utxo);
        self.balance += amount;
    }

    /// Remove UTXO
    pub fn remove_utxo(&mut self, tx_hash: &[u8; 32], output_index: u32) {
        if let Some(pos) = self
            .utxos
            .iter()
            .position(|u| &u.tx_hash == tx_hash && u.output_index == output_index)
        {
            let utxo = self.utxos.remove(pos);
            self.balance = self.balance.saturating_sub(utxo.amount);
        }
    }

    /// Get all UTXOs
    pub fn utxos(&self) -> &[UTXO] {
        &self.utxos
    }

    /// Generate a new receiving address (HD wallet derivation if mnemonic exists)
    pub fn generate_new_address(&mut self) -> Result<String, WalletError> {
        let (keypair, address) = if let Some(ref mnemonic) = self.mnemonic_phrase {
            // Use HD wallet derivation
            let account_index = self.next_address_index;
            self.next_address_index += 1;

            let keypair = mnemonic_to_keypair_hd(mnemonic, "", account_index)?;
            let public_key = keypair.public_key_bytes();
            let address = Address::from_public_key(&public_key, self.network)?;
            (keypair, address)
        } else {
            // Fall back to random generation for non-mnemonic wallets
            let keypair = Keypair::generate()?;
            let public_key = keypair.public_key_bytes();
            let address = Address::from_public_key(&public_key, self.network)?;
            (keypair, address)
        };

        let address_string = address.to_string();
        self.additional_addresses.push((keypair, address));
        Ok(address_string)
    }

    /// Get all addresses (primary + additional)
    pub fn get_all_addresses(&self) -> Vec<String> {
        let mut addresses = vec![self.address.to_string()];
        addresses.extend(
            self.additional_addresses
                .iter()
                .map(|(_, addr)| addr.to_string()),
        );
        addresses
    }

    /// Create a transaction with fee support
    pub fn create_transaction(
        &mut self,
        to_address: &str,
        amount: u64,
        fee: u64,
    ) -> Result<Transaction, WalletError> {
        if amount == 0 {
            return Err(WalletError::TransactionError(
                TransactionError::InvalidAmount,
            ));
        }

        let total_needed = amount + fee;
        if total_needed > self.balance {
            return Err(WalletError::InsufficientFunds {
                have: self.balance,
                need: total_needed,
            });
        }

        // Validate recipient address
        let recipient = Address::from_string(to_address)?;

        // Create transaction
        let mut tx = Transaction::new();
        tx.set_nonce(self.nonce);

        // Select UTXOs (simple: use first UTXOs that cover amount + fee)
        let mut input_amount = 0u64;
        let mut selected_utxos = Vec::new();

        for utxo in &self.utxos {
            selected_utxos.push(utxo.clone());
            input_amount += utxo.amount;

            if input_amount >= total_needed {
                break;
            }
        }

        if input_amount < total_needed {
            return Err(WalletError::InsufficientFunds {
                have: input_amount,
                need: total_needed,
            });
        }

        // Add inputs
        for utxo in &selected_utxos {
            let input = TxInput::new(utxo.tx_hash, utxo.output_index);
            tx.add_input(input);
        }

        // Add output to recipient (amount only, fee goes to miners)
        let output = TxOutput::new(amount, recipient);
        tx.add_output(output)?;

        // Add change output if necessary
        let change = input_amount - total_needed;
        if change > 0 {
            let change_output = TxOutput::new(change, self.address.clone());
            tx.add_output(change_output)?;
        }

        // Sign the transaction
        tx.sign_all(&self.keypair)?;

        // Update wallet state (auto-increment nonce)
        self.increment_nonce();

        Ok(tx)
    }

    /// Sign an existing transaction
    pub fn sign_transaction(&self, tx: &mut Transaction) -> Result<(), WalletError> {
        tx.sign_all(&self.keypair)?;
        Ok(())
    }

    /// Save wallet to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), WalletError> {
        let serialized =
            serde_json::to_string_pretty(self).map_err(|_| WalletError::SerializationError)?;
        fs::write(path, serialized)?;
        Ok(())
    }

    /// Load wallet from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, WalletError> {
        let data = fs::read_to_string(path)?;
        let wallet: Self =
            serde_json::from_str(&data).map_err(|_| WalletError::SerializationError)?;
        Ok(wallet)
    }

    /// Check if testnet
    pub fn is_testnet(&self) -> bool {
        self.network == NetworkType::Testnet
    }

    /// Get the keypair (for advanced use)
    /// Generate QR code for the wallet address (as ASCII art for terminal display)
    pub fn address_qr_code(&self) -> Result<String, WalletError> {
        use qrcode::QrCode;
        let code = QrCode::new(self.address_string().as_bytes())
            .map_err(|_| WalletError::SerializationError)?;
        let string = code
            .render::<char>()
            .quiet_zone(true)
            .module_dimensions(2, 1)
            .dark_color('█')
            .light_color(' ')
            .build();
        Ok(string)
    }

    /// Generate QR code as SVG string (for GUI applications)
    pub fn address_qr_code_svg(&self) -> Result<String, WalletError> {
        use qrcode::render::svg;
        use qrcode::QrCode;
        let code = QrCode::new(self.address_string().as_bytes())
            .map_err(|_| WalletError::SerializationError)?;
        let svg = code
            .render()
            .min_dimensions(200, 200)
            .dark_color(svg::Color("#000000"))
            .light_color(svg::Color("#ffffff"))
            .build();
        Ok(svg)
    }

    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }

    /// Get the xpub (extended public key) if available
    pub fn xpub(&self) -> Option<&str> {
        self.xpub.as_deref()
    }

    /// Set the xpub (extended public key)
    pub fn set_xpub(&mut self, xpub: String) {
        self.xpub = Some(xpub);
    }
}

impl std::fmt::Debug for Wallet {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Wallet")
            .field("address", &self.address.to_string())
            .field("network", &self.network)
            .field("balance", &self.balance)
            .field("nonce", &self.nonce)
            .field("utxos", &self.utxos.len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_creation() {
        let wallet = Wallet::new(NetworkType::Mainnet).unwrap();
        assert!(!wallet.is_testnet());
        assert_eq!(wallet.balance(), 0);
        assert_eq!(wallet.nonce(), 0);
    }

    #[test]
    fn test_wallet_from_secret_key() {
        let wallet1 = Wallet::new(NetworkType::Mainnet).unwrap();
        let secret_key = wallet1.secret_key();

        let wallet2 = Wallet::from_secret_key(&secret_key, NetworkType::Mainnet).unwrap();

        assert_eq!(wallet1.address_string(), wallet2.address_string());
        assert_eq!(wallet1.public_key(), wallet2.public_key());
    }

    #[test]
    fn test_wallet_from_hex() {
        let wallet1 = Wallet::new(NetworkType::Mainnet).unwrap();
        let hex_key = wallet1.export_private_key();

        let wallet2 = Wallet::from_private_key_hex(&hex_key, NetworkType::Mainnet).unwrap();

        assert_eq!(wallet1.address_string(), wallet2.address_string());
    }

    #[test]
    fn test_balance_management() {
        let mut wallet = Wallet::new(NetworkType::Mainnet).unwrap();

        let utxo = UTXO {
            tx_hash: [1u8; 32],
            output_index: 0,
            amount: 1000,
            address: wallet.address_string(),
        };

        wallet.add_utxo(utxo);
        assert_eq!(wallet.balance(), 1000);

        wallet.remove_utxo(&[1u8; 32], 0);
        assert_eq!(wallet.balance(), 0);
    }

    #[test]
    fn test_nonce_increment() {
        let mut wallet = Wallet::new(NetworkType::Mainnet).unwrap();
        assert_eq!(wallet.nonce(), 0);

        wallet.increment_nonce();
        assert_eq!(wallet.nonce(), 1);

        wallet.increment_nonce();
        assert_eq!(wallet.nonce(), 2);
    }

    #[test]
    fn test_create_transaction_with_fee() {
        let mut sender = Wallet::new(NetworkType::Mainnet).unwrap();
        let recipient = Wallet::new(NetworkType::Mainnet).unwrap();

        // Add UTXO to sender
        let utxo = UTXO {
            tx_hash: [1u8; 32],
            output_index: 0,
            amount: 10000,
            address: sender.address_string(),
        };
        sender.add_utxo(utxo);

        // Create transaction with fee
        let tx = sender
            .create_transaction(&recipient.address_string(), 1000, 50)
            .unwrap();

        assert_eq!(tx.outputs.len(), 2); // recipient + change
        assert_eq!(tx.outputs[0].amount, 1000);
        assert_eq!(tx.outputs[1].amount, 8950); // 10000 - 1000 - 50
        assert_eq!(sender.nonce(), 1); // Auto-incremented
    }

    #[test]
    fn test_insufficient_funds() {
        let mut wallet = Wallet::new(NetworkType::Mainnet).unwrap();
        let recipient = Wallet::new(NetworkType::Mainnet).unwrap();

        let utxo = UTXO {
            tx_hash: [1u8; 32],
            output_index: 0,
            amount: 100,
            address: wallet.address_string(),
        };
        wallet.add_utxo(utxo);

        let result = wallet.create_transaction(&recipient.address_string(), 1000, 50);

        assert!(result.is_err());
        match result {
            Err(WalletError::InsufficientFunds { have, need }) => {
                assert_eq!(have, 100);
                assert_eq!(need, 1050);
            }
            _ => panic!("Expected InsufficientFunds error"),
        }
    }

    #[test]
    fn test_save_and_load() {
        let wallet1 = Wallet::new(NetworkType::Mainnet).unwrap();
        let temp_file = "/tmp/test_wallet_improved.json";

        wallet1.save_to_file(temp_file).unwrap();
        let wallet2 = Wallet::load_from_file(temp_file).unwrap();

        assert_eq!(wallet1.address_string(), wallet2.address_string());
        assert_eq!(wallet1.public_key(), wallet2.public_key());

        // Cleanup
        let _ = fs::remove_file(temp_file);
    }

    #[test]
    fn test_wallet_from_mnemonic() {
        use crate::mnemonic::generate_mnemonic;

        let mnemonic = generate_mnemonic(12).unwrap();
        let wallet = Wallet::from_mnemonic(&mnemonic, "", NetworkType::Mainnet).unwrap();

        assert!(!wallet.is_testnet());
        assert_eq!(wallet.balance(), 0);
        assert_eq!(wallet.nonce(), 0);
    }

    #[test]
    fn test_wallet_mnemonic_deterministic() {
        // Same mnemonic should produce same wallet
        let test_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        let wallet1 = Wallet::from_mnemonic(test_mnemonic, "", NetworkType::Mainnet).unwrap();
        let wallet2 = Wallet::from_mnemonic(test_mnemonic, "", NetworkType::Mainnet).unwrap();

        assert_eq!(wallet1.address_string(), wallet2.address_string());
        assert_eq!(wallet1.public_key(), wallet2.public_key());
        assert_eq!(wallet1.secret_key(), wallet2.secret_key());
    }

    #[test]
    fn test_wallet_mnemonic_with_passphrase() {
        use crate::mnemonic::generate_mnemonic;

        let mnemonic = generate_mnemonic(12).unwrap();

        // Different passphrases should produce different wallets
        let wallet1 = Wallet::from_mnemonic(&mnemonic, "", NetworkType::Mainnet).unwrap();
        let wallet2 = Wallet::from_mnemonic(&mnemonic, "password", NetworkType::Mainnet).unwrap();

        assert_ne!(wallet1.address_string(), wallet2.address_string());
        assert_ne!(wallet1.public_key(), wallet2.public_key());
    }

    #[test]
    fn test_wallet_from_invalid_mnemonic() {
        let result = Wallet::from_mnemonic(
            "invalid word word word word word word word word word word word",
            "",
            NetworkType::Mainnet,
        );

        assert!(result.is_err());
    }
}
