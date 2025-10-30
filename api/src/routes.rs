use axum::extract::Path;
use serde::Serialize;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use crate::{ApiState, ApiError, ApiResult};

pub fn create_routes() -> Router<ApiState> {
    Router::new()
        .route("/", get(root))
        .route("/blockchain/info", get(get_blockchain_info))
        .route("/blockchain/block/:height", get(get_block_by_height))
        .route("/balance/{address}", get(get_balance))
        .route("/peers", get(get_peers))
        .route("/genesis", get(get_genesis))
        .route("/snapshot", get(get_snapshot))
        .route("/transaction", post(submit_transaction))
        .route("/propose", post(propose_block))
        .route("/vote", post(cast_vote))
        .route("/quorum/{block_hash}", get(check_quorum))
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
    let blockchain = state.blockchain.read().await;
    let balances = state.balances.read().await;
    let total_supply: u64 = balances.values().sum();

    Ok(Json(BlockchainInfoResponse {
        network: state.network.clone(),
        height: blockchain.chain_tip_height(),
        best_block_hash: blockchain.chain_tip_hash().to_string(),
        total_supply,
        timestamp: chrono::Utc::now().timestamp(),
    }))
}


#[derive(Serialize)]

struct BlockResponse {

    block: time_core::block::Block,

}



async fn get_block_by_height(

    Path(height): Path<u64>,

    State(state): State<ApiState>,

) -> ApiResult<Json<BlockResponse>> {

    let blockchain = state.blockchain.read().await;

    

    match blockchain.get_block_by_height(height) {

        Some(block) => Ok(Json(BlockResponse {

            block: block.clone(),

        })),

        None => Err(ApiError::TransactionNotFound(format!("Block {} not found", height))),

    }

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
        .unwrap_or_else(|_| "/root/time-coin-node/data/genesis.json".to_string());
    
    match std::fs::read_to_string(&genesis_path) {
        Ok(contents) => {
            let genesis: serde_json::Value = serde_json::from_str(&contents)
                .map_err(|e| ApiError::Internal(format!("Failed to parse genesis: {}", e)))?;
            Ok(Json(genesis))
        }
        Err(_) => Err(ApiError::Internal("Genesis block not found".to_string()))
    }
}

#[derive(serde::Serialize)]
struct SnapshotResponse {
    height: u64,
    state_hash: String,
    balances: std::collections::HashMap<String, u64>,
    masternodes: Vec<String>,
    timestamp: i64,
}

async fn get_snapshot(
    State(state): State<ApiState>,
) -> ApiResult<Json<SnapshotResponse>> {
    let balances = state.balances.read().await;
    let masternodes = state.peer_manager.get_peer_ips().await;
    
    // Calculate state hash for verification with deterministic serialization
    let mut sorted_balances: Vec<_> = balances.iter().collect();
    sorted_balances.sort_by_key(|&(k, _)| k);
    let mut sorted_masternodes = masternodes.clone();
    sorted_masternodes.sort();
    
    let state_data = format!("{:?}{:?}", sorted_balances, sorted_masternodes);
    let state_hash = format!("{:x}", md5::compute(&state_data));
    
    Ok(Json(SnapshotResponse {
        height: 0, // TODO: Track actual chain height
        state_hash,
        balances: balances.clone(),
        masternodes,
        timestamp: chrono::Utc::now().timestamp(),
    }))
}

#[derive(serde::Deserialize)]
struct TransactionRequest {
    from: String,
    to: String,
    amount: u64,
    timestamp: i64,
    signature: String,
}

#[derive(serde::Serialize)]
struct TransactionResponse {
    success: bool,
    tx_id: String,
    message: String,
}

async fn submit_transaction(
    State(_state): State<ApiState>,
    Json(tx): Json<TransactionRequest>,
) -> ApiResult<Json<TransactionResponse>> {
    // Generate transaction ID
    let tx_id = format!("{:x}", md5::compute(format!("{}{}{}{}{}", tx.from, tx.to, tx.amount, tx.timestamp, tx.signature)));
    
    println!("📝 Transaction received:");
    println!("   From:   {}...", &tx.from[..16]);
    println!("   To:     {}...", &tx.to[..16]);
    println!("   Amount: {} TIME", tx.amount);
    println!("   TX ID:  {}", tx_id);
    println!("   Signature: {}...", &tx.signature[..16]);
    
    // TODO: Actually process transaction (validate, add to mempool, broadcast)
    // For now, just accept it
    
    Ok(Json(TransactionResponse {
        success: true,
        tx_id,
        message: "Transaction accepted and queued for processing".to_string(),
    }))
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct ProposeBlockRequest {
    height: u64,
    proposer: String,
    timestamp: i64,
    transactions: Vec<String>,
    previous_hash: String,
    block_hash: String,
}

#[derive(serde::Serialize)]
struct ProposeBlockResponse {
    success: bool,
    message: String,
}

async fn propose_block(
    State(_state): State<ApiState>,
    Json(proposal): Json<ProposeBlockRequest>,
) -> ApiResult<Json<ProposeBlockResponse>> {
    println!("📬 Received block proposal:");
    println!("   Height: {}", proposal.height);
    println!("   Proposer: {}", proposal.proposer);
    println!("   Hash: {}", proposal.block_hash);
    
    // TODO: Validate proposal and store in consensus
    
    Ok(Json(ProposeBlockResponse {
        success: true,
        message: "Block proposal received and queued for voting".to_string(),
    }))
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct CastVoteRequest {
    block_hash: String,
    voter: String,
    approve: bool,
    timestamp: i64,
}

#[derive(serde::Serialize)]
struct CastVoteResponse {
    success: bool,
    message: String,
    quorum_reached: bool,
}

async fn cast_vote(
    State(_state): State<ApiState>,
    Json(vote): Json<CastVoteRequest>,
) -> ApiResult<Json<CastVoteResponse>> {
    let vote_type = if vote.approve { "APPROVE" } else { "REJECT" };
    println!("🗳️  Vote received: {} from {}", vote_type, vote.voter);
    
    // TODO: Store vote and check quorum
    
    Ok(Json(CastVoteResponse {
        success: true,
        message: format!("Vote recorded: {}", vote_type),
        quorum_reached: false,
    }))
}

#[derive(serde::Serialize)]
struct QuorumStatus {
    block_hash: String,
    has_quorum: bool,
    approvals: usize,
    rejections: usize,
    total_nodes: usize,
    required: usize,
}

async fn check_quorum(
    State(_state): State<ApiState>,
    axum::extract::Path(block_hash): axum::extract::Path<String>,
) -> ApiResult<Json<QuorumStatus>> {
    // TODO: Get actual quorum status from consensus
    
    Ok(Json(QuorumStatus {
        block_hash,
        has_quorum: false,
        approvals: 0,
        rejections: 0,
        total_nodes: 0,
        required: 0,
    }))
}
