//! Wallet operations and synchronization endpoints

use crate::wallet_send_handler::wallet_send;
use crate::wallet_sync_handlers::{register_xpub, sync_wallet_addresses, sync_wallet_xpub};
use crate::ApiState;
use axum::{routing::post, Router};

/// Register wallet operation routes
pub fn wallet_routes() -> Router<ApiState> {
    Router::new()
        .route("/sync", post(sync_wallet_addresses))
        .route("/sync-xpub", post(sync_wallet_xpub))
        .route("/register-xpub", post(register_xpub))
        .route("/send", post(wallet_send))
}
