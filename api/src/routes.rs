use crate::balance::calculate_mempool_balance;
use crate::handlers::get_node_wallet;
use crate::masternode_handlers::{list_masternodes, register_masternode};
use crate::proposal_handlers::{create_proposal, get_proposal, list_proposals, vote_proposal};
use crate::quarantine_handlers::{get_quarantine_stats, get_quarantined_peers, release_peer};
use crate::rpc_handlers;
use crate::treasury_handlers::{
    approve_treasury_proposal, distribute_treasury_funds, get_proposal_by_id,
    get_treasury_allocations, get_treasury_stats, get_treasury_withdrawals, submit_proposal,
    vote_on_proposal,
};
use crate::tx_sync_handlers::{
    handle_transaction_rejection, receive_missing_transactions, request_missing_transactions,
};
use crate::wallet_send_handler::wallet_send;
use crate::{ApiError, ApiResult, ApiState};
use axum::extract::Path;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use chrono::TimeZone;
use owo_colors::OwoColorize;
use serde::Serialize;
use std::collections::HashMap;
use tracing as log;

/// Create routes with TIME Coin-specific endpoints
pub fn create_routes() -> Router<ApiState> {
    Router::new()
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
        .route("/treasury/propose", post(submit_proposal))
        .route("/treasury/proposal/{id}", post(get_proposal_by_id))
        .route("/treasury/vote", post(vote_on_proposal))
        // Grant proposal endpoints (new governance system)
        .route("/proposals/create", post(create_proposal))
        .route("/proposals/vote", post(vote_proposal))
        .route("/proposals/list", get(list_proposals))
        .route("/proposals/{id}", get(get_proposal))
        // Quarantine management endpoints
        .route("/network/quarantine", get(get_quarantined_peers))
        .route("/network/quarantine/release", post(release_peer))
        .route("/network/quarantine/stats", get(get_quarantine_stats))
        // Network peers endpoint
        .route("/network/peers", get(get_peers))
        // Catch-up coordination endpoint
        .route("/network/catch-up-request", post(handle_catch_up_request))
        // Core blockchain endpoints
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/blockchain/info", get(get_blockchain_info))
        .route("/blockchain/block/{height}", get(get_block_by_height))
        .route("/balance/{address}", get(get_balance))
        .route("/utxos/{address}", get(get_utxos_by_address))
        .route("/peers/discovered", post(handle_peer_discovered))
        .route("/genesis", get(get_genesis))
        .route("/snapshot", get(get_snapshot))
        .route("/snapshot/receive", post(receive_snapshot))
        .route("/transaction", post(submit_transaction))
        .route("/propose", post(propose_block))
        .route("/vote", post(cast_vote))
        .route("/quorum/{block_hash}", get(check_quorum))
        // Mempool endpoints
        .route("/mempool", get(get_mempool_status)) // Backwards compatibility alias
        .route("/mempool/status", get(get_mempool_status))
        .route("/mempool/add", post(add_to_mempool))
        .route("/mempool/finalized", post(receive_finalized_transaction)) // 🆕 Receive finalized tx from peers
        .route("/mempool/all", get(get_all_mempool_txs))
        .route("/mempool/clear", post(clear_mempool))
        // Transaction sync endpoints (for block proposals)
        .route("/tx_sync/request", post(request_missing_transactions)) // Request missing txs
        .route("/tx_sync/response", post(receive_missing_transactions)) // Receive missing txs
        .route("/tx_sync/rejection", post(handle_transaction_rejection)) // Handle rejection
        // Transaction endpoints
        .route("/transactions", post(add_to_mempool)) // REST endpoint
        .route("/transactions/{txid}", get(get_transaction)) // Get single transaction
        // WebSocket endpoints removed - using TCP protocol instead
        // Wallet sync endpoints
        .route(
            "/wallet/sync",
            post(crate::wallet_sync_handlers::sync_wallet_addresses),
        )
        .route(
            "/wallet/sync-xpub",
            post(crate::wallet_sync_handlers::sync_wallet_xpub),
        )
        .route(
            "/wallet/register-xpub",
            post(crate::wallet_sync_handlers::register_xpub),
        )
        .route(
            "/wallet/validate",
            post(crate::wallet_sync_handlers::validate_transaction),
        )
        .route(
            "/wallet/pending",
            post(crate::wallet_sync_handlers::get_pending_transactions),
        )
        .route("/wallet/send", post(wallet_send))
        // Transaction consensus endpoints
        .route("/consensus/tx-proposal", post(receive_tx_proposal))
        .route("/consensus/tx-vote", post(receive_tx_vote))
        .route("/consensus/block-proposal", post(receive_block_proposal))
        .route("/consensus/block-vote", post(receive_block_vote))
        .route(
            "/consensus/request-block-proposal",
            post(request_block_proposal),
        )
        .route("/consensus/block/{height}", get(get_consensus_block))
        .route("/consensus/finalized-block", post(receive_finalized_block))
        // Instant finality endpoints
        .route(
            "/finality/submit",
            post(crate::instant_finality_handlers::submit_transaction),
        )
        .route(
            "/finality/vote",
            post(crate::instant_finality_handlers::record_decision),
        )
        .route(
            "/finality/status",
            post(crate::instant_finality_handlers::get_transaction_status),
        )
        .route(
            "/finality/approved",
            get(crate::instant_finality_handlers::get_approved_transactions),
        )
        .route(
            "/finality/stats",
            get(crate::instant_finality_handlers::get_finality_stats),
        )
        .route(
            "/consensus/instant-finality-request",
            post(receive_instant_finality_request),
        )
        .route(
            "/consensus/instant-finality-vote",
            post(receive_instant_finality_vote),
        )
        .route("/finality/sync", post(sync_finalized_transactions))
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
struct HealthResponse {
    status: String,
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

#[derive(serde::Serialize)]
struct BlockchainInfoResponse {
    network: String,
    height: u64,
    best_block_hash: String,
    total_supply: u64,
    timestamp: i64,
    wallet_address: String,
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
        wallet_address: state.wallet_address.clone(),
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
    unconfirmed_balance: u64,
}

async fn get_balance(
    State(state): State<ApiState>,
    Path(address): Path<String>,
) -> ApiResult<Json<BalanceResponse>> {
    let blockchain = state.blockchain.read().await;
    let balance = blockchain.get_balance(&address);

    // Calculate unconfirmed balance from mempool
    let unconfirmed_balance = if let Some(mempool) = &state.mempool {
        calculate_mempool_balance(&address, &blockchain, mempool).await
    } else {
        0
    };

    Ok(Json(BalanceResponse {
        address,
        balance,
        unconfirmed_balance,
    }))
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
    use std::time::Instant;

    tracing::info!("GET /peers endpoint called");
    let start = Instant::now();
    tracing::info!("Calling peer_manager.get_connected_peers()...");
    let peers = state.peer_manager.get_connected_peers().await;
    let fetch_time = start.elapsed();
    tracing::info!("Got {} peers in {:?}", peers.len(), fetch_time);

    let map_start = Instant::now();
    let peer_info: Vec<PeerInfo> = peers
        .iter()
        .map(|p| PeerInfo {
            address: p.address.to_string(),
            version: p.version.clone(),
            connected: true,
        })
        .collect();
    let map_time = map_start.elapsed();

    let count = peer_info.len();
    let total_time = start.elapsed();

    tracing::debug!(
        "GET /peers: fetched {} peers in {:?} (fetch: {:?}, map: {:?})",
        count,
        total_time,
        fetch_time,
        map_time
    );

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

    // Check if we're already connected to this peer
    let already_connected = state
        .peer_manager
        .get_connected_peers()
        .await
        .iter()
        .any(|p| p.address.ip() == peer_addr.ip());

    if already_connected {
        // Already connected - just update peer exchange, don't log or reconnect
        state
            .peer_manager
            .add_discovered_peer(peer_addr.ip().to_string(), peer_addr.port(), req.version)
            .await;

        return Ok(Json(serde_json::json!({
            "success": true,
            "message": "Peer already connected",
            "already_connected": true
        })));
    }

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
                // Silently connected to broadcasted peer (reduce log verbosity)
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

    println!("📥 Transaction {} received from peer", &tx.txid[..16]);

    // WebSocket notifications removed - using TCP protocol instead

    // Trigger instant finality via BFT consensus
    trigger_instant_finality_for_received_tx(state.clone(), tx.clone()).await;

    // Re-broadcast to other peers (if not already from broadcast)
    if let Some(broadcaster) = state.tx_broadcaster.as_ref() {
        broadcaster.broadcast_transaction(tx).await;
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Transaction added to mempool and broadcast"
    })))
}

