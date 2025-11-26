# Masternode HTTP API Audit - MUST USE TCP

## Critical Finding

Masternodes are using HTTP API (port 24101) for node-to-node communication.
This is **WRONG** - all masternode communication should use **TCP protocol** (port 24100).

## HTTP API Usage (Must be Fixed)

### ❌ chain_sync.rs - Blockchain Synchronization
**Current (HTTP):**
```rust
// Line 165
let url = format!("http://{}:24101/blockchain/info", peer_ip);
let response = reqwest::get(&url).await;

// Line 182
let url = format!("http://{}:24101/blockchain/block/{}", peer_ip, height);
let response = reqwest::get(&url).await;
```

**Should Be (TCP):**
```rust
// Use PeerManager
peer_manager.request_blockchain_info(peer_ip).await;
peer_manager.sync_recent_blocks(peer_ip, from_height, to_height).await;
```

### ❌ block_producer.rs - Block Download
**Current (HTTP):**
```rust
// Lines 317, 345, 426, 444, 636
let url = format!("http://{}:24101/blockchain/info", peer_ip);
let url = format!("http://{}:24101/blockchain/block/{}", peer_ip, height);
```

**Should Be (TCP):**
Use PeerManager TCP methods

### ❌ block_producer.rs - Finalized Block Broadcast
**Current (HTTP):**
```rust
// Lines 1413, 2446
let url = format!("http://{}:24101/consensus/finalized-block", peer_ip);
let _ = reqwest::Client::new().post(&url).json(&block).send().await;
```

**Should Be (TCP):**
```rust
// Create TCP message type or use existing UpdateTip
let message = NetworkMessage::UpdateTip {
    height: block.header.block_number,
    hash: block.hash.clone(),
};
peer_manager.send_to_peer_tcp(peer_ip, message).await;
```

### ❌ block_producer.rs - Catch-up Requests
**Current (HTTP):**
```rust
// Line 2473
let url = format!("http://{}:24101/network/catch-up-request", peer_ip);
let _ = reqwest::Client::new().post(&url).json(&request).send().await;
```

**Should Be (TCP):**
Already have TCP message: `NetworkMessage::CatchUpRequest`

### ❌ bft_consensus.rs - Finalized Block Broadcast  
**Current (HTTP):**
```rust
// Line 349
let url = format!("http://{}:24101/consensus/finalized-block", node);
let client = reqwest::Client::new();
client.post(&url_clone).json(&block_clone).send().await;
```

**Should Be (TCP):**
Use PeerManager

### ❌ bft_consensus.rs - Request Block Proposal
**Current (HTTP):**
```rust
// Line 374
let url = format!("http://{}:24101/consensus/request-block-proposal", leader);
match reqwest::Client::new().post(&url).json(&request).send().await;
```

**Should Be (TCP):**
Already have TCP message: `NetworkMessage::RequestBlockProposal`

### ❌ main.rs - Finality Sync
**Current (HTTP):**
```rust
// Line 282
let url = format!("http://{}:24101/finality/sync", ip_only);
reqwest::Client::new().post(&url).json(&request).send().await;
```

**Should Be (TCP):**
Already have TCP message: `NetworkMessage::RequestFinalizedTransactions`

### ❌ main.rs - Mempool Sync
**Current (HTTP):**
```rust
// Line 387
let url = format!("http://{}:24101/mempool/all", ip_only);
reqwest::Client::new().get(&url).send().await;
```

**Should Be (TCP):**
Already have TCP message: `NetworkMessage::MempoolQuery / MempoolResponse`

## ✅ CLI Tools (OK to use HTTP)

These are **CLIENT** tools that query masternodes, NOT masternode-to-masternode:
- `time-cli` - Command line client
- `time-dashboard` - Monitoring dashboard  
- Wallet GUI - User interface

These can continue using HTTP API because they're external clients, not part of
the masternode consensus network.

## TCP Messages Already Available

