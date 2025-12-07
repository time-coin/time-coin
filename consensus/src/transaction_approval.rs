// Transaction Approval System for TIME Coin
//
// Masternodes approve or decline transactions instead of voting.
// Each masternode makes an independent decision based on validation rules.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use time_core::utxo_state_manager::UTXOStateManager;
use time_core::Transaction;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApprovalDecision {
    Approved,
    Declined { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionApproval {
    pub txid: String,
    pub masternode: String,
    pub decision: ApprovalDecision,
    pub timestamp: DateTime<Utc>,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Approved {
        approvals: usize,
        required: usize,
        approved_at: DateTime<Utc>,
    },
    Declined {
        reason: String,
        declined_at: DateTime<Utc>,
    },
}

#[derive(Debug, Clone)]
struct PendingTransaction {
    approvals: HashMap<String, TransactionApproval>,
    created_at: DateTime<Utc>,
    transaction: Option<Transaction>, // Store transaction for UTXO state updates
}

pub struct TransactionApprovalManager {
    /// Pending transactions awaiting approval
    pending: Arc<RwLock<HashMap<String, PendingTransaction>>>,

    /// Finalized transaction statuses (cached for quick lookup)
    finalized: Arc<RwLock<HashMap<String, TransactionStatus>>>,

    /// List of active masternodes
    masternodes: Arc<RwLock<Vec<String>>>,

    /// Approval threshold (default 2/3)
    threshold: Arc<RwLock<f64>>,

    /// UTXO State Manager for instant finality
    utxo_state_manager: Option<Arc<UTXOStateManager>>,
}

impl TransactionApprovalManager {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(RwLock::new(HashMap::new())),
            finalized: Arc::new(RwLock::new(HashMap::new())),
            masternodes: Arc::new(RwLock::new(Vec::new())),
            threshold: Arc::new(RwLock::new(2.0 / 3.0)),
            utxo_state_manager: None,
        }
    }

    /// Set the UTXO state manager for instant finality
    pub fn set_utxo_state_manager(&mut self, manager: Arc<UTXOStateManager>) {
        self.utxo_state_manager = Some(manager);
    }

    /// Add a transaction to the approval queue (with UTXO state tracking)
    pub async fn add_transaction_with_utxos(&self, tx: &Transaction) -> Result<(), String> {
        let txid = tx.txid.clone();

        let mut pending = self.pending.write().await;

        if pending.contains_key(&txid) {
            return Err("Transaction already in approval queue".to_string());
        }

        let pending_tx = PendingTransaction {
            approvals: HashMap::new(),
            created_at: Utc::now(),
            transaction: Some(tx.clone()), // Store for UTXO updates
        };

        pending.insert(txid.clone(), pending_tx);
        drop(pending); // Release lock before async call

        // Mark UTXOs as SpentPending (if UTXO manager available)
        if let Some(manager) = &self.utxo_state_manager {
            let masternodes = self.masternodes.read().await;
            let total_nodes = masternodes.len();
            drop(masternodes);

            for input in &tx.inputs {
                let _ = manager
                    .mark_spent_pending(&input.previous_output, txid.clone(), 0, total_nodes)
                    .await;
            }
        }

        Ok(())
    }

    /// Add a transaction to the approval queue (legacy - no UTXO tracking)
    pub async fn add_transaction(&self, txid: String) -> Result<(), String> {
        let mut pending = self.pending.write().await;

        if pending.contains_key(&txid) {
            return Err("Transaction already in approval queue".to_string());
        }

        let tx = PendingTransaction {
            approvals: HashMap::new(),
            created_at: Utc::now(),
            transaction: None, // No transaction stored for legacy
        };

        pending.insert(txid, tx);
        Ok(())
    }

    /// Record a masternode's approval decision
    pub async fn record_approval(
        &self,
        approval: TransactionApproval,
    ) -> Result<TransactionStatus, String> {
        let txid = approval.txid.clone();
        let masternode = approval.masternode.clone();

        // Verify masternode is authorized
        {
            let masternodes = self.masternodes.read().await;
            if !masternodes.contains(&masternode) {
                return Err(format!("Unauthorized masternode: {}", masternode));
            }
        }

        let mut pending = self.pending.write().await;
        let tx = pending
            .get_mut(&txid)
            .ok_or_else(|| format!("Transaction {} not found in pending queue", txid))?;

        // Check for duplicate approval
        if tx.approvals.contains_key(&masternode) {
            return Err(format!(
                "Masternode {} already approved transaction {}",
                masternode, txid
            ));
        }

        // Record the approval
        tx.approvals.insert(masternode, approval.clone());

        // Get current approval count and transaction for UTXO state update
        let approval_count = tx.approvals.len();
        let transaction = tx.transaction.clone();
        let total_masternodes = {
            let masternodes = self.masternodes.read().await;
            masternodes.len()
        };

        // Update UTXO state with vote count (SpentPending with updated votes)
        if let (Some(manager), Some(ref trans)) = (&self.utxo_state_manager, &transaction) {
            for input in &trans.inputs {
                let _ = manager
                    .mark_spent_pending(
                        &input.previous_output,
                        txid.clone(),
                        approval_count,
                        total_masternodes,
                    )
                    .await;
            }
        }

        // Check if we've reached threshold
        let status = self.check_approval_status(&txid).await?;

        // If finalized (approved or declined), move to finalized cache
        match &status {
            TransactionStatus::Approved { approvals, .. } => {
                // **⚡ INSTANT FINALITY**: Mark UTXOs as SpentFinalized!
                if let (Some(manager), Some(ref trans)) = (&self.utxo_state_manager, &transaction) {
                    for input in &trans.inputs {
                        let _ = manager
                            .mark_spent_finalized(&input.previous_output, txid.clone(), *approvals)
                            .await;
                    }
                }

                let _tx = pending.remove(&txid).ok_or("Transaction disappeared")?;
                drop(pending);

                let mut finalized = self.finalized.write().await;
                finalized.insert(txid.clone(), status.clone());

                // Cleanup old finalized transactions (keep last hour only)
                let cutoff = Utc::now() - chrono::Duration::hours(1);
                finalized.retain(|_, s| match s {
                    TransactionStatus::Approved { approved_at, .. } => *approved_at > cutoff,
                    TransactionStatus::Declined { declined_at, .. } => *declined_at > cutoff,
                    _ => false,
                });
            }
            TransactionStatus::Declined { .. } => {
                let _tx = pending.remove(&txid).ok_or("Transaction disappeared")?;
                drop(pending);

                let mut finalized = self.finalized.write().await;
                finalized.insert(txid.clone(), status.clone());

                // Cleanup old finalized transactions (keep last hour only)
                let cutoff = Utc::now() - chrono::Duration::hours(1);
                finalized.retain(|_, s| match s {
                    TransactionStatus::Approved { approved_at, .. } => *approved_at > cutoff,
                    TransactionStatus::Declined { declined_at, .. } => *declined_at > cutoff,
                    _ => false,
                });
            }
            TransactionStatus::Pending => {}
        }

        Ok(status)
    }

    /// Check the approval status of a transaction
    pub async fn check_approval_status(&self, txid: &str) -> Result<TransactionStatus, String> {
        // Check finalized cache first
        {
            let finalized = self.finalized.read().await;
            if let Some(status) = finalized.get(txid) {
                return Ok(status.clone());
            }
        }

        // Check pending transactions
        let pending = self.pending.read().await;
        let tx = pending
            .get(txid)
            .ok_or_else(|| format!("Transaction {} not found", txid))?;

        let masternodes = self.masternodes.read().await;
        let total_masternodes = masternodes.len();

        if total_masternodes == 0 {
            return Ok(TransactionStatus::Pending);
        }

        let threshold = *self.threshold.read().await;
        let required_approvals = ((total_masternodes as f64 * threshold).ceil()) as usize;

        // Count approvals and declines
        let mut approval_count = 0;
        let mut decline_reasons = Vec::new();

        for approval in tx.approvals.values() {
            match &approval.decision {
                ApprovalDecision::Approved => approval_count += 1,
                ApprovalDecision::Declined { reason } => {
                    decline_reasons.push(reason.clone());
                }
            }
        }

        // Check if approved
        if approval_count >= required_approvals {
            return Ok(TransactionStatus::Approved {
                approvals: approval_count,
                required: required_approvals,
                approved_at: Utc::now(),
            });
        }

        // Check if any masternode declined (instant rejection)
        if !decline_reasons.is_empty() {
            return Ok(TransactionStatus::Declined {
                reason: decline_reasons.join("; "),
                declined_at: Utc::now(),
            });
        }

        // Still pending
        Ok(TransactionStatus::Pending)
    }

    /// Get transaction status (public API)
    pub async fn get_status(&self, txid: &str) -> Result<TransactionStatus, String> {
        self.check_approval_status(txid).await
    }

    /// Get all pending transactions
    pub async fn get_pending_transactions(&self) -> Vec<String> {
        let pending = self.pending.read().await;
        pending.keys().cloned().collect()
    }

    /// Set the list of masternodes
    pub async fn set_masternodes(&self, masternodes: Vec<String>) {
        let mut mn = self.masternodes.write().await;
        *mn = masternodes;
    }

    /// Get approval statistics for a transaction
    pub async fn get_approval_stats(&self, txid: &str) -> Option<(usize, usize, usize)> {
        let pending = self.pending.read().await;
        let tx = pending.get(txid)?;

        let masternodes = self.masternodes.read().await;
        let total = masternodes.len();
        let threshold = *self.threshold.read().await;
        let required = ((total as f64 * threshold).ceil()) as usize;

        let approvals = tx
            .approvals
            .values()
            .filter(|a| matches!(a.decision, ApprovalDecision::Approved))
            .count();

        Some((approvals, required, total))
    }

    /// Cleanup expired pending transactions
    pub async fn cleanup_expired(&self) {
        let mut pending = self.pending.write().await;
        let cutoff = Utc::now() - chrono::Duration::minutes(30);

        pending.retain(|_, tx| tx.created_at > cutoff);
    }

    /// Get overall statistics
    pub async fn get_stats(&self) -> ApprovalStats {
        let pending = self.pending.read().await;
        let finalized = self.finalized.read().await;
        let masternodes = self.masternodes.read().await;

        ApprovalStats {
            pending_count: pending.len(),
            finalized_count: finalized.len(),
            active_masternodes: masternodes.len(),
            threshold: *self.threshold.read().await,
        }
    }
}