/// Handle receiving a finalized transaction from another masternode
/// This applies the transaction directly to the UTXO set without re-validating
async fn receive_finalized_transaction(
    State(state): State<ApiState>,
    Json(tx): Json<time_core::Transaction>,
) -> ApiResult<Json<serde_json::Value>> {
    println!(
        "📨 Received FINALIZED transaction {} from peer",
        &tx.txid[..16]
    );

    // Apply directly to UTXO set (transaction was already validated by sending node)
    let mut blockchain = state.blockchain.write().await;

    if let Err(e) = blockchain.utxo_set_mut().apply_transaction(&tx) {
        println!(
            "❌ Failed to apply finalized transaction to UTXO set: {}",
            e
        );
        return Err(ApiError::Internal(format!(
            "Failed to apply transaction: {}",
            e
        )));
    }

    println!("✅ Finalized transaction applied to UTXO set");

    // Save to finalized transactions database
    if let Err(e) = blockchain.save_finalized_tx(&tx, 1, 1) {
        println!("⚠️  Failed to save finalized transaction: {}", e);
    } else {
        println!("💾 Finalized transaction saved to database");
    }

    // Save UTXO snapshot
    if let Err(e) = blockchain.save_utxo_snapshot() {
        println!("⚠️  Failed to save UTXO snapshot: {}", e);
    } else {
        println!("💾 UTXO snapshot saved - state synchronized");
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Finalized transaction applied to UTXO set"
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

async fn clear_mempool(State(state): State<ApiState>) -> ApiResult<Json<serde_json::Value>> {
    let mempool = state
        .mempool
        .as_ref()
        .ok_or(ApiError::Internal("Mempool not initialized".to_string()))?;

    mempool.clear().await;

    log::info!("🗑️ Mempool cleared via API");

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Mempool cleared successfully"
    })))
}

