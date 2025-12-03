//! Blockchain information and query endpoints

use crate::balance::calculate_mempool_balance;
use crate::{ApiError, ApiResult, ApiState, BlockchainService};
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;

/// Register blockchain routes
pub fn blockchain_routes() -> Router<ApiState> {
    Router::new()
        .route("/info", get(get_blockchain_info))
        .route("/block/{height}", get(get_block_by_height))
        .route("/balance/{address}", get(get_balance))
        .route("/utxos/{address}", get(get_utxos_by_address))
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
        None => Err(ApiError::BlockNotFound(format!(
            "Block at height {} not found",
            height
        ))),
    }
}

#[derive(Serialize)]
struct BalanceResponse {
    address: String,
    balance: u64,
    balance_time: String,
    unconfirmed_balance: u64,
}

async fn get_balance(
    State(state): State<ApiState>,
    Path(address): Path<String>,
) -> ApiResult<Json<BalanceResponse>> {
    // Use service for business logic
    let service = BlockchainService::new(state.blockchain.clone());
    let balance = service.get_balance(&address).await?;

    let unconfirmed_balance = if let Some(mempool) = &state.mempool {
        let blockchain = state.blockchain.read().await;
        calculate_mempool_balance(&address, &blockchain, mempool).await
    } else {
        0
    };

    // Convert balance to TIME (8 decimal places)
    let balance_time = format!("{:.8}", balance as f64 / 100_000_000.0);

    Ok(Json(BalanceResponse {
        address,
        balance,
        balance_time,
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
