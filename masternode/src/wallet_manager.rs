//! Wallet Manager for Masternode
//!
//! Manages time-wallet.dat file and provides high-level wallet operations
//!
//! The wallet uses deterministic address derivation:
//! - time-wallet.dat stores ONLY: xpub, encrypted mnemonic, master key
//! - Addresses are derived on-demand from xpub
//! - Metadata stored separately if needed
//!
//! NOTE: This is an OPTIONAL feature for advanced users. Most masternodes
//! only need a simple private key for signing (stored in masternode.conf).
//! Use this full wallet implementation only if you need HD wallet features
//! like multiple addresses or want to manage rewards on the masternode itself.

use crate::wallet_dat::{WalletDat, WalletDatError};
use serde::{Deserialize, Serialize};
use std::fs;
use wallet::{Keypair, NetworkType, Transaction, Wallet, UTXO};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddressMetadata {
    pub address: String,
    pub label: Option<String>,
    pub index: u32, // Address derivation index
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
    // Active wallet instance
    active_wallet: Wallet,
    // Next address index to derive
    next_address_index: u32,
}

impl WalletManager {
    /// Create a wallet from a BIP-39 mnemonic phrase
    /// Create a wallet from mnemonic (checks if wallet already exists first)
    pub fn create_from_mnemonic(
        network: NetworkType,
        mnemonic: &str,
    ) -> Result<Self, WalletDatError> {
        // Check if wallet already exists
        let wallet_path = WalletDat::default_path(network);
        if wallet_path.exists() {
            log::warn!(
                "Masternode wallet already exists at: {}. Loading existing wallet instead.",
                wallet_path.display()
            );
            return Self::load(network);
        }

        // Validate mnemonic first
        wallet::validate_mnemonic(mnemonic)
            .map_err(|e| WalletDatError::WalletError(wallet::WalletError::MnemonicError(e)))?;

        // Create wallet from mnemonic
        let wallet = Wallet::from_mnemonic(mnemonic, "", network)?;

        // Create wallet_dat from mnemonic (stores xpub, encrypted mnemonic, master key)
        let wallet_dat = WalletDat::from_mnemonic(mnemonic, network)?;

        log::info!(
            "Created masternode wallet with xpub: {}",
            wallet_dat.get_xpub()
        );

        // Save immediately
        wallet_dat.save()?;

        Ok(Self {
            wallet_dat,
            active_wallet: wallet,
            next_address_index: 0,
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

    /// Load existing wallet
    pub fn load(network: NetworkType) -> Result<Self, WalletDatError> {
        let wallet_dat = WalletDat::load(network)?;

        // Recreate wallet from mnemonic
        let mnemonic = wallet_dat.get_mnemonic()?;
        let wallet = Wallet::from_mnemonic(&mnemonic, "", network)?;

        Ok(Self {
            wallet_dat,
            active_wallet: wallet,
            next_address_index: 0,
        })
    }

    /// Update next_address_index based on existing addresses
    /// Should be called when syncing with the blockchain
    pub fn sync_address_index(&mut self, max_index: u32) {
        self.next_address_index = max_index + 1;
    }

    /// Check if wallet exists
    pub fn exists(network: NetworkType) -> bool {
        WalletDat::default_path(network).exists()
    }

    /// Replace existing wallet with a new one from mnemonic
    /// IMPORTANT: Creates backup before replacing. Old wallet saved as .dat.old
    pub fn replace_from_mnemonic(
        network: NetworkType,
        mnemonic: &str,
    ) -> Result<Self, WalletDatError> {
        // Validate mnemonic first
        wallet::validate_mnemonic(mnemonic)
            .map_err(|e| WalletDatError::WalletError(wallet::WalletError::MnemonicError(e)))?;

        // Create backup if wallet exists (save old wallet before replacing)
        let wallet_path = WalletDat::default_path(network);
        if wallet_path.exists() {
            let backup_path = wallet_path.with_extension("dat.old");
            fs::copy(&wallet_path, &backup_path)?;
            log::warn!(
                "Old masternode wallet backed up to: {}",
                backup_path.display()
            );
        }

        // Create wallet from mnemonic
        let wallet = Wallet::from_mnemonic(mnemonic, "", network)?;

        // Create wallet_dat from mnemonic (stores xpub, encrypted mnemonic, master key)
        let wallet_dat = WalletDat::from_mnemonic(mnemonic, network)?;

        log::info!(
            "Replaced masternode wallet with new xpub: {}",
            wallet_dat.get_xpub()
        );

        // Save (atomic write with temp file)
        wallet_dat.save()?;

        Ok(Self {
            wallet_dat,
            active_wallet: wallet,
            next_address_index: 0,
        })
    }

    /// Get the xpub for this wallet
    pub fn get_xpub(&self) -> &str {
        self.wallet_dat.get_xpub()
    }

    /// Derive an address at the given index
    pub fn derive_address(&self, index: u32) -> Result<String, WalletDatError> {
        self.wallet_dat.derive_address(index)
    }

    /// Derive a keypair at the given index
    pub fn derive_keypair(&self, index: u32) -> Result<Keypair, WalletDatError> {
        self.wallet_dat.derive_keypair(index)
    }

    /// Get the next available address (and increment counter)
    pub fn get_next_address(&mut self) -> Result<String, WalletDatError> {
        let address = self.derive_address(self.next_address_index)?;
        self.next_address_index += 1;
        Ok(address)
    }

    /// Generate a new address and get its index
    pub fn generate_new_address_with_index(&mut self) -> Result<(String, u32), WalletDatError> {
        let index = self.next_address_index;
        let address = self.derive_address(index)?;
        self.next_address_index += 1;
        Ok((address, index))
    }

    /// Get the current address count (next index)
    pub fn get_address_count(&self) -> u32 {
        self.next_address_index
    }

    /// Get network type
    pub fn network(&self) -> NetworkType {
        self.wallet_dat.network
    }

    /// Get wallet file path
    pub fn wallet_path(&self) -> std::path::PathBuf {
        WalletDat::default_path(self.wallet_dat.network)
    }

    /// Get reference to the active wallet
    pub fn wallet(&self) -> &Wallet {
        &self.active_wallet
    }

    /// Get mutable reference to the active wallet
    pub fn wallet_mut(&mut self) -> &mut Wallet {
        &mut self.active_wallet
    }

    /// Get the primary address (first derived address)
    pub fn primary_address(&self) -> Result<String, WalletDatError> {
        self.derive_address(0)
    }

    /// Sign a transaction with the wallet's keypair
    pub fn sign_transaction(&self, transaction: &mut Transaction) -> Result<(), WalletDatError> {
        self.active_wallet
            .sign_transaction(transaction)
            .map_err(WalletDatError::WalletError)
    }

    /// Create a transaction
    pub fn create_transaction(
        &mut self,
        to_address: &str,
        amount: u64,
        fee: u64,
    ) -> Result<Transaction, WalletDatError> {
        self.active_wallet
            .create_transaction(to_address, amount, fee)
            .map_err(WalletDatError::WalletError)
    }

    /// Get wallet balance
    pub fn balance(&self) -> u64 {
        self.active_wallet.balance()
    }

    /// Set wallet balance
    pub fn set_balance(&mut self, balance: u64) {
        self.active_wallet.set_balance(balance);
    }

    /// Add UTXO to wallet
    pub fn add_utxo(&mut self, utxo: UTXO) {
        self.active_wallet.add_utxo(utxo);
    }

    /// Remove UTXO from wallet
    pub fn remove_utxo(&mut self, tx_hash: &[u8; 32], output_index: u32) {
        self.active_wallet.remove_utxo(tx_hash, output_index);
    }

    /// Get UTXOs
    pub fn utxos(&self) -> &[UTXO] {
        self.active_wallet.utxos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_mnemonic() {
        let mnemonic = WalletManager::generate_mnemonic().unwrap();
        let words: Vec<&str> = mnemonic.split_whitespace().collect();
        assert_eq!(words.len(), 12);
    }

    #[test]
    fn test_validate_mnemonic() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        assert!(WalletManager::validate_mnemonic(mnemonic).is_ok());

        let bad_mnemonic = "bad mnemonic phrase";
        assert!(WalletManager::validate_mnemonic(bad_mnemonic).is_err());
    }

    #[test]
    fn test_derive_address() {
        use std::fs;
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        // Clean up any existing test wallet
        let wallet_path = WalletDat::default_path(NetworkType::Testnet);
        let _ = fs::remove_file(&wallet_path);

        let manager = WalletManager::create_from_mnemonic(NetworkType::Testnet, mnemonic).unwrap();

        let address = manager.derive_address(0).unwrap();
        assert!(!address.is_empty());
        assert!(address.starts_with("tTIME") || address.starts_with("TIME"));

        // Clean up
        let _ = fs::remove_file(&wallet_path);
    }

    #[test]
    fn test_get_next_address() {
        use std::fs;
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        // Create test directory structure
        let wallet_path = WalletDat::default_path(NetworkType::Testnet);
        if let Some(parent) = wallet_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        // Clean up any existing test wallet
        let _ = fs::remove_file(&wallet_path);

        let mut manager =
            WalletManager::create_from_mnemonic(NetworkType::Testnet, mnemonic).unwrap();

        let addr1 = manager.get_next_address().unwrap();
        let addr2 = manager.get_next_address().unwrap();
        assert_ne!(addr1, addr2);
        assert_eq!(manager.get_address_count(), 2);

        // Clean up
        let _ = fs::remove_file(&wallet_path);
    }
}
