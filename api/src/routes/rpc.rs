//! Bitcoin RPC-compatible endpoints

use crate::{rpc_handlers, ApiState};
use axum::{routing::post, Router};

/// Create RPC-compatible routes
pub fn rpc_routes() -> Router<ApiState> {
    Router::new()
        // Blockchain info
        .route("/getblockchaininfo", post(rpc_handlers::getblockchaininfo))
        .route("/getblockcount", post(rpc_handlers::getblockcount))
        .route("/getblock", post(rpc_handlers::getblock))
        .route("/getblockhash", post(rpc_handlers::getblockhash))
        // Wallet operations
        .route("/getwalletinfo", post(rpc_handlers::getwalletinfo))
        .route("/getbalance", post(rpc_handlers::getbalance))
        .route("/getnewaddress", post(rpc_handlers::getnewaddress))
        // Masternodes
        .route(
            "/getmasternodecount",
            post(rpc_handlers::getmasternodecount),
        )
        // Consensus
        .route(
            "/getconsensusstatus",
            post(rpc_handlers::getconsensusstatus),
        )
        // Treasury & Governance
        .route("/gettreasury", post(rpc_handlers::gettreasury))
        .route("/listproposals", post(rpc_handlers::listproposals))
}
