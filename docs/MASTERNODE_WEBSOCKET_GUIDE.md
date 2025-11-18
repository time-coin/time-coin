# Masternode WebSocket Implementation Guide

## 🎯 Goal

Implement WebSocket server in masternode to send real-time transaction notifications to wallets.

## 📋 Checklist

- [ ] Add WebSocket route to API
- [ ] Create WebSocket handler
- [ ] Implement subscription management
- [ ] Integrate UTXO state protocol
- [ ] Send notifications on state changes
- [ ] Test with wallet

## 🔧 Step 1: Add WebSocket Dependencies

**File**: `api/Cargo.toml`

```toml
[dependencies]
# ... existing dependencies ...
tokio-tungstenite = "0.21"
futures-util = "0.3"
```

## 🔧 Step 2: Create WebSocket Handler

**File**: `api/src/ws_handler.rs` (new file)

```rust
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

// Import from wallet-gui/src/protocol_client.rs protocol messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProtocolMessage {
    Subscribe { addresses: Vec<String> },
    Unsubscribe { address: String },
    UTXOStateChange {
        outpoint: OutPoint,
        old_state: UTXOState,
        new_state: UTXOState,
        address: String,
        amount: u64,
        timestamp: i64,
    },
    NewTransaction {
        txid: String,
        inputs: Vec<OutPoint>,
        outputs: Vec<TransactionOutput>,
        timestamp: i64,
        block_height: Option<u64>,
    },
    TransactionFinalized {
        txid: String,
        votes: usize,
        total_nodes: usize,
        finalized_at: i64,
    },
    SubscriptionConfirmed {
        addresses: Vec<String>,
    },
    Error {
        message: String,
    },
    Ping,
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutPoint {
    pub txid: String,
    pub vout: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionOutput {
    pub address: String,
    pub amount: u64,
    pub vout: u32,
}

// Subscription manager
pub struct SubscriptionManager {
    // Map of address -> Vec<client_id>
    subscriptions: Arc<RwLock<std::collections::HashMap<String, Vec<String>>>>,
    // Map of client_id -> sender channel
    clients: Arc<RwLock<std::collections::HashMap<String, mpsc::UnboundedSender<ProtocolMessage>>>>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            clients: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub async fn add_client(&self, client_id: String, sender: mpsc::UnboundedSender<ProtocolMessage>) {
        let mut clients = self.clients.write().await;
        clients.insert(client_id, sender);
    }

    pub async fn remove_client(&self, client_id: &str) {
        // Remove from clients
        let mut clients = self.clients.write().await;
        clients.remove(client_id);

        // Remove from all subscriptions
        let mut subs = self.subscriptions.write().await;
        for (_, client_list) in subs.iter_mut() {
            client_list.retain(|id| id != client_id);
        }
    }

    pub async fn subscribe(&self, client_id: String, addresses: Vec<String>) {
        let mut subs = self.subscriptions.write().await;
        for address in addresses {
            subs.entry(address)
                .or_insert_with(Vec::new)
                .push(client_id.clone());
        }
    }

    pub async fn unsubscribe(&self, client_id: &str, address: &str) {
        let mut subs = self.subscriptions.write().await;
        if let Some(clients) = subs.get_mut(address) {
            clients.retain(|id| id != client_id);
        }
    }

    /// Notify all clients subscribed to an address
    pub async fn notify_address(&self, address: &str, message: ProtocolMessage) {
        let subs = self.subscriptions.read().await;
        let clients = self.clients.read().await;

        if let Some(client_ids) = subs.get(address) {
            for client_id in client_ids {
                if let Some(sender) = clients.get(client_id) {
                    let _ = sender.send(message.clone());
                }
            }
        }
    }
}

// WebSocket upgrade handler
pub async fn ws_utxo_protocol_handler(
    ws: WebSocketUpgrade,
    State(subscription_mgr): State<Arc<SubscriptionManager>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, subscription_mgr))
}

async fn handle_socket(socket: WebSocket, subscription_mgr: Arc<SubscriptionManager>) {
    let (mut sender, mut receiver) = socket.split();
    
    // Generate unique client ID
    let client_id = uuid::Uuid::new_v4().to_string();
    log::info!("New WebSocket client connected: {}", client_id);

    // Create channel for outgoing messages
    let (tx, mut rx) = mpsc::unbounded_channel();
    subscription_mgr.add_client(client_id.clone(), tx).await;

    // Spawn task to send messages to client
    let client_id_clone = client_id.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(json)).await.is_err() {
                    log::warn!("Failed to send to client {}", client_id_clone);
                    break;
                }
            }
        }
    });

    // Handle incoming messages from client
    let subscription_mgr_clone = subscription_mgr.clone();
    let client_id_clone = client_id.clone();
    
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            match serde_json::from_str::<ProtocolMessage>(&text) {
                Ok(ProtocolMessage::Subscribe { addresses }) => {
                    log::info!("Client {} subscribing to {} addresses", client_id, addresses.len());
                    subscription_mgr_clone.subscribe(client_id.clone(), addresses.clone()).await;
                    
                    // Send confirmation
                    let confirm = ProtocolMessage::SubscriptionConfirmed { addresses };
                    if let Ok(json) = serde_json::to_string(&confirm) {
                        let _ = sender.send(Message::Text(json)).await;
                    }
                }
                Ok(ProtocolMessage::Unsubscribe { address }) => {
                    log::info!("Client {} unsubscribing from {}", client_id, address);
                    subscription_mgr_clone.unsubscribe(&client_id, &address).await;
                }
                Ok(ProtocolMessage::Ping) => {
                    let pong = ProtocolMessage::Pong;
                    if let Ok(json) = serde_json::to_string(&pong) {
                        let _ = sender.send(Message::Text(json)).await;
                    }
                }
                _ => {
                    log::debug!("Received other message type");
                }
            }
        }
    }

    // Client disconnected
    log::info!("Client {} disconnected", client_id_clone);
    subscription_mgr.remove_client(&client_id_clone).await;
}
```

