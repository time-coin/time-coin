use crate::handlers::get_node_wallet;
use crate::masternode_handlers::{list_masternodes, register_masternode};
use crate::quarantine_handlers::{get_quarantine_stats, get_quarantined_peers, release_peer};
use crate::rpc_handlers;
use crate::testnet_handlers::{get_mint_info, mint_coins};
use crate::treasury_handlers::{
    approve_treasury_proposal, distribute_treasury_funds, get_treasury_allocations,
    get_treasury_stats, get_treasury_withdrawals,
};
use crate::{ApiError, ApiResult, ApiState};
use axum::extract::Path;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use owo_colors::OwoColorize;
use serde::Serialize;
use std::collections::HashMap;

/// Create routes with TIME Coin-specific endpoints
pub fn create_routes() -> Router<ApiState> {
    Router::new()
        // Testnet-only endpoints
        .route("/testnet/mint", post(mint_coins))
        .route("/testnet/mint/info", get(get_mint_info))
        // Masternode management endpoints
        .route("/masternode/register", post(register_masternode))
        .route("/masternodes/list", get(list_masternodes))
        .route("/node/wallet", get(get_node_wallet))
        // Treasury management endpoints
        .route("/treasury/stats", get(get_treasury_stats))
        .route("/treasury/allocations", get(get_treasury_allocations))
        .route("/treasury/withdrawals", get(get_treasury_withdrawals))
        .route("/treasury/approve", post(approve_treasury_proposal))
        .route("/treasury/distribute", post(distribute_treasury_funds))
        // Quarantine management endpoints
        .route("/network/quarantine", get(get_quarantined_peers))
        .route("/network/quarantine/release", post(release_peer))
        .route("/network/quarantine/stats", get(get_quarantine_stats))
        // Core blockchain endpoints
        .route("/", get(root))
        .route("/blockchain/info", get(get_blockchain_info))
        .route("/blockchain/block/{height}", get(get_block_by_height))
        .route("/balance/{address}", get(get_balance))
        .route("/utxos/{address}", get(get_utxos_by_address))
        .route("/peers", get(get_peers))
        .route("/peers/discovered", post(handle_peer_discovered))
        .route("/genesis", get(get_genesis))
        .route("/snapshot", get(get_snapshot))
        .route("/snapshot/receive", post(receive_snapshot))
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
        .route("/consensus/block/{height}", get(get_consensus_block))
        .route("/consensus/finalized-block", post(receive_finalized_block))
        // ============================================================
        // Bitcoin-compatible RPC endpoints (maintained for compatibility)
        // ============================================================
        // Blockchain information
        .route(
            "/rpc/getblockchaininfo",
            post(rpc_handlers::getblockchaininfo),
        )
        .route("/rpc/getblockcount", post(rpc_handlers::getblockcount))
        .route("/rpc/getblockhash", post(rpc_handlers::getblockhash))
        .route("/rpc/getblock", post(rpc_handlers::getblock))
        // Transaction handling
        .route(
            "/rpc/getrawtransaction",
            post(rpc_handlers::getrawtransaction),
        )
        .route(
            "/rpc/sendrawtransaction",
            post(rpc_handlers::sendrawtransaction),
        )
        // Wallet operations
        .route("/rpc/getwalletinfo", post(rpc_handlers::getwalletinfo))
        .route("/rpc/getbalance", post(rpc_handlers::getbalance))
        .route("/rpc/getnewaddress", post(rpc_handlers::getnewaddress))
        .route("/rpc/validateaddress", post(rpc_handlers::validateaddress))
        .route("/rpc/listunspent", post(rpc_handlers::listunspent))
        // Network information
        .route("/rpc/getpeerinfo", post(rpc_handlers::getpeerinfo))
        .route("/rpc/getnetworkinfo", post(rpc_handlers::getnetworkinfo))
        // ============================================================
        // TIME Coin-specific RPC endpoints
        // ============================================================
        // 24-hour time block endpoints
        .route(
            "/rpc/gettimeblockinfo",
            post(rpc_handlers::gettimeblockinfo),
        )
        .route(
            "/rpc/gettimeblockrewards",
            post(rpc_handlers::gettimeblockrewards),
        )
        // Masternode RPC endpoints
        .route(
            "/rpc/getmasternodeinfo",
            post(rpc_handlers::getmasternodeinfo),
        )
        .route("/rpc/listmasternodes", post(rpc_handlers::listmasternodes))
        .route(
            "/rpc/getmasternodecount",
            post(rpc_handlers::getmasternodecount),
        )
        // Consensus endpoints
        .route(
            "/rpc/getconsensusstatus",
            post(rpc_handlers::getconsensusstatus),
        )
        // Treasury & Governance endpoints
        .route("/rpc/gettreasury", post(rpc_handlers::gettreasury))
        .route("/rpc/listproposals", post(rpc_handlers::listproposals))
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
        None => Err(ApiError::TransactionNotFound(format!(
            "Block {} not found",
            height
        ))),
    }
}

