//! time-wallet.dat File Format
//!
//! Similar to Bitcoin's wallet.dat, this file stores all keys and wallet metadata.
//! Currently uses unencrypted bincode serialization, with structure ready for future encryption.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use wallet::{Keypair, NetworkType};

#[derive(Debug, Error)]
pub enum WalletDatError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Wallet file not found")]
    WalletNotFound,

    #[error("Invalid wallet format")]
    InvalidFormat,

    #[error("Key generation error")]
    KeyGenerationError,

    #[error("Keypair error: {0}")]
    KeypairError(#[from] wallet::KeypairError),

    #[error("Wallet error: {0}")]
    WalletError(#[from] wallet::WalletError),
}

/// A stored key entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEntry {
    /// The keypair (secret + public key)
    pub keypair_bytes: [u8; 32],
    /// Public key bytes for quick lookup
    pub public_key: [u8; 32],
    /// Address string
    pub address: String,
    /// Label/name for this key
    pub label: String,
    /// Creation timestamp
    pub created_at: i64,
    /// Whether this is the default key
    pub is_default: bool,
    /// Optional: Name for address book
    #[serde(default)]
    pub name: Option<String>,
    /// Optional: Email for address book
    #[serde(default)]
    pub email: Option<String>,
    /// Optional: Phone for address book
    #[serde(default)]
    pub phone: Option<String>,
}

/// time-wallet.dat file format
/// This structure will be encrypted in future versions
#[derive(Debug, Serialize, Deserialize)]
pub struct WalletDat {
    /// Format version for future compatibility
    pub version: u32,
    /// Network type (mainnet/testnet)
    pub network: NetworkType,
    /// All keys stored in the wallet
    pub keys: Vec<KeyEntry>,
    /// Wallet creation timestamp
    pub created_at: i64,
    /// Last modified timestamp
    pub modified_at: i64,
    /// Future: encryption salt (placeholder for now)
    #[serde(default)]
    pub encryption_salt: Option<Vec<u8>>,
    /// Future: encrypted flag (placeholder for now)
    #[serde(default)]
    pub is_encrypted: bool,
    /// Encrypted mnemonic phrase for HD wallet (only stored if wallet was created from mnemonic)
    /// In future, this will be properly encrypted. For now it's base64 encoded.
    #[serde(default)]
    pub encrypted_mnemonic: Option<String>,
}

impl WalletDat {
    /// Current time-wallet.dat format version
    pub const VERSION: u32 = 1;

