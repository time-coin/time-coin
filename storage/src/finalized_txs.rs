//! Finalized Transactions Storage
//!
//! Stores transactions that have achieved instant finality through BFT consensus
//! but haven't yet been included in a block. This ensures:
//! - UTXO changes persist across restarts
//! - Instant finality remains instant (no waiting for blocks)
//! - Eventually included in blocks for archival

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use time_core::Transaction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalizedTransaction {
    pub transaction: Transaction,
    pub finalized_at: i64, // Unix timestamp
    pub votes_received: usize,
    pub total_voters: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FinalizedTransactionStore {
    /// Map of TXID -> FinalizedTransaction
    pub transactions: HashMap<String, FinalizedTransaction>,
}

impl FinalizedTransactionStore {
    pub fn new() -> Self {
        Self {
            transactions: HashMap::new(),
        }
    }

    /// Add a finalized transaction
    pub fn add(&mut self, tx: Transaction, votes: usize, total: usize) {
        let finalized = FinalizedTransaction {
            transaction: tx.clone(),
            finalized_at: chrono::Utc::now().timestamp(),
            votes_received: votes,
            total_voters: total,
        };
        self.transactions.insert(tx.txid.clone(), finalized);
    }

    /// Remove transaction (when it's been included in a block)
    pub fn remove(&mut self, txid: &str) {
        self.transactions.remove(txid);
    }

    /// Get all finalized transactions
    pub fn get_all(&self) -> Vec<Transaction> {
        self.transactions
            .values()
            .map(|ft| ft.transaction.clone())
            .collect()
    }

    /// Get a specific finalized transaction
    pub fn get(&self, txid: &str) -> Option<&FinalizedTransaction> {
        self.transactions.get(txid)
    }

    /// Check if a transaction is finalized
    pub fn contains(&self, txid: &str) -> bool {
        self.transactions.contains_key(txid)
    }

    /// Count of finalized transactions
    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    /// Save to disk
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Load from disk
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let json = fs::read_to_string(path)?;
        let store: Self = serde_json::from_str(&json)?;
        Ok(store)
    }

    /// Load from disk or create new if doesn't exist
    pub fn load_or_new<P: AsRef<Path>>(path: P) -> Self {
        match Self::load(&path) {
            Ok(store) => {
                println!(
                    "📂 Loaded {} finalized transactions from disk",
                    store.len()
                );
                store
            }
            Err(_) => {
                println!("📂 No existing finalized transactions file, starting fresh");
                Self::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finalized_tx_store() {
        let mut store = FinalizedTransactionStore::new();

        let tx = Transaction {
            txid: "test123".to_string(),
            inputs: vec![],
            outputs: vec![],
            timestamp: 0,
            signature: vec![],
        };

        store.add(tx.clone(), 5, 7);
        assert!(store.contains("test123"));
        assert_eq!(store.len(), 1);

        let finalized = store.get("test123").unwrap();
        assert_eq!(finalized.votes_received, 5);
        assert_eq!(finalized.total_voters, 7);

        store.remove("test123");
        assert!(!store.contains("test123"));
        assert_eq!(store.len(), 0);
    }
}