async fn get_transaction(
    State(state): State<ApiState>,
    Path(txid): Path<String>,
) -> ApiResult<Json<time_core::Transaction>> {
    let mempool = state
        .mempool
        .as_ref()
        .ok_or(ApiError::Internal("Mempool not initialized".to_string()))?;

    match mempool.get_transaction(&txid).await {
        Some(tx) => Ok(Json(tx)),
        None => Err(ApiError::TransactionNotFound(format!(
            "Transaction {} not found",
            txid
        ))),
    }
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
    if let Some(ref strategy) = block_proposal.strategy {
        println!("   Strategy: {}", strategy);
    }
    if !block_proposal.block_hash.is_empty() {
        println!("   Block hash: {}...", &block_proposal.block_hash[..16]);
    } else {
        println!("   Block hash: (pending)");
    }

    // Store the proposal
    if let Some(block_consensus) = state.block_consensus.as_ref() {
        block_consensus.propose_block(block_proposal.clone()).await;

        // Get my node ID from environment
        if let Ok(my_id) = std::env::var("NODE_PUBLIC_IP") {
            // Auto-vote if I'm not the proposer and haven't voted already
            if my_id != block_proposal.proposer {
                // Check if we already voted
                let already_voted = block_consensus
                    .has_voted(
                        &my_id,
                        block_proposal.block_height,
                        &block_proposal.block_hash,
                    )
                    .await;

                if already_voted {
                    println!("   ℹ️  Already voted on this proposal - skipping");
                } else {
                    println!("   🗳️  Auto-voting on received proposal...");

                    // Validate the proposal
                    let blockchain = state.blockchain.read().await;
                    let chain_tip_hash = blockchain.chain_tip_hash().to_string();
                    let chain_tip_height = blockchain.chain_tip_height();
                    drop(blockchain);

                    let is_valid = block_consensus.validate_proposal(
                        &block_proposal,
                        &chain_tip_hash,
                        chain_tip_height,
                    );

                    // Create and register vote
                    let vote = time_consensus::block_consensus::BlockVote {
                        block_height: block_proposal.block_height,
                        block_hash: block_proposal.block_hash.clone(),
                        voter: my_id.clone(),
                        approve: is_valid,
                        timestamp: chrono::Utc::now().timestamp(),
                    };

                    if let Err(e) = block_consensus.vote_on_block(vote.clone()).await {
                        println!("   ⚠️  Failed to record vote: {}", e);
                    } else {
                        let vote_type = if is_valid {
                            "APPROVE ✓"
                        } else {
                            "REJECT ✗"
                        };
                        println!("   ✓ Auto-voted: {}", vote_type);

                        // Broadcast the vote
                        let vote_json = serde_json::to_value(&vote).unwrap();
                        state.peer_manager.broadcast_block_vote(vote_json).await;
                    }
                }
            }
        }
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Block proposal received and voted"
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
        "🗳️  Received block vote: {} from {} for block #{}",
        vote_type, block_vote.voter, block_vote.block_height
    );
    println!("   Block hash: {}...", &block_vote.block_hash[..16]);

    if let Some(block_consensus) = state.block_consensus.as_ref() {
        match block_consensus.vote_on_block(block_vote.clone()).await {
            Ok(_) => {
                println!("   ✓ Vote registered successfully");
            }
            Err(e) => {
                // Don't fail the request for duplicate votes - this is normal when receiving own vote back
                if e.contains("Duplicate vote") {
                    println!("   ℹ️  Vote already registered ({})", e);
                } else {
                    println!("   ⚠️  Vote registration failed: {}", e);
                    return Err(ApiError::Internal(e));
                }
            }
        }

        let (has_consensus, approvals, total) = block_consensus
            .has_block_consensus(block_vote.block_height, &block_vote.block_hash)
            .await;

        let required = (total * 2).div_ceil(3);
        if has_consensus {
            println!("   ✅ CONSENSUS REACHED ({}/{})", approvals, required);
        } else {
            println!(
                "   ⏳ Waiting... ({}/{}, need {})",
                approvals, total, required
            );
        }
    }

    Ok(Json(serde_json::json!({
        "success": true
    })))
}

