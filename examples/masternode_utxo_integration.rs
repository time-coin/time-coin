//! Example: Masternode with UTXO Protocol Integration
//!
//! Demonstrates how to set up a masternode with full UTXO state tracking,
//! wallet subscriptions, and instant finality.

use std::sync::Arc;
use time_core::utxo_state_manager::UTXOStateManager;
use time_masternode::MasternodeUTXOIntegration;
use time_network::discovery::NetworkType;
use time_network::PeerManager;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("Starting masternode with UTXO protocol integration");

    // 1. Set up P2P network layer
    let listen_addr = "0.0.0.0:24000".parse()?;
    let public_addr = "127.0.0.1:24000".parse()?;
    let peer_manager = Arc::new(PeerManager::new(
        NetworkType::Testnet,
        listen_addr,
        public_addr,
    ));

    // 2. Set up UTXO state manager
    let node_id = "masternode-1".to_string();
    let utxo_manager = Arc::new(UTXOStateManager::new(node_id.clone()));

    // 3. Create integration layer
    let utxo_integration = MasternodeUTXOIntegration::new(
        node_id.clone(),
        utxo_manager.clone(),
        peer_manager.clone(),
    );

    // 4. Initialize integration (sets up notification routing)
    utxo_integration.initialize().await?;

    info!("UTXO protocol integration initialized successfully");

    // 5. Show statistics
    let stats = utxo_integration.get_utxo_stats().await;
    info!("UTXO Statistics:");
    info!("  Total UTXOs: {}", stats.total_utxos);
    info!("  Unspent: {}", stats.unspent);
    info!("  Locked: {}", stats.locked);
    info!("  Active Subscriptions: {}", stats.active_subscriptions);

    info!("Masternode UTXO integration example completed");
    Ok(())
}
