//! API handlers for instant finality transaction management

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use time_consensus::instant_finality::{TransactionStatus, TransactionVote};

use crate::{ApiResult, ApiState};

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitTransactionRequest {
    pub transaction: time_core::Transaction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitTransactionResponse {
    pub txid: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VoteTransactionRequest {
    pub vote: TransactionVote,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VoteTransactionResponse {
    pub txid: String,
    pub status: TransactionStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionStatusRequest {
    pub txid: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionStatusResponse {
    pub txid: String,
    pub status: Option<TransactionStatus>,
}

/// Submit a transaction for instant finality validation
pub async fn submit_transaction(
    State(state): State<ApiState>,
    Json(req): Json<SubmitTransactionRequest>,
) -> ApiResult<Json<SubmitTransactionResponse>> {
    let finality_manager = state.instant_finality_manager().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Instant finality not initialized".to_string(),
    ))?;

    let txid = finality_manager
        .submit_transaction(req.transaction)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    Ok(Json(SubmitTransactionResponse {
        txid: txid.clone(),
        status: "pending".to_string(),
    }))
}

/// Vote on a transaction
pub async fn vote_on_transaction(
    State(state): State<ApiState>,
    Json(req): Json<VoteTransactionRequest>,
) -> ApiResult<Json<VoteTransactionResponse>> {
    let finality_manager = state.instant_finality_manager().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Instant finality not initialized".to_string(),
    ))?;

    let status = finality_manager
        .record_vote(req.vote.clone())
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    Ok(Json(VoteTransactionResponse {
        txid: req.vote.txid,
        status,
    }))
}

/// Get transaction status
pub async fn get_transaction_status(
    State(state): State<ApiState>,
    Json(req): Json<TransactionStatusRequest>,
) -> ApiResult<Json<TransactionStatusResponse>> {
    let finality_manager = state.instant_finality_manager().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Instant finality not initialized".to_string(),
    ))?;

    let status = finality_manager.get_status(&req.txid).await;

    Ok(Json(TransactionStatusResponse {
        txid: req.txid,
        status,
    }))
}

/// Get approved transactions ready for block inclusion
pub async fn get_approved_transactions(
    State(state): State<ApiState>,
) -> ApiResult<Json<Vec<time_core::Transaction>>> {
    let finality_manager = state.instant_finality_manager().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Instant finality not initialized".to_string(),
    ))?;

    let transactions = finality_manager.get_approved_transactions().await;

    Ok(Json(transactions))
}

/// Get instant finality statistics
pub async fn get_finality_stats(
    State(state): State<ApiState>,
) -> ApiResult<Json<time_consensus::instant_finality::FinalityStats>> {
    let finality_manager = state.instant_finality_manager().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Instant finality not initialized".to_string(),
    ))?;

    let stats = finality_manager.get_stats().await;

    Ok(Json(stats))
}
