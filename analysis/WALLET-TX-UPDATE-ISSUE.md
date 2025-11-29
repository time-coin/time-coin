# Wallet Transaction Update Issue - Analysis

**Date**: November 18, 2025  
**Problem**: Wallet not receiving transaction updates despite WebSocket connection

---

## Root Cause Found

The wallet connects to masternode WebSocket successfully and subscribes to addresses, BUT:

**❌ The masternode never broadcasts transaction notifications to subscribed clients!**

## What's Working

1. ✅ Wallet connects to WebSocket
2. ✅ Wallet subscribes to addresses
3. ✅ Masternode accepts subscriptions
4. ✅ Masternode stores subscribed addresses

## What's NOT Working

1. ❌ Masternode never calls `broadcast_transaction()`
2. ❌ No integration between transaction processing and WebSocket clients
3. ❌ Transactions processed but not pushed to wallets

---

## Code Analysis

### WsBridge (masternode/src/ws_bridge.rs)

**BEFORE (Lines 20-40)**:
```rust
pub struct WsBridge {
    addr: String,
    clients: Arc<RwLock<HashMap<String, WsClient>>>,
}

// NO method to broadcast transactions!
```

**AFTER (Fixed)**:
```rust
pub struct WsBridge {
    addr: String,
    clients: Arc<RwLock<HashMap<String, WsClient>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionNotification {
    pub txid: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub amount: u64,
    pub timestamp: i64,
}

impl WsBridge {
    /// Broadcast transaction to all subscribed clients ✅ ADDED
    pub async fn broadcast_transaction(&self, notification: TransactionNotification) {
        let clients = self.clients.read().await;
        let mut relevant_addresses: Vec<String> = Vec::new();
        relevant_addresses.extend(notification.inputs.clone());
        relevant_addresses.extend(notification.outputs.clone());

        for (client_id, client) in clients.iter() {
            let is_relevant = client.addresses.iter().any(|addr| {
                relevant_addresses.contains(addr)
            });

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
}
```

---

## What Still Needs to Be Done

### 1. Integrate WsBridge with Transaction Processing

The masternode needs to:

1. **Pass WsBridge reference to transaction processor**
2. **Call broadcast_transaction() when transactions are processed**
3. **Extract addresses from transaction inputs/outputs**

### 2. Update Masternode Main (main.rs)

Current code starts WsBridge but never connects it to transaction processing:

```rust
// main.rs (current)
let bridge = Arc::new(WsBridge::new(ws_addr.clone()));
tokio::spawn(async move {
    if let Err(e) = bridge_clone.start().await {
        eprintln!("❌ WebSocket bridge error: {}", e);
    }
});

// ❌ NO transaction processing loop!
// ❌ Bridge never receives transactions!
```

**Needed**:
```rust
// Create shared bridge
let bridge = Arc::new(WsBridge::new(ws_addr));

// Start WebSocket server
let bridge_clone = bridge.clone();
tokio::spawn(async move {
    bridge_clone.start().await
});

// Start transaction processor
let bridge_clone = bridge.clone();
tokio::spawn(async move {
    transaction_processor_loop(bridge_clone).await
});
```

### 3. Create Transaction Processor Loop

Need to add a function that:

```rust
async fn transaction_processor_loop(bridge: Arc<WsBridge>) {
    // Listen for transactions from P2P network
    // or mempool, or consensus module
    
    loop {
        // Get next transaction (from where??)
        let tx = get_next_transaction().await;
        
        // Extract addresses
        let mut inputs = Vec::new();
        for input in &tx.inputs {
            inputs.push(extract_address_from_input(input));
        }
        
        let mut outputs = Vec::new();
        for output in &tx.outputs {
            outputs.push(extract_address_from_output(output));
        }
        
        // Create notification
        let notification = TransactionNotification {
            txid: tx.txid.to_string(),
            inputs,
            outputs,
            amount: tx.total_output(),
            timestamp: tx.timestamp,
        };
        
        // Broadcast to subscribed wallets
        bridge.broadcast_transaction(notification).await;
    }
}
```