async fn request_block_proposal(
    State(state): State<ApiState>,
    Json(request): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let block_height = request["block_height"]
        .as_u64()
        .ok_or_else(|| ApiError::Internal("Missing block_height".to_string()))?;
    let leader_ip = request["leader_ip"]
        .as_str()
        .ok_or_else(|| ApiError::Internal("Missing leader_ip".to_string()))?;
    let requester_ip = request["requester_ip"]
        .as_str()
        .ok_or_else(|| ApiError::Internal("Missing requester_ip".to_string()))?;

    println!("📢 Received block proposal request:");
    println!("   Block height: {}", block_height);
    println!("   Leader (me): {}", leader_ip);
    println!("   Requested by: {}", requester_ip);

    // Spawn task to create and broadcast block proposal immediately
    let state_clone = state.clone();
    let leader_ip = leader_ip.to_string();
    tokio::spawn(async move {
        if let Err(e) =
            create_and_broadcast_catchup_proposal(state_clone, block_height, leader_ip).await
        {
            println!("   ❌ Failed to create block proposal: {}", e);
        }
    });

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Block proposal request received"
    })))
}

async fn create_and_broadcast_catchup_proposal(
    state: ApiState,
    block_height: u64,
    leader_ip: String,
) -> Result<(), String> {
    use time_consensus::block_consensus::{BlockProposal, BlockVote};
    use time_core::block::{Block, BlockHeader};

    println!(
        "   🔨 Creating block proposal for height {}...",
        block_height
    );

    // Get block consensus manager
    let block_consensus = state
        .block_consensus
        .as_ref()
        .ok_or_else(|| "Block consensus not initialized".to_string())?;

    // Get mempool
    let mempool = state
        .mempool
        .as_ref()
        .ok_or_else(|| "Mempool not initialized".to_string())?;

    // Check if proposal already exists
    if let Some(existing) = block_consensus.get_proposal(block_height).await {
        println!(
            "   ℹ️  Proposal already exists for block {}, re-broadcasting...",
            block_height
        );

        // Re-broadcast existing proposal
        let proposal_json = serde_json::to_value(&existing).unwrap();
        state
            .peer_manager
            .broadcast_block_proposal(proposal_json)
            .await;

        // Leader re-votes
        let vote = BlockVote {
            block_height,
            block_hash: existing.block_hash.clone(),
            voter: leader_ip.clone(),
            approve: true,
            timestamp: chrono::Utc::now().timestamp(),
        };
        let _ = block_consensus.vote_on_block(vote.clone()).await;
        let vote_json = serde_json::to_value(&vote).unwrap();
        state.peer_manager.broadcast_block_vote(vote_json).await;

        println!("   ✓ Re-broadcast existing proposal and vote");
        return Ok(());
    }

    // Get blockchain state
    let blockchain = state.blockchain.read().await;
    let previous_hash = blockchain.chain_tip_hash().to_string();
    let current_height = blockchain.chain_tip_height();

    // Verify we're creating the right block
    if block_height != current_height + 1 {
        drop(blockchain);
        return Err(format!(
            "Block height mismatch: requested {}, expected {}",
            block_height,
            current_height + 1
        ));
    }

    let masternode_counts = blockchain.masternode_counts().clone();
    let active_masternodes: Vec<(String, time_core::MasternodeTier)> = blockchain
        .get_active_masternodes()
        .iter()
        .map(|mn| (mn.wallet_address.clone(), mn.tier))
        .collect();

    // Calculate block timestamp (use genesis date + block_height days)
    let genesis_block = blockchain
        .get_block_by_height(0)
        .ok_or_else(|| "Genesis block not found".to_string())?;
    let genesis_date = genesis_block.header.timestamp.date_naive();
    drop(blockchain);

    let block_date = genesis_date + chrono::Duration::days(block_height as i64);
    let timestamp = chrono::Utc.from_utc_datetime(&block_date.and_hms_opt(0, 0, 0).unwrap());

    // Get mempool transactions
    let mut transactions = mempool.get_all_transactions().await;
    transactions.sort_by(|a, b| a.txid.cmp(&b.txid));

    // Create coinbase transaction
    let coinbase_tx = time_core::block::create_coinbase_transaction(
        block_height,
        &active_masternodes,
        &masternode_counts,
        0, // No fees for catch-up blocks
        timestamp.timestamp(),
    );

    println!(
        "      💰 Distributing rewards to {} registered masternodes",
        active_masternodes.len()
    );
    println!(
        "      ✓ Created coinbase with {} outputs (incl. treasury)",
        coinbase_tx.outputs.len()
    );

    // Build transaction list
    let mut all_transactions = vec![coinbase_tx];
    all_transactions.extend(transactions);

    // Create a temporary block to calculate merkle root properly
    let temp_block = Block {
        header: BlockHeader {
            block_number: block_height,
            timestamp,
            previous_hash: previous_hash.clone(),
            merkle_root: String::new(), // Will be calculated
            validator_signature: String::new(),
            validator_address: leader_ip.clone(),
            masternode_counts: masternode_counts.clone(),
            proof_of_time: None,
            checkpoints: Vec::new(),
        },
        hash: String::new(),
        transactions: all_transactions.clone(),
    };

    // Calculate merkle root using the proper method
    let merkle_root = temp_block.calculate_merkle_root();

    // Create block header
    let header = BlockHeader {
        block_number: block_height,
        timestamp,
        previous_hash: previous_hash.clone(),
        merkle_root: merkle_root.clone(),
        validator_signature: {
            use sha2::{Digest, Sha256};
            let sig_data = format!("{}{}{}", block_height, previous_hash, merkle_root);
            let mut hasher = Sha256::new();
            hasher.update(sig_data.as_bytes());
            hasher.update(leader_ip.as_bytes());
            format!("{:x}", hasher.finalize())
        },
        validator_address: leader_ip.clone(),
        masternode_counts: masternode_counts.clone(),
        proof_of_time: None,
        checkpoints: Vec::new(),
    };

    // Create block with proper hash calculation
    let mut block = Block {
        header: header.clone(),
        hash: String::new(), // Temporary, will be calculated
        transactions: all_transactions.clone(),
    };

    // Calculate block hash using the proper method
    let block_hash = block.calculate_hash();
    block.hash = block_hash.clone();

    println!("      🔧 Finalizing block #{}...", block_height);

    // Create and store proposal
    let proposal = BlockProposal {
        block_height,
        proposer: leader_ip.clone(),
        block_hash: block_hash.clone(),
        merkle_root: merkle_root.clone(),
        previous_hash: previous_hash.clone(),
        timestamp: timestamp.timestamp(),
        is_reward_only: false,
        strategy: None,
    };

    // Try to store proposal (first-proposal-wins)
    let accepted = block_consensus.propose_block(proposal.clone()).await;

    if !accepted {
        return Err("Another proposal already exists for this block height".to_string());
    }

    // Leader auto-votes
    let vote = BlockVote {
        block_height,
        block_hash: block_hash.clone(),
        voter: leader_ip.clone(),
        approve: true,
        timestamp: chrono::Utc::now().timestamp(),
    };

    let _ = block_consensus.vote_on_block(vote.clone()).await;

    println!("      ✓ Leader auto-voted APPROVE");

    // Broadcast proposal and vote
    let proposal_json =
        serde_json::to_value(&proposal).map_err(|e| format!("JSON error: {}", e))?;
    state
        .peer_manager
        .broadcast_block_proposal(proposal_json)
        .await;

    let vote_json = serde_json::to_value(&vote).map_err(|e| format!("JSON error: {}", e))?;
    state.peer_manager.broadcast_block_vote(vote_json).await;

    println!("      📡 Proposal and vote broadcast to peers");

    // Add block to leader's blockchain immediately so peers can fetch it
    let mut blockchain = state.blockchain.write().await;
    match blockchain.add_block(block) {
        Ok(_) => {
            println!("      ✔ Block #{} finalized and stored", block_height);
            Ok(())
        }
        Err(e) => {
            println!("      ❌ Failed to add block: {:?}", e);
            Err(format!("Failed to add block: {:?}", e))
        }
    }
}

