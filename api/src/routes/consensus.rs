//! Consensus and finality endpoints

use crate::instant_finality_handlers::{get_finality_stats, get_transaction_status};
use crate::proposal_handlers::{create_proposal, get_proposal, list_proposals, vote_proposal};
use crate::{ApiResult, ApiState};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

/// Register consensus and proposal routes
pub fn consensus_routes() -> Router<ApiState> {
    Router::new()
        // Instant finality
        .route("/finality/stats", get(get_finality_stats))
        .route("/finality", get(get_finality_status))
        .route("/tx-status", post(get_transaction_status))
        // Consensus status
        .route("/status", get(get_consensus_status))
        .route("/voting", get(get_voting_status))
        // Grant proposals
        .route("/proposals/create", post(create_proposal))
        .route("/proposals/vote", post(vote_proposal))
        .route("/proposals/list", get(list_proposals))
        .route("/proposals/{id}", get(get_proposal))
}

#[derive(Serialize)]
struct ConsensusStatusResponse {
    active_validators: usize,
    current_round: u64,
    finalized_transactions: u64,
    pending_transactions: u64,
    consensus_health: String,
}

async fn get_consensus_status(
    State(state): State<ApiState>,
) -> ApiResult<Json<ConsensusStatusResponse>> {
    // Get active masternode count
    let blockchain = state.blockchain.read().await;
    let active_validators = blockchain.get_active_masternodes().len();

    // Get mempool stats
    let pending_transactions = if let Some(mempool) = &state.mempool {
        mempool.get_all_transactions().await.len() as u64
    } else {
        0
    };

    // Get finalized transaction count from database
    let finalized_transactions = blockchain.load_finalized_txs().unwrap_or_default().len() as u64;

    // Determine consensus health
    let consensus_health = if active_validators >= 3 {
        "healthy"
    } else if active_validators >= 1 {
        "degraded"
    } else {
        "offline"
    };

    Ok(Json(ConsensusStatusResponse {
        active_validators,
        current_round: 0, // TODO: Track actual consensus rounds
        finalized_transactions,
        pending_transactions,
        consensus_health: consensus_health.to_string(),
    }))
}

#[derive(Serialize)]
struct VotingStatusResponse {
    active_proposals: usize,
    total_votes: usize,
    eligible_voters: usize,
    current_quorum: f64,
}

async fn get_voting_status(State(state): State<ApiState>) -> ApiResult<Json<VotingStatusResponse>> {
    let blockchain = state.blockchain.read().await;
    let eligible_voters = blockchain.get_active_masternodes().len();

    // TODO: Integrate with actual governance system when available
    let active_proposals = 0;
    let total_votes = 0;

    // Calculate current quorum (2/3 of eligible voters)
    let current_quorum = if eligible_voters > 0 { 2.0 / 3.0 } else { 0.0 };

    Ok(Json(VotingStatusResponse {
        active_proposals,
        total_votes,
        eligible_voters,
        current_quorum,
    }))
}

#[derive(Serialize)]
struct FinalityStatusResponse {
    instant_finality_enabled: bool,
    finality_threshold: f64,
    average_finality_time: u64,
    finalized_today: u64,
}

async fn get_finality_status(
    State(state): State<ApiState>,
) -> ApiResult<Json<FinalityStatusResponse>> {
    let instant_finality_enabled = state.instant_finality.is_some();

    // Get finality statistics
    let blockchain = state.blockchain.read().await;
    let finalized_today = blockchain.load_finalized_txs().unwrap_or_default().len() as u64;

    Ok(Json(FinalityStatusResponse {
        instant_finality_enabled,
        finality_threshold: 0.67,    // 2/3 threshold
        average_finality_time: 2000, // TODO: Track actual finality times (ms)
        finalized_today,
    }))
}
