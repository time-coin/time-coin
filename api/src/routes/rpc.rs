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
        .route("/gettimeblockinfo", post(rpc_handlers::gettimeblockinfo))
        .route(
            "/gettimeblockrewards",
            post(rpc_handlers::gettimeblockrewards),
        )
        // Transaction operations
        .route("/getrawtransaction", post(rpc_handlers::getrawtransaction))
        .route(
            "/sendrawtransaction",
            post(rpc_handlers::sendrawtransaction),
        )
        // Wallet operations
        .route("/getwalletinfo", post(rpc_handlers::getwalletinfo))
        .route("/getbalance", post(rpc_handlers::getbalance))
        .route("/getnewaddress", post(rpc_handlers::getnewaddress))
        .route("/validateaddress", post(rpc_handlers::validateaddress))
        .route("/listunspent", post(rpc_handlers::listunspent))
        // Masternodes
        .route("/getmasternodeinfo", post(rpc_handlers::getmasternodeinfo))
        .route("/listmasternodes", post(rpc_handlers::listmasternodes))
        .route(
            "/getmasternodecount",
            post(rpc_handlers::getmasternodecount),
        )
        // Network
        .route("/getpeerinfo", post(rpc_handlers::getpeerinfo))
        .route("/getnetworkinfo", post(rpc_handlers::getnetworkinfo))
        // Consensus
        .route(
            "/getconsensusstatus",
            post(rpc_handlers::getconsensusstatus),
        )
        // Treasury & Governance
        .route("/gettreasury", post(rpc_handlers::gettreasury))
        .route("/listproposals", post(rpc_handlers::listproposals))
}
