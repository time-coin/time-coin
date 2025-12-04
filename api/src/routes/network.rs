//! Network and peer management endpoints

use crate::quarantine_handlers::{get_quarantine_stats, get_quarantined_peers, release_peer};
use crate::tx_sync_handlers::{
    handle_transaction_rejection, receive_missing_transactions, request_missing_transactions,
};
use crate::{ApiError, ApiResult, ApiState};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
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
        // Peer management
        .route("/peer/add", post(add_peer))
        .route("/peer/remove", post(remove_peer))
        // Network status
        .route("/status", get(get_network_status))
        .route("/sync", get(get_sync_status))
        .route("/info", get(get_network_info))
        // Broadcasting
        .route("/broadcast", post(broadcast_to_network))
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

#[derive(Deserialize)]
struct AddPeerRequest {
    address: String,
}

async fn add_peer(
    State(state): State<ApiState>,
    Json(req): Json<AddPeerRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    log::info!(address = %req.address, "adding_peer_via_api");

    let addr: std::net::SocketAddr = req
        .address
        .parse()
        .map_err(|e| ApiError::BadRequest(format!("Invalid address: {}", e)))?;

    // Add to discovery
    let mut discovery = state.discovery.write().await;
    let peer = time_network::PeerInfo {
        address: addr,
        version: "unknown".to_string(),
        last_seen: chrono::Utc::now().timestamp() as u64,
        network: time_network::NetworkType::Mainnet,
        commit_date: None,
        commit_count: None,
        wallet_address: None,
        failed_connections: 0,
        successful_connections: 0,
        latency_ms: None,
    };
    discovery.add_peer(peer);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Peer added successfully",
        "address": req.address
    })))
}

#[derive(Deserialize)]
struct RemovePeerRequest {
    address: String,
}

async fn remove_peer(
    State(state): State<ApiState>,
    Json(req): Json<RemovePeerRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    log::info!(address = %req.address, "removing_peer_via_api");

    let addr: std::net::SocketAddr = req
        .address
        .parse()
        .map_err(|e| ApiError::BadRequest(format!("Invalid address: {}", e)))?;

    state.peer_manager.remove_connected_peer(&addr.ip()).await;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Peer removed successfully",
        "address": req.address
    })))
}

#[derive(Serialize)]
struct NetworkStatusResponse {
    connected_peers: usize,
    known_peers: usize,
    quarantined_peers: usize,
    network: String,
    syncing: bool,
}

async fn get_network_status(
    State(state): State<ApiState>,
) -> ApiResult<Json<NetworkStatusResponse>> {
    let connected_peers = state.peer_manager.get_connected_peers().await.len();
    let discovery = state.discovery.read().await;
    let known_peers = discovery.all_peers().len();

    let quarantined_peers = 0; // TODO: Add count method to quarantine

    // Check if we're syncing by comparing our height to peers
    let blockchain = state.blockchain.read().await;
    let _our_height = blockchain.chain_tip_height();
    drop(blockchain);

    let syncing = false; // TODO: Implement proper sync detection

    Ok(Json(NetworkStatusResponse {
        connected_peers,
        known_peers,
        quarantined_peers,
        network: state.network.clone(),
        syncing,
    }))
}

#[derive(Serialize)]
struct SyncStatusResponse {
    syncing: bool,
    current_height: u64,
    target_height: Option<u64>,
    progress: f64,
}

async fn get_sync_status(State(state): State<ApiState>) -> ApiResult<Json<SyncStatusResponse>> {
    let blockchain = state.blockchain.read().await;
    let current_height = blockchain.chain_tip_height();

    // TODO: Get target height from peers
    let target_height = None;
    let syncing = false;
    let progress = 100.0;

    Ok(Json(SyncStatusResponse {
        syncing,
        current_height,
        target_height,
        progress,
    }))
}

#[derive(Serialize)]
struct NetworkInfoResponse {
    network: String,
    version: String,
    protocol_version: u32,
    connected_peers: usize,
    inbound: usize,
    outbound: usize,
}

async fn get_network_info(State(state): State<ApiState>) -> ApiResult<Json<NetworkInfoResponse>> {
    let peers = state.peer_manager.get_connected_peers().await;
    let connected_peers = peers.len();

    // TODO: Track inbound vs outbound connections
    let inbound = 0;
    let outbound = connected_peers;

    Ok(Json(NetworkInfoResponse {
        network: state.network.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        protocol_version: 1,
        connected_peers,
        inbound,
        outbound,
    }))
}

#[derive(Deserialize)]
struct BroadcastRequest {
    #[allow(dead_code)]
    message: String,
    message_type: String,
}

async fn broadcast_to_network(
    State(state): State<ApiState>,
    Json(req): Json<BroadcastRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    log::info!(
        message_type = %req.message_type,
        "broadcasting_message_via_api"
    );

    let peers = state.peer_manager.get_connected_peers().await;
    let peer_count = peers.len();

    // TODO: Implement actual message broadcasting
    log::warn!("Message broadcasting not yet implemented");

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Message queued for broadcast",
        "peers": peer_count
    })))
}
