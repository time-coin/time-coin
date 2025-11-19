//! TIME Coin Protocol Client
//!
//! Implements real-time UTXO state tracking and transaction notifications
//! via WebSocket connection to masternode.

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::tungstenite::Message;
use wallet::NetworkType;

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
    /// Submit transaction to network
    SubmitTransaction {
        transaction: wallet::Transaction,
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
    Approved { votes: usize, total_nodes: usize },
    Finalized,
    Declined { reason: String },
    Confirmed { block_height: u64 },
}

/// TIME Coin Protocol client
pub struct ProtocolClient {
    /// Network type
    network: NetworkType,
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
    pub fn new(
        network: NetworkType,
        masternodes: Vec<String>,
    ) -> (Self, mpsc::UnboundedReceiver<WalletNotification>) {
        let (notification_tx, notification_rx) = mpsc::unbounded_channel();

        (
            Self {
                network,
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
        // Extract IP from masternode URL (format: http://IP:PORT)
        let ip = masternode
            .replace("http://", "")
            .replace("https://", "")
            .split(':')
            .next()
            .ok_or("Invalid masternode URL")?
            .to_string();

        // Connect directly to TCP port (24100 for testnet, 24101 for mainnet)
        let port = if self.network == NetworkType::Testnet {
            24100
        } else {
            24101
        };
        let tcp_addr = format!("{}:{}", ip, port);

        log::info!("Connecting to masternode via TCP: {}", tcp_addr);

        let stream = TcpStream::connect(&tcp_addr)
            .await
            .map_err(|e| format!("TCP connection failed: {}", e))?;

        let ws_stream = tokio_tungstenite::accept_async(stream)
            .await
            .map_err(|e| format!("Protocol handshake failed: {}", e))?;

        let (mut write, mut read) = ws_stream.split();

        // Mark as active
        *self.active.write().await = true;

        // Subscribe to xpub if available
        let xpub = self.subscribed_xpub.read().await.clone();
        if let Some(xpub) = xpub {
            // Send RegisterXpub network message
            let register_msg =
                time_network::protocol::NetworkMessage::RegisterXpub { xpub: xpub.clone() };
            let msg_json = serde_json::to_string(&register_msg)
                .map_err(|e| format!("Failed to serialize RegisterXpub: {}", e))?;
            write
                .send(Message::Text(msg_json.into()))
                .await
                .map_err(|e| format!("Failed to send RegisterXpub: {}", e))?;

            log::info!("📤 Sent RegisterXpub message for xpub: {}...", &xpub[..20]);
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
        // Try parsing as NetworkMessage first
        if let Ok(network_msg) =
            serde_json::from_str::<time_network::protocol::NetworkMessage>(text)
        {
            match network_msg {
                time_network::protocol::NetworkMessage::UtxoUpdate { xpub, utxos } => {
                    log::info!(
                        "📦 Received UTXO update for xpub {}...: {} UTXOs",
                        &xpub[..20],
                        utxos.len()
                    );

                    // For each UTXO, create a transaction notification
                    for utxo in utxos {
                        let notification = WalletNotification {
                            txid: utxo.txid.clone(),
                            address: utxo.address.clone(),
                            amount: utxo.amount,
                            is_incoming: true,
                            timestamp: chrono::Utc::now().timestamp(),
                            state: if utxo.block_height.is_some() {
                                TransactionState::Confirmed {
                                    block_height: utxo.block_height.unwrap(),
                                }
                            } else {
                                TransactionState::Pending
                            },
                        };

                        if let Err(e) = notification_tx.send(notification) {
                            log::error!("Failed to send UTXO notification: {}", e);
                        }
                    }

                    return Ok(());
                }
                time_network::protocol::NetworkMessage::XpubRegistered { success, message } => {
                    if success {
                        log::info!("✅ Xpub registration successful: {}", message);
                    } else {
                        log::error!("❌ Xpub registration failed: {}", message);
                    }
                    return Ok(());
                }
                _ => {
                    // Not a message we handle here, fall through to old format
                }
            }
        }

        // Try parsing as API WalletNotification format
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

            ProtocolMessage::TransactionFinalized {
                txid,
                votes,
                total_nodes,
                ..
            } => {
                log::info!(
                    "✓ Transaction approved: {} ({}/{} votes)",
                    txid,
                    votes,
                    total_nodes
                );

                // Send notification for approved state
                let notification = WalletNotification {
                    txid: txid.clone(),
                    address: String::new(), // Will be updated by wallet
                    amount: 0,              // Will be updated by wallet
                    is_incoming: false,
                    timestamp: chrono::Utc::now().timestamp(),
                    state: TransactionState::Approved { votes, total_nodes },
                };

                if let Err(e) = notification_tx.send(notification) {
                    log::error!("Failed to send approval notification: {}", e);
                }
            }

            ProtocolMessage::TransactionConfirmed {
                txid, block_height, ..
            } => {
                log::info!(
                    "✓ Transaction confirmed: {} at height {}",
                    txid,
                    block_height
                );

                // Send notification for confirmed state
                let notification = WalletNotification {
                    txid: txid.clone(),
                    address: String::new(),
                    amount: 0,
                    is_incoming: false,
                    timestamp: chrono::Utc::now().timestamp(),
                    state: TransactionState::Confirmed { block_height },
                };

                if let Err(e) = notification_tx.send(notification) {
                    log::error!("Failed to send confirmation notification: {}", e);
                }
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

    /// Send a transaction to the masternode network for processing
    pub async fn send_transaction(
        &self,
        transaction: wallet::Transaction,
    ) -> Result<String, String> {
        // Get first masternode URL
        let masternode = self
            .masternodes
            .first()
            .ok_or_else(|| "No masternodes configured".to_string())?;

        let txid = transaction.txid();

        // Send transaction via HTTP API for now
        // TODO: Use WebSocket when connection management is implemented
        let client = reqwest::Client::new();
        let url = format!("{}/instant-finality/submit", masternode);

        let response = client
            .post(&url)
            .json(&serde_json::json!({
                "transaction": transaction
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to send transaction: {}", e))?;

        if response.status().is_success() {
            log::info!("✓ Transaction submitted to masternode: {}", txid);
            Ok(txid)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(format!(
                "Transaction submission failed ({}): {}",
                status, error_text
            ))
        }
    }

    /// Subscribe to xpub for all derived addresses
    pub async fn subscribe_xpub(&self, xpub: String) -> Result<(), String> {
        log::info!("Subscribing to xpub via TCP: {}...", &xpub[..20]);

        // Store xpub
        *self.subscribed_xpub.write().await = Some(xpub.clone());

        // Send RegisterXpub message via TCP protocol
        // This will be handled by the existing WebSocket connection
        // For now, just mark as subscribed - the actual registration will happen
        // when we send the RequestWalletTransactions message
        log::info!("✅ Xpub subscribed locally (will register on sync)");
        Ok(())
    }

    /// Sync wallet from xpub - gets all historical transactions and UTXOs
    /// NOTE: This requires TCP protocol implementation (not HTTP endpoints)
    /// For now, returns empty sync data - wallet will sync via historical UTXO scan
    pub async fn sync_wallet_from_xpub(&self, _xpub: &str) -> Result<WalletSyncData, String> {
        log::info!("ℹ️  Xpub sync via TCP not yet implemented - using address-based scanning");

        // Return empty sync data - wallet will fall back to historical UTXO scanning
        Ok(WalletSyncData {
            utxos: std::collections::HashMap::new(),
            total_balance: 0,
            recent_transactions: Vec::new(),
            current_height: 0,
        })
    }

    /// Sync historical UTXOs from blockchain for derived addresses
    /// Uses BIP44 gap limit of 20 - stops scanning after 20 consecutive empty addresses
    pub async fn sync_historical_utxos(
        &self,
        derive_address_fn: impl Fn(u32) -> Result<String, String>,
        wallet_db: &crate::wallet_db::WalletDb,
    ) -> Result<(), String> {
        log::info!("🔄 Starting historical UTXO sync from blockchain...");

        const GAP_LIMIT: u32 = 20;
        let mut index = 0u32;
        let mut consecutive_empty = 0u32;
        let mut total_utxos = 0usize;

        // Get first masternode for HTTP queries
        let masternode = self.masternodes.first().ok_or("No masternodes available")?;
        let base_url = masternode
            .replace("ws://", "http://")
            .replace("wss://", "https://");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        loop {
            // Derive address at current index
            let address = derive_address_fn(index)?;
            log::debug!("Checking address {} at index {}", address, index);

            // Query UTXOs for this address
            let url = format!("{}/utxos/{}", base_url, address);
            match client.get(&url).send().await {
                Ok(response) if response.status().is_success() => {
                    #[derive(serde::Deserialize)]
                    struct UtxoResponse {
                        utxos: Vec<UtxoEntry>,
                    }
                    #[derive(serde::Deserialize)]
                    struct UtxoEntry {
                        outpoint: String,
                        amount: u64,
                    }

                    match response.json::<UtxoResponse>().await {
                        Ok(utxo_response) => {
                            if utxo_response.utxos.is_empty() {
                                consecutive_empty += 1;
                                log::debug!(
                                    "Address {} is empty ({}/{})",
                                    address,
                                    consecutive_empty,
                                    GAP_LIMIT
                                );
                            } else {
                                consecutive_empty = 0;
                                log::info!(
                                    "✅ Found {} UTXOs for address {} (index {})",
                                    utxo_response.utxos.len(),
                                    address,
                                    index
                                );

                                // Store UTXOs in database
                                for utxo in utxo_response.utxos {
                                    let parts: Vec<&str> = utxo.outpoint.split(':').collect();
                                    if parts.len() == 2 {
                                        let txid = parts[0].to_string();
                                        let vout: u32 = parts[1].parse().unwrap_or(0);

                                        let tx_record = crate::wallet_db::TransactionRecord {
                                            tx_hash: txid.clone(),
                                            timestamp: chrono::Utc::now().timestamp(),
                                            from_address: None,
                                            to_address: address.clone(),
                                            amount: utxo.amount,
                                            status: crate::wallet_db::TransactionStatus::Confirmed,
                                            block_height: None,
                                            notes: Some(
                                                "Historical UTXO from blockchain sync".to_string(),
                                            ),
                                        };

                                        if let Err(e) = wallet_db.save_transaction(&tx_record) {
                                            log::warn!(
                                                "Failed to store UTXO {}: {}",
                                                utxo.outpoint,
                                                e
                                            );
                                        } else {
                                            total_utxos += 1;
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to parse UTXO response for {}: {}", address, e);
                        }
                    }
                }
                Ok(response) => {
                    log::warn!(
                        "Failed to fetch UTXOs for address {}: HTTP {}",
                        address,
                        response.status()
                    );
                }
                Err(e) => {
                    log::warn!("Network error fetching UTXOs for {}: {}", address, e);
                }
            }

            // Check if we've hit the gap limit
            if consecutive_empty >= GAP_LIMIT {
                log::info!(
                    "✅ UTXO sync complete: scanned {} addresses, found {} UTXOs",
                    index + 1,
                    total_utxos
                );
                break;
            }

            index += 1;

            // Safety limit to prevent infinite loops
            if index > 1000 {
                log::warn!("⚠️  Reached maximum address scan limit (1000)");
                break;
            }
        }

        Ok(())
    }
}

/// Wallet sync response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSyncData {
    pub utxos: std::collections::HashMap<String, Vec<WalletUtxoInfo>>,
    pub total_balance: u64,
    pub recent_transactions: Vec<WalletTransactionInfo>,
    pub current_height: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletUtxoInfo {
    pub tx_hash: String,
    pub output_index: u32,
    pub amount: u64,
    pub address: String,
    pub block_height: u64,
    pub confirmations: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTransactionInfo {
    pub txid: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: u64,
    pub block_height: Option<u64>,
    pub timestamp: i64,
    pub confirmations: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let masternodes = vec!["http://localhost:24101".to_string()];
        let (client, _rx) = ProtocolClient::new(wallet::NetworkType::Testnet, masternodes);
        assert!(!client.is_connected().await);
    }

    #[tokio::test]
    async fn test_subscription() {
        let masternodes = vec!["http://localhost:24101".to_string()];
        let (client, _rx) = ProtocolClient::new(wallet::NetworkType::Testnet, masternodes);

        let addresses = vec!["addr1".to_string(), "addr2".to_string()];
        client.subscribe(addresses.clone()).await.unwrap();

        let subscribed = client.subscribed_addresses().await;
        assert_eq!(subscribed.len(), 2);
        assert!(subscribed.contains(&"addr1".to_string()));
    }
}
