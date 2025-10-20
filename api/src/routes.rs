//! API Routes

use crate::{grant_handlers, handlers, state::ApiState};
use axum::{
    routing::{get, post},
    Router,
};

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
        // Grant System
        .route("/grant/apply", post(grant_handlers::apply_for_grant))
        .route("/grant/verify/:token", get(grant_handlers::verify_grant))
        .route(
            "/grant/status/:email",
            get(grant_handlers::get_grant_status),
        )
        .route(
            "/masternode/activate",
            post(grant_handlers::activate_masternode),
        )
        .route(
            "/masternode/decommission",
            post(grant_handlers::decommission_masternode),
        )
        // Admin
        .route(
            "/admin/emails/export",
            get(grant_handlers::export_email_list),
        )
        .with_state(state)
}