impl Default for TransactionApprovalManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize)]
pub struct ApprovalStats {
    pub pending_count: usize,
    pub finalized_count: usize,
    pub active_masternodes: usize,
    pub threshold: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transaction_approval_reaches_threshold() {
        let manager = TransactionApprovalManager::new();

        manager
            .set_masternodes(vec![
                "192.168.1.1".to_string(),
                "192.168.1.2".to_string(),
                "192.168.1.3".to_string(),
            ])
            .await;

        let txid = "test_tx_001".to_string();
        manager.add_transaction(txid.clone()).await.unwrap();

        // First approval
        let approval1 = TransactionApproval {
            txid: txid.clone(),
            masternode: "192.168.1.1".to_string(),
            decision: ApprovalDecision::Approved,
            timestamp: Utc::now(),
            signature: "sig1".to_string(),
        };
        manager.record_approval(approval1).await.unwrap();

        // Check status - should still be pending (need 2/3 = 2 approvals)
        let status = manager.get_status(&txid).await.unwrap();
        assert!(matches!(status, TransactionStatus::Pending));

        // Second approval - should reach threshold
        let approval2 = TransactionApproval {
            txid: txid.clone(),
            masternode: "192.168.1.2".to_string(),
            decision: ApprovalDecision::Approved,
            timestamp: Utc::now(),
            signature: "sig2".to_string(),
        };
        let status = manager.record_approval(approval2).await.unwrap();

        assert!(matches!(status, TransactionStatus::Approved { .. }));
    }

