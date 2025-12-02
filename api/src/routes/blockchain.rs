//! Blockchain information and query endpoints

use crate::balance::calculate_mempool_balance;
use crate::{ApiError, ApiResult, ApiState};
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
        .route("/block/:height", get(get_block_by_height))
        .route("/balance/:address", get(get_balance))
        .route("/utxos/:address", get(get_utxos_by_address))
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
    unconfirmed_balance: u64,
}

async fn get_balance(
    State(state): State<ApiState>,
    Path(address): Path<String>,
) -> ApiResult<Json<BalanceResponse>> {
    let blockchain = state.blockchain.read().await;
    let balance = blockchain.get_balance(&address);

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
