use axum::{
    extract::State,
    routing::get,
    Json, Router,
};
use crate::{ApiState, ApiError, ApiResult};

pub fn create_routes() -> Router<ApiState> {
    Router::new()
        .route("/", get(root))
        .route("/blockchain/info", get(get_blockchain_info))
        .route("/balance/:address", get(get_balance))
        .route("/peers", get(get_peers))
        .route("/genesis", get(get_genesis))
}

async fn root() -> &'static str {
    "TIME Coin API"
}

#[derive(serde::Serialize)]
struct BlockchainInfoResponse {
    network: String,
    height: u64,
    best_block_hash: String,
    total_supply: u64,
    timestamp: i64,
}

async fn get_blockchain_info(
    State(state): State<ApiState>,
) -> ApiResult<Json<BlockchainInfoResponse>> {
    let balances = state.balances.read().await;
    let total_supply: u64 = balances.values().sum();
    
    Ok(Json(BlockchainInfoResponse {
        network: state.network.clone(),
        height: 0,
        best_block_hash: "00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048"
            .to_string(),
        total_supply,
        timestamp: chrono::Utc::now().timestamp(),
    }))
}

#[derive(serde::Serialize)]
struct BalanceResponse {
    address: String,
    balance: u64,
}

async fn get_balance(
    State(state): State<ApiState>,
    axum::extract::Path(address): axum::extract::Path<String>,
) -> ApiResult<Json<BalanceResponse>> {
    let balances = state.balances.read().await;
    let balance = balances.get(&address).copied().unwrap_or(0);
    
    Ok(Json(BalanceResponse { address, balance }))
}

#[derive(serde::Serialize)]
struct PeerInfo {
    address: String,
    version: String,
    connected: bool,
}

#[derive(serde::Serialize)]
struct PeersResponse {
    peers: Vec<PeerInfo>,
    count: usize,
}

async fn get_peers(
    State(state): State<ApiState>,
) -> ApiResult<Json<PeersResponse>> {
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
    
    Ok(Json(PeersResponse {
        peers: peer_info,
        count,
    }))
}

async fn get_genesis(
    State(_state): State<ApiState>,
) -> ApiResult<Json<serde_json::Value>> {
    // Read genesis from file if it exists
    let genesis_path = std::env::var("GENESIS_PATH")
        .unwrap_or_else(|_| "/root/time-coin-node/config/genesis-testnet.json".to_string());
    
    match std::fs::read_to_string(&genesis_path) {
        Ok(contents) => {
            let genesis: serde_json::Value = serde_json::from_str(&contents)
                .map_err(|e| ApiError::Internal(format!("Failed to parse genesis: {}", e)))?;
            Ok(Json(genesis))
        }
        Err(_) => Err(ApiError::Internal("Genesis block not found".to_string()))
    }
}
