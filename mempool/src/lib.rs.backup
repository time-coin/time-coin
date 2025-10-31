//! Mempool - Pending Transaction Pool for TIME Coin
//!
//! Manages pending transactions that haven't been included in blocks yet.
//! Provides validation, ordering, and transaction selection for block production.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use time_core::{Transaction, TransactionError};

/// Transaction pool for pending transactions
pub struct Mempool {
    /// Pending transactions by txid
    transactions: Arc<RwLock<HashMap<String, MempoolEntry>>>,
    /// Maximum size of mempool
    max_size: usize,
    /// Reference to blockchain for UTXO validation (optional)
    blockchain: Option<Arc<tokio::sync::RwLock<time_core::state::BlockchainState>>>,
}

/// Entry in the mempool with metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MempoolEntry {
    /// The transaction
    pub transaction: Transaction,
    /// When it was added to mempool
    pub added_at: i64,
    /// Priority score (higher = included sooner)
    pub priority: u64,
}

impl Mempool {
    /// Create a new mempool
    pub fn new(max_size: usize) -> Self {
        Self {
            transactions: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            blockchain: None,
        }
    }

    /// Create mempool with blockchain validation
    pub fn with_blockchain(
        max_size: usize,
        blockchain: Arc<tokio::sync::RwLock<time_core::state::BlockchainState>>,
    ) -> Self {
        Self {
            transactions: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            blockchain: Some(blockchain),
        }
    }

    /// Add a transaction to the mempool
    pub async fn add_transaction(&self, tx: Transaction) -> Result<(), MempoolError> {
        // Validate transaction structure
        tx.validate_structure()
            .map_err(MempoolError::InvalidTransaction)?;

        // Validate UTXO if blockchain is available
        if let Some(blockchain) = &self.blockchain {
            self.validate_utxo(&tx, blockchain).await?;
        }

        let mut pool = self.transactions.write().await;

        // Check if already in mempool
        if pool.contains_key(&tx.txid) {
            return Err(MempoolError::DuplicateTransaction);
        }

        // Check size limit
        if pool.len() >= self.max_size {
            return Err(MempoolError::MemPoolFull);
        }

        // Calculate priority (fee per byte, roughly)
        let tx_size = tx.txid.len() + tx.inputs.len() * 64 + tx.outputs.len() * 64;
        let total_fee = self.calculate_fee(&tx);
        let priority = if tx_size > 0 {
            (total_fee * 1000) / tx_size as u64
        } else {
            0
        };

        let entry = MempoolEntry {
            transaction: tx.clone(),
            added_at: chrono::Utc::now().timestamp(),
            priority,
        };

        pool.insert(tx.txid.clone(), entry);

        println!("📝 Added transaction {} to mempool (priority: {})", 
            &tx.txid[..16], priority);

        Ok(())
    }

    /// Validate transaction against UTXO set
    async fn validate_utxo(
        &self,
        tx: &Transaction,
        blockchain: &Arc<tokio::sync::RwLock<time_core::state::BlockchainState>>,
    ) -> Result<(), MempoolError> {
        // Coinbase transactions should ONLY be created by block producers
        if tx.is_coinbase() {
            return Err(MempoolError::InvalidTransaction(time_core::TransactionError::InvalidInput));
        }

        let chain = blockchain.read().await;
        let utxo_set = chain.utxo_set();
        
        let mut input_sum = 0u64;
        
        // Validate all inputs exist and are unspent
        for input in &tx.inputs {
            match utxo_set.get(&input.previous_output) {
                Some(utxo) => {
                    input_sum = input_sum.checked_add(utxo.amount)
                        .ok_or(MempoolError::InvalidTransaction(
                            time_core::TransactionError::InvalidAmount
                        ))?;
                }
                None => {
                    // Input does not exist or already spent
                    return Err(MempoolError::InvalidTransaction(
                        time_core::TransactionError::InvalidInput
                    ));
                }
            }
        }
        
        // Calculate output sum
        let output_sum: u64 = tx.outputs.iter().map(|o| o.amount).sum();
        
        // Inputs must be >= outputs
        if input_sum < output_sum {
            return Err(MempoolError::InvalidTransaction(
                time_core::TransactionError::InsufficientFunds
            ));
        }
        
        Ok(())
    }

    /// Remove a transaction from mempool (after inclusion in block)
    pub async fn remove_transaction(&self, txid: &str) -> Option<Transaction> {
        let mut pool = self.transactions.write().await;
        pool.remove(txid).map(|entry| entry.transaction)
    }

    /// Get transaction by ID
    pub async fn get_transaction(&self, txid: &str) -> Option<Transaction> {
        let pool = self.transactions.read().await;
        pool.get(txid).map(|entry| entry.transaction.clone())
    }

    /// Check if transaction exists in mempool
    pub async fn contains(&self, txid: &str) -> bool {
        let pool = self.transactions.read().await;
        pool.contains_key(txid)
    }

    /// Get all transactions (for broadcasting)
    pub async fn get_all_transactions(&self) -> Vec<Transaction> {
        let pool = self.transactions.read().await;
        pool.values()
            .map(|entry| entry.transaction.clone())
            .collect()
    }

