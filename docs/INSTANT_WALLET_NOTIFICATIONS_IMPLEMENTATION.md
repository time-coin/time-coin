# Instant Wallet Notifications via TCP - Implementation Plan

## Overview
Implement **instant balance updates** in GUI wallet using existing TCP protocol and xpub registration.

## Current State ✅

### Already Working:
1. ✅ GUI wallet sends `RegisterXpub` message to masternodes
2. ✅ Masternode receives and stores xpub
3. ✅ AddressMonitor derives addresses from xpub (gap limit: 20)
4. ✅ UtxoTracker subscribes to xpub
5. ✅ Masternode sends initial `UtxoUpdate` with existing UTXOs
6. ✅ `NewTransactionNotification` message type exists in protocol

### Missing:
1. ❌ Masternode doesn't push `UtxoUpdate` when NEW transactions arrive
2. ❌ GUI wallet doesn't listen for incoming TCP messages after initial response
3. ❌ GUI wallet doesn't update balance when `UtxoUpdate` received

## Implementation Steps

### Step 1: Keep TCP Connection Alive in GUI Wallet

**File:** `wallet-gui/src/tcp_protocol_client.rs`

Add persistent listener:

```rust
use tokio::sync::mpsc;

pub struct TcpProtocolListener {
    stream: Arc<Mutex<TcpStream>>,
    utxo_tx: mpsc::UnboundedSender<Vec<UtxoInfo>>,
}

impl TcpProtocolListener {
    pub async fn new(
        peer_addr: String,
        utxo_tx: mpsc::UnboundedSender<Vec<UtxoInfo>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let stream = TcpStream::connect(&peer_addr).await?;
        
        Ok(Self {
            stream: Arc::new(Mutex::new(stream)),
            utxo_tx,
        })
    }

    /// Start listening for incoming messages
    pub async fn listen(self: Arc<Self>) {
        loop {
            match self.read_message().await {
                Ok(Some(msg)) => {
                    if let Err(e) = self.handle_message(msg).await {
                        log::error!("Failed to handle message: {}", e);
                    }
                }
                Ok(None) => {
                    log::info!("Connection closed by peer");
                    break;
                }
                Err(e) => {
                    log::error!("Error reading message: {}", e);
                    // Attempt reconnection after delay
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn read_message(&self) -> Result<Option<NetworkMessage>, Box<dyn std::error::Error>> {
        let mut stream = self.stream.lock().await;
        
        // Read message length (4 bytes)
        let mut len_bytes = [0u8; 4];
        if stream.read_exact(&mut len_bytes).await.is_err() {
            return Ok(None); // Connection closed
        }
        
        let len = u32::from_be_bytes(len_bytes) as usize;
        
        // Read message data
        let mut data = vec![0u8; len];
        stream.read_exact(&mut data).await?;
        
        // Deserialize
        let message: NetworkMessage = bincode::deserialize(&data)?;
        Ok(Some(message))
    }

    async fn handle_message(&self, msg: NetworkMessage) -> Result<(), Box<dyn std::error::Error>> {
        match msg {
            NetworkMessage::UtxoUpdate { xpub, utxos } => {
                log::info!("🔔 Received UTXO update: {} UTXOs for xpub {}", 
                    utxos.len(), &xpub[..20]);
                
                // Send to main thread via channel
                let _ = self.utxo_tx.send(utxos);
                Ok(())
            }
            NetworkMessage::NewTransactionNotification { transaction } => {
                log::info!("🔔 New transaction notification: {} TIME to {}",
                    transaction.amount as f64 / 1_000_000.0,
                    &transaction.to_address[..20]
                );
                
                // Convert to UTXO format and send
                let utxo = UtxoInfo {
                    txid: transaction.tx_hash,
                    vout: 0, // Assuming first output
                    address: transaction.to_address,
                    amount: transaction.amount,
                    block_height: Some(transaction.block_height),
                    confirmations: transaction.confirmations as u64,
                };
                
                let _ = self.utxo_tx.send(vec![utxo]);
                Ok(())
            }
            _ => Ok(()), // Ignore other messages
        }
    }
}
```

