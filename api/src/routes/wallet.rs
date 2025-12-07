//! Wallet operations and synchronization endpoints

use crate::wallet_send_handler::wallet_send;
use crate::wallet_sync_handlers::{
    get_xpub_balance, get_xpub_transactions, get_xpub_utxos, register_xpub, sync_wallet_addresses,
    sync_wallet_xpub,
};
use crate::ApiState;
use axum::{
    routing::{get, post},
    Router,
};

/// Register wallet operation routes
pub fn wallet_routes() -> Router<ApiState> {
    Router::new()
        // Node wallet transaction sending
        .route("/send", post(wallet_send))
        // Wallet synchronization endpoints
        .route("/sync", post(sync_wallet_addresses))
        .route("/sync-xpub", post(sync_wallet_xpub))
        .route("/register-xpub", post(register_xpub))
        // Thin client query endpoints
        .route("/balance", get(get_xpub_balance))
        .route("/transactions", get(get_xpub_transactions))
        .route("/utxos", get(get_xpub_utxos))
}