    /// Select transactions for a block (by priority)
    pub async fn select_transactions(&self, max_count: usize) -> Vec<Transaction> {
        let pool = self.transactions.read().await;
        
        let mut entries: Vec<_> = pool.values().collect();
        
        // Sort by priority (highest first), then by time (oldest first)
        entries.sort_by(|a, b| {
            b.priority.cmp(&a.priority)
                .then(a.added_at.cmp(&b.added_at))
        });

        entries.into_iter()
            .take(max_count)
            .map(|entry| entry.transaction.clone())
            .collect()
    }

    /// Get mempool size
    pub async fn size(&self) -> usize {
        let pool = self.transactions.read().await;
        pool.len()
    }

    /// Clear all transactions (e.g., after chain reorganization)
    pub async fn clear(&self) {
        let mut pool = self.transactions.write().await;
        pool.clear();
    }

    /// Remove transactions that are now invalid
    pub async fn remove_invalid_transactions(&self, invalid_txids: Vec<String>) {
        let mut pool = self.transactions.write().await;
        for txid in invalid_txids {
            pool.remove(&txid);
        }
    }

    /// Save mempool to disk
    pub async fn save_to_disk(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let pool = self.transactions.read().await;
        let entries: Vec<&MempoolEntry> = pool.values().collect();
        
        // Create directory if it doesn't exist
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let json = serde_json::to_string_pretty(&entries)?;
        std::fs::write(path, json)?;
        
        Ok(())
    }

    /// Load mempool from disk
    pub async fn load_from_disk(&self, path: &str) -> Result<usize, Box<dyn std::error::Error>> {
        if !std::path::Path::new(path).exists() {
            return Ok(0);
        }
        
        let json = std::fs::read_to_string(path)?;
        let entries: Vec<MempoolEntry> = serde_json::from_str(&json)?;
        
        let mut pool = self.transactions.write().await;
        let mut loaded = 0;
        let now = chrono::Utc::now().timestamp();
        
        for entry in entries {
            // Skip transactions older than 24 hours
            if now - entry.added_at > 86400 {
                continue;
            }
            
            // Skip if mempool is full
            if pool.len() >= self.max_size {
                break;
            }
            
            pool.insert(entry.transaction.txid.clone(), entry);
            loaded += 1;
        }
        
        Ok(loaded)
    }

    /// Clean up stale transactions (older than 24 hours)
    pub async fn cleanup_stale(&self) -> usize {
        let mut pool = self.transactions.write().await;
        let now = chrono::Utc::now().timestamp();
        let mut removed = 0;
        
        pool.retain(|_, entry| {
            let is_fresh = now - entry.added_at < 86400;
            if !is_fresh {
                removed += 1;
            }
            is_fresh
        });
        
        removed
    }

    /// Calculate total fee for a transaction
    fn calculate_fee(&self, tx: &Transaction) -> u64 {
        // Fee = sum(inputs) - sum(outputs)
        // For now, we'll use a simple estimation
        // In production, you'd need UTXO set to get input values
        
        let output_sum: u64 = tx.outputs.iter().map(|o| o.amount).sum();
        
        // Estimate: assume inputs are worth slightly more than outputs
        // This is a placeholder - real implementation needs UTXO lookup
        output_sum / 100 // 1% fee estimation
    }
}

#[derive(Debug, Clone)]
pub enum MempoolError {
    DuplicateTransaction,
    MemPoolFull,
    InvalidTransaction(TransactionError),
}

impl std::fmt::Display for MempoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MempoolError::DuplicateTransaction => write!(f, "Transaction already in mempool"),
            MempoolError::MemPoolFull => write!(f, "Mempool is full"),
            MempoolError::InvalidTransaction(e) => write!(f, "Invalid transaction: {}", e),
        }
    }
}

impl std::error::Error for MempoolError {}

#[cfg(test)]
mod tests {
    use super::*;
    use time_core::TxOutput;

    #[tokio::test]
    async fn test_mempool_add_and_get() {
        let mempool = Mempool::new(100);
        
        let tx = Transaction {
            txid: "test_tx_1".to_string(),
            version: 1,
            inputs: vec![],
            outputs: vec![TxOutput {
                amount: 1000,
                address: "addr1".to_string(),
            }],
            lock_time: 0,
            timestamp: 1234567890,
        };

        mempool.add_transaction(tx.clone()).await.unwrap();
        
        assert_eq!(mempool.size().await, 1);
        assert!(mempool.contains("test_tx_1").await);
        
        let retrieved = mempool.get_transaction("test_tx_1").await.unwrap();
        assert_eq!(retrieved.txid, tx.txid);
    }

    #[tokio::test]
    async fn test_mempool_priority_selection() {
        let mempool = Mempool::new(100);
        
        // Add transactions with different priorities
        for i in 0..5 {
            let tx = Transaction {
                txid: format!("tx_{}", i),
                version: 1,
                inputs: vec![],
                outputs: vec![TxOutput {
                    amount: 1000 * (i + 1),
                    address: "addr".to_string(),
                }],
                lock_time: 0,
                timestamp: 1234567890 + i as i64,
            };
            mempool.add_transaction(tx).await.unwrap();
        }

        let selected = mempool.select_transactions(3).await;
        assert_eq!(selected.len(), 3);
    }
}
