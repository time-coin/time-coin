//! Wallet state management

use wallet::NetworkType;

pub struct WalletState {
    manager: crate::wallet_manager_impl::WalletManager,
}

impl WalletState {
    /// Check if wallet exists
    pub fn exists(network: NetworkType) -> bool {
        crate::wallet_manager_impl::WalletManager::exists(network)
    }

    /// Check if wallet is encrypted
    pub fn is_encrypted(network: NetworkType) -> Result<bool, String> {
        crate::wallet_manager_impl::WalletManager::is_encrypted(network).map_err(|e| e.to_string())
    }

    /// Create new wallet with mnemonic and password
    pub fn create(network: NetworkType, mnemonic: &str, password: &str) -> Result<Self, String> {
        let manager = crate::wallet_manager_impl::WalletManager::create_from_mnemonic_encrypted(
            network, mnemonic, password,
        )
        .map_err(|e| e.to_string())?;

        Ok(Self { manager })
    }

    /// Load wallet with password
    pub fn load(network: NetworkType, password: &str) -> Result<Self, String> {
        let manager =
            crate::wallet_manager_impl::WalletManager::load_with_password(network, password)
                .map_err(|e| format!("Failed to unlock: {}", e))?;

        Ok(Self { manager })
    }

    pub fn get_xpub(&self) -> String {
        self.manager.get_xpub().to_string()
    }

    pub fn get_mnemonic(&self) -> String {
        // Return empty string since mnemonic is encrypted
        // User should save it during wallet creation
        String::new()
    }

    pub fn get_address(&self) -> String {
        match self.manager.derive_address(0) {
            Ok(addr) => addr,
            Err(_) => "Error deriving address".to_string(),
        }
    }
}
