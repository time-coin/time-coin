//! WebSocket notifications for real-time wallet updates
//!
//! Provides notification system for:
//! - Transaction invalidation (double-spend, etc.)
//! - Transaction confirmations
//! - Incoming payments

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// Re-export types from consensus
pub use time_consensus::tx_validation::{InvalidationReason, TxInvalidationEvent};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxConfirmationEvent {
    pub txid: String,
    pub block_height: u64,
    pub confirmations: u64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxRejectionEvent {
    pub txid: String,
    pub reason: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingPaymentEvent {
    pub txid: String,
    pub amount: u64,
    pub from_address: Option<String>,
    pub to_address: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WalletNotification {
    TxInvalidated {
        txid: String,
        reason: InvalidationReason,
        timestamp: i64,
    },
    TxRejected {
        txid: String,
        reason: String,
        timestamp: i64,
    },
    TxConfirmed {
        txid: String,
        block_height: u64,
        confirmations: u64,
        timestamp: i64,
    },
    IncomingPayment {
        txid: String,
        amount: u64,
        from_address: Option<String>,
        timestamp: i64,
    },
}

/// WebSocket connection manager
pub struct WsConnectionManager {
    /// Active connections: address -> WebSocket
    connections: Arc<RwLock<HashMap<String, tokio::sync::mpsc::UnboundedSender<Message>>>>,
    /// xpub subscriptions: xpub -> WebSocket
    xpub_connections: Arc<RwLock<HashMap<String, tokio::sync::mpsc::UnboundedSender<Message>>>>,
}

impl WsConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            xpub_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new WebSocket connection for an address
    pub async fn register(&self, address: String, tx: tokio::sync::mpsc::UnboundedSender<Message>) {
        let mut conns = self.connections.write().await;
        conns.insert(address, tx);
    }

    /// Unregister a WebSocket connection
    pub async fn unregister(&self, address: &str) {
        let mut conns = self.connections.write().await;
        conns.remove(address);
    }

    /// Register xpub subscription
    pub async fn register_xpub(
        &self,
        xpub: String,
        tx: tokio::sync::mpsc::UnboundedSender<Message>,
    ) {
        let mut conns = self.xpub_connections.write().await;
        conns.insert(xpub, tx);
    }

    /// Unregister xpub subscription
    pub async fn unregister_xpub(&self, xpub: &str) {
        let mut conns = self.xpub_connections.write().await;
        conns.remove(xpub);
    }

    /// Send notification to a specific address
    pub async fn notify_address(&self, address: &str, notification: WalletNotification) {
        let conns = self.connections.read().await;
        if let Some(tx) = conns.get(address) {
            let json = serde_json::to_string(&notification).unwrap_or_default();
            let _ = tx.send(Message::Text(json.into()));
        }
    }

    /// Notify transaction invalidation to all affected addresses
    pub async fn notify_tx_invalidated(&self, event: TxInvalidationEvent) {
        let notification = WalletNotification::TxInvalidated {
            txid: event.txid.clone(),
            reason: event.reason,
            timestamp: event.timestamp,
        };

        for address in &event.affected_addresses {
            self.notify_address(address, notification.clone()).await;
        }
    }

    /// Notify transaction confirmation
    pub async fn notify_tx_confirmed(&self, event: TxConfirmationEvent, address: &str) {
        let notification = WalletNotification::TxConfirmed {
            txid: event.txid,
            block_height: event.block_height,
            confirmations: event.confirmations,
            timestamp: event.timestamp,
        };

        self.notify_address(address, notification).await;
    }

    /// Notify transaction rejection
    pub async fn notify_tx_rejected(&self, event: TxRejectionEvent, address: &str) {
        let notification = WalletNotification::TxRejected {
            txid: event.txid,
            reason: event.reason,
            timestamp: event.timestamp,
        };

        self.notify_address(address, notification).await;
    }

    /// Notify incoming payment
    pub async fn notify_incoming_payment(&self, event: IncomingPaymentEvent) {
        let notification = WalletNotification::IncomingPayment {
            txid: event.txid,
            amount: event.amount,
            from_address: event.from_address,
            timestamp: event.timestamp,
        };

        self.notify_address(&event.to_address, notification).await;
    }

    /// Notify all xpub subscribers (broadcasts to all)
    pub async fn notify_all_xpub_subscribers(&self, notification: WalletNotification) {
        let conns = self.xpub_connections.read().await;
        let json = serde_json::to_string(&notification).unwrap_or_default();

        for tx in conns.values() {
            let _ = tx.send(Message::Text(json.clone().into()));
        }
    }
}

impl Default for WsConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket handler for wallet notifications
pub async fn wallet_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<crate::ApiState>,
) -> impl IntoResponse {
    let manager = state.ws_manager.clone();
    ws.on_upgrade(|socket| handle_wallet_socket(socket, manager))
}

async fn handle_wallet_socket(socket: WebSocket, manager: Arc<WsConnectionManager>) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    // Spawn task to forward messages from channel to WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages (subscription requests)
    let manager_clone = manager.clone();
    let mut recv_task = tokio::spawn(async move {
        let mut current_subscription: Option<Subscription> = None;

        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                if let Ok(sub) = serde_json::from_str::<SubscriptionRequest>(&text) {
                    // Unregister old subscription if any
                    if let Some(old_sub) = current_subscription.take() {
                        match old_sub {
                            Subscription::Address(addr) => {
                                manager_clone.unregister(&addr).await;
                            }
                            Subscription::Xpub(xpub) => {
                                manager_clone.unregister_xpub(&xpub).await;
                            }
                        }
                    }

                    // Register new subscription
                    match sub {
                        SubscriptionRequest::Address { address } => {
                            manager_clone.register(address.clone(), tx.clone()).await;
                            current_subscription = Some(Subscription::Address(address));
                        }
                        SubscriptionRequest::Xpub { xpub } => {
                            manager_clone.register_xpub(xpub.clone(), tx.clone()).await;
                            current_subscription = Some(Subscription::Xpub(xpub));
                        }
                    }
                }
            }
        }

        // Cleanup on disconnect
        if let Some(sub) = current_subscription {
            match sub {
                Subscription::Address(addr) => {
                    manager_clone.unregister(&addr).await;
                }
                Subscription::Xpub(xpub) => {
                    manager_clone.unregister_xpub(&xpub).await;
                }
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum SubscriptionRequest {
    Address { address: String },
    Xpub { xpub: String },
}

#[derive(Debug)]
enum Subscription {
    Address(String),
    Xpub(String),
}

use futures_util::stream::StreamExt;
use futures_util::SinkExt;