async fn get_consensus_block(
    State(state): State<ApiState>,
    Path(height): Path<u64>,
) -> ApiResult<Json<BlockResponse>> {
    let blockchain = state.blockchain.read().await;

    match blockchain.get_block_by_height(height) {
        Some(block) => Ok(Json(BlockResponse {
            block: block.clone(),
        })),
        None => Err(ApiError::TransactionNotFound(format!(
            "Block {} not found",
            height
        ))),
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

    Ok(Json(UtxoResponse { address, utxos }))
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

async fn get_peers(State(state): State<ApiState>) -> ApiResult<Json<PeersResponse>> {
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

#[derive(serde::Deserialize)]
struct PeerDiscoveredRequest {
    address: String,
    version: String,
}

async fn handle_peer_discovered(
    State(state): State<ApiState>,
    Json(req): Json<PeerDiscoveredRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    use std::net::SocketAddr;
    use time_network::PeerInfo as NetworkPeerInfo;

    let mut peer_addr: SocketAddr = req
        .address
        .parse()
        .map_err(|e| ApiError::BadRequest(format!("Invalid peer address: {}", e)))?;

    let network = if state.network == "mainnet" {
        time_network::NetworkType::Mainnet
    } else {
        time_network::NetworkType::Testnet
    };

    // Detect ephemeral ports (49152-65535) and replace with network-aware standard port
    if peer_addr.port() >= 49152 {
        let standard_port = match network {
            time_network::NetworkType::Mainnet => 24000,
            time_network::NetworkType::Testnet => 24100,
        };
        peer_addr.set_port(standard_port);
    }

    {
        let mut broadcasts = state.recent_broadcasts.write().await;
        let now = std::time::Instant::now();
        // Use IP-only for deduplication key
        let peer_key = peer_addr.ip().to_string();

        if let Some(&last_seen) = broadcasts.get(&peer_key) {
            if now.duration_since(last_seen) < std::time::Duration::from_secs(300) {
                return Ok(Json(serde_json::json!({
                    "success": true,
                    "message": "Peer already recently processed",
                    "deduplicated": true
                })));
            }
        }

        broadcasts.insert(peer_key, now);
    }

    let peer_info = NetworkPeerInfo::with_version(peer_addr, network, req.version.clone());

    state
        .peer_manager
        .add_discovered_peer(peer_addr.ip().to_string(), peer_addr.port(), req.version)
        .await;

    println!(
        "📡 Learned about new peer {} from peer broadcast",
        peer_addr
    );

    let peer_manager_clone = state.peer_manager.clone();
    tokio::spawn(async move {
        match peer_manager_clone.connect_to_peer(peer_info).await {
            Ok(_) => {
                println!("✓ Successfully connected to broadcasted peer {}", peer_addr);
            }
            Err(e) => {
                println!(
                    "⚠ Failed to connect to broadcasted peer {}: {}",
                    peer_addr, e
                );
            }
        }
    });

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Peer discovered and connection attempted"
    })))
}

async fn get_genesis(State(_state): State<ApiState>) -> ApiResult<Json<serde_json::Value>> {
    let genesis_path = std::env::var("GENESIS_PATH")
        .unwrap_or_else(|_| "/root/time-coin-node/config/genesis-testnet.json".to_string());

    match std::fs::read_to_string(&genesis_path) {
        Ok(contents) => {
            let genesis: serde_json::Value = serde_json::from_str(&contents)
                .map_err(|e| ApiError::Internal(format!("Failed to parse genesis: {}", e)))?;
            Ok(Json(genesis))
        }
        Err(_) => Err(ApiError::Internal("Genesis block not found".to_string())),
    }
}

#[derive(serde::Serialize)]
struct SnapshotResponse {
    height: u64,
    state_hash: String,
    balances: HashMap<String, u64>,
    masternodes: Vec<String>,
    timestamp: i64,
}

