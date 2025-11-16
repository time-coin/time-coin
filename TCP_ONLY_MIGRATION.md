# TCP-Only Communication Migration

## Summary

This document describes the migration of TIME Coin's node-to-node communication from a mixed HTTP/TCP model to a pure TCP-only model using existing connections. This change improves security, reduces latency, and simplifies the network protocol.

## Changes Made

### 1. Protocol Extensions (`network/src/protocol.rs`)

Added new message types to `NetworkMessage` enum for TCP-only communication:

```rust
pub enum NetworkMessage {
    // ... existing messages ...
    
    // Additional message types for full TCP communication
    GetGenesis,
    GenesisBlock(String),           // JSON serialized genesis block
    GetMempool,
    GetBlockchainInfo,
    BlockchainInfo { height: u64, best_block_hash: String },
    GetSnapshot,
    Snapshot(String),               // JSON serialized snapshot
    GetPeerList,
    PeerList(Vec<PeerAddress>),
    ConsensusTxProposal(String),    // JSON serialized tx proposal
    ConsensusTxVote(String),        // JSON serialized tx vote
}
```

### 2. Network Manager Updates (`network/src/manager.rs`)

#### Converted HTTP Methods to TCP:

**`request_genesis()`**
- **Before**: HTTP GET to `http://peer:24101/genesis`
- **After**: TCP message `GetGenesis` → `GenesisBlock` response

**`request_mempool()`**
- **Before**: HTTP GET to `http://peer:24101/mempool/all`
- **After**: TCP message `GetMempool` → `MempoolResponse` response

**`request_blockchain_info()`**
- **Before**: HTTP GET to `http://peer:24101/blockchain/info`
- **After**: TCP message `GetBlockchainInfo` → `BlockchainInfo` response

**`request_snapshot()`**
- **Before**: HTTP GET to `http://peer:24101/snapshot`
- **After**: TCP message `GetSnapshot` → `Snapshot` response

**`fetch_peers_from_api()`**
- **Before**: HTTP GET to `http://peer:24101/peers`
- **After**: TCP message `GetPeerList` → `PeerList` response

#### Removed HTTP Fallbacks:

**`broadcast_block_proposal()`**
- **Before**: Try TCP, fallback to HTTP POST
- **After**: TCP-only via `ConsensusBlockProposal` message

**`broadcast_block_vote()`**
- **Before**: Try TCP, fallback to HTTP POST
- **After**: TCP-only via `ConsensusBlockVote` message

**`broadcast_newly_connected_peer()`**
- **Before**: HTTP POST to `http://peer:24101/peers/discovered`
- **After**: TCP `PeerList` message with new peer info

#### Made Public:

**`send_to_peer_tcp()`** - Changed from `async fn` to `pub async fn` to allow external modules to send TCP messages directly.

### 3. Transaction Broadcasting (`network/src/lib.rs`)

Updated `tx_broadcast` module:

**`broadcast_tx_proposal()`**
- **Before**: HTTP POST to `http://peer:24101/consensus/tx-proposal`
- **After**: TCP message `ConsensusTxProposal`

**`broadcast_tx_vote()`**
- **Before**: HTTP POST to `http://peer:24101/consensus/tx-vote`
- **After**: TCP message `ConsensusTxVote`

Added `use tracing::debug;` for logging support.

## What Still Uses HTTP

### Acceptable HTTP Usage:

1. **External Peer Discovery** (`network/src/discovery.rs`)
   - `HttpDiscovery::fetch_peers()` - Queries `https://time-coin.io/api/peers`
   - This is acceptable as it's for bootstrapping from an external service

2. **Local API Server** (`api/src/lib.rs`)
   - The HTTP API server for CLI/GUI tools connecting to localhost
   - This is acceptable as it's for local tool interaction, not node-to-node communication

3. **CLI Tools** (`cli/src/`)
   - CLI tools use HTTP to connect to localhost API
   - This is acceptable for local administration

4. **Wallet GUI** (`wallet-gui/src/`)
   - GUI uses HTTP to connect to local node API
   - This is acceptable for local wallet operations

5. **Dashboard Tools** (`tools/masternode-dashboard/`)
   - Dashboard uses HTTP to connect to local node API
   - This is acceptable for monitoring

## Benefits of TCP-Only Node Communication

1. **Security**: Eliminates HTTP endpoints between nodes that could be exploited
2. **Efficiency**: Reuses established TCP connections instead of creating new HTTP requests
3. **Latency**: Reduced overhead from HTTP connection setup/teardown
4. **Simplicity**: Single protocol for all node-to-node communication
5. **Reliability**: TCP provides guaranteed delivery with built-in error handling
6. **Stateful**: Maintains connection state for better peer management

## Required API Server Changes

The API server needs to be updated to handle incoming TCP messages and respond appropriately. This includes:

1. Adding message handlers for:
   - `GetGenesis` → respond with `GenesisBlock`
   - `GetMempool` → respond with `MempoolResponse`
   - `GetBlockchainInfo` → respond with `BlockchainInfo`
   - `GetSnapshot` → respond with `Snapshot`
   - `GetPeerList` → respond with `PeerList`
   - `ConsensusTxProposal` → forward to consensus engine
   - `ConsensusTxVote` → forward to consensus engine
   - `ConsensusBlockProposal` → forward to block consensus
   - `ConsensusBlockVote` → forward to block consensus

2. The message handling should be implemented in the main node event loop where TCP connections are managed.

## Migration Path

### Phase 1: ✅ Complete
- Add new TCP message types to protocol
- Update network manager methods to use TCP
- Remove HTTP fallbacks
- Make `send_to_peer_tcp()` public
- Update transaction broadcasting to use TCP

### Phase 2: TODO
- Add TCP message handlers in the node's main event loop
- Test peer-to-peer TCP communication
- Verify all message types work correctly

### Phase 3: TODO (Optional Cleanup)
- Remove unused HTTP client dependencies if no longer needed for node communication
- Update documentation to reflect TCP-only design
- Consider removing unused `consensus.rs` fragment file

## Testing Recommendations

1. Test peer discovery and connection establishment
2. Verify genesis block synchronization via TCP
3. Test mempool synchronization between peers
4. Verify blockchain info queries work
5. Test snapshot transfer via TCP
6. Verify peer list exchange works
7. Test transaction proposal and voting
8. Test block proposal and voting
9. Verify instant finality protocol works over TCP

## Notes

- All node-to-node communication now uses existing TCP connections
- HTTP is only used for:
  - External peer discovery (bootstrap)
  - Local CLI/GUI tool connections
  - Local dashboard monitoring
- The magic bytes protocol ensures message integrity
- Each message includes a length prefix for proper framing
- Timeouts are configured per message type (5-30 seconds)