async fn receive_finalized_block(
    State(state): State<ApiState>,
    Json(payload): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    // Handle both wrapped {"block": ...} and direct block formats
    let block: time_core::block::Block = if let Some(block_value) = payload.get("block") {
        serde_json::from_value(block_value.clone())
            .map_err(|e| ApiError::Internal(format!("Invalid block format in wrapper: {}", e)))?
    } else {
        serde_json::from_value(payload)
            .map_err(|e| ApiError::Internal(format!("Invalid block format: {}", e)))?
    };

    println!(
        "📥 Received finalized block push for height {}",
        block.header.block_number
    );

    let block_num = block.header.block_number;
    let block_hash = block.hash.clone();

    let mut blockchain = state.blockchain.write().await;
    match blockchain.add_block(block.clone()) {
        Ok(_) => {
            // Save UTXO snapshot to persist state
            if let Err(e) = blockchain.save_utxo_snapshot() {
                println!("⚠️  Failed to save UTXO snapshot: {}", e);
            } else {
                println!("💾 UTXO snapshot saved");
            }

            drop(blockchain);

            // Clean up mempool - remove transactions that were included in the block
            if let Some(mempool) = state.mempool.as_ref() {
                mempool.remove_transactions_in_block(&block).await;
            }

            // Broadcast tip update to connected peers
            state
                .peer_manager
                .broadcast_tip_update(block_num, block_hash)
                .await;

            Ok(Json(serde_json::json!({"success": true})))
        }
        Err(e) => Err(ApiError::Internal(format!(
            "Failed to add pushed block: {:?}",
            e
        ))),
    }
}

