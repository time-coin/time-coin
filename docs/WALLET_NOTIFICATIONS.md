# Wallet Notification Architecture for TIME Coin

## Problem
GUI wallets need real-time notifications when transactions arrive, without constantly polling.

## Solution Options

### ✅ RECOMMENDED: Server-Sent Events (SSE)

#### Architecture
```
┌─────────────────┐                    ┌──────────────────┐
│   GUI Wallet    │                    │  Masternode API  │
│                 │                    │                  │
│ 1. Register     │──── POST ────────→ │ Store addresses  │
│    addresses    │    /wallet/register│ in memory map    │
│                 │                    │                  │
│ 2. Open SSE     │──── GET ─────────→ │ Keep connection  │
│    stream       │    /wallet/events  │ alive            │
│                 │←──── events ────── │                  │
│                 │                    │                  │
│ 3. When TX      │                    │ Check if address │
│    arrives      │                    │ matches wallet   │
│                 │                    │                  │
│                 │←── notification ── │ Send SSE event   │
│ Update balance  │                    │                  │
└─────────────────┘                    └──────────────────┘
```

#### Benefits
- ✅ Simple HTTP-based (no WebSocket complexity)
- ✅ Auto-reconnects on disconnect
- ✅ Works through firewalls
- ✅ Lightweight (~1KB per event)
- ✅ Efficient for 1-way communication
- ✅ Built into reqwest

#### Implementation

**Masternode Side (api/src/sse_handler.rs):**
```rust
use axum::response::sse::{Event, Sse};
use futures::stream::Stream;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

// Global registry of wallets
type WalletRegistry = Arc<RwLock<HashMap<String, Vec<String>>>>;

pub struct SseManager {
    wallet_registry: WalletRegistry,
    tx_broadcast: broadcast::Sender<TransactionNotification>,
}

#[derive(Clone, Debug)]
pub struct TransactionNotification {
    pub txid: String,
    pub address: String,
    pub amount: u64,
    pub block_height: u64,
}

impl SseManager {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            wallet_registry: Arc::new(RwLock::new(HashMap::new())),
            tx_broadcast: tx,
        }
    }

    // Register wallet addresses
    pub async fn register_wallet(&self, wallet_id: String, addresses: Vec<String>) {
        let mut registry = self.wallet_registry.write().await;
        registry.insert(wallet_id, addresses);
    }

    // Notify about new transaction
    pub async fn notify_transaction(&self, notification: TransactionNotification) {
        let _ = self.tx_broadcast.send(notification);
    }

    // Create SSE stream for a wallet
    pub async fn create_stream(
        &self,
        wallet_id: String,
    ) -> impl Stream<Item = Result<Event, std::io::Error>> {
        let mut rx = self.tx_broadcast.subscribe();
        let registry = self.wallet_registry.clone();

        async_stream::stream! {
            loop {
                match rx.recv().await {
                    Ok(notification) => {
                        // Check if this wallet cares about this address
                        let wallet_addresses = {
                            let reg = registry.read().await;
                            reg.get(&wallet_id).cloned()
                        };

                        if let Some(addresses) = wallet_addresses {
                            if addresses.contains(&notification.address) {
                                let event = Event::default()
                                    .event("new_transaction")
                                    .json_data(&notification)
                                    .unwrap();
                                yield Ok(event);
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    }
}

// API Routes
pub async fn register_wallet_handler(
    State(state): State<ApiState>,
    Json(request): Json<RegisterWalletRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .sse_manager
        .register_wallet(request.wallet_id, request.addresses)
        .await;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Wallet registered for notifications"
    })))
}

pub async fn wallet_events_handler(
    State(state): State<ApiState>,
    Query(params): Query<HashMap<String, String>>,
) -> Sse<impl Stream<Item = Result<Event, std::io::Error>>> {
    let wallet_id = params.get("wallet_id").cloned().unwrap_or_default();
    let stream = state.sse_manager.create_stream(wallet_id).await;
    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keepalive"),
    )
}
```