    #[tokio::test]
    async fn test_instant_decline() {
        let manager = TransactionApprovalManager::new();

        manager
            .set_masternodes(vec!["192.168.1.1".to_string(), "192.168.1.2".to_string()])
            .await;

        let txid = "test_tx_002".to_string();
        manager.add_transaction(txid.clone()).await.unwrap();

        // One masternode declines
        let decline = TransactionApproval {
            txid: txid.clone(),
            masternode: "192.168.1.1".to_string(),
            decision: ApprovalDecision::Declined {
                reason: "Invalid signature".to_string(),
            },
            timestamp: Utc::now(),
            signature: "sig1".to_string(),
        };

        let status = manager.record_approval(decline).await.unwrap();

        // Should be instantly declined
        assert!(matches!(status, TransactionStatus::Declined { .. }));
    }

    #[tokio::test]
    async fn test_duplicate_approval_rejected() {
        let manager = TransactionApprovalManager::new();

        manager
            .set_masternodes(vec!["192.168.1.1".to_string()])
            .await;

        let txid = "test_tx_003".to_string();
        manager.add_transaction(txid.clone()).await.unwrap();

        let approval = TransactionApproval {
            txid: txid.clone(),
            masternode: "192.168.1.1".to_string(),
            decision: ApprovalDecision::Approved,
            timestamp: Utc::now(),
            signature: "sig1".to_string(),
        };

        // First approval should succeed
        manager.record_approval(approval.clone()).await.unwrap();

        // Duplicate should fail
        let result = manager.record_approval(approval).await;
        assert!(result.is_err());
    }
}
