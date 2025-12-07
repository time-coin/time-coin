//! API handlers for instant finality transaction management

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use time_consensus::instant_finality::{TransactionDecision, TransactionStatus};
use time_core::utxo_state_manager::UTXOState;
use time_core::OutPoint;

use crate::{ApiResult, ApiState};

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitTransactionRequest {
    pub transaction: time_core::Transaction,
    pub wallet_xpubs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitTransactionResponse {
    pub txid: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DecisionRequest {
    pub decision: TransactionDecision,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DecisionResponse {
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
        .submit_transaction(req.transaction, req.wallet_xpubs)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    Ok(Json(SubmitTransactionResponse {
        txid: txid.clone(),
        status: "pending".to_string(),
    }))
}

/// Record a decision on a transaction (approve or decline)
pub async fn record_decision(
    State(state): State<ApiState>,
    Json(req): Json<DecisionRequest>,
) -> ApiResult<Json<DecisionResponse>> {
    let finality_manager = state.instant_finality_manager().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Instant finality not initialized".to_string(),
    ))?;

    let status = finality_manager
        .record_decision(req.decision.clone())
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    Ok(Json(DecisionResponse {
        txid: req.decision.txid,
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

// ═══════════════════════════════════════════════════════════════
// UTXO State Tracking Endpoints (New for Instant Finality)
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize)]
pub struct UTXOStateResponse {
    pub txid: String,
    pub vout: u32,
    pub state: String,
    pub details: Option<UTXOStateDetails>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UTXOStateDetails {
    pub spending_txid: Option<String>,
    pub votes: Option<usize>,
    pub total_nodes: Option<usize>,
    pub block_height: Option<u64>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionFinalityResponse {
    pub txid: String,
    pub is_finalized: bool,
    pub finality_status: String,
    pub votes: Option<usize>,
    pub required_votes: Option<usize>,
    pub finalized_at: Option<String>,
}

/// Get UTXO state for a specific output
/// GET /api/utxo/:txid/:vout/state
pub async fn get_utxo_state(
    State(state): State<ApiState>,
    Path((txid, vout)): Path<(String, u32)>,
) -> ApiResult<Json<UTXOStateResponse>> {
    let blockchain = state.blockchain.read().await;
    let utxo_manager = blockchain.utxo_state_manager();

    let outpoint = OutPoint {
        txid: txid.clone(),
        vout,
    };

    match utxo_manager.get_utxo_info(&outpoint).await {
        Some(info) => {
            let (state_str, details) = match &info.state {
                UTXOState::Unspent => ("unspent".to_string(), None),
                UTXOState::Locked { txid, locked_at } => (
                    "locked".to_string(),
                    Some(UTXOStateDetails {
                        spending_txid: Some(txid.clone()),
                        votes: None,
                        total_nodes: None,
                        block_height: None,
                        timestamp: Some(locked_at.to_string()),
                    }),
                ),
                UTXOState::SpentPending {
                    txid,
                    votes,
                    total_nodes,
                    spent_at,
                } => (
                    "spent_pending".to_string(),
                    Some(UTXOStateDetails {
                        spending_txid: Some(txid.clone()),
                        votes: Some(*votes),
                        total_nodes: Some(*total_nodes),
                        block_height: None,
                        timestamp: Some(spent_at.to_string()),
                    }),
                ),
                UTXOState::SpentFinalized {
                    txid,
                    votes,
                    finalized_at,
                } => (
                    "spent_finalized".to_string(),
                    Some(UTXOStateDetails {
                        spending_txid: Some(txid.clone()),
                        votes: Some(*votes),
                        total_nodes: None,
                        block_height: None,
                        timestamp: Some(finalized_at.to_string()),
                    }),
                ),
                UTXOState::Confirmed {
                    txid,
                    block_height,
                    confirmed_at,
                } => (
                    "confirmed".to_string(),
                    Some(UTXOStateDetails {
                        spending_txid: Some(txid.clone()),
                        votes: None,
                        total_nodes: None,
                        block_height: Some(*block_height),
                        timestamp: Some(confirmed_at.to_string()),
                    }),
                ),
            };

            Ok(Json(UTXOStateResponse {
                txid: outpoint.txid,
                vout: outpoint.vout,
                state: state_str,
                details,
            }))
        }
        None => Err(crate::ApiError::NotFound(format!(
            "UTXO {}:{} not found",
            txid, vout
        ))),
    }
}

/// Check if a transaction has reached instant finality
/// GET /api/transaction/:txid/finality
pub async fn check_transaction_finality(
    State(state): State<ApiState>,
    Path(txid): Path<String>,
) -> ApiResult<Json<TransactionFinalityResponse>> {
    // Check approval manager first
    if let Some(approval_manager) = &state.approval_manager {
        match approval_manager.check_approval_status(&txid).await {
            Ok(time_consensus::transaction_approval::TransactionStatus::Approved {
                approvals,
                required,
                approved_at,
            }) => {
                return Ok(Json(TransactionFinalityResponse {
                    txid,
                    is_finalized: true,
                    finality_status: "finalized".to_string(),
                    votes: Some(approvals),
                    required_votes: Some(required),
                    finalized_at: Some(approved_at.to_rfc3339()),
                }));
            }
            Ok(time_consensus::transaction_approval::TransactionStatus::Pending) => {
                // Check pending status via UTXO states
                let blockchain = state.blockchain.read().await;

                // Try to find transaction in mempool to check its inputs
                if let Some(mempool) = &state.mempool {
                    if let Some(tx) = mempool.get_transaction(&txid).await {
                        // Check first input's UTXO state
                        if let Some(input) = tx.inputs.first() {
                            if let Some(info) = blockchain
                                .utxo_state_manager()
                                .get_utxo_info(&input.previous_output)
                                .await
                            {
                                if let UTXOState::SpentPending {
                                    votes, total_nodes, ..
                                } = info.state
                                {
                                    return Ok(Json(TransactionFinalityResponse {
                                        txid,
                                        is_finalized: false,
                                        finality_status: "pending".to_string(),
                                        votes: Some(votes),
                                        required_votes: Some((total_nodes * 2) / 3 + 1),
                                        finalized_at: None,
                                    }));
                                }
                            }
                        }
                    }
                }

                return Ok(Json(TransactionFinalityResponse {
                    txid,
                    is_finalized: false,
                    finality_status: "pending".to_string(),
                    votes: None,
                    required_votes: None,
                    finalized_at: None,
                }));
            }
            Ok(time_consensus::transaction_approval::TransactionStatus::Declined {
                reason,
                ..
            }) => {
                return Ok(Json(TransactionFinalityResponse {
                    txid,
                    is_finalized: false,
                    finality_status: format!("declined: {}", reason),
                    votes: None,
                    required_votes: None,
                    finalized_at: None,
                }));
            }
            Err(_) => {
                // Not found in approval manager, check blockchain
            }
        }
    }

    // Not found
    Err(crate::ApiError::TransactionNotFound(txid))
}
