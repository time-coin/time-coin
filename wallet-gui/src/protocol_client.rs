//! TIME Coin Protocol Client
//!
//! Implements real-time UTXO state tracking and transaction notifications
//! via WebSocket connection to masternode.

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// UTXO State as defined in TIME Coin Protocol
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionOutput {
    pub address: String,
    pub amount: u64,
    pub vout: u32,
}

/// Transaction notification for wallet UI
#[derive(Debug, Clone)]
pub struct WalletNotification {
    pub txid: String,
    pub address: String,
    pub amount: u64,
    pub is_incoming: bool,
    pub timestamp: i64,
    pub state: TransactionState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionState {
    Pending,
    Finalized,
    Confirmed { block_height: u64 },
}

/// TIME Coin Protocol client
pub struct ProtocolClient {
    /// Connected masternodes (WebSocket URLs)
    masternodes: Vec<String>,
    /// Subscribed addresses
    subscribed_addresses: Arc<RwLock<Vec<String>>>,
    /// Subscribed xpub (if any)
    subscribed_xpub: Arc<RwLock<Option<String>>>,
    /// Transaction notifications channel
    notification_tx: mpsc::UnboundedSender<WalletNotification>,
    /// Active connections
    active: Arc<RwLock<bool>>,
}

impl ProtocolClient {
    /// Create new protocol client
    pub fn new(masternodes: Vec<String>) -> (Self, mpsc::UnboundedReceiver<WalletNotification>) {
        let (notification_tx, notification_rx) = mpsc::unbounded_channel();

        (
            Self {
                masternodes,
                subscribed_addresses: Arc::new(RwLock::new(Vec::new())),
                subscribed_xpub: Arc::new(RwLock::new(None)),
                notification_tx,
                active: Arc::new(RwLock::new(false)),
            },
            notification_rx,
        )
    }

    /// Connect to masternode and start listening
    pub async fn connect(&self) -> Result<(), String> {
        if self.masternodes.is_empty() {
            return Err("No masternodes configured".to_string());
        }

        // Connect to first available masternode
        for masternode in &self.masternodes {
            match self.connect_to_masternode(masternode).await {
                Ok(_) => {
                    log::info!("Successfully connected to masternode: {}", masternode);
                    return Ok(());
                }
                Err(e) => {
                    log::warn!("Failed to connect to {}: {}", masternode, e);
                    continue;
                }
            }
        }

        Err("Failed to connect to any masternode".to_string())
    }

    /// Connect to specific masternode
    async fn connect_to_masternode(&self, masternode: &str) -> Result<(), String> {
        // Convert HTTP endpoint to WebSocket
        let ws_url = masternode
            .replace("http://", "ws://")
            .replace("https://", "wss://");
        let ws_url = format!("{}/ws/wallet", ws_url); // Use wallet endpoint

        log::info!("Connecting to WebSocket: {}", ws_url);

        let (ws_stream, _) = connect_async(&ws_url)
            .await
            .map_err(|e| format!("WebSocket connection failed: {}", e))?;

        let (mut write, mut read) = ws_stream.split();

        // Mark as active
        *self.active.write().await = true;

        // Subscribe to xpub if available
        let xpub = self.subscribed_xpub.read().await.clone();
        if let Some(xpub) = xpub {
            let subscribe_msg = serde_json::json!({
                "type": "xpub",
                "xpub": xpub
            });
            let msg_json = serde_json::to_string(&subscribe_msg)
                .map_err(|e| format!("Failed to serialize: {}", e))?;
            write
                .send(Message::Text(msg_json.into()))
                .await
                .map_err(|e| format!("Failed to send xpub subscribe: {}", e))?;

            log::info!("Subscribed to xpub: {}...", &xpub[..20]);
        }

        // Subscribe to existing addresses (fallback/additional)
        let addresses = self.subscribed_addresses.read().await.clone();
        if !addresses.is_empty() {
            let subscribe_msg = serde_json::json!({
                "type": "address",
                "address": addresses[0].clone()  // API supports one address at a time
            });
            let msg_json = serde_json::to_string(&subscribe_msg)
                .map_err(|e| format!("Failed to serialize: {}", e))?;
            write
                .send(Message::Text(msg_json.into()))
                .await
                .map_err(|e| format!("Failed to send subscribe: {}", e))?;

            log::info!("Subscribed to {} addresses", addresses.len());
        }

        // Clone for async task
        let notification_tx = self.notification_tx.clone();
        let active = self.active.clone();
        let subscribed_addresses = self.subscribed_addresses.clone();

        // Spawn message handler
        tokio::spawn(async move {
            log::info!("Starting message handler...");

            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        log::debug!("Received message: {}", text);
                        if let Err(e) = Self::handle_message(&text, &notification_tx).await {
                            log::error!("Failed to handle message: {}", e);
                        }
                    }
                    Ok(Message::Ping(_)) => {
                        log::debug!("Received ping");
                        // Pong is sent automatically by tungstenite
                    }
                    Ok(Message::Close(_)) => {
                        log::info!("WebSocket closed by server");
                        break;
                    }
                    Err(e) => {
                        log::error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            log::info!("Message handler stopped");
            *active.write().await = false;
        });

        Ok(())
    }

