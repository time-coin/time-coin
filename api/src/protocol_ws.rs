//! TIME Coin Protocol WebSocket Handler
//!
//! Implements real-time UTXO state tracking and transaction notifications
//! via WebSocket connection to wallets.
//!
//! Protocol Messages:
//! - Subscribe/Unsubscribe to addresses
//! - UTXOStateChange notifications
//! - NewTransaction notifications
//! - TransactionFinalized (instant finality)
//! - TransactionConfirmed (block inclusion)

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

/// UTXO State as defined in TIME Coin Protocol
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum UTXOState {
    Unspent,
    Locked {
        txid: String,
        locked_at: i64,
    },
    SpentPending {
        txid: String,
        votes: usize,
        total_nodes: usize,
        spent_at: i64,
    },
    SpentFinalized {
        txid: String,
        finalized_at: i64,
        votes: usize,
    },
    Confirmed {
        txid: String,
        block_height: u64,
        confirmed_at: i64,
    },
}

/// UTXO identifier (txid:vout)
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct OutPoint {
    pub txid: String,
    pub vout: u32,
}

/// Transaction output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionOutput {
    pub address: String,
    pub amount: u64,
    pub vout: u32,
}

/// Protocol messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProtocolMessage {
    /// Subscribe to address notifications
    Subscribe {
        addresses: Vec<String>,
    },
    /// Unsubscribe from address
    Unsubscribe {
        address: String,
    },
    /// UTXO state change notification
    UTXOStateChange {
        outpoint: OutPoint,
        old_state: UTXOState,
        new_state: UTXOState,
        address: String,
        amount: u64,
        timestamp: i64,
    },
    /// New transaction notification
    NewTransaction {
        txid: String,
        inputs: Vec<OutPoint>,
        outputs: Vec<TransactionOutput>,
        timestamp: i64,
        block_height: Option<u64>,
    },
    /// Transaction finalized (instant finality achieved)
    TransactionFinalized {
        txid: String,
        votes: usize,
        total_nodes: usize,
        finalized_at: i64,
    },
    /// Transaction rejected by consensus
    TransactionRejected {
        txid: String,
        rejections: usize,
        total_nodes: usize,
        rejected_at: i64,
    },
    /// Transaction confirmed in block
    TransactionConfirmed {
        txid: String,
        block_height: u64,
        confirmed_at: i64,
    },
    /// Subscription confirmation
    SubscriptionConfirmed {
        addresses: Vec<String>,
    },
    /// Error response
    Error {
        message: String,
    },
    /// Ping/Pong for keepalive
    Ping,
    Pong,
}

/// Subscription manager for TIME Coin Protocol
pub struct SubscriptionManager {
    /// Map of address -> Vec<client_id>
    subscriptions: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Map of client_id -> sender channel
    clients: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<ProtocolMessage>>>>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a new client
    pub async fn add_client(
        &self,
        client_id: String,
        sender: mpsc::UnboundedSender<ProtocolMessage>,
    ) {
        let mut clients = self.clients.write().await;
        clients.insert(client_id, sender);
        tracing::info!("Client added, total clients: {}", clients.len());
    }

    /// Remove a client
    pub async fn remove_client(&self, client_id: &str) {
        // Remove from clients
        let mut clients = self.clients.write().await;
        clients.remove(client_id);
        tracing::info!("Client removed, remaining clients: {}", clients.len());

        // Remove from all subscriptions
        let mut subs = self.subscriptions.write().await;
        for (_, client_list) in subs.iter_mut() {
            client_list.retain(|id| id != client_id);
        }
    }

    /// Subscribe client to addresses
    pub async fn subscribe(&self, client_id: String, addresses: Vec<String>) {
        let mut subs = self.subscriptions.write().await;
        for address in &addresses {
            subs.entry(address.clone())
                .or_insert_with(Vec::new)
                .push(client_id.clone());
        }
        tracing::info!(
            "Client {} subscribed to {} addresses",
            client_id,
            addresses.len()
        );
    }

    /// Unsubscribe client from address
    pub async fn unsubscribe(&self, client_id: &str, address: &str) {
        let mut subs = self.subscriptions.write().await;
        if let Some(clients) = subs.get_mut(address) {
            clients.retain(|id| id != client_id);
        }
        tracing::info!("Client {} unsubscribed from {}", client_id, address);
    }

    /// Notify all clients subscribed to an address
    pub async fn notify_address(&self, address: &str, message: ProtocolMessage) {
        let subs = self.subscriptions.read().await;
        let clients = self.clients.read().await;

        if let Some(client_ids) = subs.get(address) {
            tracing::debug!(
                "Notifying {} clients subscribed to {}",
                client_ids.len(),
                address
            );
            for client_id in client_ids {
                if let Some(sender) = clients.get(client_id) {
                    if let Err(e) = sender.send(message.clone()) {
                        tracing::warn!("Failed to send to client {}: {}", client_id, e);
                    }
                }
            }
        }
    }

    /// Broadcast to all clients
    pub async fn broadcast(&self, message: ProtocolMessage) {
        let clients = self.clients.read().await;
        tracing::debug!("Broadcasting to {} clients", clients.len());
        for (client_id, sender) in clients.iter() {
            if let Err(e) = sender.send(message.clone()) {
                tracing::warn!("Failed to broadcast to client {}: {}", client_id, e);
            }
        }
    }

