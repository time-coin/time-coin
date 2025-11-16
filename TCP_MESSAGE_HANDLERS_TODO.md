# TCP Message Handlers - Implementation Guide

## Overview

This document provides implementation guidance for the TCP message handlers needed to complete the TCP-only communication migration.

## Where to Implement

The message handlers should be implemented in the main node event loop where TCP connections are managed. This is typically in the node's main listening loop where it receives messages from peers.

## Required Message Handlers

### 1. GetGenesis Handler

**Request**: `NetworkMessage::GetGenesis`
**Response**: `NetworkMessage::GenesisBlock(String)`

```rust
NetworkMessage::GetGenesis => {
    // Get genesis block from blockchain state
    let chain = blockchain_state.read().await;
    let genesis_block = chain.get_block(0)?;
    let genesis_json = serde_json::to_string(&genesis_block)?;
    
    // Send response
    connection.send_message(
        NetworkMessage::GenesisBlock(genesis_json)
    ).await?;
}
```

### 2. GetMempool Handler

**Request**: `NetworkMessage::GetMempool`
**Response**: `NetworkMessage::MempoolResponse(Vec<Transaction>)`

```rust
NetworkMessage::GetMempool => {
    // Get all transactions from mempool
    let transactions = mempool.get_all_transactions().await;
    
    // Send response
    connection.send_message(
        NetworkMessage::MempoolResponse(transactions)
    ).await?;
}
```

### 3. GetBlockchainInfo Handler

**Request**: `NetworkMessage::GetBlockchainInfo`
**Response**: `NetworkMessage::BlockchainInfo { height, best_block_hash }`

```rust
NetworkMessage::GetBlockchainInfo => {
    // Get blockchain info
    let chain = blockchain_state.read().await;
    let height = chain.height();
    let best_block_hash = chain.best_block_hash().to_string();
    
    // Send response
    connection.send_message(
        NetworkMessage::BlockchainInfo {
            height,
            best_block_hash,
        }
    ).await?;
}
```

### 4. GetSnapshot Handler

**Request**: `NetworkMessage::GetSnapshot`
**Response**: `NetworkMessage::Snapshot(String)`

```rust
NetworkMessage::GetSnapshot => {
    // Create snapshot from current state
    let chain = blockchain_state.read().await;
    let snapshot = Snapshot {
        height: chain.height(),
        state_hash: chain.state_hash().to_string(),
        balances: chain.get_all_balances(),
        masternodes: chain.get_all_masternodes(),
        timestamp: chrono::Utc::now().timestamp(),
    };
    let snapshot_json = serde_json::to_string(&snapshot)?;
    
    // Send response
    connection.send_message(
        NetworkMessage::Snapshot(snapshot_json)
    ).await?;
}
```

### 5. GetPeerList Handler

**Request**: `NetworkMessage::GetPeerList`
**Response**: `NetworkMessage::PeerList(Vec<PeerAddress>)`

```rust
NetworkMessage::GetPeerList => {
    // Get connected peers
    let peers = peer_manager.get_connected_peers().await;
    
    // Convert to PeerAddress format
    let peer_addresses: Vec<PeerAddress> = peers.iter().map(|p| {
        PeerAddress {
            ip: p.address.ip().to_string(),
            port: p.address.port(),
            version: p.version.clone(),
        }
    }).collect();
    
    // Send response
    connection.send_message(
        NetworkMessage::PeerList(peer_addresses)
    ).await?;
}
```

### 6. Receive PeerList Handler

**Message**: `NetworkMessage::PeerList(Vec<PeerAddress>)`
**Action**: Add discovered peers to peer manager

```rust
NetworkMessage::PeerList(peer_addresses) => {
    // Add each peer to peer manager
    for peer_addr in peer_addresses {
        peer_manager.add_discovered_peer(
            peer_addr.ip,
            peer_addr.port,
            peer_addr.version,
        ).await;
    }
}
```

### 7. ConsensusTxProposal Handler

**Message**: `NetworkMessage::ConsensusTxProposal(String)`
**Action**: Forward to consensus engine

```rust
NetworkMessage::ConsensusTxProposal(proposal_json) => {
    // Parse proposal
    let proposal: TxProposal = serde_json::from_str(&proposal_json)?;
    
    // Forward to consensus engine
    consensus_engine.handle_tx_proposal(proposal).await?;
}
```

