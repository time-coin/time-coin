# Async Message Handling Architecture

## Problem Solved

**Before:** Protocol collision on shared TCP connections
- Heartbeat ping/pong blocked by vote ACK waiting
- Sync messages arrived when expecting vote responses
- Connections removed due to "unexpected message" errors

## Architecture

### Message Handler Pattern

The `MessageHandler` provides **full-duplex async communication** with:

1. **Separate Send/Receive Loops**
   - Send loop: Processes outgoing message queue
   - Receive loop: Reads incoming messages continuously
   - No blocking between send and receive operations

2. **Two Communication Modes**

#### Fire-and-Forget (Broadcasts)
```rust
handler.send(NetworkMessage::TransactionBroadcast(tx))?;
// Returns immediately, doesn't wait for response
```

#### Request-Response (Queries)
```rust
let response = handler.send_with_response(
    NetworkMessage::GetBlockchainInfo,
    Duration::from_secs(3)
).await?;
// Waits for response with timeout
```

3. **Message Multiplexing**
   - Multiple concurrent requests supported
   - Each request gets unique ID
   - Responses routed to correct waiting channel
   - Broadcast messages routed to handler function

## Usage Example

```rust
// Create handler with broadcast message processor
let handler = MessageHandler::new(
    tcp_stream,
    Arc::new(|msg| {
        match msg {
            NetworkMessage::TransactionBroadcast(tx) => process_tx(tx),
            NetworkMessage::ConsensusProposal(p) => handle_proposal(p),
            _ => {}
        }
    })
);

// Fire-and-forget broadcast
handler.send(NetworkMessage::TransactionBroadcast(tx))?;

// Request with response
let info = handler.send_with_response(
    NetworkMessage::GetBlockchainInfo,
    Duration::from_secs(3)
).await?;
```

## Benefits

✅ **No Protocol Collision** - Send and receive operate independently
✅ **Concurrent Operations** - Multiple requests in-flight simultaneously  
✅ **Non-Blocking Broadcasts** - Fire-and-forget for speed
✅ **Request-Response** - When you need confirmation
✅ **Auto Message Routing** - Responses matched to pending requests
✅ **Clean Separation** - Heartbeats, broadcasts, queries don't interfere

## Integration Status

**Phase 1 (Current):** Infrastructure created
- `MessageHandler` struct with send/receive loops
- Request-response multiplexing
- Broadcast handler support

**Phase 2 (Next):** Integrate with PeerConnection
- Replace direct `send_message`/`receive_message` calls
- Use MessageHandler for all peer communication
- Remove manual ACK waiting logic

**Phase 3 (Future):** Protocol Enhancement
- Add request IDs to NetworkMessage enum
- Perfect request-response matching
- Implement proper message acknowledgment system
