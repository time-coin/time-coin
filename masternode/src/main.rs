//! TIME Coin Masternode Binary
//! Masternode with TIME Coin Protocol via P2P network and WebSocket bridge for wallets

use std::sync::Arc;
use time_consensus::utxo_state_protocol::UTXOStateManager;
use time_masternode::{MasternodeRegistry, MasternodeUTXOIntegration, WsBridge};
use time_network::{discovery::NetworkType, PeerManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("🚀 Starting TIME Coin Masternode...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Initialize masternode registry
    let _registry = Arc::new(MasternodeRegistry::new());
    println!("✅ Masternode registry initialized");

    // Start WebSocket bridge for wallet connections
    let ws_addr = std::env::var("WS_ADDR").unwrap_or_else(|_| "0.0.0.0:24002".to_string());
    let bridge = Arc::new(WsBridge::new(ws_addr.clone()));
    println!("✅ WebSocket bridge configured on {}", ws_addr);

    // Initialize UTXO state manager
    let node_id = "masternode-1".to_string();
    let utxo_manager = Arc::new(UTXOStateManager::new(node_id.clone()));
    println!("✅ UTXO state manager initialized");

    // Initialize P2P network peer manager
    let p2p_addr = std::env::var("P2P_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:24000".to_string())
        .parse()
        .expect("Invalid P2P address");
    let peer_manager = Arc::new(PeerManager::new(NetworkType::Mainnet, p2p_addr, p2p_addr));
    println!("✅ P2P network manager initialized on {}", p2p_addr);

    // Initialize UTXO integration and connect WebSocket bridge
    let mut utxo_integration =
        MasternodeUTXOIntegration::new(node_id.clone(), utxo_manager.clone(), peer_manager.clone());
    utxo_integration.set_ws_bridge(bridge.clone());
    let utxo_integration = Arc::new(utxo_integration);

    if let Err(e) = utxo_integration.initialize().await {
        eprintln!("❌ Failed to initialize UTXO integration: {}", e);
        return Err(e.into());
    }
    println!("✅ UTXO integration initialized with WebSocket bridge");

    // Start WebSocket server
    let bridge_clone = bridge.clone();
    tokio::spawn(async move {
        if let Err(e) = bridge_clone.start().await {
            eprintln!("❌ WebSocket bridge error: {}", e);
        }
    });

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎉 TIME Coin Masternode is running!");
    println!();
    println!("P2P Network: {}", p2p_addr);
    println!("WebSocket Bridge: ws://{}", ws_addr);
    println!("Node ID: {}", node_id);
    println!("Press Ctrl+C to stop");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Keep running
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}