The protocol already defines these TCP messages:
```rust
pub enum NetworkMessage {
    GetBlockchainInfo,
    BlockchainInfo { height: u64, best_block_hash: String },
    GetBlocks { start_height: u64, end_height: u64 },
    BlocksData(Vec<BlockData>),
    CatchUpRequest { requester, current_height, expected_height },
    CatchUpAcknowledge { responder },
    MempoolQuery,
    MempoolResponse(Vec<Transaction>),
    RequestFinalizedTransactions { since_timestamp },
    FinalizedTransactionsResponse { transactions, finalized_at },
    ConsensusBlockProposal(String),
    ConsensusBlockVote(String),
    RequestBlockProposal { block_height, leader_ip, requester_ip },
    UpdateTip { height, hash },
}
```

## PeerManager Methods Available

```rust
impl PeerManager {
    // Already implemented:
    pub async fn request_blockchain_info(&self, peer_addr: &str) -> Result<u64>;
    pub async fn sync_recent_blocks(&self, peer_addr, from, to) -> Result<Vec<Block>>;
    pub async fn request_mempool(&self, peer_addr: &str) -> Result<Vec<Transaction>>;
    pub async fn broadcast_block_proposal(&self, proposal: serde_json::Value);
    pub async fn broadcast_block_vote(&self, vote: serde_json::Value);
    pub async fn send_to_peer_tcp(&self, peer_ip: IpAddr, message: NetworkMessage);
}
```

## Action Plan

1. Replace all HTTP calls in `chain_sync.rs` with TCP
2. Replace all HTTP calls in `block_producer.rs` with TCP
3. Replace all HTTP calls in `bft_consensus.rs` with TCP
4. Replace all HTTP calls in `main.rs` (masternode startup) with TCP
5. Keep HTTP API only for:
   - External clients (time-cli, wallet-gui, dashboard)
   - Initial peer discovery from time-coin.io
   - RPC interface for management

## Why This Matters

**HTTP API Problems:**
- ❌ Requires port 24101 open (extra firewall config)
- ❌ Different protocol than consensus
- ❌ No proper handshake/magic bytes validation
- ❌ Not part of BFT protocol
- ❌ Can be blocked/filtered separately from TCP
- ❌ Extra dependency (reqwest crate)

**TCP Protocol Benefits:**
- ✅ Single port 24100 for all masternode communication
- ✅ Proper handshake with magic bytes
- ✅ Part of BFT consensus protocol
- ✅ Persistent connections (faster)
- ✅ Built-in message framing
- ✅ Consistent with consensus messages

## Current State

**Working:**
- ✅ Consensus proposals/votes now use TCP (just fixed)

**Broken:**
- ❌ Blockchain sync still uses HTTP
- ❌ Block downloads still use HTTP
- ❌ Mempool sync still uses HTTP  
- ❌ Finality sync still uses HTTP
- ❌ Catch-up coordination still uses HTTP

**Result:** Masternodes can't communicate properly because:
1. TCP connections established for consensus
2. But blockchain sync tries HTTP
3. HTTP not accessible/configured
4. Sync fails, nodes stay behind
5. Consensus fails because nodes at different heights

## Fix Priority

**CRITICAL (blocks consensus):**
1. chain_sync.rs - blockchain info/blocks
2. block_producer.rs - catch-up/sync
3. main.rs - mempool sync

**Important (reduces reliability):**
4. bft_consensus.rs - finalized block broadcast
5. block_producer.rs - finalized block broadcast

## Testing After Fix

```bash
# On each masternode, verify ONLY TCP is used:
sudo tcpdump -i any 'port 24100' -nn

# Should see TCP traffic
# Should NOT see port 24101 traffic between masternodes

# Port 24101 should only be used by:
# - time-cli (external client)
# - wallet-gui (external client)
# - time-dashboard (external client)
```

## Summary

**Problem:** Masternodes using HTTP API for node-to-node communication
**Solution:** Replace ALL HTTP with TCP protocol messages  
**Impact:** Consensus will work, nodes will sync properly, single port config
