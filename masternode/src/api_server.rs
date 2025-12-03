//! HTTP API Server for Masternode Wallet Services
//!
//! Provides REST API endpoints for thin wallet clients.

use crate::wallet_api::{Balance, TransactionRecord, WalletApiHandler, UTXO, AddressInfo};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// API server state
#[derive(Clone)]
pub struct ApiState {
    wallet_handler: Arc<WalletApiHandler>,
}

/// Query parameters for balance endpoint
#[derive(Debug, Deserialize)]
pub struct BalanceQuery {
    pub xpub: String,
}

/// Query parameters for transactions endpoint
#[derive(Debug, Deserialize)]
pub struct TransactionsQuery {
    pub xpub: String,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 {
    100
}

/// Request body for broadcast transaction
#[derive(Debug, Deserialize)]
pub struct BroadcastRequest {
    pub tx: String,
}

/// Response for broadcast transaction
#[derive(Debug, Serialize)]
pub struct BroadcastResponse {
    pub txid: String,
}

/// Standard error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Create the API router
pub fn create_router(wallet_handler: Arc<WalletApiHandler>) -> Router {
    let state = ApiState { wallet_handler };

    Router::new()
        .route("/wallet/balance", get(get_balance))
        .route("/wallet/transactions", get(get_transactions))
        .route("/wallet/utxos", get(get_utxos))
        .route("/transaction/broadcast", post(broadcast_transaction))
        .route("/address/:address", get(get_address_info))
        .route("/health", get(health_check))
        .with_state(state)
}

/// GET /wallet/balance?xpub=<xpub>
async fn get_balance(
    State(state): State<ApiState>,
    Query(params): Query<BalanceQuery>,
) -> Result<Json<Balance>, ApiError> {
    let balance = state
        .wallet_handler
        .get_balance(&params.xpub)
        .await
        .map_err(ApiError::from)?;

    log::info!("📊 Balance query for xpub: {} satoshis", balance.total);

    Ok(Json(balance))
}

/// GET /wallet/transactions?xpub=<xpub>&limit=100
async fn get_transactions(
    State(state): State<ApiState>,
    Query(params): Query<TransactionsQuery>,
) -> Result<Json<Vec<TransactionRecord>>, ApiError> {
    let transactions = state
        .wallet_handler
        .get_transactions(&params.xpub, params.limit)
        .await
        .map_err(ApiError::from)?;

    log::info!(
        "📜 Transaction query for xpub: {} results",
        transactions.len()
    );

    Ok(Json(transactions))
}

/// GET /wallet/utxos?xpub=<xpub>
async fn get_utxos(
    State(state): State<ApiState>,
    Query(params): Query<BalanceQuery>,
) -> Result<Json<Vec<UTXO>>, ApiError> {
    let utxos = state
        .wallet_handler
        .get_utxos(&params.xpub)
        .await
        .map_err(ApiError::from)?;

    log::info!("💰 UTXO query for xpub: {} UTXOs", utxos.len());

    Ok(Json(utxos))
}

/// POST /transaction/broadcast
async fn broadcast_transaction(
    State(state): State<ApiState>,
    Json(request): Json<BroadcastRequest>,
) -> Result<Json<BroadcastResponse>, ApiError> {
    let txid = state
        .wallet_handler
        .broadcast_transaction(&request.tx)
        .await
        .map_err(ApiError::from)?;

    log::info!("📡 Transaction broadcast: {}", txid);

    Ok(Json(BroadcastResponse { txid }))
}

/// GET /address/:address
async fn get_address_info(
    State(state): State<ApiState>,
    Path(address): Path<String>,
) -> Result<Json<AddressInfo>, ApiError> {
    let info = state
        .wallet_handler
        .get_address_info(&address)
        .await
        .map_err(ApiError::from)?;

    log::info!("🔍 Address query: {} (balance: {})", address, info.balance);

    Ok(Json(info))
}

/// GET /health
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "time-masternode-wallet-api",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// API error wrapper
#[derive(Debug)]
pub struct ApiError {
    message: String,
    status: StatusCode,
}

impl From<crate::error::MasternodeError> for ApiError {
    fn from(err: crate::error::MasternodeError) -> Self {
        ApiError {
            message: err.to_string(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(ErrorResponse {
            error: self.message,
        });

        (self.status, body).into_response()
    }
}

/// Start the API server
pub async fn start_server(
    wallet_handler: Arc<WalletApiHandler>,
    bind_addr: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = create_router(wallet_handler);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    
    log::info!("🚀 Masternode Wallet API listening on {}", bind_addr);
    log::info!("📡 Endpoints:");
    log::info!("   GET  /wallet/balance?xpub=<xpub>");
    log::info!("   GET  /wallet/transactions?xpub=<xpub>&limit=100");
    log::info!("   GET  /wallet/utxos?xpub=<xpub>");
    log::info!("   POST /transaction/broadcast");
    log::info!("   GET  /address/:address");
    log::info!("   GET  /health");

    axum::serve(listener, app).await?;

    Ok(())
}