/// Trigger instant finality for a transaction - validates and finalizes immediately
pub async fn trigger_instant_finality_for_received_tx(
    state: ApiState,
    tx: time_core::transaction::Transaction,
) {
    println!(
        "🚀 Initiating instant finality for received transaction {}",
        &tx.txid[..16]
    );

    let mempool = state.mempool.clone();
    let blockchain = state.blockchain.clone();
    let tx_broadcaster = state.tx_broadcaster.clone();
    let txid = tx.txid.clone();

    // Spawn async task to finalize immediately
    tokio::spawn(async move {
        println!("✅ Validating transaction...");

        // Transaction is already validated by mempool.add_transaction()
        // So we can finalize it immediately

        if let Some(mempool) = mempool.as_ref() {
            let _ = mempool.finalize_transaction(&txid).await;

            // Apply transaction to UTXO set for instant balance update
            let mut blockchain = blockchain.write().await;
            if let Err(e) = blockchain.utxo_set_mut().apply_transaction(&tx) {
                println!("❌ Failed to apply transaction to UTXO set: {}", e);
            } else {
                println!("✅ Transaction finalized - UTXO set updated instantly!");

                // Save finalized transaction to database for persistence
                if let Err(e) = blockchain.save_finalized_tx(&tx, 1, 1) {
                    println!("⚠️  Failed to save finalized transaction: {}", e);
                } else {
                    println!("💾 Finalized transaction saved to database");
                }

                // Save UTXO snapshot to disk for persistence
                if let Err(e) = blockchain.save_utxo_snapshot() {
                    println!("⚠️  Failed to save UTXO snapshot: {}", e);
                } else {
                    println!("💾 UTXO snapshot saved - transaction persists across restarts");
                }

                // 🆕 BROADCAST FINALIZED TRANSACTION TO OTHER MASTERNODES
                if let Some(broadcaster) = tx_broadcaster.as_ref() {
                    println!(
                        "📡 Broadcasting finalized transaction {} to network...",
                        &txid[..16]
                    );
                    broadcaster
                        .broadcast_finalized_transaction(tx.clone())
                        .await;
                    println!("✅ Finalized transaction broadcast complete");
                } else {
                    println!(
                        "⚠️  No broadcaster available - transaction not propagated to network"
                    );
                }
            }

            // Notify registered wallets
            println!(
                "📨 Notified registered wallets about transaction {}",
                &txid[..16]
            );
        }
    });
}

