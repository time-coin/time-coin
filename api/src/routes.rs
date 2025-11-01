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
        .route("/blockchain/block/{height}", get(get_block_by_height))
        .route("/balance/{address}", get(get_balance))
        .route("/utxos/{address}", get(get_utxos_by_address))
        .route("/peers", get(get_peers))
        .route("/genesis", get(get_genesis))
        .route("/snapshot", get(get_snapshot))
        .route("/transaction", post(submit_transaction))
        .route("/propose", post(propose_block))
        .route("/vote", post(cast_vote))
        .route("/quorum/{block_hash}", get(check_quorum))
        // Mempool endpoints
        .route("/mempool/status", get(get_mempool_status))
        .route("/mempool/add", post(add_to_mempool))
        .route("/mempool/all", get(get_all_mempool_txs))
        // Transaction consensus endpoints
        .route("/consensus/tx-proposal", post(receive_tx_proposal))
        .route("/consensus/tx-vote", post(receive_tx_vote))
        .route("/consensus/block-proposal", post(receive_block_proposal))
        .route("/consensus/block-vote", post(receive_block_vote))
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
    State(state): State<ApiState>,
    Path(height): Path<u64>,
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
    Path(address): Path<String>,
) -> ApiResult<Json<BalanceResponse>> {
    let balances = state.balances.read().await;
    let balance = balances.get(&address).copied().unwrap_or(0);

    Ok(Json(BalanceResponse { address, balance }))
}

#[derive(Serialize)]
struct UtxoResponse {
    address: String,
    utxos: Vec<UtxoInfo>,
}

#[derive(Serialize)]
struct UtxoInfo {
    txid: String,
    vout: u32,
    amount: u64,
}

async fn get_utxos_by_address(
    State(state): State<ApiState>,
    Path(address): Path<String>,
) -> ApiResult<Json<UtxoResponse>> {
    let blockchain = state.blockchain.read().await;
    let utxo_set = blockchain.utxo_set();
    
    let mut utxos = Vec::new();
    
    for (outpoint, output) in utxo_set.get_utxos_by_address(&address) {
        utxos.push(UtxoInfo {
            txid: outpoint.txid.clone(),
            vout: outpoint.vout,
            amount: output.amount,
        });
    }
    
    Ok(Json(UtxoResponse {
        address,
        utxos,
    }))
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

    let mut sorted_balances: Vec<_> = balances.iter().collect();
    sorted_balances.sort_by_key(|&(k, _)| k);
    let mut sorted_masternodes = masternodes.clone();
    sorted_masternodes.sort();

    let state_data = format!("{:?}{:?}", sorted_balances, sorted_masternodes);
    let state_hash = format!("{:x}", md5::compute(&state_data));

    Ok(Json(SnapshotResponse {
        height: 0,
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
    let tx_id = format!("{:x}", md5::compute(format!("{}{}{}{}{}", tx.from, tx.to, tx.amount, tx.timestamp, tx.signature)));

    println!("📝 Transaction received:");
    println!("   From:   {}...", &tx.from[..16]);
    println!("   To:     {}...", &tx.to[..16]);
    println!("   Amount: {} TIME", tx.amount);
    println!("   TX ID:  {}", tx_id);

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
    println!("   Hash: {}", proposal.block_hash);

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
    Path(block_hash): Path<String>,
) -> ApiResult<Json<QuorumStatus>> {
    Ok(Json(QuorumStatus {
        block_hash,
        has_quorum: false,
        approvals: 0,
        rejections: 0,
        total_nodes: 0,
        required: 0,
    }))
}

// Mempool endpoints
#[derive(serde::Serialize)]
struct MempoolStatusResponse {
    size: usize,
    transactions: Vec<String>,
}

async fn get_mempool_status(
    State(state): State<ApiState>,
) -> ApiResult<Json<MempoolStatusResponse>> {
    let mempool = state.mempool.as_ref()
        .ok_or(ApiError::Internal("Mempool not initialized".to_string()))?;
    
    let transactions = mempool.get_all_transactions().await;
    let tx_ids: Vec<String> = transactions.iter()
        .map(|tx| tx.txid.clone())
        .collect();
    
    Ok(Json(MempoolStatusResponse {
        size: tx_ids.len(),
        transactions: tx_ids,
    }))
}

async fn add_to_mempool(
    State(state): State<ApiState>,
    Json(tx): Json<time_core::Transaction>,
) -> ApiResult<Json<serde_json::Value>> {
    let mempool = state.mempool.as_ref()
        .ok_or(ApiError::Internal("Mempool not initialized".to_string()))?;
    
    mempool.add_transaction(tx.clone()).await
        .map_err(|e| ApiError::Internal(format!("Failed to add transaction: {}", e)))?;
    
    // Broadcast to peers
    if let Some(broadcaster) = state.tx_broadcaster.as_ref() {
        broadcaster.broadcast_transaction(tx).await;
    }
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Transaction added to mempool and broadcast"
    })))
}

async fn get_all_mempool_txs(
    State(state): State<ApiState>,
) -> ApiResult<Json<Vec<time_core::Transaction>>> {
    let mempool = state.mempool.as_ref()
        .ok_or(ApiError::Internal("Mempool not initialized".to_string()))?;
    
    let transactions = mempool.get_all_transactions().await;
    Ok(Json(transactions))
}

