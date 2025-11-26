# HTTP to TCP Migration - Remaining Work

## ✅ Completed
- **chain_sync.rs**: Blockchain height queries and finalized block broadcast now use TCP
- **block_producer.rs (consensus)**: Block proposals/votes now use TCP (committed earlier)
- **verify-protocol.sh**: Updated to check TCP connectivity instead of HTTP API

## ❌ Remaining HTTP Calls

### block_producer.rs (10 HTTP calls)
```
Line 317:  Blockchain info query (catch-up)
Line 345:  Block download (catch-up)  
Line 426:  Blockchain info query (incremental sync)
Line 444:  Block download (incremental sync)
Line 636:  Masternode height check
Line 1413: Finalized block broadcast
Line 1444: Block download from producer
Line 2259: Request block proposal
Line 2446: Finalized block broadcast
Line 2473: Catch-up request broadcast
```

### bft_consensus.rs (2 HTTP calls)
```
Line 349:  Finalized block broadcast
Line 374:  Request block proposal
```

### main.rs (2 HTTP calls)
```
Line 282:  Finality sync
Line 387:  Mempool sync
```

## Replacement Strategy

### For Blockchain Queries
**HTTP:** `GET http://node:24101/blockchain/info`
**TCP:** `peer_manager.request_blockchain_info("node_ip:port")`

### For Block Downloads
**HTTP:** `GET http://node:24101/blockchain/block/{height}`
**TCP:** Need to implement using `GetBlocks` / `BlocksData` messages
- Currently stubbed in chain_sync.rs
- Requires implementing in PeerManager

### For Finalized Block Broadcast
**HTTP:** `POST http://node:24101/consensus/finalized-block`
**TCP:** `NetworkMessage::UpdateTip { height, hash }`
- Already working in chain_sync.rs
- Apply same pattern to block_producer.rs and bft_consensus.rs

### For Catch-up Requests
**HTTP:** `POST http://node:24101/network/catch-up-request`
**TCP:** `NetworkMessage::CatchUpRequest { requester, current_height, expected_height }`
- Message already exists in protocol
- Just needs to be used

### For Request Block Proposal  
**HTTP:** `POST http://node:24101/consensus/request-block-proposal`
**TCP:** `NetworkMessage::RequestBlockProposal { block_height, leader_ip, requester_ip }`
- Message already exists
- Just needs to be used

### For Mempool Sync
**HTTP:** `GET http://node:24101/mempool/all`
**TCP:** `peer_manager.request_mempool(peer_addr)`
- Method already exists in PeerManager
- Just needs to be called

### For Finality Sync
**HTTP:** `POST http://node:24101/finality/sync`
**TCP:** `NetworkMessage::RequestFinalizedTransactions { since_timestamp }`
- Message already exists
- Method may need to be added to PeerManager

## Implementation Plan

### Phase 1: Easy Wins (use existing methods)
1. ✅ chain_sync.rs - blockchain height queries
2. ⏳ block_producer.rs - lines 317, 426, 636 (blockchain info)
3. ⏳ main.rs - line 387 (mempool sync)
4. ⏳ block_producer.rs - line 2473 (catch-up request)
5. ⏳ bft_consensus.rs - line 374 (request proposal)
6. ⏳ block_producer.rs - line 2259 (request proposal)

### Phase 2: Broadcast Updates
1. ✅ chain_sync.rs - UpdateTip message
2. ⏳ block_producer.rs - lines 1413, 2446 (finalized block)
3. ⏳ bft_consensus.rs - line 349 (finalized block)

### Phase 3: Block Downloads (needs full implementation)
1. ⏳ Implement proper GetBlocks/BlocksData handling in PeerManager
2. ⏳ block_producer.rs - lines 345, 444, 1444 (block downloads)
3. ⏳ chain_sync.rs - download_block() (currently stubbed)

### Phase 4: Testing
1. ⏳ Verify all nodes communicate via TCP only
2. ⏳ Test blockchain sync via TCP
3. ⏳ Test consensus via TCP
4. ⏳ Verify no HTTP traffic between masternodes

## Network Awareness

All TCP calls must use the correct port:
```rust
fn get_p2p_port(&self) -> u16 {
    match self.peer_manager.network {
        time_network::discovery::NetworkType::Mainnet => 24000,
        time_network::discovery::NetworkType::Testnet => 24100,
    }
}
```

## Testing Commands

### Verify TCP Traffic Only
```bash
# On each masternode
sudo tcpdump -i any 'port 24100' -nn
# Should see TCP handshakes and messages

# Verify NO HTTP between nodes  
sudo tcpdump -i any 'port 24101 and not src host CLIENT_IP' -nn
# Should be silent (no node-to-node HTTP)
```

### Check Connectivity
```bash
# Test TCP port
nc -zv node_ip 24100

# Verify nodes can connect
./scripts/verify-protocol.sh
```

## Current Status

**Files Fixed:** 1/3 (chain_sync.rs)
**HTTP Calls Removed:** 3/14 (21%)
**Estimated Time:** 2-3 hours for complete migration

## Notes

- HTTP API should ONLY be used by external clients (time-cli, wallet-gui, dashboard)
- Masternode-to-masternode communication MUST use TCP protocol
- All TCP messages need proper network awareness (mainnet vs testnet ports)
- Block download via TCP needs full GetBlocks/BlocksData implementation