**Hook into transaction processing:**
```rust
// In api/src/routes.rs - post_transaction handler
async fn post_transaction(
    State(state): State<ApiState>,
    Json(tx): Json<Transaction>,
) -> ApiResult<Json<serde_json::Value>> {
    // ... existing validation ...
    
    // Add to mempool
    state.mempool.add_transaction(tx.clone()).await;
    
    // Trigger instant finality
    trigger_instant_finality_for_received_tx(state.clone(), tx.clone()).await;
    
    // 🔔 NEW: Notify wallets about this transaction
    for output in &tx.outputs {
        state.sse_manager.notify_transaction(TransactionNotification {
            txid: tx.txid.clone(),
            address: output.address.clone(),
            amount: output.amount,
            block_height: 0, // 0 = unconfirmed
        }).await;
    }
    
    Ok(Json(serde_json::json!({
        "success": true,
        "txid": tx.txid
    })))
}
```

**Wallet GUI Side (wallet-gui/src/sse_client.rs):**
```rust
use eventsource_client::{Client, SSE};
use futures::StreamExt;

pub async fn listen_for_transactions(
    api_endpoint: String,
    wallet_id: String,
    tx_sender: mpsc::UnboundedSender<TransactionNotification>,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/wallet/events?wallet_id={}", api_endpoint, wallet_id);
    
    let client = eventsource_client::ClientBuilder::for_url(&url)?
        .reconnect(
            eventsource_client::ReconnectOptions::reconnect(true)
                .retry_initial(false)
                .delay(std::time::Duration::from_secs(1))
                .backoff_factor(2)
                .delay_max(std::time::Duration::from_secs(60))
                .build(),
        )
        .build();

    let mut stream = client.stream();

    while let Some(event) = stream.next().await {
        match event {
            Ok(SSE::Event(evt)) => {
                if evt.event_type == "new_transaction" {
                    if let Ok(notification) =
                        serde_json::from_str::<TransactionNotification>(&evt.data)
                    {
                        log::info!("🔔 New transaction: {} TIME to {}",
                            notification.amount as f64 / 1_000_000.0,
                            &notification.address[..20]
                        );
                        let _ = tx_sender.send(notification);
                    }
                }
            }
            Ok(SSE::Comment(_)) => {
                // Keepalive
            }
            Err(e) => {
                log::error!("SSE error: {}", e);
            }
        }
    }

    Ok(())
}
```

---

### Option 2: WebSocket (More Complex)

**Pros:**
- ✅ Bi-directional
- ✅ Lower latency
- ✅ Full-duplex

**Cons:**
- ❌ More complex to implement
- ❌ Requires connection management
- ❌ Harder to debug
- ❌ Firewall issues

---

### Option 3: Polling (Current - Not Recommended)

**Pros:**
- ✅ Simple
- ✅ Works everywhere

**Cons:**
- ❌ Wastes bandwidth
- ❌ High latency (30s delay)
- ❌ Server load

---

### Option 4: TCP Push Notifications

**Pros:**
- ✅ Uses existing TCP protocol
- ✅ Fast

**Cons:**
- ❌ Wallet must maintain TCP connection
- ❌ NAT/firewall issues
- ❌ Complex reconnection logic

---

## Recommendation: SSE

**Why SSE is best for TIME Coin:**

1. **Perfect for wallet notifications** - wallets only need to *receive* updates, not send
2. **Built-in reconnection** - automatically handles network issues
3. **Simple implementation** - just HTTP + streaming
4. **Firewall-friendly** - works everywhere HTTP works
5. **Efficient** - one connection, many notifications
6. **Low latency** - ~100ms notification time

**Dependencies needed:**
```toml
# Masternode (api/Cargo.toml)
axum = { version = "0.7", features = ["sse"] }
futures = "0.3"
async-stream = "0.3"

# Wallet GUI (wallet-gui/Cargo.toml)
eventsource-client = "0.12"
```

**Estimated implementation time:** 2-3 hours

**Result:** Real-time balance updates without polling! ⚡