async fn get_snapshot(State(state): State<ApiState>) -> ApiResult<Json<SnapshotResponse>> {
    let balances = state.balances.read().await;
    let masternodes = state.peer_manager.get_peer_ips().await;

    let mut sorted_balances: Vec<_> = balances.iter().collect();
    sorted_balances.sort_by_key(|&(k, _)| k);
    let mut sorted_masternodes = masternodes.clone();
    sorted_masternodes.sort();

    let state_data = format!("{:?}{:?}", sorted_balances, sorted_masternodes);
    let state_hash = format!("{:x}", md5::compute(&state_data));

    let height = {
        let chain = state.blockchain.read().await;
        chain.chain_tip_height()
    };

    Ok(Json(SnapshotResponse {
        height,
        state_hash,
        balances: balances.clone(),
        masternodes,
        timestamp: chrono::Utc::now().timestamp(),
    }))
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SnapshotReceiveRequest {
    height: u64,
    state_hash: String,
    balances: HashMap<String, u64>,
    masternodes: Vec<String>,
    timestamp: i64,
}

async fn receive_snapshot(
    State(state): State<ApiState>,
    Json(req): Json<SnapshotReceiveRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut sorted_balances: Vec<_> = req.balances.iter().collect();
    sorted_balances.sort_by_key(|&(k, _)| k);
    let mut sorted_masternodes = req.masternodes.clone();
    sorted_masternodes.sort();

    let state_data = format!("{:?}{:?}", sorted_balances, sorted_masternodes);
    let computed_hash = format!("{:x}", md5::compute(&state_data));

    if computed_hash != req.state_hash {
        eprintln!(
            "{} Snapshot - received snapshot hash mismatch (reported={}, computed={})",
            "⚠".yellow(),
            req.state_hash,
            computed_hash
        );
        return Err(ApiError::BadRequest("Snapshot hash mismatch".to_string()));
    }

    {
        let mut balances = state.balances.write().await;
        *balances = req.balances.clone();
    }

    {
        let mut mn = req.masternodes.clone();
        mn.sort();
        println!(
            "{} Received snapshot (height: {}) from peer. Masternodes: {:?}",
            "✓".green(),
            req.height,
            mn
        );
    }

    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| ".".to_string());
    let filename = format!(
        "{}/snapshot_received_{}.json",
        data_dir,
        chrono::Utc::now().timestamp()
    );
    match serde_json::to_string_pretty(&req) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&filename, json) {
                eprintln!(
                    "{} Failed to persist snapshot to {}: {:?}",
                    "⚠".yellow(),
                    filename,
                    e
                );
            } else {
                println!("{} Snapshot written to {}", "💾".green(), filename);
            }
        }
        Err(e) => {
            eprintln!(
                "{} Failed to serialize snapshot for disk: {:?}",
                "⚠".yellow(),
                e
            );
        }
    }

    Ok(Json(serde_json::json!({ "result": "ok" })))
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
    let tx_id = format!(
        "{:x}",
        md5::compute(format!(
            "{}{}{}{}{}",
            tx.from, tx.to, tx.amount, tx.timestamp, tx.signature
        ))
    );

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

#[derive(serde::Serialize)]
struct MempoolStatusResponse {
    size: usize,
    transactions: Vec<String>,
}

async fn get_mempool_status(
    State(state): State<ApiState>,
) -> ApiResult<Json<MempoolStatusResponse>> {
    let mempool = state
        .mempool
        .as_ref()
        .ok_or(ApiError::Internal("Mempool not initialized".to_string()))?;

    let transactions = mempool.get_all_transactions().await;
    let tx_ids: Vec<String> = transactions.iter().map(|tx| tx.txid.clone()).collect();

    Ok(Json(MempoolStatusResponse {
        size: tx_ids.len(),
        transactions: tx_ids,
    }))
}

async fn add_to_mempool(
    State(state): State<ApiState>,
    Json(tx): Json<time_core::Transaction>,
) -> ApiResult<Json<serde_json::Value>> {
    let mempool = state
        .mempool
        .as_ref()
        .ok_or(ApiError::Internal("Mempool not initialized".to_string()))?;

    mempool
        .add_transaction(tx.clone())
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to add transaction: {}", e)))?;

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
    let mempool = state
        .mempool
        .as_ref()
        .ok_or(ApiError::Internal("Mempool not initialized".to_string()))?;

    let transactions = mempool.get_all_transactions().await;
    Ok(Json(transactions))
}

