//! WebSocket Bridge for TIME Coin Protocol
//!
//! Provides a WebSocket interface that bridges to the P2P TCP network.
//! Allows wallets to use WebSocket while masternodes communicate via TCP P2P.

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message};

/// WebSocket client connection
struct WsClient {
    addresses: Vec<String>,
    tx: mpsc::UnboundedSender<Message>,
}

/// WebSocket bridge server
pub struct WsBridge {
    addr: String,
    clients: Arc<RwLock<HashMap<String, WsClient>>>,
    tx_handler: Option<mpsc::UnboundedSender<time_core::Transaction>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum WsMessage {
    Subscribe { addresses: Vec<String> },
    SubmitTransaction { transaction: time_core::Transaction },
    Ping,
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionNotification {
    pub txid: String,
    pub inputs: Vec<String>,  // Addresses spending
    pub outputs: Vec<String>, // Addresses receiving
    pub amount: u64,
    pub timestamp: i64,
}

impl WsBridge {
    pub fn new(addr: String) -> Self {
        Self {
            addr,
            clients: Arc::new(RwLock::new(HashMap::new())),
            tx_handler: None,
        }
    }

    /// Set transaction handler channel
    pub fn set_transaction_handler(&mut self, tx: mpsc::UnboundedSender<time_core::Transaction>) {
        self.tx_handler = Some(tx);
    }

    /// Broadcast transaction to all subscribed clients
    pub async fn broadcast_transaction(&self, notification: TransactionNotification) {
        let clients = self.clients.read().await;
        let mut relevant_addresses: Vec<String> = Vec::new();
        relevant_addresses.extend(notification.inputs.clone());
        relevant_addresses.extend(notification.outputs.clone());

        for (client_id, client) in clients.iter() {
            // Check if client is subscribed to any of the addresses in this transaction
            let is_relevant = client
                .addresses
                .iter()
                .any(|addr| relevant_addresses.contains(addr));

            if is_relevant {
                let msg = serde_json::json!({
                    "type": "NewTransaction",
                    "transaction": notification
                });

                if let Ok(json) = serde_json::to_string(&msg) {
                    let _ = client.tx.send(Message::Text(json.into()));
                    log::info!("Sent transaction notification to client {}", client_id);
                }
            }
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(&self.addr).await?;
        log::info!("WebSocket bridge listening on {}", self.addr);

        while let Ok((stream, addr)) = listener.accept().await {
            log::info!("New WebSocket connection from {}", addr);
            let clients = self.clients.clone();
            let tx_handler = self.tx_handler.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, clients, tx_handler).await {
                    log::error!("WebSocket error: {}", e);
                }
            });
        }

        Ok(())
    }
}

async fn handle_connection(
    stream: TcpStream,
    clients: Arc<RwLock<HashMap<String, WsClient>>>,
    tx_handler: Option<mpsc::UnboundedSender<time_core::Transaction>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws_stream = accept_async(stream).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let client_id = uuid::Uuid::new_v4().to_string();
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Add client
    clients.write().await.insert(
        client_id.clone(),
        WsClient {
            addresses: vec![],
            tx: tx.clone(),
        },
    );

    // Spawn task to forward messages to WebSocket
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    while let Some(msg) = ws_receiver.next().await {
        match msg? {
            Message::Text(text) => {
                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                    match ws_msg {
                        WsMessage::Subscribe { addresses } => {
                            let mut clients_guard = clients.write().await;
                            if let Some(client) = clients_guard.get_mut(&client_id) {
                                client.addresses = addresses.clone();
                                log::info!(
                                    "Client {} subscribed to {} addresses",
                                    client_id,
                                    addresses.len()
                                );

                                // Send confirmation
                                let confirm = serde_json::json!({
                                    "type": "SubscriptionConfirmed",
                                    "addresses": addresses
                                });
                                let _ = client.tx.send(Message::Text(confirm.to_string().into()));
                            }
                        }
                        WsMessage::SubmitTransaction { transaction } => {
                            let txid = transaction.calculate_txid();
                            log::info!("Received transaction from wallet: {}", txid);

                            // Forward to transaction handler
                            if let Some(ref handler) = tx_handler {
                                if let Err(e) = handler.send(transaction.clone()) {
                                    log::error!("Failed to forward transaction to handler: {}", e);
                                    // Send error response to client
                                    if let Some(client) = clients.read().await.get(&client_id) {
                                        let error = serde_json::json!({
                                            "type": "Error",
                                            "message": "Failed to process transaction"
                                        });
                                        let _ =
                                            client.tx.send(Message::Text(error.to_string().into()));
                                    }
                                } else {
                                    log::info!("Transaction {} forwarded to handler", txid);
                                    // Send acknowledgment
                                    if let Some(client) = clients.read().await.get(&client_id) {
                                        let ack = serde_json::json!({
                                            "type": "TransactionReceived",
                                            "txid": txid
                                        });
                                        let _ =
                                            client.tx.send(Message::Text(ack.to_string().into()));
                                    }
                                }
                            } else {
                                log::error!("No transaction handler configured");
                                // Send error response
                                if let Some(client) = clients.read().await.get(&client_id) {
                                    let error = serde_json::json!({
                                        "type": "Error",
                                        "message": "Transaction handler not available"
                                    });
                                    let _ = client.tx.send(Message::Text(error.to_string().into()));
                                }
                            }
                        }
                        WsMessage::Ping => {
                            if let Some(client) = clients.read().await.get(&client_id) {
                                let _ = client.tx.send(Message::Text(
                                    serde_json::json!({"type": "Pong"}).to_string().into(),
                                ));
                            }
                        }
                        _ => {}
                    }
                }
            }
            Message::Ping(_) => {
                // Handled automatically
            }
            Message::Close(_) => {
                break;
            }
            _ => {}
        }
    }

    // Remove client on disconnect
    clients.write().await.remove(&client_id);
    log::info!("Client {} disconnected", client_id);

    Ok(())
}
