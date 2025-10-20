//! Transaction Pool (Mempool)
//!
//! Manages pending transactions

use crate::state::{Address, Transaction};
use std::collections::{HashMap, VecDeque};

/// Maximum transactions in mempool
const MAX_MEMPOOL_SIZE: usize = 10_000;

/// Transaction pool for pending transactions
pub struct TransactionPool {
    /// Pending transactions by txid
    transactions: HashMap<String, Transaction>,

    /// Ordered queue for processing
    queue: VecDeque<String>,

    /// Track nonces per address to prevent double-spend
    nonces: HashMap<Address, u64>,
}

impl TransactionPool {
    pub fn new() -> Self {
        Self {
            transactions: HashMap::new(),
            queue: VecDeque::new(),
            nonces: HashMap::new(),
        }
    }

    /// Add transaction to pool
    pub fn add(&mut self, tx: Transaction) -> Result<(), String> {
        // Check size limit
        if self.transactions.len() >= MAX_MEMPOOL_SIZE {
            return Err("Mempool full".to_string());
        }

        // Check duplicate
        if self.transactions.contains_key(&tx.txid) {
            return Err("Transaction already in mempool".to_string());
        }

        // Add to pool
        let txid = tx.txid.clone();
        self.transactions.insert(txid.clone(), tx);
        self.queue.push_back(txid);

        Ok(())
    }

    /// Remove transaction from pool
    pub fn remove(&mut self, txid: &str) -> Option<Transaction> {
        if let Some(tx) = self.transactions.remove(txid) {
            self.queue.retain(|id| id != txid);
            Some(tx)
        } else {
            None
        }
    }

    /// Get transaction by ID
    pub fn get(&self, txid: &str) -> Option<&Transaction> {
        self.transactions.get(txid)
    }

    /// Get next transaction for processing
    pub fn next(&mut self) -> Option<Transaction> {
        if let Some(txid) = self.queue.pop_front() {
            self.transactions.remove(&txid)
        } else {
            None
        }
    }

    /// Get number of pending transactions
    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    /// Check if pool is empty
    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    /// Clear all transactions
    pub fn clear(&mut self) {
        self.transactions.clear();
        self.queue.clear();
        self.nonces.clear();
    }

    /// Get all transactions by address
    pub fn get_by_address(&self, address: &Address) -> Vec<&Transaction> {
        self.transactions
            .values()
            .filter(|tx| &tx.from == address || &tx.to == address)
            .collect()
    }
}

impl Default for TransactionPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_tx(from: &str, to: &str, amount: u64) -> Transaction {
        Transaction {
            txid: format!("tx_{}_{}_{}", from, to, amount),
            from: from.to_string(),
            to: to.to_string(),
            amount,
            fee: 1,
            timestamp: Utc::now().timestamp(),
            signature: vec![1, 2, 3],
        }
    }

    #[test]
    fn test_add_transaction() {
        let mut pool = TransactionPool::new();
        let tx = create_test_tx("addr1", "addr2", 100);

        assert!(pool.add(tx).is_ok());
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn test_duplicate_transaction() {
        let mut pool = TransactionPool::new();
        let tx = create_test_tx("addr1", "addr2", 100);

        pool.add(tx.clone()).unwrap();
        assert!(pool.add(tx).is_err());
    }

    #[test]
    fn test_remove_transaction() {
        let mut pool = TransactionPool::new();
        let tx = create_test_tx("addr1", "addr2", 100);
        let txid = tx.txid.clone();

        pool.add(tx).unwrap();
        assert!(pool.remove(&txid).is_some());
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn test_next_transaction() {
        let mut pool = TransactionPool::new();
        pool.add(create_test_tx("addr1", "addr2", 100)).unwrap();
        pool.add(create_test_tx("addr2", "addr3", 50)).unwrap();

        let tx1 = pool.next();
        assert!(tx1.is_some());
        assert_eq!(pool.len(), 1);

        let tx2 = pool.next();
        assert!(tx2.is_some());
        assert_eq!(pool.len(), 0);
    }
}