    /// Create a new empty time-wallet.dat
    pub fn new(network: NetworkType) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            version: Self::VERSION,
            network,
            keys: Vec::new(),
            created_at: now,
            modified_at: now,
            encryption_salt: None,
            is_encrypted: false,
            encrypted_mnemonic: None,
        }
    }

    /// Create a new wallet with a generated key
    pub fn new_with_key(network: NetworkType, label: String) -> Result<Self, WalletDatError> {
        let mut wallet = Self::new(network);
        wallet.generate_key(label, true)?;
        Ok(wallet)
    }

    /// Generate a new key and add it to the wallet
    pub fn generate_key(
        &mut self,
        label: String,
        is_default: bool,
    ) -> Result<&KeyEntry, WalletDatError> {
        let keypair = Keypair::generate()?;
        self.add_keypair(keypair, label, is_default)
    }

    /// Add an existing keypair to the wallet
    pub fn add_keypair(
        &mut self,
        keypair: Keypair,
        label: String,
        is_default: bool,
    ) -> Result<&KeyEntry, WalletDatError> {
        let public_key = keypair.public_key_bytes();
        let keypair_bytes = keypair.secret_key_bytes();
        let address = wallet::Address::from_public_key(&public_key, self.network)
            .map_err(|_| WalletDatError::KeyGenerationError)?
            .to_string();

        // If this is set as default, unset all other defaults
        if is_default {
            for key in &mut self.keys {
                key.is_default = false;
            }
        }

        let entry = KeyEntry {
            keypair_bytes,
            public_key,
            address,
            label,
            created_at: chrono::Utc::now().timestamp(),
            is_default,
            name: None,
            email: None,
            phone: None,
        };

        self.keys.push(entry);
        self.modified_at = chrono::Utc::now().timestamp();

        Ok(self.keys.last().unwrap())
    }

    /// Get the default key
    pub fn get_default_key(&self) -> Option<&KeyEntry> {
        self.keys.iter().find(|k| k.is_default)
    }

    /// Get default key or first key
    pub fn get_primary_key(&self) -> Option<&KeyEntry> {
        self.get_default_key().or_else(|| self.keys.first())
    }

    /// Get all keys
    pub fn get_keys(&self) -> &[KeyEntry] {
        &self.keys
    }

    /// Set a key as default
    pub fn set_default_key(&mut self, address: &str) -> Result<(), WalletDatError> {
        let mut found = false;
        for key in &mut self.keys {
            if key.address == address {
                key.is_default = true;
                found = true;
            } else {
                key.is_default = false;
            }
        }

        if found {
            self.modified_at = chrono::Utc::now().timestamp();
            Ok(())
        } else {
            Err(WalletDatError::InvalidFormat)
        }
    }

    /// Save wallet to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), WalletDatError> {
        // Serialize to bincode (unencrypted for now)
        let data = bincode::serialize(self)
            .map_err(|e| WalletDatError::SerializationError(e.to_string()))?;

        // Write to file with proper permissions
        fs::write(path.as_ref(), data)?;

        // On Unix, set restrictive permissions (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path.as_ref())?.permissions();
            perms.set_mode(0o600); // rw-------
            fs::set_permissions(path.as_ref(), perms)?;
        }

        Ok(())
    }

    /// Load wallet from file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, WalletDatError> {
        if !path.as_ref().exists() {
            return Err(WalletDatError::WalletNotFound);
        }

        let data = fs::read(path.as_ref())?;
        
        // Try to deserialize the wallet
        match bincode::deserialize::<Self>(&data) {
            Ok(wallet) => Ok(wallet),
            Err(e) => {
                // If deserialization fails, it might be an old format
                // Try to migrate from old wallet.dat format
                eprintln!("Warning: Wallet file format mismatch. Attempting migration...");
                eprintln!("Error details: {}", e);
                
                // Backup the old file
                let backup_path = path.as_ref().with_extension(format!(
                    "dat.backup.{}",
                    chrono::Utc::now().format("%Y%m%d_%H%M%S")
                ));
                fs::copy(path.as_ref(), &backup_path)?;
                eprintln!("✓ Old wallet backed up to: {:?}", backup_path);
                
                // Return error with helpful message
                Err(WalletDatError::SerializationError(format!(
                    "Wallet file format is incompatible. Your old wallet has been backed up to {:?}. \
                    Please delete the wallet file and create a new one, or restore from your recovery phrase.",
                    backup_path
                )))
            }
        }
    }

    /// Get the default wallet path for the given network
    pub fn default_path(network: NetworkType) -> PathBuf {
        let data_dir = Self::get_data_dir();
        let network_dir = match network {
            NetworkType::Mainnet => "mainnet",
            NetworkType::Testnet => "testnet",
        };
        data_dir.join(network_dir).join("time-wallet.dat")
    }

    /// Get the TIME Coin data directory
    pub fn get_data_dir() -> PathBuf {
        if let Some(dir) = dirs::data_dir() {
            dir.join("time-coin")
        } else {
            // Fallback to current directory
            PathBuf::from(".")
        }
    }

    /// Create data directory if it doesn't exist
    pub fn ensure_data_dir(network: NetworkType) -> Result<PathBuf, WalletDatError> {
        let wallet_path = Self::default_path(network);
        if let Some(parent) = wallet_path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(wallet_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_dat_creation() {
        let wallet = WalletDat::new(NetworkType::Testnet);
        assert_eq!(wallet.version, WalletDat::VERSION);
        assert_eq!(wallet.network, NetworkType::Testnet);
        assert_eq!(wallet.keys.len(), 0);
        assert!(!wallet.is_encrypted);
    }

    #[test]
    fn test_wallet_dat_with_key() {
        let wallet = WalletDat::new_with_key(NetworkType::Testnet, "Default".to_string()).unwrap();
        assert_eq!(wallet.keys.len(), 1);
        assert!(wallet.get_default_key().is_some());
        assert!(wallet.get_primary_key().is_some());
    }

    #[test]
    fn test_add_multiple_keys() {
        let mut wallet = WalletDat::new(NetworkType::Testnet);
        wallet.generate_key("Key 1".to_string(), true).unwrap();
        wallet.generate_key("Key 2".to_string(), false).unwrap();
        wallet.generate_key("Key 3".to_string(), false).unwrap();

        assert_eq!(wallet.keys.len(), 3);
        let default_key = wallet.get_default_key().unwrap();
        assert_eq!(default_key.label, "Key 1");
    }

    #[test]
    fn test_set_default_key() {
        let mut wallet = WalletDat::new(NetworkType::Testnet);
        wallet.generate_key("Key 1".to_string(), true).unwrap();
        let key2 = wallet.generate_key("Key 2".to_string(), false).unwrap();
        let key2_address = key2.address.clone();

        wallet.set_default_key(&key2_address).unwrap();
        let default = wallet.get_default_key().unwrap();
        assert_eq!(default.label, "Key 2");
    }

    #[test]
    fn test_save_and_load() {
        let mut wallet = WalletDat::new(NetworkType::Testnet);
        wallet.generate_key("Test Key".to_string(), true).unwrap();

        let temp_path = "/tmp/test_wallet.dat";
        wallet.save(temp_path).unwrap();

        let loaded = WalletDat::load(temp_path).unwrap();
        assert_eq!(loaded.version, wallet.version);
        assert_eq!(loaded.network, wallet.network);
        assert_eq!(loaded.keys.len(), wallet.keys.len());
        assert_eq!(loaded.keys[0].label, "Test Key");

        // Cleanup
        let _ = fs::remove_file(temp_path);
    }
}