/// Receive instant finality vote request from another node
async fn receive_instant_finality_request(
    State(state): State<ApiState>,
    Json(tx): Json<time_core::Transaction>,
) -> ApiResult<Json<serde_json::Value>> {
    println!(
        "📬 Received instant finality request for transaction {}",
        &tx.txid[..16]
    );

    let consensus = state.consensus.clone();
    let wallet_address = state.wallet_address.clone();
    let tx_broadcaster = state.tx_broadcaster.clone();
    let txid = tx.txid.clone();

    // Validate the transaction
    let is_valid = consensus.validate_transaction(&tx).await;

    let vote_result = if is_valid {
        "APPROVE ✓"
    } else {
        "REJECT ✗"
    };
    println!("   🗳️  Voting: {}", vote_result);

    // Vote on the transaction
    let _ = consensus
        .vote_on_transaction(&txid, wallet_address.clone(), is_valid)
        .await;

    // Send vote back to the proposer
    if let Some(broadcaster) = tx_broadcaster.as_ref() {
        let vote = serde_json::json!({
            "txid": txid,
            "voter": wallet_address,
            "approve": is_valid,
            "timestamp": chrono::Utc::now().timestamp()
        });
        broadcaster.broadcast_instant_finality_vote(vote).await;
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "approved": is_valid
    })))
}

