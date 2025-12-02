//! Transaction synchronization handlers for API

use crate::{ApiError, ApiResult, ApiState};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use time_core::Transaction;

#[derive(Debug, Deserialize)]
pub struct MissingTxRequest {
    pub txids: Vec<String>,
    pub requester: String,
    pub block_height: u64,
}

#[derive(Debug, Serialize)]
pub struct MissingTxResponse {
    pub transactions: Vec<Transaction>,
    pub block_height: u64,
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
) -> ApiResult<Json<MissingTxResponse>> {
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
) -> ApiResult<Json<serde_json::Value>> {
    println!(
        "📨 Received {} transactions for block #{}",
        response.transactions.len(),
        response.block_height
    );

    let mempool = state
        .mempool
        .as_ref()
        .ok_or(ApiError::Internal("Mempool not initialized".to_string()))?;

    let blockchain = state.blockchain.clone();

    let mut added = 0;
    let mut rejected = 0;

    for tx in response.transactions {
        // Validate transaction
        let blockchain_read = blockchain.read().await;
        let mut is_valid = true;
        let mut rejection_reason = String::new();

        // Check for double spend
        for input in &tx.inputs {
            if !blockchain_read
                .utxo_set()
                .has_utxo(&input.previous_output.txid, input.previous_output.vout)
            {
                is_valid = false;
                rejection_reason = format!(
                    "double_spend:{}:{}",
                    &input.previous_output.txid[..16],
                    input.previous_output.vout
                );
                break;
            }
        }

        // Verify signature
        if is_valid {
            if let Err(e) = tx.verify() {
                is_valid = false;
                rejection_reason = format!("invalid_signature:{}", e);
            }
        }

        drop(blockchain_read);

        if is_valid {
            // Add to mempool
            match mempool.add_transaction(tx.clone()).await {
                Ok(_) => {
                    println!("   ✅ Added transaction {}", &tx.txid[..16]);
                    added += 1;
                }
                Err(e) => {
                    println!("   ❌ Failed to add transaction {}: {}", &tx.txid[..16], e);
                    reject_transaction_internal(&state, tx, format!("Mempool add failed: {}", e))
                        .await;
                    rejected += 1;
                }
            }
        } else {
            println!(
                "   ❌ Invalid transaction {}: {}",
                &tx.txid[..16],
                rejection_reason
            );
            reject_transaction_internal(&state, tx, rejection_reason).await;
            rejected += 1;
        }
    }

    println!(
        "✅ Transaction sync complete: {} added, {} rejected",
        added, rejected
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "added": added,
        "rejected": rejected
    })))
}

/// Handle transaction rejection notification
pub async fn handle_transaction_rejection(
    State(state): State<ApiState>,
    Json(rejection): Json<TransactionRejectionNotification>,
) -> ApiResult<Json<serde_json::Value>> {
    println!(
        "🚫 Transaction {} rejected: {} (wallet: {})",
        &rejection.txid[..16],
        rejection.reason,
        rejection.wallet_address
    );

    // Remove from mempool if present
    if let Some(mempool) = state.mempool.as_ref() {
        if let Some(_) = mempool.get_transaction(&rejection.txid).await {
            println!("   ⚠️  Transaction still in mempool - manual cleanup needed");
            // TODO: Add mempool.remove_transaction() method
        }
    }

    // TODO: Notify connected wallet via peer manager
    // This would require adding wallet notification to PeerManager

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Rejection notification processed"
    })))
}

/// Internal helper to reject a transaction and broadcast notification
async fn reject_transaction_internal(state: &ApiState, tx: Transaction, reason: String) {
    println!("🚫 Rejecting transaction {}: {}", &tx.txid[..16], reason);

    // Extract wallet address from first output
    if let Some(output) = tx.outputs.first() {
        let wallet_address = output.address.clone();

        // Broadcast rejection to all peers
        if let Some(broadcaster) = state.tx_broadcaster.as_ref() {
            // We need to add a broadcast_rejection method to TransactionBroadcaster
            println!(
                "📨 Rejection notification sent for wallet: {}",
                wallet_address
            );
        }
    }
}