    /// Get subscription statistics
    pub async fn stats(&self) -> (usize, usize) {
        let clients = self.clients.read().await;
        let subs = self.subscriptions.read().await;
        (clients.len(), subs.len())
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket upgrade handler for TIME Coin Protocol
pub async fn protocol_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<crate::ApiState>,
) -> impl IntoResponse {
    tracing::info!("New protocol WebSocket connection request");
    let subscription_mgr = state.protocol_subscriptions.clone();
    ws.on_upgrade(|socket| handle_protocol_socket(socket, subscription_mgr))
}

/// Handle WebSocket connection
async fn handle_protocol_socket(socket: WebSocket, subscription_mgr: Arc<SubscriptionManager>) {
    let (mut sender, mut receiver) = socket.split();

    // Generate unique client ID
    let client_id = Uuid::new_v4().to_string();
    tracing::info!("Protocol WebSocket client connected: {}", client_id);

    // Create channel for outgoing messages
    let (tx, mut rx) = mpsc::unbounded_channel();
    subscription_mgr.add_client(client_id.clone(), tx).await;

    // Spawn task to send messages to client
    let client_id_send = client_id.clone();
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match serde_json::to_string(&msg) {
                Ok(json) => {
                    if sender.send(Message::Text(json.into())).await.is_err() {
                        tracing::warn!("Failed to send to client {}", client_id_send);
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to serialize message: {}", e);
                }
            }
        }
        tracing::info!("Send task ended for client {}", client_id_send);
    });

    // Handle incoming messages from client
    let subscription_mgr_clone = subscription_mgr.clone();
    let client_id_recv = client_id.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(result) = receiver.next().await {
            match result {
                Ok(Message::Text(text)) => {
                    tracing::debug!("Received from {}: {}", client_id_recv, text);

                    match serde_json::from_str::<ProtocolMessage>(&text) {
                        Ok(ProtocolMessage::Subscribe { addresses }) => {
                            tracing::info!(
                                "Client {} subscribing to {} addresses",
                                client_id_recv,
                                addresses.len()
                            );
                            subscription_mgr_clone
                                .subscribe(client_id_recv.clone(), addresses.clone())
                                .await;

                            // Send confirmation
                            let confirm = ProtocolMessage::SubscriptionConfirmed { addresses };
                            if let Some(sender) = subscription_mgr_clone
                                .clients
                                .read()
                                .await
                                .get(&client_id_recv)
                            {
                                let _ = sender.send(confirm);
                            }
                        }
                        Ok(ProtocolMessage::Unsubscribe { address }) => {
                            tracing::info!(
                                "Client {} unsubscribing from {}",
                                client_id_recv,
                                address
                            );
                            subscription_mgr_clone
                                .unsubscribe(&client_id_recv, &address)
                                .await;
                        }
                        Ok(ProtocolMessage::Ping) => {
                            tracing::debug!("Ping from client {}", client_id_recv);
                            if let Some(sender) = subscription_mgr_clone
                                .clients
                                .read()
                                .await
                                .get(&client_id_recv)
                            {
                                let _ = sender.send(ProtocolMessage::Pong);
                            }
                        }
                        Ok(_) => {
                            tracing::debug!("Received other message type from {}", client_id_recv);
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to parse message from {}: {}",
                                client_id_recv,
                                e
                            );
                            if let Some(sender) = subscription_mgr_clone
                                .clients
                                .read()
                                .await
                                .get(&client_id_recv)
                            {
                                let _ = sender.send(ProtocolMessage::Error {
                                    message: format!("Invalid message format: {}", e),
                                });
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("Client {} sent close", client_id_recv);
                    break;
                }
                Ok(Message::Ping(_)) => {
                    // Pong is sent automatically by axum
                    tracing::debug!("Ping from client {}", client_id_recv);
                }
                Err(e) => {
                    tracing::warn!("WebSocket error for client {}: {}", client_id_recv, e);
                    break;
                }
                _ => {}
            }
        }
        tracing::info!("Receive task ended for client {}", client_id_recv);
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        }
        _ = &mut recv_task => {
            send_task.abort();
        }
    }

    // Client disconnected
    tracing::info!("Protocol WebSocket client disconnected: {}", client_id);
    subscription_mgr.remove_client(&client_id).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_subscription_manager() {
        let manager = SubscriptionManager::new();
        let (tx, _rx) = mpsc::unbounded_channel();

        manager.add_client("client1".to_string(), tx).await;
        manager
            .subscribe("client1".to_string(), vec!["addr1".to_string()])
            .await;

        let (clients, subs) = manager.stats().await;
        assert_eq!(clients, 1);
        assert_eq!(subs, 1);
    }

    #[tokio::test]
    async fn test_multiple_subscriptions() {
        let manager = SubscriptionManager::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();

        manager.add_client("client1".to_string(), tx1).await;
        manager.add_client("client2".to_string(), tx2).await;

        manager
            .subscribe(
                "client1".to_string(),
                vec!["addr1".to_string(), "addr2".to_string()],
            )
            .await;
        manager
            .subscribe("client2".to_string(), vec!["addr1".to_string()])
            .await;

        let (clients, subs) = manager.stats().await;
        assert_eq!(clients, 2);
        assert_eq!(subs, 2);
    }
}
