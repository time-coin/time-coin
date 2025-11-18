//! time-wallet.dat File Format
//!
//! Similar to Bitcoin's wallet.dat, this file stores all keys and wallet metadata.
//! Currently uses unencrypted bincode serialization, with structure ready for future encryption.

use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
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

/// time-wallet.dat file format
/// This structure will be encrypted in future versions
/// Stores ONLY cryptographic material - no addresses or metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct WalletDat {
    /// Format version for future compatibility
    pub version: u32,
    /// Network type (mainnet/testnet)
    pub network: NetworkType,
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
    /// Encrypted mnemonic phrase for HD wallet
    /// In future, this will be properly encrypted. For now it's base64 encoded.
    pub encrypted_mnemonic: String,
    /// Extended Public Key (xpub) for deterministic address derivation
    /// Used by masternode to discover all wallet addresses
    pub xpub: String,
    /// Master private key (encrypted, for signing transactions)
    /// Derived from mnemonic, stored for quick access
    pub master_key: [u8; 32],
}

impl WalletDat {
    /// Current time-wallet.dat format version
    pub const VERSION: u32 = 2;

    /// Create a new wallet from mnemonic
    pub fn from_mnemonic(mnemonic: &str, network: NetworkType) -> Result<Self, WalletDatError> {
        // Generate xpub from mnemonic
        use wallet::mnemonic::mnemonic_to_xpub;
        let xpub = mnemonic_to_xpub(mnemonic, "", 0)
            .map_err(|e| WalletDatError::WalletError(wallet::WalletError::MnemonicError(e)))?;

        // Get master key (first derived key)
        use wallet::mnemonic::mnemonic_to_keypair_hd;
        let keypair = mnemonic_to_keypair_hd(mnemonic, "", 0)
            .map_err(|e| WalletDatError::WalletError(wallet::WalletError::MnemonicError(e)))?;
        let master_key = keypair.secret_key_bytes();

        // Store encrypted mnemonic (TODO: proper encryption)
        let encrypted_mnemonic = general_purpose::STANDARD.encode(mnemonic.as_bytes());

        let now = chrono::Utc::now().timestamp();
        Ok(Self {
            version: Self::VERSION,
            network,
            created_at: now,
            modified_at: now,
            encryption_salt: None,
            is_encrypted: false,
            encrypted_mnemonic,
            xpub,
            master_key,
        })
    }

    /// Derive a keypair at the given index
    pub fn derive_keypair(&self, index: u32) -> Result<Keypair, WalletDatError> {
        // Decrypt mnemonic (TODO: proper decryption)
        let mnemonic_bytes = general_purpose::STANDARD
            .decode(&self.encrypted_mnemonic)
            .map_err(|e| WalletDatError::SerializationError(e.to_string()))?;
        let mnemonic = String::from_utf8(mnemonic_bytes)
            .map_err(|e| WalletDatError::SerializationError(e.to_string()))?;

        use wallet::mnemonic::mnemonic_to_keypair_hd;
        let keypair = mnemonic_to_keypair_hd(&mnemonic, "", index)
            .map_err(|e| WalletDatError::WalletError(wallet::WalletError::MnemonicError(e)))?;
        Ok(keypair)
    }

    /// Derive an address at the given index
    pub fn derive_address(&self, index: u32) -> Result<String, WalletDatError> {
        let keypair = self.derive_keypair(index)?;
        let public_key = keypair.public_key_bytes();
        let address = wallet::Address::from_public_key(&public_key, self.network)
            .map_err(|_| WalletDatError::KeyGenerationError)?
            .to_string();
        Ok(address)
    }

    /// Get the xpub for this wallet
    pub fn get_xpub(&self) -> &str {
        &self.xpub
    }

    /// Get the mnemonic (decrypted)
    pub fn get_mnemonic(&self) -> Result<String, WalletDatError> {
        // Decrypt mnemonic (TODO: proper decryption)
        let mnemonic_bytes = general_purpose::STANDARD
            .decode(&self.encrypted_mnemonic)
            .map_err(|e| WalletDatError::SerializationError(e.to_string()))?;
        let mnemonic = String::from_utf8(mnemonic_bytes)
            .map_err(|e| WalletDatError::SerializationError(e.to_string()))?;
        Ok(mnemonic)
    }

    /// Save wallet to file
    pub fn save(&self) -> Result<(), WalletDatError> {
        let path = Self::default_path(self.network);

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Serialize to bincode (unencrypted for now)
        let data = bincode::serialize(self)
            .map_err(|e| WalletDatError::SerializationError(e.to_string()))?;

        // Write to file with proper permissions
        fs::write(&path, data)?;

        // On Unix, set restrictive permissions (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path)?.permissions();
            perms.set_mode(0o600); // rw-------
            fs::set_permissions(&path, perms)?;
        }

        Ok(())
    }

    /// Load wallet from file
    pub fn load(network: NetworkType) -> Result<Self, WalletDatError> {
        let path = Self::default_path(network);

        if !path.exists() {
            return Err(WalletDatError::WalletNotFound);
        }

        let data = fs::read(&path)?;

        // Try to deserialize the wallet
        match bincode::deserialize::<Self>(&data) {
            Ok(wallet) => Ok(wallet),
            Err(e) => Err(WalletDatError::SerializationError(format!(
                "Failed to deserialize wallet file: {}",
                e
            ))),
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
    fn test_wallet_from_mnemonic() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = WalletDat::from_mnemonic(mnemonic, NetworkType::Testnet).unwrap();
        assert_eq!(wallet.version, WalletDat::VERSION);
        assert_eq!(wallet.network, NetworkType::Testnet);
        assert!(!wallet.is_encrypted);
        assert!(!wallet.xpub.is_empty());
    }

    #[test]
    fn test_derive_address() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = WalletDat::from_mnemonic(mnemonic, NetworkType::Testnet).unwrap();

        let addr0 = wallet.derive_address(0).unwrap();
        let addr1 = wallet.derive_address(1).unwrap();

        // Addresses should be different
        assert_ne!(addr0, addr1);

        // Should be deterministic - derive again and get same result
        let addr0_again = wallet.derive_address(0).unwrap();
        assert_eq!(addr0, addr0_again);
    }

    #[test]
    fn test_save_and_load() {
        use std::env;
        
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = WalletDat::from_mnemonic(mnemonic, NetworkType::Testnet).unwrap();
        let original_xpub = wallet.xpub.clone();

        // Get the default path but create a test-specific path in temp directory
        let test_dir = env::temp_dir().join("time-coin-wallet-test");
        std::fs::create_dir_all(&test_dir).unwrap();
        let test_wallet_path = test_dir.join("time-wallet.dat");
        
        // Manually save to test path
        let data = bincode::serialize(&wallet).unwrap();
        std::fs::write(&test_wallet_path, data).unwrap();
        
        // Manually load from test path
        let data = std::fs::read(&test_wallet_path).unwrap();
        let loaded: WalletDat = bincode::deserialize(&data).unwrap();

        assert_eq!(loaded.version, wallet.version);
        assert_eq!(loaded.network, wallet.network);
        assert_eq!(loaded.xpub, original_xpub);

        // Cleanup test file only (NOT production wallet)
        let _ = std::fs::remove_file(&test_wallet_path);
        let _ = std::fs::remove_dir(&test_dir);
    }
}