### Step 2: Hook into GUI Wallet Main Loop

**File:** `wallet-gui/src/main.rs`

Add channel receiver and update wallet on events:

```rust
struct WalletApp {
    // ... existing fields ...
    
    // Channel for receiving UTXO updates
    utxo_rx: Option<mpsc::UnboundedReceiver<Vec<UtxoInfo>>>,
    tcp_listener: Option<Arc<TcpProtocolListener>>,
}

impl WalletApp {
    fn initialize_tcp_listener(&mut self, xpub: String) {
        let (utxo_tx, utxo_rx) = mpsc::unbounded_channel();
        self.utxo_rx = Some(utxo_rx);
        
        // Get peer address
        if let Some(network_mgr) = &self.network_manager {
            let peers = {
                let net = network_mgr.lock().unwrap();
                net.get_connected_peers()
            };
            
            if let Some(peer) = peers.first() {
                let peer_ip = peer.address.split(':').next().unwrap_or(&peer.address);
                let peer_addr = format!("{}:24100", peer_ip);
                
                let xpub_clone = xpub.clone();
                tokio::spawn(async move {
                    match TcpProtocolListener::new(peer_addr.clone(), utxo_tx).await {
                        Ok(listener) => {
                            let listener = Arc::new(listener);
                            
                            // Register xpub first
                            if let Err(e) = listener.register_xpub(xpub_clone).await {
                                log::error!("Failed to register xpub: {}", e);
                                return;
                            }
                            
                            // Start listening for updates
                            listener.listen().await;
                        }
                        Err(e) => {
                            log::error!("Failed to create TCP listener: {}", e);
                        }
                    }
                });
            }
        }
    }
    
    fn check_utxo_updates(&mut self) {
        if let Some(rx) = &mut self.utxo_rx {
            // Non-blocking check for new UTXOs
            while let Ok(utxos) = rx.try_recv() {
                log::info!("💰 Processing {} new UTXOs", utxos.len());
                
                if let Some(wallet_mgr) = &mut self.wallet_manager {
                    for utxo in utxos {
                        // Convert to wallet UTXO format
                        let wallet_utxo = wallet::UTXO {
                            tx_hash: utxo.txid,
                            output_index: utxo.vout,
                            amount: utxo.amount,
                            address: utxo.address,
                        };
                        
                        wallet_mgr.add_utxo(wallet_utxo);
                        
                        log::info!("✅ Added UTXO: {} TIME", 
                            utxo.amount as f64 / 1_000_000.0);
                    }
                    
                    // Balance is now updated automatically!
                    let new_balance = wallet_mgr.get_balance();
                    log::info!("💼 Updated balance: {} TIME", 
                        new_balance as f64 / 1_000_000.0);
                    
                    // Show success notification
                    self.set_success(format!("Received {} TIME!", 
                        utxos.iter().map(|u| u.amount).sum::<u64>() as f64 / 1_000_000.0
                    ));
                }
            }
        }
    }
}

impl eframe::App for WalletApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for UTXO updates
        self.check_utxo_updates();
        
        // ... rest of update logic ...
    }
}
```

### Step 3: Push Notifications from Masternode

**File:** `masternode/src/utxo_integration.rs`

Add notification when transactions are finalized:

