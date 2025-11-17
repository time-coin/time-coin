//! Transaction validation during consensus
//!
//! Handles validation of transactions during block voting and notifies
//! affected parties when transactions are invalid.

use time_core::{Transaction, TransactionError};

/// Reason why a transaction was invalidated
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum InvalidationReason {
    DoubleSpend,
    InvalidSignature,
    InsufficientFunds,
    InvalidInput,
    Other(String),
}

impl From<TransactionError> for InvalidationReason {
    fn from(err: TransactionError) -> Self {
        match err {
            TransactionError::DuplicateInput => InvalidationReason::DoubleSpend,
            TransactionError::InvalidSignature => InvalidationReason::InvalidSignature,
            TransactionError::InsufficientFunds => InvalidationReason::InsufficientFunds,
            TransactionError::InvalidInput => InvalidationReason::InvalidInput,
            _ => InvalidationReason::Other(err.to_string()),
        }
    }
}

/// Event data for transaction invalidation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TxInvalidationEvent {
    pub txid: String,
    pub reason: InvalidationReason,
    pub affected_addresses: Vec<String>,
    pub timestamp: i64,
    pub source_node: String,
}

/// Result of transaction validation during consensus
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub txid: String,
    pub is_valid: bool,
    pub error: Option<TransactionError>,
    pub affected_addresses: Vec<String>,
}

/// Transaction validator for consensus
pub struct ConsensusValidator {
    /// Node ID for tracking validation source
    node_id: String,
}

impl ConsensusValidator {
    pub fn new(node_id: String) -> Self {
        Self { node_id }
    }

    /// Validate a transaction during block voting
    /// Returns validation result with affected addresses for notification
    pub async fn validate_transaction(
        &self,
        tx: &Transaction,
        mempool: &time_mempool::Mempool,
    ) -> ValidationResult {
        let txid = tx.txid.clone();
        let affected_addresses: Vec<String> = tx
            .outputs
            .iter()
            .map(|output| output.address.clone())
            .collect();

        match mempool.validate_transaction_detailed(tx).await {
            Ok(()) => ValidationResult {
                txid,
                is_valid: true,
                error: None,
                affected_addresses,
            },
            Err(err) => {
                let error = match err {
                    time_mempool::MempoolError::DoubleSpend => {
                        Some(TransactionError::DuplicateInput)
                    }
                    time_mempool::MempoolError::InvalidTransaction(tx_err) => Some(tx_err),
                    _ => Some(TransactionError::InvalidInput),
                };

                ValidationResult {
                    txid,
                    is_valid: false,
                    error,
                    affected_addresses,
                }
            }
        }
    }

    /// Create invalidation event from validation result
    /// This can be sent to a notification system
    pub fn create_invalidation_event(
        &self,
        validation_result: ValidationResult,
    ) -> Option<TxInvalidationEvent> {
        if let Some(error) = validation_result.error {
            Some(TxInvalidationEvent {
                txid: validation_result.txid,
                reason: error.into(),
                affected_addresses: validation_result.affected_addresses,
                timestamp: chrono::Utc::now().timestamp(),
                source_node: self.node_id.clone(),
            })
        } else {
            None
        }
    }

    /// Broadcast missing transaction to other nodes
    /// Called when a node has a transaction others don't have
    pub async fn broadcast_missing_transaction(
        &self,
        tx: &Transaction,
        peer_nodes: &[String],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();

        for peer in peer_nodes {
            let url = format!("http://{}:24101/api/v1/transactions", peer);

            match client
                .post(&url)
                .json(tx)
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {
                    log::info!("✓ Broadcast transaction {} to {}", tx.txid, peer);
                }
                Ok(response) => {
                    log::warn!(
                        "⚠️  Failed to broadcast transaction {} to {}: HTTP {}",
                        tx.txid,
                        peer,
                        response.status()
                    );
                }
                Err(e) => {
                    log::warn!(
                        "⚠️  Failed to broadcast transaction {} to {}: {}",
                        tx.txid,
                        peer,
                        e
                    );
                }
            }
        }

        Ok(())
    }

    /// Request missing transaction from other nodes
    /// Called when this node is missing a transaction that others have
    pub async fn request_missing_transaction(
        &self,
        txid: &str,
        peer_nodes: &[String],
    ) -> Option<Transaction> {
        let client = reqwest::Client::new();

        for peer in peer_nodes {
            let url = format!("http://{}:24101/api/v1/transactions/{}", peer, txid);

            match client
                .get(&url)
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {
                    if let Ok(tx) = response.json::<Transaction>().await {
                        log::info!("✓ Retrieved transaction {} from {}", txid, peer);
                        return Some(tx);
                    }
                }
                _ => continue,
            }
        }

        log::warn!("⚠️  Could not retrieve transaction {} from any peer", txid);
        None
    }
}

/// Compare two block proposals and identify transaction differences
pub fn find_transaction_differences(
    local_txs: &[Transaction],
    remote_txs: &[Transaction],
) -> (Vec<String>, Vec<String>) {
    let local_txids: std::collections::HashSet<_> =
        local_txs.iter().map(|tx| tx.txid.as_str()).collect();
    let remote_txids: std::collections::HashSet<_> =
        remote_txs.iter().map(|tx| tx.txid.as_str()).collect();

    // Transactions in local but not in remote
    let missing_in_remote: Vec<String> = local_txids
        .difference(&remote_txids)
        .map(|&s| s.to_string())
        .collect();

    // Transactions in remote but not in local
    let missing_in_local: Vec<String> = remote_txids
        .difference(&local_txids)
        .map(|&s| s.to_string())
        .collect();

    (missing_in_local, missing_in_remote)
}