### 8. ConsensusTxVote Handler

**Message**: `NetworkMessage::ConsensusTxVote(String)`
**Action**: Forward to consensus engine

```rust
NetworkMessage::ConsensusTxVote(vote_json) => {
    // Parse vote
    let vote: TxVote = serde_json::from_str(&vote_json)?;
    
    // Forward to consensus engine
    consensus_engine.handle_tx_vote(vote).await?;
}
```

### 9. ConsensusBlockProposal Handler

**Message**: `NetworkMessage::ConsensusBlockProposal(String)`
**Action**: Forward to block consensus manager

```rust
NetworkMessage::ConsensusBlockProposal(proposal_json) => {
    // Parse block proposal
    let proposal: BlockProposal = serde_json::from_str(&proposal_json)?;
    
    // Forward to block consensus manager
    block_consensus_manager.handle_block_proposal(proposal).await?;
}
```

### 10. ConsensusBlockVote Handler

**Message**: `NetworkMessage::ConsensusBlockVote(String)`
**Action**: Forward to block consensus manager

```rust
NetworkMessage::ConsensusBlockVote(vote_json) => {
    // Parse block vote
    let vote: BlockVote = serde_json::from_str(&vote_json)?;
    
    // Forward to block consensus manager
    block_consensus_manager.handle_block_vote(vote).await?;
}
```

## Main Event Loop Structure

```rust
// Main node event loop (pseudocode)
pub async fn run_node(
    peer_manager: Arc<PeerManager>,
    blockchain_state: Arc<RwLock<BlockchainState>>,
    mempool: Arc<Mempool>,
    consensus_engine: Arc<ConsensusEngine>,
    block_consensus_manager: Arc<BlockConsensusManager>,
) -> Result<(), Error> {
    let listener = TcpListener::bind(listen_addr).await?;
    
    loop {
        // Accept incoming connections
        let (stream, peer_addr) = listener.accept().await?;
        
        let peer_manager = peer_manager.clone();
        let blockchain_state = blockchain_state.clone();
        let mempool = mempool.clone();
        let consensus_engine = consensus_engine.clone();
        let block_consensus_manager = block_consensus_manager.clone();
        
        tokio::spawn(async move {
            let mut connection = PeerConnection::new(stream);
            
            loop {
                match connection.receive_message().await {
                    Ok(message) => {
                        // Handle message based on type
                        match message {
                            NetworkMessage::GetGenesis => { /* handler */ },
                            NetworkMessage::GetMempool => { /* handler */ },
                            NetworkMessage::GetBlockchainInfo => { /* handler */ },
                            NetworkMessage::GetSnapshot => { /* handler */ },
                            NetworkMessage::GetPeerList => { /* handler */ },
                            NetworkMessage::PeerList(_) => { /* handler */ },
                            NetworkMessage::ConsensusTxProposal(_) => { /* handler */ },
                            NetworkMessage::ConsensusTxVote(_) => { /* handler */ },
                            NetworkMessage::ConsensusBlockProposal(_) => { /* handler */ },
                            NetworkMessage::ConsensusBlockVote(_) => { /* handler */ },
                            // ... other message types ...
                        }
                    }
                    Err(e) => {
                        warn!("Connection error: {}", e);
                        break;
                    }
                }
            }
        });
    }
}
```

## Error Handling

All handlers should include proper error handling:

```rust
match message {
    NetworkMessage::GetGenesis => {
        match handle_get_genesis(&blockchain_state, &mut connection).await {
            Ok(_) => {},
            Err(e) => {
                error!("Failed to handle GetGenesis: {}", e);
                // Optionally send error response
            }
        }
    }
    // ... other handlers ...
}
```

## Testing

After implementing the handlers, test each message type:

1. Test genesis block request/response
2. Test mempool synchronization
3. Test blockchain info query
4. Test snapshot transfer
5. Test peer list exchange
6. Test transaction proposal flow
7. Test transaction vote flow
8. Test block proposal flow
9. Test block vote flow

## Notes

- All handlers run asynchronously
- Use proper timeouts for responses (see `TCP_ONLY_MIGRATION.md`)
- Log all message handling for debugging
- Handle malformed messages gracefully
- Validate message contents before processing
- Consider rate limiting for certain message types
