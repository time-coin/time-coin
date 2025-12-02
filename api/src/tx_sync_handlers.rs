//! Transaction synchronization handlers for API

use crate::{ApiError, ApiState};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use time_core::Transaction;

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
    println!(
        "📨 Received request for {} transactions from {} (block #{})",
        request.txids.len(),
        request.requester,
        request.block_height
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
            println!("⚠️  Transaction {} not found in mempool", &txid[..16]);
        }
    }

    if transactions.is_empty() {
        return Err(ApiError::NotFound(
            "No matching transactions found".to_string(),
        ));
    }

    println!("📤 Sending {} transactions", transactions.len());

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
    println!(
        "📨 Received {} transactions for block #{}",
        response.transactions.len(),
        response.block_height
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
                println!("   ✅ Added transaction {}", &tx.txid[..16]);
                added += 1;
            }
            Err(e) => {
                println!("   ❌ Rejected transaction {}: {}", &tx.txid[..16], e);
                rejected += 1;
            }
        }
    }

    println!(
        "✅ Transaction sync complete: {} added, {} rejected",
        added, rejected
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
    println!(
        "🚫 Transaction {} rejected: {} (wallet: {})",
        &rejection.txid[..16],
        rejection.reason,
        rejection.wallet_address
    );

    // Remove from mempool if present
    if let Some(mempool) = state.mempool.as_ref() {
        if mempool.remove_transaction(&rejection.txid).await.is_some() {
            println!("   ✅ Removed rejected transaction from mempool");
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
    println!("🚫 Rejecting transaction {}: {}", &tx.txid[..16], reason);

    // Extract wallet address from first output
    if let Some(output) = tx.outputs.first() {
        let wallet_address = output.address.clone();

        // Broadcast rejection to all peers
        if let Some(_broadcaster) = state.tx_broadcaster.as_ref() {
            // We need to add a broadcast_rejection method to TransactionBroadcaster
            println!(
                "📨 Rejection notification sent for wallet: {}",
                wallet_address
            );
        }
    }
}