async fn receive_tx_proposal(
    State(state): State<ApiState>,
    Json(proposal): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    use time_consensus::tx_consensus::TransactionProposal;

    let tx_proposal: TransactionProposal = serde_json::from_value(proposal)
        .map_err(|e| ApiError::Internal(format!("Invalid proposal format: {}", e)))?;

    println!(
        "📬 Received transaction proposal for block {}",
        tx_proposal.block_height
    );
    println!("   Proposer: {}", tx_proposal.proposer);
    println!("   Transactions: {}", tx_proposal.tx_ids.len());
    println!("   Merkle root: {}...", &tx_proposal.merkle_root[..16]);

    if let Some(tx_consensus) = state.tx_consensus.as_ref() {
        tx_consensus.propose_tx_set(tx_proposal.clone()).await;

        let blockchain = state.blockchain.read().await;
        let node_id = blockchain.chain_tip_hash().to_string();
        drop(blockchain);

        if node_id != tx_proposal.proposer {
            let mut all_valid = true;
            if let Some(mempool) = state.mempool.as_ref() {
                for txid in &tx_proposal.tx_ids {
                    if !mempool.contains(txid).await {
                        println!("   ⚠️  Transaction {} not in our mempool", &txid[..16]);
                        all_valid = false;
                    }
                }
            }

            let vote = time_consensus::tx_consensus::TxSetVote {
                block_height: tx_proposal.block_height,
                merkle_root: tx_proposal.merkle_root.clone(),
                voter: node_id.to_string(),
                approve: all_valid,
                timestamp: chrono::Utc::now().timestamp(),
            };

            let _ = tx_consensus.vote_on_tx_set(vote.clone()).await;

            let vote_type = if all_valid {
                "APPROVE ✓"
            } else {
                "REJECT ✗"
            };
            println!("   🗳️  Auto-voted: {}", vote_type);

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
    use std::net::IpAddr;
    use time_consensus::tx_consensus::TxSetVote;

    let tx_vote: TxSetVote = serde_json::from_value(vote)
        .map_err(|e| ApiError::Internal(format!("Invalid vote format: {}", e)))?;

    // Check if voter is quarantined
    if let Some(quarantine) = state.quarantine.as_ref() {
        if let Ok(voter_ip) = tx_vote.voter.parse::<IpAddr>() {
            if quarantine.is_quarantined(&voter_ip).await {
                if let Some(reason) = quarantine.get_reason(&voter_ip).await {
                    println!(
                        "🚫 Rejecting vote from quarantined peer {} (reason: {})",
                        tx_vote.voter, reason
                    );
                }
                return Err(ApiError::Internal(format!(
                    "Voter {} is quarantined",
                    tx_vote.voter
                )));
            }
        }
    }

    let vote_type = if tx_vote.approve {
        "APPROVE ✓"
    } else {
        "REJECT ✗"
    };
    println!(
        "🗳️  Received transaction set vote: {} from {}",
        vote_type, tx_vote.voter
    );

    if let Some(tx_consensus) = state.tx_consensus.as_ref() {
        tx_consensus
            .vote_on_tx_set(tx_vote.clone())
            .await
            .map_err(ApiError::Internal)?;

        let (has_consensus, approvals, total) = tx_consensus
            .has_tx_consensus(tx_vote.block_height, &tx_vote.merkle_root)
            .await;

        if has_consensus {
            println!(
                "   ✅ Transaction set consensus reached! ({}/{})",
                approvals, total
            );
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

    println!(
        "📦 Received block proposal for height {}",
        block_proposal.block_height
    );
    println!("   Proposer: {}", block_proposal.proposer);
    if !block_proposal.block_hash.is_empty() {
        println!("   Block hash: {}...", &block_proposal.block_hash[..16]);
    } else {
        println!("   Block hash: (pending)");
    }

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
    use std::net::IpAddr;
    use time_consensus::block_consensus::BlockVote;

    let block_vote: BlockVote = serde_json::from_value(vote)
        .map_err(|e| ApiError::Internal(format!("Invalid vote format: {}", e)))?;

    // Check if voter is quarantined
    if let Some(quarantine) = state.quarantine.as_ref() {
        if let Ok(voter_ip) = block_vote.voter.parse::<IpAddr>() {
            if quarantine.is_quarantined(&voter_ip).await {
                if let Some(reason) = quarantine.get_reason(&voter_ip).await {
                    println!(
                        "🚫 Rejecting vote from quarantined peer {} (reason: {})",
                        block_vote.voter, reason
                    );
                }
                return Err(ApiError::Internal(format!(
                    "Voter {} is quarantined",
                    block_vote.voter
                )));
            }
        }
    }

    let vote_type = if block_vote.approve {
        "APPROVE ✓"
    } else {
        "REJECT ✗"
    };
    println!(
        "🗳️  Received block vote: {} from {}",
        vote_type, block_vote.voter
    );

    if let Some(block_consensus) = state.block_consensus.as_ref() {
        block_consensus
            .vote_on_block(block_vote.clone())
            .await
            .map_err(ApiError::Internal)?;

        let (has_consensus, approvals, total) = block_consensus
            .has_block_consensus(block_vote.block_height, &block_vote.block_hash)
            .await;

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

async fn receive_finalized_block(
    State(state): State<ApiState>,
    Json(block): Json<time_core::block::Block>,
) -> ApiResult<Json<serde_json::Value>> {
    println!(
        "📥 Received finalized block push for height {}",
        block.header.block_number
    );

    let mut blockchain = state.blockchain.write().await;
    match blockchain.add_block(block) {
        Ok(_) => Ok(Json(serde_json::json!({"success": true}))),
        Err(e) => Err(ApiError::Internal(format!(
            "Failed to add pushed block: {:?}",
            e
        ))),
    }
}
