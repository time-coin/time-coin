//! API Routes

use axum::{
    routing::{get, post},
    Router,
};
use crate::{handlers, state::ApiState};

pub fn create_router(state: ApiState) -> Router {
    Router::new()
        // Health
        .route("/health", get(handlers::health_check))
        
        // Transactions
        .route("/transaction/create", post(handlers::create_transaction))
        .route("/transaction/:txid", get(handlers::get_transaction))
        
        // Balances
        .route("/balance/:address", get(handlers::get_balance))
        
        // Blockchain
        .route("/blockchain/info", get(handlers::get_blockchain_info))
        
        // Utilities
        .route("/keypair/generate", post(handlers::generate_keypair))
        
        .with_state(state)
}
