//! TIME Coin Masternode Binary
//! Masternode with TIME Coin Protocol via P2P network and WebSocket bridge for wallets

use std::sync::Arc;
use time_masternode::{MasternodeRegistry, WsBridge};

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

    let bridge_clone = bridge.clone();
    tokio::spawn(async move {
        if let Err(e) = bridge_clone.start().await {
            eprintln!("❌ WebSocket bridge error: {}", e);
        }
    });

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎉 TIME Coin Masternode is running!");
    println!();
    println!("P2P Network: port 24000");
    println!("WebSocket Bridge: ws://{}", ws_addr);
    println!("Press Ctrl+C to stop");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Keep running
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}
