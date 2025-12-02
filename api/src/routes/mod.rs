//! API routes organization
//!
//! This module contains all HTTP route definitions organized by domain:
//! - `blockchain` - Chain info, blocks, balances, UTXOs
//! - `mempool` - Transaction pool management  
//! - `network` - Peer management and synchronization
//! - `consensus` - Block proposals and voting
//! - `treasury` - Treasury management and proposals
//! - `wallet` - Wallet operations and synchronization
//! - `masternode` - Masternode registration and management
//! - `rpc` - Bitcoin RPC-compatible endpoints
//!
//! Each submodule is responsible for its own domain and exports a router function.

mod blockchain;
mod consensus;
mod masternode;
pub mod mempool;
mod network;
mod rpc;
mod treasury;
mod wallet;

use crate::ApiState;
use axum::{routing::get, Json, Router};
use serde::Serialize;

/// Create the main router with all API endpoints
///
/// Routes are organized by domain and nested under appropriate prefixes.
pub fn create_routes() -> Router<ApiState> {
    Router::new()
        // Core application routes
        .route("/", get(root))
        .route("/health", get(health_check))
        // Domain-specific route groups
        .nest("/blockchain", blockchain::blockchain_routes())
        .nest("/mempool", mempool::mempool_routes())
        .nest("/network", network::network_routes())
        .nest("/consensus", consensus::consensus_routes())
        .nest("/treasury", treasury::treasury_routes())
        .nest("/wallet", wallet::wallet_routes())
        .nest("/masternode", masternode::masternode_routes())
        .nest("/rpc", rpc::rpc_routes())
        // Legacy route redirects for backward compatibility
        .route(
            "/masternodes/list",
            get(|| async { axum::response::Redirect::permanent("/masternode/list") }),
        )
        .route(
            "/node/wallet",
            get(|| async { axum::response::Redirect::permanent("/masternode/wallet") }),
        )
}

// Root endpoints

async fn root() -> &'static str {
    "TIME Coin API"
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}
