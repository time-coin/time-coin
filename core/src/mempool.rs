//! Transaction Pool (Mempool) for TIME Coin
//!
//! Manages pending transactions with UTXO validation

use crate::transaction::{Transaction, TransactionError};
use crate::utxo_set::UTXOSet;
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

/// Maximum transactions in mempool
const MAX_MEMPOOL_SIZE: usize = 10_000;

#[derive(Debug, Clone)]
pub enum MempoolError {
    TransactionError(TransactionError),
    MempoolFull,
    DuplicateTransaction,
    InvalidTransaction,
    ConflictingTransaction,
}

impl std::fmt::Display for MempoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MempoolError::TransactionError(e) => write!(f, "Transaction error: {}", e),
            MempoolError::MempoolFull => write!(f, "Mempool is full"),
            MempoolError::DuplicateTransaction => write!(f, "Transaction already in mempool"),
            MempoolError::InvalidTransaction => write!(f, "Invalid transaction"),
            MempoolError::ConflictingTransaction => write!(f, "Transaction conflicts with another in mempool"),
        }
    }
}

impl std::error::Error for MempoolError {}

impl From<TransactionError> for MempoolError {
    fn from(err: TransactionError) -> Self {
        MempoolError::TransactionError(err)
    }
}

/// Transaction with priority for ordering
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MempoolTransaction {
    transaction: Transaction,
    fee_per_byte: u64,
    added_timestamp: i64,
}

/// Transaction pool for pending transactions
#[derive(Debug, Clone)]
pub struct Mempool {
    /// Pending transactions by txid
    transactions: HashMap<String, MempoolTransaction>,
    
    /// Track which UTXOs are being spent (to prevent double-spend in mempool)
    spent_outputs: HashSet<String>, // Format: "txid:vout"
    
    /// Maximum size
    max_size: usize,
}

impl Mempool {
    /// Create a new mempool
    pub fn new() -> Self {
        Self {
            transactions: HashMap::new(),
            spent_outputs: HashSet::new(),
            max_size: MAX_MEMPOOL_SIZE,
        }
    }

    /// Create a mempool with custom max size
    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            transactions: HashMap::new(),
            spent_outputs: HashSet::new(),
            max_size,
        }
    }

    /// Add a transaction to the mempool
    pub fn add_transaction(&mut self, tx: Transaction, utxo_set: &UTXOSet) -> Result<(), MempoolError> {
        // Check if mempool is full
        if self.transactions.len() >= self.max_size {
            return Err(MempoolError::MempoolFull);
        }

        // Check if transaction already exists
        if self.transactions.contains_key(&tx.txid) {
            return Err(MempoolError::DuplicateTransaction);
        }

        // Validate transaction structure
        tx.validate_structure()?;

        // Don't allow coinbase transactions in mempool
        if tx.is_coinbase() {
            return Err(MempoolError::InvalidTransaction);
        }

        // Check for conflicts with other mempool transactions
        for input in &tx.inputs {
            let output_key = format!("{}:{}", input.previous_output.txid, input.previous_output.vout);
            if self.spent_outputs.contains(&output_key) {
                return Err(MempoolError::ConflictingTransaction);
            }
        }

        // Validate against UTXO set
        for input in &tx.inputs {
            if !utxo_set.contains(&input.previous_output) {
                return Err(MempoolError::InvalidTransaction);
            }
        }

        // Calculate fee per byte for priority
        let fee = tx.fee(utxo_set.utxos())?;
        let tx_size = serde_json::to_string(&tx).map(|s| s.len()).unwrap_or(1000);
        let fee_per_byte = fee / tx_size.max(1) as u64;

        // Add to mempool
        let mempool_tx = MempoolTransaction {
            transaction: tx.clone(),
            fee_per_byte,
            added_timestamp: chrono::Utc::now().timestamp(),
        };

        // Mark outputs as spent
        for input in &tx.inputs {
            let output_key = format!("{}:{}", input.previous_output.txid, input.previous_output.vout);
            self.spent_outputs.insert(output_key);
        }

        self.transactions.insert(tx.txid.clone(), mempool_tx);

        Ok(())
    }

    /// Remove a transaction from the mempool
    pub fn remove_transaction(&mut self, txid: &str) -> Option<Transaction> {
        if let Some(mempool_tx) = self.transactions.remove(txid) {
            // Unmark spent outputs
            for input in &mempool_tx.transaction.inputs {
                let output_key = format!("{}:{}", input.previous_output.txid, input.previous_output.vout);
                self.spent_outputs.remove(&output_key);
            }
            Some(mempool_tx.transaction)
        } else {
            None
        }
    }

    /// Get a transaction from the mempool
    pub fn get_transaction(&self, txid: &str) -> Option<&Transaction> {
        self.transactions.get(txid).map(|mt| &mt.transaction)
    }

    /// Get transactions ordered by fee (highest first)
    pub fn get_transactions_by_fee(&self, limit: usize) -> Vec<Transaction> {
        let mut txs: Vec<_> = self.transactions.values().collect();
        txs.sort_by(|a, b| {
            // Sort by fee per byte (descending), then by timestamp (ascending)
            b.fee_per_byte.cmp(&a.fee_per_byte)
                .then(a.added_timestamp.cmp(&b.added_timestamp))
        });

        txs.into_iter()
            .take(limit)
            .map(|mt| mt.transaction.clone())
            .collect()
    }

    /// Get all transactions
    pub fn get_all_transactions(&self) -> Vec<Transaction> {
        self.transactions
            .values()
            .map(|mt| mt.transaction.clone())
            .collect()
    }

    /// Get transactions for a specific address (appears in inputs or outputs)
    pub fn get_transactions_for_address(&self, address: &str) -> Vec<Transaction> {
        self.transactions
            .values()
            .filter(|mt| {
                // Check if address appears in any output
                mt.transaction.outputs.iter().any(|out| out.address == address)
            })
            .map(|mt| mt.transaction.clone())
            .collect()
    }

    /// Clear all transactions
    pub fn clear(&mut self) {
        self.transactions.clear();
        self.spent_outputs.clear();
    }

    /// Get mempool size
    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    /// Check if mempool is empty
    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    /// Remove transactions that are now invalid (UTXOs spent in blockchain)
    pub fn remove_invalid_transactions(&mut self, utxo_set: &UTXOSet) -> Vec<String> {
        let mut removed = Vec::new();

        // Find transactions with invalid inputs
        let invalid_txids: Vec<String> = self.transactions
            .iter()
            .filter(|(_, mt)| {
                // Check if any input no longer exists in UTXO set
                mt.transaction.inputs.iter().any(|input| {
                    !utxo_set.contains(&input.previous_output)
                })
            })
            .map(|(txid, _)| txid.clone())
            .collect();

        // Remove invalid transactions
        for txid in invalid_txids {
            if self.remove_transaction(&txid).is_some() {
                removed.push(txid);
            }
        }

        removed
    }

    /// Get mempool statistics
    pub fn get_stats(&self) -> MempoolStats {
        let total_fees: u64 = self.transactions
            .values()
            .map(|mt| {
                // Estimate fee based on fee_per_byte
                let size = serde_json::to_string(&mt.transaction)
                    .map(|s| s.len())
                    .unwrap_or(1000);
                mt.fee_per_byte * size as u64
            })
            .sum();

        MempoolStats {
            transaction_count: self.transactions.len(),
            total_fees,
            max_fee_per_byte: self.transactions
                .values()
                .map(|mt| mt.fee_per_byte)
                .max()
                .unwrap_or(0),
            min_fee_per_byte: self.transactions
                .values()
                .map(|mt| mt.fee_per_byte)
                .min()
                .unwrap_or(0),
        }
    }
}

