//! Masternode management endpoints

use crate::handlers::get_node_wallet;
use crate::masternode_handlers::{list_masternodes, register_masternode};
use crate::ApiState;
use axum::{
    routing::{get, post},
    Router,
};

/// Register masternode management routes
pub fn masternode_routes() -> Router<ApiState> {
    Router::new()
        .route("/register", post(register_masternode))
        .route("/list", get(list_masternodes))
        .route("/wallet", get(get_node_wallet))
}
