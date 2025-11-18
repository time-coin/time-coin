//! TIME Coin Masternode Binary
//! Starts the masternode with WebSocket server for TIME Coin Protocol

use std::sync::Arc;
use time_masternode::{MasternodeRegistry, WsServer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting TIME Coin Masternode...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Initialize masternode registry
    let _registry = Arc::new(MasternodeRegistry::new());
    println!("✅ Masternode registry initialized");

    // Create WebSocket server for TIME Coin Protocol
    let ws_addr = std::env::var("WS_ADDR").unwrap_or_else(|_| "0.0.0.0:8765".to_string());

    let ws_server = Arc::new(WsServer::new(ws_addr.clone()));
    println!("✅ WebSocket server configured on {}", ws_addr);

    // Start WebSocket server
    let ws_server_clone = ws_server.clone();
    let ws_task = tokio::spawn(async move {
        if let Err(e) = ws_server_clone.start().await {
            eprintln!("❌ WebSocket server error: {}", e);
        }
    });

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎉 TIME Coin Masternode is running!");
    println!();
    println!("WebSocket: ws://{}", ws_addr);
    println!("Press Ctrl+C to stop");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Wait for WebSocket task
    ws_task.await?;

    Ok(())
}