## 🔧 Step 3: Add Route to API

**File**: `api/src/main.rs`

```rust
mod ws_handler;

use ws_handler::{ws_utxo_protocol_handler, SubscriptionManager};

// In main() or router setup:
let subscription_mgr = Arc::new(SubscriptionManager::new());

let app = Router::new()
    // ... existing routes ...
    .route("/ws/utxo-protocol", get(ws_utxo_protocol_handler))
    .with_state(subscription_mgr.clone());

// Store subscription_mgr in app state for use in transaction handlers
```

## 🔧 Step 4: Integrate with Transaction Processing

**File**: `masternode/src/transaction_handler.rs` (or wherever transactions are processed)

```rust
use crate::ws_handler::{ProtocolMessage, SubscriptionManager};

// When new transaction arrives:
async fn handle_new_transaction(
    tx: Transaction,
    subscription_mgr: Arc<SubscriptionManager>,
) {
    log::info!("New transaction: {}", tx.txid());

    // Extract outputs for our subscribed addresses
    for (vout, output) in tx.outputs.iter().enumerate() {
        let address = output.address.clone();
        
        // Notify subscribed wallets
        let notification = ProtocolMessage::NewTransaction {
            txid: tx.txid(),
            inputs: tx.inputs.iter().map(|i| OutPoint {
                txid: i.previous_output.txid.clone(),
                vout: i.previous_output.vout,
            }).collect(),
            outputs: tx.outputs.iter().enumerate().map(|(idx, o)| TransactionOutput {
                address: o.address.clone(),
                amount: o.value,
                vout: idx as u32,
            }).collect(),
            timestamp: chrono::Utc::now().timestamp(),
            block_height: None,
        };

        subscription_mgr.notify_address(&address, notification).await;
    }
}

// When transaction reaches finality:
async fn handle_transaction_finalized(
    txid: String,
    votes: usize,
    total_nodes: usize,
    subscription_mgr: Arc<SubscriptionManager>,
) {
    let notification = ProtocolMessage::TransactionFinalized {
        txid,
        votes,
        total_nodes,
        finalized_at: chrono::Utc::now().timestamp(),
    };

    // Notify all clients (or track which clients care about this tx)
    // For now, you'd need to track txid -> addresses mapping
}
```

## 🔧 Step 5: Test

```bash
# Terminal 1: Start masternode
cd api
cargo run

# Terminal 2: Test WebSocket connection
wscat -c ws://localhost:24101/ws/utxo-protocol

# Send subscription:
{"type":"Subscribe","addresses":["addr1","addr2"]}

# Should receive:
{"type":"SubscriptionConfirmed","addresses":["addr1","addr2"]}

# Terminal 3: Send transaction to addr1
# Wallet should receive notification!
```

## 🎯 Integration with UTXO State Protocol

```rust
use time_consensus::utxo_state_protocol::UTXOStateManager;

// In masternode initialization:
let utxo_manager = Arc::new(UTXOStateManager::new("masternode_1".to_string()));

// Set notification callback:
let subscription_mgr_clone = subscription_mgr.clone();
utxo_manager.set_notification_handler(move |notification| {
    let subscription_mgr = subscription_mgr_clone.clone();
    async move {
        // Convert UTXO notification to ProtocolMessage
        let msg = ProtocolMessage::UTXOStateChange {
            outpoint: notification.outpoint,
            old_state: notification.old_state,
            new_state: notification.new_state,
            address: notification.address.clone(),
            amount: notification.amount,
            timestamp: chrono::Utc::now().timestamp(),
        };

        subscription_mgr.notify_address(&notification.address, msg).await;
    }
}).await;
```

## ✅ Done!

Once this is implemented:

1. Wallet connects via WebSocket
2. Subscribes to addresses
3. Masternode tracks those addresses
4. When transaction arrives → instant notification
5. Wallet shows "Received X TIME" in real-time!

**The TIME Coin Protocol is complete!** 🎉
