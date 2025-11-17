//! Wallet Manager
//!
//! Manages time-wallet.dat file and provides high-level wallet operations
//!
//! The wallet now uses deterministic address derivation:
//! - time-wallet.dat stores ONLY: xpub, encrypted mnemonic, master key
//! - Addresses are derived on-demand from xpub
//! - Contact info and metadata stored separately in wallet.db

use crate::wallet_dat::{WalletDat, WalletDatError};
use serde::{Deserialize, Serialize};
use wallet::{Keypair, NetworkType, Transaction, Wallet, UTXO};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddressMetadata {
    pub address: String,
    pub label: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
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
    pub fn create_from_mnemonic(
        network: NetworkType,
        mnemonic: &str,
    ) -> Result<Self, WalletDatError> {
        // Validate mnemonic first
        wallet::validate_mnemonic(mnemonic)
            .map_err(|e| WalletDatError::WalletError(wallet::WalletError::MnemonicError(e)))?;

        // Create wallet from mnemonic
        let wallet = Wallet::from_mnemonic(mnemonic, "", network)?;

        // Create wallet_dat from mnemonic (stores xpub, encrypted mnemonic, master key)
        let wallet_dat = WalletDat::from_mnemonic(mnemonic, network)?;

        log::info!("Created wallet with xpub: {}", wallet_dat.get_xpub());

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

    /// Check if wallet exists
    pub fn exists(network: NetworkType) -> bool {
        WalletDat::default_path(network).exists()
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

    /// Get the active wallet
    pub fn get_active_wallet(&self) -> &Wallet {
        &self.active_wallet
    }

    /// Get mutable active wallet
    pub fn get_active_wallet_mut(&mut self) -> &mut Wallet {
        &mut self.active_wallet
    }

    /// Get current balance from active wallet
    pub fn get_balance(&self) -> u64 {
        self.active_wallet.balance()
    }

    /// Get primary address (first derived address)
    pub fn get_primary_address(&self) -> Result<String, WalletDatError> {
        self.derive_address(0)
    }

    /// Add UTXO to active wallet
    pub fn add_utxo(&mut self, utxo: UTXO) {
        self.active_wallet.add_utxo(utxo);
    }

    /// Get all UTXOs from active wallet
    pub fn get_utxos(&self) -> Vec<UTXO> {
        self.active_wallet.utxos().to_vec()
    }

    /// Create a transaction
    pub fn create_transaction(
        &mut self,
        to_address: &str,
        amount: u64,
        fee: u64,
    ) -> Result<Transaction, String> {
        self.active_wallet
            .create_transaction(to_address, amount, fee)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_manager_creation_from_mnemonic() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let manager = WalletManager::create_from_mnemonic(NetworkType::Testnet, mnemonic).unwrap();

        assert!(!manager.get_xpub().is_empty());
        assert_eq!(manager.get_balance(), 0);

        // Can derive addresses
        let addr0 = manager.derive_address(0).unwrap();
        assert!(!addr0.is_empty());

        // Cleanup
        let _ = std::fs::remove_file(WalletDat::default_path(NetworkType::Testnet));
    }

    #[test]
    fn test_balance_management() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let mut manager =
            WalletManager::create_from_mnemonic(NetworkType::Testnet, mnemonic).unwrap();

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
        let _ = std::fs::remove_file(WalletDat::default_path(NetworkType::Testnet));
    }
}
