// Wallet implementation
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub address: String,
    pub balance: u64,
    pub locked_collateral: u64,
    pub tier: MasternodeTier,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MasternodeTier {
    Free,
    Bronze,
    Silver,
    Gold,
}

impl Wallet {
    pub fn new(address: String, _pubkey: String) -> Self {
        Self {
            address,
            balance: 0,
            locked_collateral: 0,
            tier: MasternodeTier::Free,
        }
    }

    pub fn add_reward(&mut self, amount: u64) {
        self.balance += amount;
    }

    pub fn get_balance(&self) -> u64 {
        self.balance
    }
}

pub struct WalletManager {
    wallets: HashMap<String, Wallet>,
    _db_path: PathBuf,
}

impl WalletManager {
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            wallets: HashMap::new(),
            _db_path: db_path,
        }
    }

    pub fn create_wallet(&mut self, address: String, pubkey: String) -> Result<(), String> {
        if self.wallets.contains_key(&address) {
            return Err("Wallet already exists".to_string());
        }
        self.wallets
            .insert(address.clone(), Wallet::new(address, pubkey));
        Ok(())
    }

    pub fn get_wallet(&self, address: &str) -> Option<&Wallet> {
        self.wallets.get(address)
    }
}
