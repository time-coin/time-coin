//! Wallet operations and synchronization endpoints
//!
//! Note: The `/wallet/send` endpoint has been removed because nodes do not have
//! access to private keys. To send transactions:
//! 1. Use the CLI wallet tool (`time-cli wallet send`)
//! 2. Or submit signed transactions via `/transaction/send_raw`

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
        // Wallet synchronization endpoints
        .route("/sync", post(sync_wallet_addresses))
        .route("/sync-xpub", post(sync_wallet_xpub))
        .route("/register-xpub", post(register_xpub))
        // Thin client query endpoints
        .route("/balance", get(get_xpub_balance))
        .route("/transactions", get(get_xpub_transactions))
        .route("/utxos", get(get_xpub_utxos))
}
