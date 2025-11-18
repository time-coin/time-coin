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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum WsMessage {
    Subscribe { addresses: Vec<String> },
    Ping,
    Pong,
}

impl WsBridge {
    pub fn new(addr: String) -> Self {
        Self {
            addr,
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(&self.addr).await?;
        log::info!("WebSocket bridge listening on {}", self.addr);

        while let Ok((stream, addr)) = listener.accept().await {
            log::info!("New WebSocket connection from {}", addr);
            let clients = self.clients.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, clients).await {
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
