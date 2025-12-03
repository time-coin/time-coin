//! Standalone Masternode API Server for Wallet Clients
//!
//! This binary starts an HTTP API server that provides wallet services
//! to thin clients (wallet-gui).

use std::sync::Arc;
use time_masternode::api_server::create_router;
use time_masternode::wallet_api::WalletApiHandler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("🚀 Starting Masternode API Server...");

    // Create wallet API handler with test data
    let wallet_handler = Arc::new(WalletApiHandler::new_with_test_data().await);

    // Create router
    let app = create_router(wallet_handler);

    // Start server
    let addr = "127.0.0.1:24100";
    log::info!("📡 Listening on http://{}", addr);
    log::info!("📋 Available endpoints:");
    log::info!("   GET  /wallet/balance?xpub=<xpub>");
    log::info!("   GET  /wallet/transactions?xpub=<xpub>");
    log::info!("   GET  /wallet/utxos?xpub=<xpub>");
    log::info!("   POST /transaction/broadcast");
    log::info!("   GET  /address/<address>");
    log::info!("   GET  /health");
    log::info!("");
    log::info!("💡 Test with:");
    log::info!("   curl 'http://127.0.0.1:24100/wallet/balance?xpub=xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz'");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
