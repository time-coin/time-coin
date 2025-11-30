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
    /// Transaction validated by this node, awaiting network confirmation
    Validated,
    /// Transaction approved by masternode (instantly final)
    Approved { masternode: String, timestamp: i64 },
    /// Transaction declined by masternode
    Declined {
        masternode: String,
        reason: String,
        timestamp: i64,
    },
    /// Transaction rejected by network after approval (conflict detected)
    NetworkRejected { reason: String, timestamp: i64 },
    /// Transaction included in a block
    Confirmed { block_height: u64 },
}

/// Decision on a transaction from a masternode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionDecision {
    /// Transaction ID
    pub txid: String,
    /// Masternode IP address
    pub masternode: String,
    /// Decision (true = approved, false = declined)
    pub approved: bool,
    /// Reason for decision
    pub reason: String,
    /// Timestamp of decision
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
    /// Decisions received from masternodes
    pub decisions: HashMap<String, TransactionDecision>, // masternode_id -> decision
    /// When transaction was first seen
    pub first_seen: i64,
    /// When transaction reached finality (if applicable)
    pub finalized_at: Option<i64>,
    /// UTXOs spent by this transaction
    pub spent_utxos: Vec<OutPoint>,
    /// Wallets to notify about this transaction
    pub subscribed_wallets: Vec<String>,
}

/// Type alias for notification callback
type NotificationCallback = Box<dyn Fn(String, TransactionStatus) + Send + Sync>;

/// Instant Finality Manager
pub struct InstantFinalityManager {
    /// Pending and finalized transactions
    transactions: Arc<RwLock<HashMap<String, FinalityEntry>>>,
    /// Active masternode list
    masternodes: Arc<RwLock<Vec<String>>>,
    /// Track which UTXOs are locked by pending transactions
    locked_utxos: Arc<RwLock<HashMap<OutPoint, String>>>, // outpoint -> txid
    /// History of finalized transactions (for auditing)
    finality_history: Arc<RwLock<Vec<(String, TransactionStatus, i64)>>>,
    /// Callback for notifying wallets of transaction status changes
    notification_callback: Arc<RwLock<Option<NotificationCallback>>>,
}

impl Default for InstantFinalityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InstantFinalityManager {
    /// Create new instant finality manager
    pub fn new() -> Self {
        Self {
            transactions: Arc::new(RwLock::new(HashMap::new())),
            masternodes: Arc::new(RwLock::new(Vec::new())),
            locked_utxos: Arc::new(RwLock::new(HashMap::new())),
            finality_history: Arc::new(RwLock::new(Vec::new())),
            notification_callback: Arc::new(RwLock::new(None)),
        }
    }

    /// Set notification callback for wallet updates
    pub async fn set_notification_callback<F>(&self, callback: F)
    where
        F: Fn(String, TransactionStatus) + Send + Sync + 'static,
    {
        let mut cb = self.notification_callback.write().await;
        *cb = Some(Box::new(callback));
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
        wallet_xpubs: Vec<String>,
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
            decisions: HashMap::new(),
            first_seen: chrono::Utc::now().timestamp(),
            finalized_at: None,
            spent_utxos: transaction
                .inputs
                .iter()
                .map(|i| i.previous_output.clone())
                .collect(),
            subscribed_wallets: wallet_xpubs,
        };

        let mut txs = self.transactions.write().await;
        txs.insert(txid.clone(), entry);

        Ok(txid)
    }

    /// Record a decision from a masternode (approve or decline)
    pub async fn record_decision(
        &self,
        decision: TransactionDecision,
    ) -> Result<TransactionStatus, String> {
        let mut txs = self.transactions.write().await;

        let entry = txs
            .get_mut(&decision.txid)
            .ok_or_else(|| format!("Transaction {} not found", decision.txid))?;

        // Don't allow decisions on already finalized transactions
        if matches!(
            entry.status,
            TransactionStatus::Approved { .. }
                | TransactionStatus::Declined { .. }
                | TransactionStatus::NetworkRejected { .. }
        ) {
            return Ok(entry.status.clone());
        }

        // Record decision
        entry
            .decisions
            .insert(decision.masternode.clone(), decision.clone());

        // Update status based on decision
        let new_status = if decision.approved {
            TransactionStatus::Approved {
                masternode: decision.masternode.clone(),
                timestamp: decision.timestamp,
            }
        } else {
            TransactionStatus::Declined {
                masternode: decision.masternode.clone(),
                reason: decision.reason.clone(),
                timestamp: decision.timestamp,
            }
        };

        entry.status = new_status.clone();
        entry.finalized_at = Some(decision.timestamp);

        // Log to history
        let mut history = self.finality_history.write().await;
        history.push((
            decision.txid.clone(),
            new_status.clone(),
            decision.timestamp,
        ));

        // Notify subscribed wallets
        self.notify_wallets(
            &entry.subscribed_wallets,
            decision.txid.clone(),
            new_status.clone(),
        )
        .await;

        Ok(new_status)
    }

    /// Mark a transaction as rejected by network (after approval)
    pub async fn mark_network_rejected(&self, txid: &str, reason: String) -> Result<(), String> {
        let mut txs = self.transactions.write().await;

        let entry = txs
            .get_mut(txid)
            .ok_or_else(|| format!("Transaction {} not found", txid))?;

        let new_status = TransactionStatus::NetworkRejected {
            reason: reason.clone(),
            timestamp: chrono::Utc::now().timestamp(),
        };

        entry.status = new_status.clone();

        // Notify subscribed wallets about rejection
        self.notify_wallets(&entry.subscribed_wallets, txid.to_string(), new_status)
            .await;

        Ok(())
    }

    /// Notify subscribed wallets about transaction status change
    async fn notify_wallets(
        &self,
        wallet_xpubs: &[String],
        txid: String,
        status: TransactionStatus,
    ) {
        let callback = self.notification_callback.read().await;
        if let Some(ref cb) = *callback {
            for _xpub in wallet_xpubs {
                cb(txid.clone(), status.clone());
            }
        }
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

        entry.status = TransactionStatus::NetworkRejected {
            reason: reason.clone(),
            timestamp: chrono::Utc::now().timestamp(),
        };
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
            if matches!(
                entry.status,
                TransactionStatus::Pending | TransactionStatus::Validated
            ) {
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
                TransactionStatus::Declined { .. } | TransactionStatus::NetworkRejected { .. } => {
                    stats.rejected += 1
                }
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
        manager
            .update_masternodes(vec![
                "192.168.1.1".to_string(),
                "192.168.1.2".to_string(),
                "192.168.1.3".to_string(),
            ])
            .await;

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