impl Default for Mempool {
    fn default() -> Self {
        Self::new()
    }
}

/// Mempool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolStats {
    pub transaction_count: usize,
    pub total_fees: u64,
    pub max_fee_per_byte: u64,
    pub min_fee_per_byte: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::{TxInput, TxOutput, OutPoint};

    #[test]
    fn test_mempool_creation() {
        let mempool = Mempool::new();
        assert_eq!(mempool.len(), 0);
        assert!(mempool.is_empty());
    }

    #[test]
    fn test_add_transaction() {
        let mut mempool = Mempool::new();
        let mut utxo_set = UTXOSet::new();

        // Add UTXO for the transaction to spend
        let prev_outpoint = OutPoint::new("prev_tx".to_string(), 0);
        let prev_output = TxOutput::new(2000, "addr1".to_string());
        utxo_set.add_utxo(prev_outpoint.clone(), prev_output);

        // Create transaction
        let input = TxInput::new("prev_tx".to_string(), 0, vec![1, 2, 3], vec![4, 5, 6]);
        let output = TxOutput::new(1900, "addr2".to_string());
        let tx = Transaction::new(vec![input], vec![output]);

        // Add to mempool
        let result = mempool.add_transaction(tx.clone(), &utxo_set);
        assert!(result.is_ok());
        assert_eq!(mempool.len(), 1);
        assert!(mempool.get_transaction(&tx.txid).is_some());
    }

    #[test]
    fn test_duplicate_transaction() {
        let mut mempool = Mempool::new();
        let mut utxo_set = UTXOSet::new();

        let prev_outpoint = OutPoint::new("prev_tx".to_string(), 0);
        let prev_output = TxOutput::new(2000, "addr1".to_string());
        utxo_set.add_utxo(prev_outpoint, prev_output);

        let input = TxInput::new("prev_tx".to_string(), 0, vec![1, 2, 3], vec![4, 5, 6]);
        let output = TxOutput::new(1900, "addr2".to_string());
        let tx = Transaction::new(vec![input], vec![output]);

        mempool.add_transaction(tx.clone(), &utxo_set).unwrap();
        
        // Try to add same transaction again
        let result = mempool.add_transaction(tx, &utxo_set);
        assert!(matches!(result, Err(MempoolError::DuplicateTransaction)));
    }
}
