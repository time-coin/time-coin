# TCP Heartbeat/Keepalive Consolidation

## Problem Identified

**Root cause of broken pipe errors**: TCP connections were dying between heartbeat cycles.

### Previous Architecture Issues

1. **Heartbeat loop** (main.rs) - Ran every 60 seconds
   - Only counted nodes and printed status
   - **Did NOT test TCP connections**

2. **TCP Ping** (peer discovery) - Ran every 5 minutes  
   - Tested connection health
   - Too infrequent - connections died in between

3. **Result**: Connections became stale/broken between 5-minute ping cycles
   - Broadcasts failed with "Broken pipe (errno 32)"
   - Votes couldn't propagate
   - Consensus failed

## Solution Implemented

**Consolidated TCP keepalive into the 1-minute heartbeat:**

```rust
// TCP keepalive ping - send to all connected peers every heartbeat
for peer in peers.iter() {
    // Fire-and-forget ping to keep connection alive
    let _ = peer_mgr_heartbeat
        .send_message_to_peer(
            peer.address,
            time_network::protocol::NetworkMessage::Ping,
        )
        .await;
}
```

### Benefits

1. **Prevents connection staleness** - Ping every 60s keeps sockets alive
2. **No blocking** - Fire-and-forget approach doesn't slow heartbeat
3. **Simple** - One mechanism for both heartbeat and keepalive
4. **Reliable** - Connections stay healthy for broadcast/voting

## About Multiple Heartbeat Files

### Current Structure

- `masternode/src/heartbeat.rs` - Uptime/downtime tracking for slashing
- `consensus/src/heartbeat.rs` - Network synchronization (unused)  
- `cli/src/main.rs` - **Active heartbeat loop** with TCP keepalive

### Should consensus/ and masternode/ be merged?

**Analysis:**

**masternode/** - Node management concerns:
- Node registry
- Collateral tracking
- Rewards distribution
- Slashing/violations
- Reputation system

**consensus/** - Block production concerns:
- Leader election
- Voting protocols
- Proposal management
- Quorum calculation
- Block finalization

**Recommendation: KEEP SEPARATE**

These are distinct concerns with different responsibilities. However:

1. **Remove unused heartbeat implementations**:
   - `consensus/src/heartbeat.rs` (phased protocol - not used)
   - `masternode/src/heartbeat.rs` (uptime tracking - redundant)

2. **Keep the active heartbeat in cli/main.rs** with TCP keepalive

3. **Better names would help**:
   - `masternode/` → `node_management/`
   - `consensus/` → `block_production/`

This makes the separation clearer without mixing concerns.

## Testing

Deploy updated nodes and monitor:
- Connection stability (no more broken pipes)
- Heartbeat output shows stable node counts
- Votes propagate successfully
- Consensus reaches 4/6 threshold

## Related Issues

- Auto-vote logic fixed (see CONSENSUS_AUTO_VOTE.md)
- Self-connection filtering working
- TCP connection cleanup on errors