async fn receive_tx_proposal(
    State(state): State<ApiState>,
    Json(proposal): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    use time_consensus::tx_consensus::TransactionProposal;
    
    // Parse the proposal
    let tx_proposal: TransactionProposal = serde_json::from_value(proposal)
        .map_err(|e| ApiError::Internal(format!("Invalid proposal format: {}", e)))?;
    
    println!("📬 Received transaction proposal for block {}", tx_proposal.block_height);
    println!("   Proposer: {}", tx_proposal.proposer);
    println!("   Transactions: {}", tx_proposal.tx_ids.len());
    println!("   Merkle root: {}...", &tx_proposal.merkle_root[..16]);
    
    // Store proposal in tx_consensus
    if let Some(tx_consensus) = state.tx_consensus.as_ref() {
        tx_consensus.propose_tx_set(tx_proposal.clone()).await;
        
        // Auto-vote if we're a validator (not the proposer)
        let blockchain = state.blockchain.read().await;
        let node_id = blockchain.chain_tip_hash().to_string(); // Use our node ID
        drop(blockchain);
        
        if node_id != tx_proposal.proposer {
            // Validate the transactions exist in our mempool
            let mut all_valid = true;
            if let Some(mempool) = state.mempool.as_ref() {
                for txid in &tx_proposal.tx_ids {
                    if !mempool.contains(txid).await {
                        println!("   ⚠️  Transaction {} not in our mempool", &txid[..16]);
                        all_valid = false;
                    }
                }
            }
            
            // Cast our vote
            let vote = time_consensus::tx_consensus::TxSetVote {
                block_height: tx_proposal.block_height,
                merkle_root: tx_proposal.merkle_root.clone(),
                voter: node_id.to_string(),
                approve: all_valid,
                timestamp: chrono::Utc::now().timestamp(),
            };
            
            let _ = tx_consensus.vote_on_tx_set(vote.clone()).await;
            
            let vote_type = if all_valid { "APPROVE ✓" } else { "REJECT ✗" };
            println!("   🗳️  Auto-voted: {}", vote_type);
            
            // Broadcast our vote to other nodes
            if let Some(broadcaster) = state.tx_broadcaster.as_ref() {
                let vote_json = serde_json::to_value(&vote).unwrap();
                broadcaster.broadcast_tx_vote(vote_json).await;
            }
        }
    }
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Transaction proposal received and processed"
    })))
}

async fn receive_tx_vote(
    State(state): State<ApiState>,
    Json(vote): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    use time_consensus::tx_consensus::TxSetVote;
    
    // Parse the vote
    let tx_vote: TxSetVote = serde_json::from_value(vote)
        .map_err(|e| ApiError::Internal(format!("Invalid vote format: {}", e)))?;
    
    let vote_type = if tx_vote.approve { "APPROVE ✓" } else { "REJECT ✗" };
    println!("🗳️  Received transaction set vote: {} from {}", vote_type, tx_vote.voter);
    
    // Store vote in tx_consensus
    if let Some(tx_consensus) = state.tx_consensus.as_ref() {
        tx_consensus.vote_on_tx_set(tx_vote.clone()).await
            .map_err(|e| ApiError::Internal(e))?;
        
        // Check if we now have consensus
        let (has_consensus, approvals, total) = tx_consensus
            .has_tx_consensus(tx_vote.block_height, &tx_vote.merkle_root).await;
        
        if has_consensus {
            println!("   ✅ Transaction set consensus reached! ({}/{})", approvals, total);
        } else {
            println!("   ⏳ Waiting for consensus... ({}/{})", approvals, total);
        }
    }
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Vote recorded"
    })))
}
async fn receive_block_proposal(
    State(state): State<ApiState>,
    Json(proposal): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    use time_consensus::block_consensus::BlockProposal;
    
    let block_proposal: BlockProposal = serde_json::from_value(proposal)
        .map_err(|e| ApiError::Internal(format!("Invalid proposal format: {}", e)))?;
    
    println!("📦 Received block proposal for height {}", block_proposal.block_height);
    println!("   Proposer: {}", block_proposal.proposer);
    println!("   Block hash: {}...", &block_proposal.block_hash[..16]);
    
    if let Some(block_consensus) = state.block_consensus.as_ref() {
        block_consensus.propose_block(block_proposal.clone()).await;
    }
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Block proposal received"
    })))
}

async fn receive_block_vote(
    State(state): State<ApiState>,
    Json(vote): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    use time_consensus::block_consensus::BlockVote;
    
    let block_vote: BlockVote = serde_json::from_value(vote)
        .map_err(|e| ApiError::Internal(format!("Invalid vote format: {}", e)))?;
    
    let vote_type = if block_vote.approve { "APPROVE ✓" } else { "REJECT ✗" };
    println!("🗳️  Received block vote: {} from {}", vote_type, block_vote.voter);
    
    if let Some(block_consensus) = state.block_consensus.as_ref() {
        block_consensus.vote_on_block(block_vote.clone()).await
            .map_err(|e| ApiError::Internal(e))?;
        
        let (has_consensus, approvals, total) = block_consensus
            .has_block_consensus(block_vote.block_height, &block_vote.block_hash).await;
        
        if has_consensus {
            println!("   ✅ CONSENSUS REACHED ({}/{})", approvals, total);
        } else {
            println!("   ⏳ Waiting... ({}/{})", approvals, total);
        }
    }
    
    Ok(Json(serde_json::json!({
        "success": true
    })))
}