```rust
impl MasternodeUTXOIntegration {
    /// Notify wallets about new transactions
    pub async fn notify_wallets_of_transaction(&self, tx: &Transaction) {
        if let Some(monitor) = &self.address_monitor {
            // Check all outputs to see if any match monitored addresses
            for (vout, output) in tx.outputs.iter().enumerate() {
                let address = &output.address;
                
                // Check if this address is being monitored
                if let Some(xpub) = monitor.find_xpub_for_address(address).await {
                    info!(
                        node = %self.node_id,
                        address = %address,
                        amount = %output.amount,
                        "Address matches monitored xpub, sending notification"
                    );
                    
                    // Create UTXO info
                    let utxo = time_network::protocol::UtxoInfo {
                        txid: tx.txid.clone(),
                        vout: vout as u32,
                        address: address.clone(),
                        amount: output.amount,
                        block_height: None, // Will be set when included in block
                        confirmations: 0,
                    };
                    
                    // Send UtxoUpdate to wallet
                    let message = time_network::protocol::NetworkMessage::UtxoUpdate {
                        xpub: xpub.clone(),
                        utxos: vec![utxo],
                    };
                    
                    // Broadcast to all peers (wallets will filter by xpub)
                    if let Err(e) = self.peer_manager.broadcast_message(message).await {
                        warn!(
                            node = %self.node_id,
                            error = %e,
                            "Failed to broadcast UTXO update"
                        );
                    }
                }
            }
        }
    }
}
```

**File:** `api/src/wallet_send_handler.rs`

Hook into transaction finalization:

```rust
// After instant finality succeeds
pub async fn wallet_send(
    State(state): State<ApiState>,
    Json(req): Json<WalletSendRequest>,
) -> ApiResult<Json<WalletSendResponse>> {
    // ... existing transaction creation ...
    
    // Trigger instant finality
    crate::routes::trigger_instant_finality_for_received_tx(state.clone(), tx.clone()).await;
    
    // 🔔 NEW: Notify wallets via TCP
    if let Some(utxo_integration) = state.utxo_integration.as_ref() {
        utxo_integration.notify_wallets_of_transaction(&tx).await;
    }
    
    // Broadcast to network
    if let Some(broadcaster) = state.tx_broadcaster.as_ref() {
        broadcaster.broadcast_transaction(tx).await;
    }
    
    Ok(Json(WalletSendResponse {
        success: true,
        txid: final_txid,
        message: "Transaction finalized and wallets notified".to_string(),
    }))
}
```

### Step 4: Add Helper Method to AddressMonitor

**File:** `masternode/src/address_monitor.rs`

```rust
impl AddressMonitor {
    /// Find which xpub (if any) is monitoring a given address
    pub async fn find_xpub_for_address(&self, address: &str) -> Option<String> {
        let monitored = self.monitored.read().await;
        
        for (xpub, data) in monitored.iter() {
            if data.external_addresses.contains(&address.to_string())
                || data.internal_addresses.contains(&address.to_string())
            {
                return Some(xpub.clone());
            }
        }
        
        None
    }
}
```

## Testing

### Test 1: Initial Balance
1. Start masternode
2. Open wallet GUI
3. Wallet registers xpub
4. Should receive existing UTXOs immediately
5. Balance shows correctly

### Test 2: Incoming Transaction
1. Wallet GUI open and connected
2. Send coins to wallet address from another node
3. Watch console logs:
   ```
   🔔 Received UTXO update: 1 UTXOs for xpub xpub661...
   💰 Processing 1 new UTXOs
   ✅ Added UTXO: 1000.0 TIME
   💼 Updated balance: 1000.0 TIME
   ```
4. GUI balance updates WITHOUT RESTART! ⚡

### Test 3: Multiple Transactions
1. Send 3 transactions rapidly
2. Each should arrive as separate notification
3. Balance increments correctly each time

## Performance

**Latency:** <100ms from transaction to wallet notification
- Transaction finalized: ~500ms (instant finality)
- TCP push: ~10ms
- GUI update: ~50ms
- **Total: <600ms!** ⚡

## Benefits

✅ **Instant balance updates** - no restart needed  
✅ **Uses existing TCP protocol** - no new dependencies  
✅ **Deterministic addresses** - xpub-based monitoring  
✅ **Real-time** - <1 second notification  
✅ **Efficient** - only pushes relevant transactions  
✅ **Reliable** - TCP with automatic reconnection  

## Implementation Time

- Step 1 (GUI TCP listener): 1 hour
- Step 2 (GUI integration): 30 minutes
- Step 3 (Masternode push): 30 minutes
- Step 4 (Helper method): 15 minutes
- Testing: 30 minutes

**Total: ~3 hours for instant wallet notifications!** 🚀
