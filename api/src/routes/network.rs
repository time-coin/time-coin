//! Network and peer management endpoints

use crate::quarantine_handlers::{get_quarantine_stats, get_quarantined_peers, release_peer};
use crate::tx_sync_handlers::{
    handle_transaction_rejection, receive_missing_transactions, request_missing_transactions,
};
use crate::{ApiResult, ApiState};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use tracing as log;

#[derive(Serialize)]
pub struct PeerInfo {
    address: String,
    version: String,
    connected: bool,
}

#[derive(Serialize)]
pub struct PeersResponse {
    peers: Vec<PeerInfo>,
    count: usize,
}

/// Register network management routes
pub fn network_routes() -> Router<ApiState> {
    Router::new()
        // Peer list endpoint
        .route("/peers", get(get_peers))
        // Quarantine management
        .route("/quarantine", get(get_quarantined_peers))
        .route("/quarantine/release", post(release_peer))
        .route("/quarantine/stats", get(get_quarantine_stats))
        // Transaction synchronization
        .route("/sync/request", post(request_missing_transactions))
        .route("/sync/response", post(receive_missing_transactions))
        .route("/sync/rejection", post(handle_transaction_rejection))
}

async fn get_peers(State(state): State<ApiState>) -> ApiResult<Json<PeersResponse>> {
    log::info!("fetching_connected_peers");

    let peers = state.peer_manager.get_connected_peers().await;

    let peer_info: Vec<PeerInfo> = peers
        .iter()
        .map(|p| PeerInfo {
            address: p.address.to_string(),
            version: p.version.clone(),
            connected: true,
        })
        .collect();

    let count = peer_info.len();
    log::info!(count = count, "peers_fetched");

    Ok(Json(PeersResponse {
        peers: peer_info,
        count,
    }))
}

/// Legacy endpoint for backward compatibility with CLI
pub async fn get_peers_legacy(state: State<ApiState>) -> ApiResult<Json<PeersResponse>> {
    get_peers(state).await
}
