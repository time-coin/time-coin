//! Wallet Manager
//!
//! Manages time-wallet.dat file and provides high-level wallet operations

use crate::wallet_dat::{KeyEntry, WalletDat, WalletDatError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use wallet::{Keypair, NetworkType, Transaction, Wallet, UTXO};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddressMetadata {
    pub address: String,
    pub label: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionRecord {
    pub tx_hash: String,
    pub from_address: Option<String>,
    pub to_address: String,
    pub amount: u64,
    pub timestamp: i64,
    pub status: String,
}

#[derive(Debug)]
pub struct WalletManager {
    wallet_dat: WalletDat,
    wallet_path: PathBuf,
    // Active wallet instance for the primary key
    active_wallet: Option<Wallet>,
    // Sled database for metadata (contacts, labels, transaction history)
    db: sled::Db,
}

impl WalletManager {
    /// Initialize the sled database for metadata storage
    fn init_database(network: NetworkType) -> Result<sled::Db, WalletDatError> {
        let data_dir = WalletDat::ensure_data_dir(network)?;
        let db_path = data_dir.parent().unwrap().join("wallet.db");

        let db = sled::open(&db_path)
            .map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?;

        Ok(db)
    }

    /// Create a new wallet
    pub fn create_new(network: NetworkType, label: String) -> Result<Self, WalletDatError> {
        let wallet_path = WalletDat::ensure_data_dir(network)?;
        let wallet_dat = WalletDat::new_with_key(network, label)?;

        // Save immediately
        wallet_dat.save(&wallet_path)?;

        // Create active wallet from primary key
        let active_wallet =
            Self::create_wallet_from_key(&wallet_dat, wallet_dat.get_primary_key().unwrap())?;

        // Initialize database
        let db = Self::init_database(network)?;

        Ok(Self {
            wallet_dat,
            wallet_path,
            active_wallet: Some(active_wallet),
            db,
        })
    }

    /// Create a wallet from a BIP-39 mnemonic phrase
    /// NOTE: The mnemonic is NOT stored in the wallet file for security
    pub fn create_from_mnemonic(
        network: NetworkType,
        mnemonic: &str,
        passphrase: &str,
        label: String,
    ) -> Result<Self, WalletDatError> {
        // Validate mnemonic first
        wallet::validate_mnemonic(mnemonic)
            .map_err(|e| WalletDatError::WalletError(wallet::WalletError::MnemonicError(e)))?;

        // Create wallet from mnemonic
        let wallet = Wallet::from_mnemonic(mnemonic, passphrase, network)?;

        // Create wallet_dat and add the keypair
        let wallet_path = WalletDat::ensure_data_dir(network)?;
        let mut wallet_dat = WalletDat::new(network);

        // DO NOT store the mnemonic - it should only be shown during creation
        // The user must write it down and keep it safe offline

        // Add the keypair from the mnemonic
        let keypair = Keypair::from_secret_key(&wallet.secret_key())?;
        wallet_dat.add_keypair(keypair, label, true)?;

        // Save immediately
        wallet_dat.save(&wallet_path)?;

        // Create active wallet
        let active_wallet = Some(wallet);

        // Initialize database
        let db = Self::init_database(network)?;

        Ok(Self {
            wallet_dat,
            wallet_path,
            active_wallet,
            db,
        })
    }

    /// Generate a new 12-word BIP-39 mnemonic phrase
    pub fn generate_mnemonic() -> Result<String, WalletDatError> {
        wallet::generate_mnemonic(12)
            .map_err(|e| WalletDatError::WalletError(wallet::WalletError::MnemonicError(e)))
    }

    /// Validate a BIP-39 mnemonic phrase
    pub fn validate_mnemonic(phrase: &str) -> Result<(), WalletDatError> {
        wallet::validate_mnemonic(phrase)
            .map_err(|e| WalletDatError::WalletError(wallet::WalletError::MnemonicError(e)))
    }

    /// Load existing wallet from default path
    pub fn load_default(network: NetworkType) -> Result<Self, WalletDatError> {
        let wallet_path = WalletDat::default_path(network);
        Self::load_from_path(wallet_path, network)
    }

    /// Load wallet from specific path
    pub fn load_from_path(
        wallet_path: PathBuf,
        network: NetworkType,
    ) -> Result<Self, WalletDatError> {
        let wallet_dat = WalletDat::load(&wallet_path)?;

        // Create active wallet from primary key
        let active_wallet = if let Some(key) = wallet_dat.get_primary_key() {
            Some(Self::create_wallet_from_key(&wallet_dat, key)?)
        } else {
            None
        };

        // Initialize database
        let db = Self::init_database(network)?;

        Ok(Self {
            wallet_dat,
            wallet_path,
            active_wallet,
            db,
        })
    }

    /// Helper to create a Wallet instance from a KeyEntry
    fn create_wallet_from_key(
        wallet_dat: &WalletDat,
        key: &KeyEntry,
    ) -> Result<Wallet, WalletDatError> {
        let wallet = Wallet::from_secret_key(&key.keypair_bytes, wallet_dat.network)?;
        Ok(wallet)
    }

    /// Get the active wallet (for the primary key)
    pub fn get_active_wallet(&self) -> Option<&Wallet> {
        self.active_wallet.as_ref()
    }

    /// Get mutable active wallet
    pub fn get_active_wallet_mut(&mut self) -> Option<&mut Wallet> {
        self.active_wallet.as_mut()
    }

    /// Get time-wallet.dat reference
    pub fn wallet_dat(&self) -> &WalletDat {
        &self.wallet_dat
    }

    /// Get all keys
    pub fn get_keys(&self) -> &[KeyEntry] {
        self.wallet_dat.get_keys()
    }

    /// Get primary key
    pub fn get_primary_key(&self) -> Option<&KeyEntry> {
        self.wallet_dat.get_primary_key()
    }

    /// Generate a new key
    pub fn generate_new_key(
        &mut self,
        label: String,
        set_as_default: bool,
    ) -> Result<String, WalletDatError> {
        let key = self.wallet_dat.generate_key(label, set_as_default)?;
        let address = key.address.clone();

        // If this is the new default, update active wallet
        if set_as_default {
            let key_entry = self.wallet_dat.get_primary_key().unwrap();
            self.active_wallet = Some(Self::create_wallet_from_key(&self.wallet_dat, key_entry)?);
        }

        self.save()?;
        Ok(address)
    }

    /// Set a key as default
    pub fn set_default_key(&mut self, address: &str) -> Result<(), WalletDatError> {
        self.wallet_dat.set_default_key(address)?;

        // Update active wallet
        if let Some(key) = self.wallet_dat.get_primary_key() {
            self.active_wallet = Some(Self::create_wallet_from_key(&self.wallet_dat, key)?);
        }

        self.save()?;
        Ok(())
    }

    /// Get current balance from active wallet
    pub fn get_balance(&self) -> u64 {
        self.active_wallet
            .as_ref()
            .map(|w| w.balance())
            .unwrap_or(0)
    }

    /// Get primary address
    pub fn get_primary_address(&self) -> Option<String> {
        self.get_primary_key().map(|k| k.address.clone())
    }

    /// Add UTXO to active wallet
    pub fn add_utxo(&mut self, utxo: UTXO) {
        if let Some(wallet) = self.active_wallet.as_mut() {
            wallet.add_utxo(utxo);
        }
    }

    /// Get all UTXOs from active wallet
    pub fn get_utxos(&self) -> Vec<UTXO> {
        self.active_wallet
            .as_ref()
            .map(|w| w.utxos().to_vec())
            .unwrap_or_default()
    }

    /// Create a transaction
    pub fn create_transaction(
        &mut self,
        to_address: &str,
        amount: u64,
        fee: u64,
    ) -> Result<Transaction, String> {
        if let Some(wallet) = self.active_wallet.as_mut() {
            wallet
                .create_transaction(to_address, amount, fee)
                .map_err(|e| e.to_string())
        } else {
            Err("No active wallet".to_string())
        }
    }

    /// Save time-wallet.dat to disk
    pub fn save(&self) -> Result<(), WalletDatError> {
        self.wallet_dat.save(&self.wallet_path)
    }

    /// Get network type
    pub fn network(&self) -> NetworkType {
        self.wallet_dat.network
    }

    /// Check if wallet exists at default path
    pub fn exists(network: NetworkType) -> bool {
        WalletDat::default_path(network).exists()
    }

    /// Get wallet file path
    pub fn wallet_path(&self) -> &PathBuf {
        &self.wallet_path
    }

    /// Export private key for a specific address
    pub fn export_private_key(&self, address: &str) -> Option<String> {
        self.wallet_dat
            .get_keys()
            .iter()
            .find(|k| k.address == address)
            .map(|k| hex::encode(k.keypair_bytes))
    }

    /// Update contact information for an address
    pub fn update_contact_info(
        &mut self,
        address: &str,
        name: Option<String>,
        email: Option<String>,
        phone: Option<String>,
    ) -> Result<(), WalletDatError> {
        self.wallet_dat
            .update_contact_info(address, name, email, phone)?;
        self.save()
    }

    /// Get contact information for an address
    pub fn get_contact_info(
        &self,
        address: &str,
    ) -> Option<(Option<String>, Option<String>, Option<String>)> {
        self.wallet_dat
            .get_keys()
            .iter()
            .find(|k| k.address == address)
            .map(|k| (k.name.clone(), k.email.clone(), k.phone.clone()))
    }

    /// Import private key
    /// Get QR code for an address as SVG
    pub fn get_address_qr_code_svg(&self, address: &str) -> Result<String, String> {
        self.wallet_dat
            .get_keys()
            .iter()
            .find(|k| k.address == address)
            .ok_or_else(|| "Address not found".to_string())
            .and_then(|_| {
                let wallet = Wallet::from_secret_key(
                    &self
                        .wallet_dat
                        .get_keys()
                        .iter()
                        .find(|k| k.address == address)
                        .unwrap()
                        .keypair_bytes,
                    self.wallet_dat.network,
                )
                .map_err(|e| e.to_string())?;
                wallet.address_qr_code_svg().map_err(|e| e.to_string())
            })
    }

    pub fn import_private_key(
        &mut self,
        hex_key: &str,
        label: String,
    ) -> Result<String, WalletDatError> {
        let keypair = Keypair::from_hex(hex_key)?;
        let key = self.wallet_dat.add_keypair(keypair, label, false)?;
        let address = key.address.clone();
        self.save()?;
        Ok(address)
    }

    /// Remove metadata for an address (address itself is never deleted)
    pub fn remove_address_metadata(&mut self, address: &str) -> Result<(), WalletDatError> {
        let key = format!("addr_meta:{}", address);

        // Get existing metadata
        if let Some(data) = self
            .db
            .get(key.as_bytes())
            .map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?
        {
            let mut metadata: AddressMetadata = bincode::deserialize(&data)
                .map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?;

            // Clear contact info but keep address and default status
            metadata.name = None;
            metadata.email = None;
            metadata.phone = None;
            metadata.label = None;

            let encoded = bincode::serialize(&metadata)
                .map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?;

            self.db
                .insert(key.as_bytes(), encoded)
                .map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?;
        }

        self.db
            .flush()
            .map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Save address metadata to database
    pub fn save_address_metadata(
        &mut self,
        address: &str,
        label: &str,
        name: Option<&str>,
        email: Option<&str>,
        phone: Option<&str>,
        is_default: bool,
    ) -> Result<(), WalletDatError> {
        let metadata = AddressMetadata {
            address: address.to_string(),
            label: Some(label.to_string()),
            name: name.map(|s| s.to_string()),
            email: email.map(|s| s.to_string()),
            phone: phone.map(|s| s.to_string()),
            is_default,
        };

        let key = format!("addr_meta:{}", address);
        let encoded = bincode::serialize(&metadata)
            .map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?;

        self.db
            .insert(key.as_bytes(), encoded)
            .map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?;

        self.db
            .flush()
            .map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Get address metadata from database
    pub fn get_address_metadata(
        &self,
        address: &str,
    ) -> Result<Option<AddressMetadata>, WalletDatError> {
        let key = format!("addr_meta:{}", address);

        if let Some(data) = self
            .db
            .get(key.as_bytes())
            .map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?
        {
            let metadata: AddressMetadata = bincode::deserialize(&data)
                .map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?;

            Ok(Some(metadata))
        } else {
            Ok(None)
        }
    }

    /// Set an address as the default
    pub fn set_default_address(&mut self, address: &str) -> Result<(), WalletDatError> {
        // First, unset all defaults
        for item in self.db.scan_prefix(b"addr_meta:") {
            let (key, value) =
                item.map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?;

            let mut metadata: AddressMetadata = bincode::deserialize(&value)
                .map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?;

            if metadata.is_default {
                metadata.is_default = false;
                let encoded = bincode::serialize(&metadata)
                    .map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?;
                self.db
                    .insert(&key, encoded)
                    .map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?;
            }
        }

        // Then set the new default
        let key = format!("addr_meta:{}", address);
        if let Some(data) = self
            .db
            .get(key.as_bytes())
            .map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?
        {
            let mut metadata: AddressMetadata = bincode::deserialize(&data)
                .map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?;

            metadata.is_default = true;

            let encoded = bincode::serialize(&metadata)
                .map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?;

            self.db
                .insert(key.as_bytes(), encoded)
                .map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?;
        }

        self.db
            .flush()
            .map_err(|e| WalletDatError::IoError(std::io::Error::other(e.to_string())))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_manager_creation() {
        let temp_dir = std::env::temp_dir().join("test-wallet-manager");
        let _ = std::fs::create_dir_all(&temp_dir);

        // Override default path for testing
        let _test_path = temp_dir.join("test_wallet.dat");

        let manager = WalletManager::create_new(NetworkType::Testnet, "Test".to_string()).unwrap();
        assert!(manager.get_active_wallet().is_some());
        assert_eq!(manager.get_keys().len(), 1);
        assert!(manager.get_primary_address().is_some());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_balance_management() {
        let temp_dir = std::env::temp_dir().join("test-wallet-balance");
        let _ = std::fs::create_dir_all(&temp_dir);

        let mut manager =
            WalletManager::create_new(NetworkType::Testnet, "Test".to_string()).unwrap();
        assert_eq!(manager.get_balance(), 0);

        let address = manager.get_primary_address().unwrap();
        let utxo = UTXO {
            tx_hash: [1u8; 32],
            output_index: 0,
            amount: 1000,
            address: address.clone(),
        };

        manager.add_utxo(utxo);
        assert_eq!(manager.get_balance(), 1000);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
