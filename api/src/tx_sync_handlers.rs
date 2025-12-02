//! Transaction synchronization handlers for API

use crate::{ApiError, ApiState};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use time_core::Transaction;
use tracing as log;

#[derive(Debug, Deserialize)]
pub struct MissingTxRequest {
    pub txids: Vec<String>,
    pub requester: String,
    pub block_height: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingTxResponse {
    pub transactions: Vec<Transaction>,
    pub block_height: u64,
}

#[derive(Debug, Serialize)]
pub struct TxSyncResult {
    pub success: bool,
    pub added: usize,
    pub rejected: usize,
}

#[derive(Debug, Serialize)]
pub struct RejectionResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct TransactionRejectionNotification {
    pub txid: String,
    pub reason: String,
    pub wallet_address: String,
}

/// Handle request for missing transactions
pub async fn request_missing_transactions(
    State(state): State<ApiState>,
    Json(request): Json<MissingTxRequest>,
) -> Result<Json<MissingTxResponse>, ApiError> {
    log::info!(
        txid_count = request.txids.len(),
        requester = %request.requester,
        block_height = request.block_height,
        "missing_transactions_requested"
    );

    let mempool = state
        .mempool
        .as_ref()
        .ok_or(ApiError::Internal("Mempool not initialized".to_string()))?;

    let mut transactions = Vec::new();

    // Collect requested transactions from mempool
    for txid in &request.txids {
        if let Some(tx) = mempool.get_transaction(txid).await {
            transactions.push(tx);
        } else {
            log::warn!(txid = %&txid[..16], "transaction_not_found_in_mempool");
        }
    }

    if transactions.is_empty() {
        return Err(ApiError::NotFound(
            "No matching transactions found".to_string(),
        ));
    }

    log::info!(count = transactions.len(), "sending_transactions");

    Ok(Json(MissingTxResponse {
        transactions,
        block_height: request.block_height,
    }))
}

/// Handle incoming missing transactions (to add to mempool)
pub async fn receive_missing_transactions(
    State(state): State<ApiState>,
    Json(response): Json<MissingTxResponse>,
) -> Result<Json<TxSyncResult>, ApiError> {
    log::info!(
        transaction_count = response.transactions.len(),
        block_height = response.block_height,
        "received_missing_transactions"
    );

    let mempool = state
        .mempool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Mempool not initialized".to_string()))?;

    let mut added = 0;
    let mut rejected = 0;

    for tx in response.transactions {
        // Simply add to mempool - validation happens there
        match mempool.add_transaction(tx.clone()).await {
            Ok(_) => {
                log::debug!(txid = %&tx.txid[..16], "transaction_added");
                added += 1;
            }
            Err(e) => {
                log::warn!(txid = %&tx.txid[..16], error = %e, "transaction_rejected");
                rejected += 1;
            }
        }
    }

    log::info!(
        added = added,
        rejected = rejected,
        "transaction_sync_complete"
    );

    Ok(Json(TxSyncResult {
        success: true,
        added,
        rejected,
    }))
}

/// Handle transaction rejection notification
pub async fn handle_transaction_rejection(
    State(state): State<ApiState>,
    Json(rejection): Json<TransactionRejectionNotification>,
) -> Result<Json<RejectionResult>, ApiError> {
    log::warn!(
        txid = %&rejection.txid[..16],
        reason = %rejection.reason,
        wallet_address = %rejection.wallet_address,
        "transaction_rejected"
    );

    // Remove from mempool if present
    if let Some(mempool) = state.mempool.as_ref() {
        if mempool.remove_transaction(&rejection.txid).await.is_some() {
            log::info!(txid = %rejection.txid, "removed_rejected_transaction_from_mempool");
        }
    }

    // Wallet notification happens automatically via the wallet_sync notification system

    Ok(Json(RejectionResult {
        success: true,
        message: "Rejection notification processed".to_string(),
    }))
}

/// Internal helper to reject a transaction and broadcast notification
#[allow(dead_code)]
async fn reject_transaction_internal(state: &ApiState, tx: Transaction, reason: String) {
    log::warn!(txid = %&tx.txid[..16], reason = %reason, "rejecting_transaction");

    // Extract wallet address from first output
    if let Some(output) = tx.outputs.first() {
        let wallet_address = output.address.clone();

        // Broadcast rejection to all peers
        if let Some(_broadcaster) = state.tx_broadcaster.as_ref() {
            // We need to add a broadcast_rejection method to TransactionBroadcaster
            log::debug!(
                wallet_address = %wallet_address,
                "rejection_notification_sent"
            );
        }
    }
}
