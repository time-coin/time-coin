//! WebSocket server for TIME Coin Protocol
//! Broadcasts UTXO state changes and instant finality notifications to connected wallets

use crate::error::{MasternodeError, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use time_consensus::utxo_state_protocol::{UTXOState, UTXOStateNotification};
use time_core::{OutPoint, Transaction};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message};

/// WebSocket message types for TIME Coin Protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    /// Subscribe to specific address updates
    Subscribe { addresses: Vec<String> },
    
    /// Unsubscribe from address updates
    Unsubscribe { addresses: Vec<String> },
    
    /// UTXO state change notification
    UtxoStateChange {
        outpoint: OutPoint,
        state: UTXOState,
        transaction: Option<Transaction>,
    },
    
    /// Transaction received and pending
    TransactionPending { tx: Transaction },
    
    /// Transaction achieved instant finality
    TransactionFinalized { txid: String, block_height: Option<u64> },
    
    /// New block received
    NewBlock { height: u64, hash: String },
    
    /// Heartbeat/ping
    Ping,
    
    /// Response to ping
    Pong,
    
    /// Error message
    Error { message: String },
}

/// Client connection tracking
struct WsClient {
    sender: tokio::sync::mpsc::UnboundedSender<Message>,
    subscribed_addresses: Vec<String>,
}

/// WebSocket server for broadcasting TIME Coin Protocol events
pub struct WsServer {
    /// Connected clients
    clients: Arc<RwLock<HashMap<String, WsClient>>>,
    
    /// Broadcast channel for global events
    broadcast_tx: broadcast::Sender<WsMessage>,
    
    /// Listen address
    addr: String,
}

impl WsServer {
    /// Create a new WebSocket server
    pub fn new(addr: String) -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
            addr,
        }
    }
    
    /// Start the WebSocket server
    pub async fn start(self: Arc<Self>) -> Result<()> {
        let listener = tokio::net::TcpListener::bind(&self.addr)
            .await
            .map_err(|e| MasternodeError::NetworkError(format!("Failed to bind: {}", e)))?;
        
        println!("🌐 WebSocket server listening on {}", self.addr);
        
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    println!("📱 New WebSocket connection from: {}", addr);
                    let server = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = server.handle_connection(stream).await {
                            eprintln!("❌ WebSocket error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("❌ Failed to accept connection: {}", e);
                }
            }
        }
    }
    
    /// Handle a single WebSocket connection
    async fn handle_connection(&self, stream: tokio::net::TcpStream) -> Result<()> {
        let ws_stream = accept_async(stream)
            .await
            .map_err(|e| MasternodeError::NetworkError(format!("WebSocket handshake failed: {}", e)))?;
        
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        
        // Generate client ID
        let client_id = uuid::Uuid::new_v4().to_string();
        
        // Add client to registry
        {
            let mut clients = self.clients.write().await;
            clients.insert(
                client_id.clone(),
                WsClient {
                    sender: tx,
                    subscribed_addresses: Vec::new(),
                },
            );
        }
        
        // Subscribe to broadcast channel
        let mut broadcast_rx = self.broadcast_tx.subscribe();
        
        // Task to send messages to client
        let client_id_clone = client_id.clone();
        let clients_clone = self.clients.clone();
        let send_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Messages from client's queue
                    Some(msg) = rx.recv() => {
                        if ws_sender.send(msg).await.is_err() {
                            break;
                        }
                    }
                    // Broadcast messages
                    Ok(ws_msg) = broadcast_rx.recv() => {
                        // Check if client is subscribed to this message
                        let should_send = {
                            let clients = clients_clone.read().await;
                            clients.get(&client_id_clone).map_or(false, |_client| {
                                match &ws_msg {
                                    WsMessage::UtxoStateChange { .. } => true,
                                    WsMessage::TransactionPending { .. } => true,
                                    WsMessage::TransactionFinalized { .. } => true,
                                    WsMessage::NewBlock { .. } => true,
                                    _ => false,
                                }
                            })
                        };
                        
                        if should_send {
                            let json = serde_json::to_string(&ws_msg).unwrap();
                            if ws_sender.send(Message::Text(json)).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });
        
        // Task to receive messages from client
        let client_id_clone = client_id.clone();
        let clients_clone = self.clients.clone();
        let recv_task = tokio::spawn(async move {
            while let Some(msg) = ws_receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                            match ws_msg {
                                WsMessage::Subscribe { addresses } => {
                                    let mut clients = clients_clone.write().await;
                                    if let Some(client) = clients.get_mut(&client_id_clone) {
                                        client.subscribed_addresses.extend(addresses.clone());
                                        println!("📢 Client {} subscribed to {} addresses", 
                                                 client_id_clone, addresses.len());
                                    }
                                }
                                WsMessage::Unsubscribe { addresses } => {
                                    let mut clients = clients_clone.write().await;
                                    if let Some(client) = clients.get_mut(&client_id_clone) {
                                        client.subscribed_addresses
                                            .retain(|addr| !addresses.contains(addr));
                                    }
                                }
                                WsMessage::Ping => {
                                    let mut clients = clients_clone.write().await;
                                    if let Some(client) = clients.get_mut(&client_id_clone) {
                                        let pong = serde_json::to_string(&WsMessage::Pong).unwrap();
                                        let _ = client.sender.send(Message::Text(pong));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Ok(Message::Ping(data)) => {
                        let mut clients = clients_clone.write().await;
                        if let Some(client) = clients.get_mut(&client_id_clone) {
                            let _ = client.sender.send(Message::Pong(data));
                        }
                    }
                    Err(_) => break,
                    _ => {}
                }
            }
        });
        
        // Wait for either task to finish
        tokio::select! {
            _ = send_task => {},
            _ = recv_task => {},
        }
        
        // Clean up client
        {
            let mut clients = self.clients.write().await;
            clients.remove(&client_id);
            println!("👋 Client {} disconnected", client_id);
        }
        
        Ok(())
    }
    
    /// Broadcast UTXO state change to all connected clients
    pub async fn broadcast_utxo_state(&self, notification: UTXOStateNotification) {
        let msg = WsMessage::UtxoStateChange {
            outpoint: notification.outpoint,
            state: notification.new_state,
            transaction: None, // Could be filled in if needed
        };
        
        let _ = self.broadcast_tx.send(msg);
    }
    
    /// Broadcast pending transaction
    pub async fn broadcast_transaction_pending(&self, tx: Transaction) {
        let msg = WsMessage::TransactionPending { tx };
        let _ = self.broadcast_tx.send(msg);
    }
    
    /// Broadcast finalized transaction
    pub async fn broadcast_transaction_finalized(&self, txid: String, block_height: Option<u64>) {
        let msg = WsMessage::TransactionFinalized { txid, block_height };
        let _ = self.broadcast_tx.send(msg);
    }
    
    /// Broadcast new block
    pub async fn broadcast_new_block(&self, height: u64, hash: String) {
        let msg = WsMessage::NewBlock { height, hash };
        let _ = self.broadcast_tx.send(msg);
    }
    
    /// Get number of connected clients
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_ws_server_creation() {
        let server = WsServer::new("127.0.0.1:8080".to_string());
        assert_eq!(server.client_count().await, 0);
    }
}
