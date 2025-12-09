//! Blockchain information and query endpoints

use crate::balance::calculate_mempool_balance;
use crate::constants::satoshis_to_time;
use crate::{ApiError, ApiResult, ApiState, BlockchainService};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

/// Register blockchain routes
pub fn blockchain_routes() -> Router<ApiState> {
    Router::new()
        .route("/info", get(get_blockchain_info))
        .route("/height", get(get_blockchain_height))
        .route("/block/{height}", get(get_block_by_height))
        .route("/block/{height}/hash", get(get_block_hash_by_height))
        .route("/balance/{address}", get(get_balance))
        .route("/utxos/{address}", get(get_utxos_by_address))
        .route("/reindex", post(reindex_blockchain))
}

#[derive(Serialize)]
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

    // Calculate total supply from UTXO set
    let total_supply: u64 = blockchain
        .utxo_set()
        .utxos()
        .values()
        .map(|output| output.amount)
        .sum();

    Ok(Json(BlockchainInfoResponse {
        network: state.network.clone(),
        height: blockchain.chain_tip_height(),
        best_block_hash: blockchain.chain_tip_hash().to_string(),
        total_supply,
        timestamp: chrono::Utc::now().timestamp(),
        wallet_address: state.wallet_address.clone(),
    }))
}

/// Get current blockchain height (Phase 3 endpoint)
async fn get_blockchain_height(State(state): State<ApiState>) -> ApiResult<Json<u64>> {
    let blockchain = state.blockchain.read().await;
    Ok(Json(blockchain.chain_tip_height()))
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
    let chain_tip = blockchain.chain_tip_height();

    // Validate height is within bounds
    if height > chain_tip {
        return Err(ApiError::BadRequest(format!(
            "Height {} exceeds chain tip {}",
            height, chain_tip
        )));
    }

    match blockchain.get_block_by_height(height) {
        Some(block) => Ok(Json(BlockResponse {
            block: block.clone(),
        })),
        None => Err(ApiError::BlockNotFound(format!(
            "Block at height {} not found",
            height
        ))),
    }
}

/// Get block hash at specific height (Phase 3 endpoint)
async fn get_block_hash_by_height(
    State(state): State<ApiState>,
    Path(height): Path<u64>,
) -> ApiResult<String> {
    let blockchain = state.blockchain.read().await;

    match blockchain.get_block_by_height(height) {
        Some(block) => Ok(block.hash.clone()),
        None => Err(ApiError::BlockNotFound(format!(
            "Block at height {} not found",
            height
        ))),
    }
}

#[derive(Serialize)]
pub struct BalanceResponse {
    address: String,
    balance: u64,
    available_balance: u64,
    balance_time: String,
    available_time: String,
    unconfirmed_balance: u64,
}

async fn get_balance(
    State(state): State<ApiState>,
    Path(address): Path<String>,
) -> ApiResult<Json<BalanceResponse>> {
    // Validate address format
    if !address.starts_with("TIME0") && !address.starts_with("TIME1") {
        return Err(ApiError::InvalidAddress(
            "Address must start with TIME0 (testnet) or TIME1 (mainnet)".into(),
        ));
    }

    // Use service for business logic
    let service = BlockchainService::new(state.blockchain.clone());
    let balance = service.get_balance(&address).await?;
    let available_balance = service.get_available_balance(&address).await?;

    let unconfirmed_balance = if let Some(mempool) = &state.mempool {
        let blockchain = state.blockchain.read().await;
        calculate_mempool_balance(&address, &blockchain, mempool).await
    } else {
        0
    };

    // Convert balances to TIME (8 decimal places)
    let balance_time = format!("{:.8}", satoshis_to_time(balance));
    let available_time = format!("{:.8}", satoshis_to_time(available_balance));

    Ok(Json(BalanceResponse {
        address,
        balance,
        available_balance,
        balance_time,
        available_time,
        unconfirmed_balance,
    }))
}