### 4. Connect to Actual Transaction Source

The big question: **Where do transactions come from?**

Options:
1. **P2P Network**: Transactions received from other nodes
2. **Mempool**: Transactions waiting for confirmation
3. **Consensus Module**: Transactions being validated
4. **RPC/API**: Transactions submitted via API

Currently, the masternode main.rs has NO transaction processing!

---

## Architectural Issue

The masternode has these components but they're NOT CONNECTED:

```
┌─────────────────────────────────────────┐
│           Masternode                    │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────────┐    ┌──────────────┐  │
│  │  WsBridge    │    │ Transaction  │  │
│  │  (WebSocket) │ ❌ │  Processing  │  │
│  │              │    │   (MISSING)  │  │
│  └──────────────┘    └──────────────┘  │
│                                         │
│  ┌──────────────┐    ┌──────────────┐  │
│  │  P2P Network │    │   Mempool    │  │
│  │  (port 24000)│    │  (exists?)   │  │
│  └──────────────┘    └──────────────┘  │
│                                         │
└─────────────────────────────────────────┘

                ❌ NO CONNECTION!
```

**Needed**:
```
┌─────────────────────────────────────────┐
│           Masternode                    │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────────┐    ┌──────────────┐  │
│  │  WsBridge    │◄───│ Transaction  │  │
│  │  (WebSocket) │    │  Processor   │  │
│  └──────────────┘    └──────┬───────┘  │
│         ▲                   │          │
│         │                   │          │
│         │     ┌──────────────▼──────┐  │
│         └─────┤   Mempool/P2P       │  │
│               │   Transaction       │  │
│               │   Events            │  │
│               └─────────────────────┘  │
│                                         │
└─────────────────────────────────────────┘
```

---

## Immediate Next Steps

1. ✅ **DONE**: Added `broadcast_transaction()` method to WsBridge
2. ✅ **DONE**: Exported `TransactionNotification`
3. ⏳ **TODO**: Find where transactions are received/processed
4. ⏳ **TODO**: Integrate WsBridge with transaction flow
5. ⏳ **TODO**: Test end-to-end notification

---

## Questions to Answer

1. **Where does the masternode receive transactions?**
   - From P2P network?
   - From mempool?
   - From consensus module?
   - Via RPC API?

2. **Is there a mempool module?**
   - Check `time-mempool` crate
   - Is it integrated with masternode?

3. **How are transactions validated?**
   - Does `utxo_integration.rs` process them?
   - Is there a transaction queue?

4. **What triggers transaction processing?**
   - P2P message handler?
   - Periodic polling?
   - Event-driven?

---

## Files Modified

1. ✅ `masternode/src/ws_bridge.rs`
   - Added `TransactionNotification` struct
   - Added `broadcast_transaction()` method

2. ✅ `masternode/src/lib.rs`
   - Exported `TransactionNotification`

3. ⏳ `masternode/src/main.rs` (needs update)
   - Need to integrate WsBridge with transaction source

---

## Testing Plan

Once integrated:

```bash
# Terminal 1: Start masternode
cargo run --bin time-masternode

# Terminal 2: Start wallet
cargo run --bin wallet-gui

# Terminal 3: Send test transaction
time-cli send --from=<addr> --to=<addr> --amount=100

# Verify:
# 1. Masternode logs show "Sent transaction notification to client X"
# 2. Wallet GUI shows new transaction in UI
# 3. Wallet balance updates
```

---

## Status

**Current State**: Diagnosed but not fully fixed

**Blocking Issue**: Need to find transaction source/processor in masternode

**Next Action**: Investigate time-mempool integration or P2P transaction handling

---

**Analysis by**: GitHub Copilot CLI  
**Date**: November 18, 2025  
**Status**: Partially fixed - broadcast method added, integration pending