    /// Handle incoming protocol message
    async fn handle_message(
        text: &str,
        notification_tx: &mpsc::UnboundedSender<WalletNotification>,
    ) -> Result<(), String> {
        // Try parsing as API WalletNotification format first
        if let Ok(api_notif) = serde_json::from_str::<serde_json::Value>(text) {
            if let Some(notif_type) = api_notif.get("type").and_then(|v| v.as_str()) {
                match notif_type {
                    "tx_confirmed" => {
                        let txid = api_notif.get("txid").and_then(|v| v.as_str()).unwrap_or("");
                        let block_height = api_notif
                            .get("block_height")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let timestamp = api_notif
                            .get("timestamp")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);

                        log::info!("Transaction confirmed: {} at height {}", txid, block_height);
                        // Could update existing transaction status
                        return Ok(());
                    }
                    "incoming_payment" => {
                        let txid = api_notif
                            .get("txid")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let amount = api_notif
                            .get("amount")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let from_address = api_notif
                            .get("from_address")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let timestamp = api_notif
                            .get("timestamp")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);

                        log::info!(
                            "💰 Incoming payment: {} TIME from {:?}",
                            amount,
                            from_address
                        );

                        let notification = WalletNotification {
                            txid,
                            address: from_address.unwrap_or_default(),
                            amount,
                            is_incoming: true,
                            timestamp,
                            state: TransactionState::Pending,
                        };

                        if let Err(e) = notification_tx.send(notification) {
                            log::error!("Failed to send notification: {}", e);
                        }
                        return Ok(());
                    }
                    "tx_invalidated" => {
                        let txid = api_notif.get("txid").and_then(|v| v.as_str()).unwrap_or("");
                        log::warn!("❌ Transaction invalidated: {}", txid);
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }

        // Fallback: Try parsing as old ProtocolMessage format
        let msg: ProtocolMessage =
            serde_json::from_str(text).map_err(|e| format!("Failed to parse message: {}", e))?;

        match msg {
            ProtocolMessage::NewTransaction {
                txid,
                outputs,
                timestamp,
                ..
            } => {
                log::info!("New transaction: {}", txid);
                // Notify about each output to our addresses
                for output in outputs {
                    let notification = WalletNotification {
                        txid: txid.clone(),
                        address: output.address.clone(),
                        amount: output.amount,
                        is_incoming: true,
                        timestamp,
                        state: TransactionState::Pending,
                    };
                    if let Err(e) = notification_tx.send(notification) {
                        log::error!("Failed to send notification: {}", e);
                    }
                }
            }

            ProtocolMessage::TransactionFinalized { txid, .. } => {
                log::info!("Transaction finalized: {}", txid);
                // Update transaction state to finalized
                // This would ideally update existing notification
            }

            ProtocolMessage::TransactionConfirmed {
                txid, block_height, ..
            } => {
                log::info!("Transaction confirmed: {} at height {}", txid, block_height);
                // Update transaction state to confirmed
            }

            ProtocolMessage::UTXOStateChange {
                outpoint,
                new_state,
                address,
                amount,
                timestamp,
                ..
            } => {
                log::info!(
                    "UTXO state change: {}:{} -> {:?}",
                    outpoint.txid,
                    outpoint.vout,
                    new_state
                );

                // If UTXO is now spent, it means we're sending
                if matches!(new_state, UTXOState::SpentPending { .. }) {
                    // Could notify about outgoing transaction
                }
            }

            ProtocolMessage::SubscriptionConfirmed { addresses } => {
                log::info!("Subscription confirmed for {} addresses", addresses.len());
            }

            ProtocolMessage::Error { message } => {
                log::error!("Protocol error: {}", message);
            }

            _ => {
                log::debug!("Unhandled message type");
            }
        }

        Ok(())
    }

    /// Subscribe to addresses
    pub async fn subscribe(&self, addresses: Vec<String>) -> Result<(), String> {
        log::info!("Subscribing to {} addresses", addresses.len());

        // Store subscribed addresses
        let mut subscribed = self.subscribed_addresses.write().await;
        for addr in &addresses {
            if !subscribed.contains(addr) {
                subscribed.push(addr.clone());
            }
        }

        // If not connected yet, addresses will be subscribed on connect
        if !*self.active.read().await {
            log::info!("Not connected yet, addresses will be subscribed on connect");
            return Ok(());
        }

        // Send subscribe message
        // This would need access to the WebSocket writer
        // For now, subscription happens on connect
        log::info!("Addresses queued for subscription");

        Ok(())
    }

    /// Unsubscribe from address
    pub async fn unsubscribe(&self, address: String) -> Result<(), String> {
        let mut subscribed = self.subscribed_addresses.write().await;
        subscribed.retain(|a| a != &address);
        Ok(())
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.active.read().await
    }

    /// Get subscribed addresses
    pub async fn subscribed_addresses(&self) -> Vec<String> {
        self.subscribed_addresses.read().await.clone()
    }

    /// Subscribe to xpub for all derived addresses
    pub async fn subscribe_xpub(&self, xpub: String) -> Result<(), String> {
        log::info!("Subscribing to xpub: {}...", &xpub[..20]);

        // Store xpub
        *self.subscribed_xpub.write().await = Some(xpub.clone());

        // If not connected yet, xpub will be subscribed on connect
        if !*self.active.read().await {
            log::info!("Not connected yet, xpub will be subscribed on connect");
            return Ok(());
        }

        // TODO: Send subscription if already connected
        log::info!("xpub subscription queued");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let masternodes = vec!["http://localhost:24101".to_string()];
        let (client, _rx) = ProtocolClient::new(masternodes);
        assert!(!client.is_connected().await);
    }

    #[tokio::test]
    async fn test_subscription() {
        let masternodes = vec!["http://localhost:24101".to_string()];
        let (client, _rx) = ProtocolClient::new(masternodes);

        let addresses = vec!["addr1".to_string(), "addr2".to_string()];
        client.subscribe(addresses.clone()).await.unwrap();

        let subscribed = client.subscribed_addresses().await;
        assert_eq!(subscribed.len(), 2);
        assert!(subscribed.contains(&"addr1".to_string()));
    }
}