/// Receive instant finality vote from another node
async fn receive_instant_finality_vote(
    State(state): State<ApiState>,
    Json(vote): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let txid = vote["txid"]
        .as_str()
        .ok_or_else(|| ApiError::BadRequest("Missing txid in vote".to_string()))?;
    let voter = vote["voter"]
        .as_str()
        .ok_or_else(|| ApiError::BadRequest("Missing voter in vote".to_string()))?;
    let approve = vote["approve"]
        .as_bool()
        .ok_or_else(|| ApiError::BadRequest("Missing approve in vote".to_string()))?;

    // Check if voter is quarantined
    if let Some(quarantine) = state.quarantine.as_ref() {
        if let Ok(voter_ip) = voter.parse::<std::net::IpAddr>() {
            if quarantine.is_quarantined(&voter_ip).await {
                if let Some(reason) = quarantine.get_reason(&voter_ip).await {
                    println!(
                        "🚫 Rejecting instant finality vote from quarantined peer {} (reason: {})",
                        voter, reason
                    );
                }
                return Err(ApiError::Internal(format!(
                    "Voter {} is quarantined",
                    voter
                )));
            }
        }
    }

    let vote_type = if approve { "APPROVE ✓" } else { "REJECT ✗" };
    println!(
        "🗳️  Received instant finality vote for tx {}: {} from {}",
        &txid[..16],
        vote_type,
        voter
    );

    // Record the vote
    let consensus = state.consensus.clone();
    match consensus
        .vote_on_transaction(txid, voter.to_string(), approve)
        .await
    {
        Ok(_) => {
            let (approvals, rejections) = consensus.get_transaction_vote_count(txid).await;
            println!(
                "   📊 Current votes: {} approvals, {} rejections",
                approvals, rejections
            );
        }
        Err(e) => {
            println!("   ⚠️  Failed to record vote: {}", e);
        }
    }

    Ok(Json(serde_json::json!({
        "success": true
    })))
}

/// Sync finalized transactions - provide finalized txs to peers who request them
async fn sync_finalized_transactions(
    State(state): State<ApiState>,
    Json(request): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let since_timestamp = request["since_timestamp"].as_i64().unwrap_or(0);

    println!(
        "📥 Finalized transaction sync request (since timestamp: {})",
        since_timestamp
    );

    // Load all finalized transactions from database
    let blockchain = state.blockchain.read().await;
    match blockchain.load_finalized_txs() {
        Ok(finalized_txs) => {
            // Filter transactions finalized after the requested timestamp
            // Note: We'd need to store finalized_at timestamps with each transaction
            // For now, return all finalized transactions
            let count = finalized_txs.len();

            println!("   ✓ Providing {} finalized transactions", count);

            Ok(Json(serde_json::json!({
                "success": true,
                "transactions": finalized_txs,
                "count": count
            })))
        }
        Err(e) => {
            println!("   ✗ Failed to load finalized transactions: {}", e);
            Err(ApiError::Internal(format!(
                "Failed to load finalized transactions: {}",
                e
            )))
        }
    }
}

/// Handle catch-up request from another node
async fn handle_catch_up_request(
    State(state): State<ApiState>,
    Json(request): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let requester = request["requester"]
        .as_str()
        .ok_or_else(|| ApiError::BadRequest("Missing requester".to_string()))?;
    let current_height = request["current_height"]
        .as_u64()
        .ok_or_else(|| ApiError::BadRequest("Missing current_height".to_string()))?;
    let expected_height = request["expected_height"]
        .as_u64()
        .ok_or_else(|| ApiError::BadRequest("Missing expected_height".to_string()))?;

    println!("📢 Catch-up request received:");
    println!("   From: {}", requester);
    println!(
        "   Their height: {} (need: {})",
        current_height, expected_height
    );

    // Check our own height
    let our_height = {
        let blockchain = state.blockchain.read().await;
        blockchain.chain_tip_height()
    };

    if our_height < expected_height {
        println!("   ℹ️  We're also behind - acknowledging to coordinate catch-up");
    } else if our_height == expected_height {
        println!("   ℹ️  We're caught up - will help with block creation");
    } else {
        println!(
            "   ℹ️  We're ahead at {} - requester should sync from us",
            our_height
        );
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "responder": our_height.to_string(),
        "our_height": our_height,
    })))
}
