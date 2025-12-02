//! Consensus and finality endpoints

use crate::instant_finality_handlers::{get_finality_stats, get_transaction_status};
use crate::proposal_handlers::{create_proposal, get_proposal, list_proposals, vote_proposal};
use crate::ApiState;
use axum::{
    routing::{get, post},
    Router,
};

/// Register consensus and proposal routes
pub fn consensus_routes() -> Router<ApiState> {
    Router::new()
        // Instant finality
        .route("/finality/stats", get(get_finality_stats))
        .route("/tx-status", post(get_transaction_status))
        // Grant proposals
        .route("/proposals/create", post(create_proposal))
        .route("/proposals/vote", post(vote_proposal))
        .route("/proposals/list", get(list_proposals))
        .route("/proposals/:id", get(get_proposal))
}
