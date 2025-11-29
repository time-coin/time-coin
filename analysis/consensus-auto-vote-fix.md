# Consensus Auto-Vote Fix - 2025-11-28

## Problem Identified

Nodes were failing to reach consensus during catch-up because they were not voting on proposals received from other nodes.

### Root Cause

When a node created a block proposal during catch-up:
1. Node creates deterministic block proposal
2. Node votes APPROVE on its own proposal
3. Node broadcasts proposal to peers
4. **BUG**: When node receives identical proposal from peer, it skips voting because it "already voted"
5. Result: Only 1 vote collected (from itself), consensus fails (needs 4/6 votes)

### Key Diagnostic Logs

```
📤 Broadcasting proposal to 5 peers
   ✓ Proposal sent to 165.232.154.150
   ✓ Proposal sent to 50.28.104.50
   ...
   ✓ Voted APPROVE on block f375cae1f9efbbaa
📤 Broadcasting vote to 5 peers
   ...
   ⚡ 1/6 votes (need 4) - 1025ms elapsed
   ⚡ 1/6 votes (need 4) - 2050ms elapsed
   ⚠️  Vote stalled at 1/6 after 2101ms - ending attempt
   ...
   📋 Block 1 proposal received from network!
   ℹ️  Skipping auto-vote (already voted)  ← THE BUG
```

## The Fix

Changed the auto-vote logic in `cli/src/block_producer.rs`:

### Before
```rust
// Check if we already voted
if self.block_consensus.has_voted(block_num, &self.node_id).await {
    println!("   ℹ️  Skipping auto-vote (already voted)");
    continue;
}
```

### After
```rust
// Always auto-vote on proposals received from network to help consensus
// Since blocks are deterministic, multiple nodes will create identical blocks
// and we should vote on matching proposals even if we already voted locally
{
    // Auto-vote on the proposal to help reach consensus
    println!("   🗳️  Auto-voting APPROVE to help consensus...");
    
    let vote = BlockVote {
        block_height: block_num,
        voter: self.node_id.clone(),
        block_hash: proposal.block_hash.clone(),
        approve: true,
        timestamp: Utc::now().timestamp(),
    };
    
    // Send vote to consensus manager
    if let Err(e) = self.block_consensus.vote_on_block(vote.clone()).await {
        eprintln!("   ⚠️  Auto-vote failed: {}", e);
    } else {
        println!("   ✅ Auto-vote successful!");
        
        // Broadcast the vote to other nodes
        if let Ok(vote_value) = serde_json::to_value(&vote) {
            self.peer_manager.broadcast_block_vote(vote_value).await;
        }
    }
}
```

## Why This Works

In deterministic consensus:
- All nodes create **identical** blocks at the same height
- When Node A receives a proposal from Node B, it's the **same block** Node A created
- Nodes should vote on **any** matching proposal to help reach consensus quickly
- The consensus manager (`BlockConsensusManager`) handles duplicate votes internally
- This ensures votes aggregate properly: if 4 nodes create identical blocks and all vote on all proposals they receive, consensus reaches 4/4 quickly

## Expected Behavior After Fix

1. All 6 nodes create identical block #4
2. Each node votes on its own proposal → 6 votes recorded locally
3. Each node broadcasts proposal to 5 peers
4. When Node A receives proposal from Node B (same block hash):
   - Node A auto-votes APPROVE
   - Node A broadcasts vote to all peers
5. Votes aggregate quickly: 4/6 votes achieved in <500ms
6. Block finalizes, all nodes advance to block #5

## Network Communication Health

Also discovered and fixed during debugging:
- Added self-connection filtering (nodes don't try to connect to themselves)
- Added broken pipe detection and reconnection logic
- Added diagnostic logging for TCP connection health

## Files Modified

- `cli/src/block_producer.rs` - Removed "skip if already voted" check, always vote on received proposals
- `network/src/peer_manager.rs` - Added self-connection filtering, broken pipe handling

## Testing

Run on testnet with 6 masternodes to verify:
1. Catch-up consensus completes successfully
2. Blocks finalize in <2 seconds
3. No "vote stalled" messages
4. Expected log: "✅ Auto-vote successful!" when receiving proposals
