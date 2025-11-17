//! Wallet database for storing metadata, contacts, and transaction history
//! Separate from wallet.dat which only stores keys

use serde::{Deserialize, Serialize};
use sled::Db;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WalletDbError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sled::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] bincode::Error),

    #[error("Not found: {0}")]
    NotFound(String),
}

/// Contact information for an address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressContact {
    pub address: String,
    pub label: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub notes: Option<String>,
    pub is_default: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Transaction record for history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    pub tx_hash: String,
    pub timestamp: i64,
    pub from_address: Option<String>,
    pub to_address: String,
    pub amount: u64,
    pub status: TransactionStatus,
    pub block_height: Option<u64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
}

/// Wallet metadata database
pub struct WalletDb {
    db: Db,
}

impl WalletDb {
    /// Open or create the wallet database
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, WalletDbError> {
        let db = sled::open(path)?;
        Ok(WalletDb { db })
    }

    // ==================== Address Contacts ====================

    /// Save or update contact information for an address
    pub fn save_contact(&self, contact: &AddressContact) -> Result<(), WalletDbError> {
        let key = format!("contact:{}", contact.address);
        let value = bincode::serialize(contact)?;
        self.db.insert(key.as_bytes(), value)?;
        self.db.flush()?;
        Ok(())
    }

    /// Get contact information for an address
    pub fn get_contact(&self, address: &str) -> Result<Option<AddressContact>, WalletDbError> {
        let key = format!("contact:{}", address);
        match self.db.get(key.as_bytes())? {
            Some(data) => Ok(Some(bincode::deserialize(&data)?)),
            None => Ok(None),
        }
    }

    /// Get all contacts
    pub fn get_all_contacts(&self) -> Result<Vec<AddressContact>, WalletDbError> {
        let mut contacts = Vec::new();
        let prefix = b"contact:";

        for item in self.db.scan_prefix(prefix) {
            let (_key, value) = item?;
            let contact: AddressContact = bincode::deserialize(&value)?;
            contacts.push(contact);
        }

        contacts.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(contacts)
    }

    /// Delete contact
    pub fn delete_contact(&self, address: &str) -> Result<(), WalletDbError> {
        let key = format!("contact:{}", address);
        self.db.remove(key.as_bytes())?;
        self.db.flush()?;
        Ok(())
    }

    /// Set default address
    pub fn set_default_address(&self, address: &str) -> Result<(), WalletDbError> {
        // First, unset all defaults
        for contact in self.get_all_contacts()? {
            if contact.is_default {
                let mut updated = contact;
                updated.is_default = false;
                updated.updated_at = chrono::Utc::now().timestamp();
                self.save_contact(&updated)?;
            }
        }

        // Set the new default
        if let Some(mut contact) = self.get_contact(address)? {
            contact.is_default = true;
            contact.updated_at = chrono::Utc::now().timestamp();
            self.save_contact(&contact)?;
        }

        Ok(())
    }

    /// Get default address
    pub fn get_default_address(&self) -> Result<Option<AddressContact>, WalletDbError> {
        for contact in self.get_all_contacts()? {
            if contact.is_default {
                return Ok(Some(contact));
            }
        }
        Ok(None)
    }

    // ==================== Transaction History ====================

    /// Save transaction to history
    pub fn save_transaction(&self, tx: &TransactionRecord) -> Result<(), WalletDbError> {
        let key = format!("tx:{}", tx.tx_hash);
        let value = bincode::serialize(tx)?;
        self.db.insert(key.as_bytes(), value)?;
        self.db.flush()?;
        Ok(())
    }

    /// Get transaction by hash
    pub fn get_transaction(
        &self,
        tx_hash: &str,
    ) -> Result<Option<TransactionRecord>, WalletDbError> {
        let key = format!("tx:{}", tx_hash);
        match self.db.get(key.as_bytes())? {
            Some(data) => Ok(Some(bincode::deserialize(&data)?)),
            None => Ok(None),
        }
    }

    /// Get all transactions, sorted by timestamp (newest first)
    pub fn get_all_transactions(&self) -> Result<Vec<TransactionRecord>, WalletDbError> {
        let mut transactions = Vec::new();
        let prefix = b"tx:";

        for item in self.db.scan_prefix(prefix) {
            let (_key, value) = item?;
            let tx: TransactionRecord = bincode::deserialize(&value)?;
            transactions.push(tx);
        }

        transactions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(transactions)
    }

    /// Get transactions for a specific address
    pub fn get_transactions_for_address(
        &self,
        address: &str,
    ) -> Result<Vec<TransactionRecord>, WalletDbError> {
        let all_txs = self.get_all_transactions()?;
        Ok(all_txs
            .into_iter()
            .filter(|tx| {
                tx.to_address == address
                    || tx.from_address.as_ref().is_some_and(|from| from == address)
            })
            .collect())
    }

    /// Update transaction status
    pub fn update_transaction_status(
        &self,
        tx_hash: &str,
        status: TransactionStatus,
        block_height: Option<u64>,
    ) -> Result<(), WalletDbError> {
        if let Some(mut tx) = self.get_transaction(tx_hash)? {
            tx.status = status;
            if let Some(height) = block_height {
                tx.block_height = Some(height);
            }
            self.save_transaction(&tx)?;
        }
        Ok(())
    }

    // ==================== Settings ====================

    /// Save a setting
    pub fn save_setting(&self, key: &str, value: &str) -> Result<(), WalletDbError> {
        let db_key = format!("setting:{}", key);
        self.db.insert(db_key.as_bytes(), value.as_bytes())?;
        self.db.flush()?;
        Ok(())
    }

    /// Get a setting
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, WalletDbError> {
        let db_key = format!("setting:{}", key);
        match self.db.get(db_key.as_bytes())? {
            Some(data) => Ok(Some(String::from_utf8_lossy(&data).to_string())),
            None => Ok(None),
        }
    }
}