/// Legacy endpoint for backward compatibility with CLI
pub async fn get_balance_legacy(
    state: State<ApiState>,
    address: Path<String>,
) -> ApiResult<Json<BalanceResponse>> {
    get_balance(state, address).await
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
    // Use service for business logic
    let service = BlockchainService::new(state.blockchain.clone());
    let utxos_data = service.get_utxos(&address).await?;

    let utxos: Vec<UtxoInfo> = utxos_data
        .iter()
        .map(|(outpoint, output)| UtxoInfo {
            txid: outpoint.txid.clone(),
            vout: outpoint.vout,
            amount: output.amount,
        })
        .collect();

    Ok(Json(UtxoResponse { address, utxos }))
}

#[derive(Serialize)]
struct ReindexResponse {
    success: bool,
    blocks_processed: u64,
    utxo_count: usize,
    total_supply: u64,
    wallet_balances: std::collections::HashMap<String, u64>,
    processing_time_ms: u128,
}

/// Reindex the blockchain - rebuild UTXO set from all blocks
async fn reindex_blockchain(State(state): State<ApiState>) -> ApiResult<Json<ReindexResponse>> {
    let start = std::time::Instant::now();

    tracing::info!("starting_blockchain_reindex");

    let mut blockchain = state.blockchain.write().await;

    // Load all blocks from database
    tracing::info!("loading_blocks_from_database");
    let blocks = blockchain
        .db()
        .load_all_blocks()
        .map_err(|e| ApiError::Internal(format!("Failed to load blocks: {}", e)))?;

    if blocks.is_empty() {
        return Err(ApiError::Internal(
            "No blocks found in database".to_string(),
        ));
    }

    tracing::info!(block_count = blocks.len(), "blocks_loaded_successfully");

    // Create a new empty UTXO set
    tracing::info!("rebuilding_utxo_set_from_blocks");
    let mut utxo_set = time_core::utxo_set::UTXOSet::new();

    // Apply each block's transactions to rebuild UTXO set
    for (i, block) in blocks.iter().enumerate() {
        if i % 100 == 0 && i > 0 {
            tracing::info!(
                processed = i,
                total = blocks.len(),
                percent = (i * 100) / blocks.len(),
                "reindex_progress"
            );
        }

        for tx in &block.transactions {
            utxo_set
                .apply_transaction(tx)
                .map_err(|e| ApiError::Internal(format!("Failed to apply transaction: {}", e)))?;
        }
    }

    tracing::info!(utxo_count = utxo_set.len(), "utxo_set_rebuilt_successfully");

    // Replace the blockchain's UTXO set with the rebuilt one
    let snapshot = utxo_set.snapshot();
    blockchain.utxo_set_mut().restore(snapshot.clone());

    // Save UTXO snapshot to database
    tracing::info!("saving_utxo_snapshot");
    blockchain
        .db()
        .save_utxo_snapshot(&utxo_set)
        .map_err(|e| ApiError::Internal(format!("Failed to save UTXO snapshot: {}", e)))?;

    // Calculate and save wallet balances for all addresses
    tracing::info!("saving_wallet_balances");
    let mut wallet_balances = std::collections::HashMap::new();

    for output in utxo_set.utxos().values() {
        if output.address != "TREASURY" && output.address != "BURNED" {
            *wallet_balances.entry(output.address.clone()).or_insert(0) += output.amount;
        }
    }

    for (address, balance) in &wallet_balances {
        blockchain
            .db()
            .save_wallet_balance(address, *balance)
            .map_err(|e| ApiError::Internal(format!("Failed to save wallet balance: {}", e)))?;
    }

    let processing_time = start.elapsed();

    tracing::info!(
        duration_secs = processing_time.as_secs(),
        duration_ms = processing_time.as_millis(),
        blocks_processed = blocks.len(),
        utxos_created = utxo_set.len(),
        total_supply = utxo_set.total_supply(),
        "reindex_completed"
    );

    Ok(Json(ReindexResponse {
        success: true,
        blocks_processed: blocks.len() as u64,
        utxo_count: utxo_set.len(),
        total_supply: utxo_set.total_supply(),
        wallet_balances,
        processing_time_ms: processing_time.as_millis(),
    }))
}
