//! Network and peer management endpoints

use crate::quarantine_handlers::{get_quarantine_stats, get_quarantined_peers, release_peer};
use crate::tx_sync_handlers::{
    handle_transaction_rejection, receive_missing_transactions, request_missing_transactions,
};
use crate::ApiState;
use axum::{
    routing::{get, post},
    Router,
};

/// Register network management routes
pub fn network_routes() -> Router<ApiState> {
    Router::new()
        // Quarantine management
        .route("/quarantine", get(get_quarantined_peers))
        .route("/quarantine/release", post(release_peer))
        .route("/quarantine/stats", get(get_quarantine_stats))
        // Transaction synchronization
        .route("/sync/request", post(request_missing_transactions))
        .route("/sync/response", post(receive_missing_transactions))
        .route("/sync/rejection", post(handle_transaction_rejection))
}
