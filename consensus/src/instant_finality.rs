//! Instant Finality System
//!
//! Implements quorum-based instant transaction finality where transactions are
//! validated and confirmed by masternode consensus before being included in blocks.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use time_core::{OutPoint, Transaction};
use tokio::sync::RwLock;

/// Status of a transaction in the instant finality system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    /// Transaction received, pending validation
    Pending,
    /// Transaction validated by this node, awaiting quorum
    Validated,
    /// Transaction approved by quorum (instantly final)
    Approved { votes: usize, total_nodes: usize },
    /// Transaction rejected by quorum
    Rejected { reason: String },
    /// Transaction included in a block
    Confirmed { block_height: u64 },
}

/// Vote on a transaction from a masternode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionVote {
    /// Transaction ID being voted on
    pub txid: String,
    /// Masternode IP address
    pub voter: String,
    /// Vote decision
    pub approved: bool,
    /// Reason for rejection (if applicable)
    pub reason: Option<String>,
    /// Timestamp of vote
    pub timestamp: i64,
    /// Signature from masternode
    pub signature: Vec<u8>,
}

/// Entry tracking transaction finality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalityEntry {
    /// Transaction ID
    pub txid: String,
    /// The transaction itself
    pub transaction: Transaction,
    /// Current status
    pub status: TransactionStatus,
    /// Votes received
    pub votes: HashMap<String, bool>, // masternode_id -> approved
    /// When transaction was first seen
    pub first_seen: i64,
    /// When transaction reached finality (if applicable)
    pub finalized_at: Option<i64>,
    /// UTXOs spent by this transaction
    pub spent_utxos: Vec<OutPoint>,
}

/// Instant Finality Manager
pub struct InstantFinalityManager {
    /// Pending and finalized transactions
    transactions: Arc<RwLock<HashMap<String, FinalityEntry>>>,
    /// Active masternode list
    masternodes: Arc<RwLock<Vec<String>>>,
    /// Quorum threshold (percentage, e.g., 67 for 67%)
    quorum_threshold: u8,
    /// Track which UTXOs are locked by pending transactions
    locked_utxos: Arc<RwLock<HashMap<OutPoint, String>>>, // outpoint -> txid
    /// History of finalized transactions (for auditing)
    finality_history: Arc<RwLock<Vec<(String, TransactionStatus, i64)>>>,
}

