//! Mempool management and transaction endpoints

use crate::{ApiError, ApiResult, ApiState};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use tracing as log;

/// Register mempool routes
pub fn mempool_routes() -> Router<ApiState> {
    Router::new()
        .route("/", get(get_mempool_status)) // Add root endpoint for dashboard
        .route("/status", get(get_mempool_status))
        .route("/add", post(add_to_mempool))
        .route("/finalized", post(receive_finalized_transaction))
        .route("/all", get(get_all_mempool_txs))
        .route("/clear", post(clear_mempool))
}

#[derive(Serialize)]
struct MempoolStatusResponse {
    size: usize,
    pending: usize, // Alias for dashboard compatibility
    transactions: Vec<String>,
}

async fn get_mempool_status(
    State(state): State<ApiState>,
) -> ApiResult<Json<MempoolStatusResponse>> {
    let mempool = state
        .mempool
        .as_ref()
        .ok_or(ApiError::Internal("Mempool not initialized".into()))?;

    let transactions = mempool.get_all_transactions().await;
    let tx_ids: Vec<String> = transactions.iter().map(|tx| tx.txid.clone()).collect();
    let count = tx_ids.len();

    Ok(Json(MempoolStatusResponse {
        size: count,
        pending: count, // Same value for dashboard compatibility
        transactions: tx_ids,
    }))
}

async fn add_to_mempool(
    State(state): State<ApiState>,
    Json(tx): Json<time_core::Transaction>,
) -> ApiResult<Json<serde_json::Value>> {
    let mempool = state
        .mempool
        .as_ref()
        .ok_or(ApiError::Internal("Mempool not initialized".into()))?;

    mempool
        .add_transaction(tx.clone())
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to add transaction: {}", e)))?;

    log::info!(txid = %tx.txid, "transaction_received_from_peer");

    // Trigger instant finality via BFT consensus
    trigger_instant_finality_for_received_tx(state.clone(), tx.clone()).await;

    // Re-broadcast to other peers
    if let Some(broadcaster) = state.tx_broadcaster.as_ref() {
        broadcaster.broadcast_transaction(tx).await;
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Transaction added to mempool and broadcast"
    })))
}

async fn receive_finalized_transaction(
    State(state): State<ApiState>,
    Json(tx): Json<time_core::Transaction>,
) -> ApiResult<Json<serde_json::Value>> {
    log::info!(
        txid = %tx.txid,
        "finalized_transaction_received_from_peer"
    );

    let mut blockchain = state.blockchain.write().await;

    if let Err(e) = blockchain.utxo_set_mut().apply_transaction(&tx) {
        log::error!(
            txid = %tx.txid,
            error = %e,
            "failed_to_apply_finalized_transaction"
        );
        return Err(ApiError::Internal(format!(
            "Failed to apply transaction: {}",
            e
        )));
    }

    log::info!(txid = %tx.txid, "finalized_transaction_applied_to_utxo_set");

    if let Err(e) = blockchain.save_finalized_tx(&tx, 1, 1) {
        log::warn!(
            txid = %tx.txid,
            error = %e,
            "failed_to_save_finalized_transaction"
        );
    }

    // Note: UTXO snapshot is saved at block creation time, not per-transaction
    // This keeps the live UTXO state in memory for instant finality

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Finalized transaction applied to UTXO set"
    })))
}

async fn get_all_mempool_txs(
    State(state): State<ApiState>,
) -> ApiResult<Json<Vec<time_core::Transaction>>> {
    let mempool = state
        .mempool
        .as_ref()
        .ok_or(ApiError::Internal("Mempool not initialized".into()))?;

    let transactions = mempool.get_all_transactions().await;
    Ok(Json(transactions))
}

async fn clear_mempool(State(state): State<ApiState>) -> ApiResult<Json<serde_json::Value>> {
    let mempool = state
        .mempool
        .as_ref()
        .ok_or(ApiError::Internal("Mempool not initialized".into()))?;

    mempool.clear().await;
    log::info!("mempool_cleared_via_api");

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Mempool cleared successfully"
    })))
}

// Helper function referenced in add_to_mempool
pub async fn trigger_instant_finality_for_received_tx(state: ApiState, tx: time_core::Transaction) {
    if let Some(handler) = &state.instant_finality {
        // Pass empty wallet_xpubs since we don't track them at API level
        let wallet_xpubs = Vec::new();

        let result = handler.submit_transaction(tx.clone(), wallet_xpubs).await;

        if let Err(e) = result {
            log::warn!(
                txid = %tx.txid,
                error = %e,
                "instant_finality_trigger_failed"
            );
        }
    }
}
