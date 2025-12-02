//! Treasury management and proposal endpoints

use crate::treasury_handlers::{
    approve_treasury_proposal, distribute_treasury_funds, get_proposal_by_id,
    get_treasury_allocations, get_treasury_stats, get_treasury_withdrawals, submit_proposal,
    vote_on_proposal,
};
use crate::ApiState;
use axum::{
    routing::{get, post},
    Router,
};

/// Register treasury management routes
pub fn treasury_routes() -> Router<ApiState> {
    Router::new()
        .route("/stats", get(get_treasury_stats))
        .route("/allocations", get(get_treasury_allocations))
        .route("/withdrawals", get(get_treasury_withdrawals))
        .route("/approve", post(approve_treasury_proposal))
        .route("/distribute", post(distribute_treasury_funds))
        .route("/propose", post(submit_proposal))
        .route("/proposal/{id}", post(get_proposal_by_id))
        .route("/vote", post(vote_on_proposal))
}