impl InstantFinalityManager {
    /// Create new instant finality manager
    pub fn new(quorum_threshold: u8) -> Self {
        Self {
            transactions: Arc::new(RwLock::new(HashMap::new())),
            masternodes: Arc::new(RwLock::new(Vec::new())),
            quorum_threshold,
            locked_utxos: Arc::new(RwLock::new(HashMap::new())),
            finality_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Update masternode list
    pub async fn update_masternodes(&self, masternodes: Vec<String>) {
        let mut nodes = self.masternodes.write().await;
        *nodes = masternodes;
    }

    /// Submit a new transaction for instant finality validation
    pub async fn submit_transaction(
        &self,
        transaction: Transaction,
    ) -> Result<String, String> {
        let txid = transaction.txid.clone();

        // Check for double-spend with locked UTXOs
        let mut locked = self.locked_utxos.write().await;
        for input in &transaction.inputs {
            if let Some(existing_txid) = locked.get(&input.previous_output) {
                if existing_txid != &txid {
                    return Err(format!(
                        "Double-spend detected: UTXO already locked by transaction {}",
                        existing_txid
                    ));
                }
            }
        }

        // Lock UTXOs for this transaction
        for input in &transaction.inputs {
            locked.insert(input.previous_output.clone(), txid.clone());
        }
        drop(locked);

        // Create finality entry
        let entry = FinalityEntry {
            txid: txid.clone(),
            transaction: transaction.clone(),
            status: TransactionStatus::Pending,
            votes: HashMap::new(),
            first_seen: chrono::Utc::now().timestamp(),
            finalized_at: None,
            spent_utxos: transaction.inputs.iter().map(|i| i.previous_output.clone()).collect(),
        };

        let mut txs = self.transactions.write().await;
        txs.insert(txid.clone(), entry);

        Ok(txid)
    }

    /// Record a vote from a masternode
    pub async fn record_vote(&self, vote: TransactionVote) -> Result<TransactionStatus, String> {
        let mut txs = self.transactions.write().await;
        
        let entry = txs
            .get_mut(&vote.txid)
            .ok_or_else(|| format!("Transaction {} not found", vote.txid))?;

        // Don't allow votes on already finalized transactions
        if matches!(entry.status, TransactionStatus::Approved { .. } | TransactionStatus::Rejected { .. }) {
            return Ok(entry.status.clone());
        }

        // Record vote
        entry.votes.insert(vote.voter.clone(), vote.approved);

        // Check if quorum reached
        let masternodes = self.masternodes.read().await;
        let total_nodes = masternodes.len();
        let required_votes = (total_nodes * self.quorum_threshold as usize) / 100;
        
        let approve_votes = entry.votes.values().filter(|&&v| v).count();
        let _reject_votes = entry.votes.values().filter(|&&v| !v).count();

        // Check for approval quorum
        if approve_votes >= required_votes {
            entry.status = TransactionStatus::Approved {
                votes: approve_votes,
                total_nodes,
            };
            entry.finalized_at = Some(chrono::Utc::now().timestamp());
            
            // Log to history
            let mut history = self.finality_history.write().await;
            history.push((
                vote.txid.clone(),
                entry.status.clone(),
                entry.finalized_at.unwrap(),
            ));
            
            return Ok(entry.status.clone());
        }

        // Check for rejection quorum
        let votes_remaining = total_nodes - entry.votes.len();
        let max_possible_approvals = approve_votes + votes_remaining;
        
        if max_possible_approvals < required_votes {
            // Can't reach approval quorum anymore - rejected
            let reason = vote.reason.unwrap_or_else(|| "Failed to reach approval quorum".to_string());
            entry.status = TransactionStatus::Rejected { reason: reason.clone() };
            entry.finalized_at = Some(chrono::Utc::now().timestamp());
            
            // Unlock UTXOs
            let mut locked = self.locked_utxos.write().await;
            for outpoint in &entry.spent_utxos {
                locked.remove(outpoint);
            }
            
            // Log to history
            let mut history = self.finality_history.write().await;
            history.push((
                vote.txid.clone(),
                entry.status.clone(),
                entry.finalized_at.unwrap(),
            ));
            
            return Ok(entry.status.clone());
        }

        // Still waiting for more votes
        Ok(TransactionStatus::Validated)
    }

    /// Get transaction status
    pub async fn get_status(&self, txid: &str) -> Option<TransactionStatus> {
        let txs = self.transactions.read().await;
        txs.get(txid).map(|e| e.status.clone())
    }

    /// Get all approved transactions ready for block inclusion
    pub async fn get_approved_transactions(&self) -> Vec<Transaction> {
        let txs = self.transactions.read().await;
        txs.values()
            .filter(|e| matches!(e.status, TransactionStatus::Approved { .. }))
            .map(|e| e.transaction.clone())
            .collect()
    }

    /// Mark transaction as confirmed in a block
    pub async fn mark_confirmed(&self, txid: &str, block_height: u64) -> Result<(), String> {
        let mut txs = self.transactions.write().await;
        
        let entry = txs
            .get_mut(txid)
            .ok_or_else(|| format!("Transaction {} not found", txid))?;

        // Only approved transactions can be confirmed
        if !matches!(entry.status, TransactionStatus::Approved { .. }) {
            return Err(format!("Transaction {} is not approved", txid));
        }

        entry.status = TransactionStatus::Confirmed { block_height };

        // Unlock UTXOs since they're now confirmed on-chain
        let mut locked = self.locked_utxos.write().await;
        for outpoint in &entry.spent_utxos {
            locked.remove(outpoint);
        }

        Ok(())
    }

    /// Reverse a transaction (if it was approved but later rejected by network)
    pub async fn reverse_transaction(&self, txid: &str, reason: String) -> Result<(), String> {
        let mut txs = self.transactions.write().await;
        
        let entry = txs
            .get_mut(txid)
            .ok_or_else(|| format!("Transaction {} not found", txid))?;

        entry.status = TransactionStatus::Rejected { reason: reason.clone() };
        entry.finalized_at = Some(chrono::Utc::now().timestamp());

        // Unlock UTXOs
        let mut locked = self.locked_utxos.write().await;
        for outpoint in &entry.spent_utxos {
            locked.remove(outpoint);
        }

        // Log reversal to history
        let mut history = self.finality_history.write().await;
        history.push((
            txid.to_string(),
            entry.status.clone(),
            chrono::Utc::now().timestamp(),
        ));

        Ok(())
    }

    /// Clean up old finalized transactions
    pub async fn cleanup_old_transactions(&self, max_age_seconds: i64) {
        let now = chrono::Utc::now().timestamp();
        let mut txs = self.transactions.write().await;
        
        txs.retain(|_, entry| {
            // Keep pending/validated transactions
            if matches!(entry.status, TransactionStatus::Pending | TransactionStatus::Validated) {
                return true;
            }
            
            // Keep recent finalized transactions
            if let Some(finalized_at) = entry.finalized_at {
                return (now - finalized_at) < max_age_seconds;
            }
            
            true
        });
    }

    /// Get statistics
    pub async fn get_stats(&self) -> FinalityStats {
        let txs = self.transactions.read().await;
        let locked = self.locked_utxos.read().await;
        
        let mut stats = FinalityStats {
            total_transactions: txs.len(),
            pending: 0,
            validated: 0,
            approved: 0,
            rejected: 0,
            confirmed: 0,
            locked_utxos: locked.len(),
        };

        for entry in txs.values() {
            match entry.status {
                TransactionStatus::Pending => stats.pending += 1,
                TransactionStatus::Validated => stats.validated += 1,
                TransactionStatus::Approved { .. } => stats.approved += 1,
                TransactionStatus::Rejected { .. } => stats.rejected += 1,
                TransactionStatus::Confirmed { .. } => stats.confirmed += 1,
            }
        }

        stats
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FinalityStats {
    pub total_transactions: usize,
    pub pending: usize,
    pub validated: usize,
    pub approved: usize,
    pub rejected: usize,
    pub confirmed: usize,
    pub locked_utxos: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_instant_finality_approval() {
        let manager = InstantFinalityManager::new(67); // 67% quorum
        
        // Setup 3 masternodes
        manager.update_masternodes(vec![
            "192.168.1.1".to_string(),
            "192.168.1.2".to_string(),
            "192.168.1.3".to_string(),
        ]).await;

        // Create a mock transaction
        let tx = Transaction {
            txid: "test_tx_1".to_string(),
            version: 1,
            lock_time: 0,
            timestamp: chrono::Utc::now().timestamp(),
            inputs: vec![],
            outputs: vec![],
        };

        // Submit transaction
        let txid = manager.submit_transaction(tx).await.unwrap();

        // Vote from 2 nodes (67% of 3 = 2)
        let vote1 = TransactionVote {
            txid: txid.clone(),
            voter: "192.168.1.1".to_string(),
            approved: true,
            reason: None,
            timestamp: chrono::Utc::now().timestamp(),
            signature: vec![],
        };
        
        let status1 = manager.record_vote(vote1).await.unwrap();
        assert_eq!(status1, TransactionStatus::Validated);

        let vote2 = TransactionVote {
            txid: txid.clone(),
            voter: "192.168.1.2".to_string(),
            approved: true,
            reason: None,
            timestamp: chrono::Utc::now().timestamp(),
            signature: vec![],
        };
        
        let status2 = manager.record_vote(vote2).await.unwrap();
        assert!(matches!(status2, TransactionStatus::Approved { .. }));
    }
}
