//! TIME Coin Masternode Binary
//! Masternode with TIME Coin Protocol via P2P network and WebSocket bridge for wallets

use std::sync::Arc;
use time_consensus::utxo_state_protocol::UTXOStateManager;
use time_masternode::{
    address_monitor::AddressMonitor, MasternodeRegistry, MasternodeUTXOIntegration, WsBridge,
};
use time_network::{connection::PeerListener, discovery::NetworkType, PeerManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("🚀 Starting TIME Coin Masternode...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Initialize masternode registry
    let _registry = Arc::new(MasternodeRegistry::new());
    println!("✅ Masternode registry initialized");

    // Initialize address monitor for xpub tracking
    let address_monitor = Arc::new(AddressMonitor::new());
    println!("✅ Address monitor initialized");

    // Start WebSocket bridge for wallet connections
    let ws_addr = std::env::var("WS_ADDR").unwrap_or_else(|_| "0.0.0.0:24002".to_string());
    let bridge = Arc::new(WsBridge::new(ws_addr.clone()));
    println!("✅ WebSocket bridge configured on {}", ws_addr);

    // Initialize UTXO state manager
    let node_id = "masternode-1".to_string();
    let utxo_manager = Arc::new(UTXOStateManager::new(node_id.clone()));
    println!("✅ UTXO state manager initialized");

    // Initialize P2P network peer manager
    // Default to testnet for development
    let network_type = if std::env::var("NETWORK").as_deref() == Ok("mainnet") {
        NetworkType::Mainnet
    } else {
        NetworkType::Testnet
    };

    let default_port = match network_type {
        NetworkType::Mainnet => "0.0.0.0:24000",
        NetworkType::Testnet => "0.0.0.0:24100",
    };

    let p2p_addr = std::env::var("P2P_ADDR")
        .unwrap_or_else(|_| default_port.to_string())
        .parse()
        .expect("Invalid P2P address");
    let peer_manager = Arc::new(PeerManager::new(network_type, p2p_addr, p2p_addr));
    println!(
        "✅ P2P network manager initialized on {} ({:?})",
        p2p_addr, network_type
    );

    // Initialize UTXO integration and connect WebSocket bridge and address monitor
    let mut utxo_integration =
        MasternodeUTXOIntegration::new(node_id.clone(), utxo_manager.clone(), peer_manager.clone());
    utxo_integration.set_ws_bridge(bridge.clone());
    utxo_integration.set_address_monitor(address_monitor.clone());
    let utxo_integration = Arc::new(utxo_integration);

    if let Err(e) = utxo_integration.initialize().await {
        eprintln!("❌ Failed to initialize UTXO integration: {}", e);
        return Err(e.into());
    }
    println!("✅ UTXO integration initialized with WebSocket bridge");

    // Start vote cleanup task
    utxo_integration.start_cleanup_task();
    println!("✅ Vote cleanup task started");

    // Start mempool synchronization task
    utxo_integration.start_mempool_sync_task();
    println!("✅ Mempool sync task started");

    // Start finality retry task for unfinalized transactions
    utxo_integration.start_finality_retry_task();
    println!("✅ Finality retry task started");

    // Start WebSocket server
    let bridge_clone = bridge.clone();
    tokio::spawn(async move {
        if let Err(e) = bridge_clone.start().await {
            eprintln!("❌ WebSocket bridge error: {}", e);
        }
    });

    // Start P2P message listener
    let listener = PeerListener::bind(p2p_addr, network_type, p2p_addr, None, None, None)
        .await
        .expect("Failed to bind P2P listener");
    println!("✅ P2P listener started");

    let utxo_integration_clone = utxo_integration.clone();
    let peer_manager_clone = peer_manager.clone();
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok(mut connection) => {
                    let integration = utxo_integration_clone.clone();
                    let pm = peer_manager_clone.clone();

                    tokio::spawn(async move {
                        loop {
                            match connection.receive_message().await {
                                Ok(message) => {
                                    let peer_info = connection.peer_info().await;
                                    let peer_ip = peer_info.address.ip();

                                    // Handle message via UTXO integration
                                    match integration
                                        .handle_network_message(&message, peer_ip)
                                        .await
                                    {
                                        Ok(Some(response)) => {
                                            log::info!(
                                                "📤 Sending response: {:?}",
                                                std::mem::discriminant(&response)
                                            );
                                            match connection.send_message(response).await {
                                                Ok(_) => {
                                                    log::info!("✅ Response sent successfully")
                                                }
                                                Err(e) => {
                                                    log::error!("❌ Failed to send response: {}", e)
                                                }
                                            }
                                        }
                                        Ok(None) => {
                                            // Check if it's a TransactionBroadcast to re-broadcast
                                            if let time_network::protocol::NetworkMessage::TransactionBroadcast(
                                                tx,
                                            ) = message
                                            {
                                                log::info!(
                                                    "Received transaction {} from peer, re-broadcasting",
                                                    tx.txid
                                                );

                                                // Re-broadcast to other peers
                                                pm.broadcast_message(
                                                    time_network::protocol::NetworkMessage::TransactionBroadcast(tx),
                                                )
                                                .await;
                                            }
                                        }
                                        Err(e) => {
                                            log::warn!("Error handling message: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::debug!("Connection closed or error: {}", e);
                                    break;
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    log::error!("Failed to accept connection: {}", e);
                }
            }
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
