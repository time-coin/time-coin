# TIME Coin Consensus Documentation

## Overview

TIME Coin uses a VRF-based leader selection mechanism with Byzantine Fault Tolerance (BFT) to achieve secure, scalable consensus.

## Consensus Safety Properties

### Byzantine Fault Tolerance

TIME Coin's consensus mechanism is designed to tolerate Byzantine (malicious) nodes:

- **Assumption**: At most f malicious nodes among n total nodes
- **Requirement**: n ≥ 3f + 1 (total nodes ≥ 3 × malicious + 1)
- **Quorum**: ⌈2n/3⌉ (at least 2/3 of nodes must agree)

This ensures:
1. Even with f malicious nodes, honest nodes (n - f ≥ 2f + 1) can form quorum
2. A malicious coalition of f nodes cannot force consensus (f < n/3 < 2n/3)
3. Consensus is safe as long as ≤ f nodes are Byzantine

### Examples

| Total Nodes | Required Votes | Byzantine Tolerance |
|-------------|----------------|---------------------|
| 3           | 2 (67%)        | 0 Byzantine         |
| 4           | 3 (75%)        | 1 Byzantine         |
| 7           | 5 (71%)        | 2 Byzantine         |
| 10          | 7 (70%)        | 3 Byzantine         |
| 100         | 67 (67%)       | 33 Byzantine        |

### Key Properties

1. **Safety**: No two different blocks can be finalized at the same height
2. **Liveness**: Network can make progress as long as > 2/3 nodes are honest and online
3. **Byzantine Resistance**: Can tolerate up to 1/3 malicious nodes
4. **Fork Prevention**: VRF seed includes chain state when synced

## VRF Design

### Purpose

Verifiable Random Function (VRF) provides:
- **Unpredictability**: Attackers cannot predict future leaders beyond current block
- **Verifiability**: Anyone can verify leader selection was correct
- **Determinism**: All honest nodes agree on the same leader
- **Fairness**: Each node has proportional chance of selection

### Seed Generation

The VRF seed is generated differently based on synchronization state:

```rust
fn generate_seed(height: u64, previous_hash: &str, is_synced: bool) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(b"TIME_COIN_VRF_SEED");
    hasher.update(height.to_le_bytes());
    
    // Only include hash when synced
    if is_synced && height > 0 {
        hasher.update(previous_hash.as_bytes());
    }
    
    hasher.finalize().to_vec()
}
```

### Why Conditional Hash Inclusion?

**During Bootstrap/Sync** (`is_synced = false`):
- Nodes may have different chain tips
- Using previous_hash would cause disagreement on leader
- Uses ONLY height for universal agreement

**When Synchronized** (`is_synced = true`):
- All nodes share the same canonical chain
- Including previous_hash makes leader selection fork-specific
- Prevents same leader on both sides of a fork
- Adds chain-state dependency for additional security

### Attack Resistance

1. **Leader Prediction**: Requires breaking SHA256 to predict future leaders
2. **Leader Manipulation**: Cannot manipulate selection without controlling 2/3 of nodes
3. **Fork Creation**: Different forks select different leaders (when synced)
4. **Grinding**: Single attempt per block height prevents grinding attacks

## Consensus Flow

### 1. Leader Selection

```
For block height H:
1. All nodes compute VRF seed from (height, prev_hash, sync_state)
2. VRF selects leader from masternode list
3. Leader is deterministic and verifiable
```

### 2. Block Proposal

```
Leader creates block:
1. Collect transactions from mempool
2. Calculate deterministic rewards
3. Create merkle root
4. Sign and broadcast proposal
```

### 3. Validation & Voting

```
Other nodes:
1. Verify leader is correct (VRF check)
2. Validate block structure
3. Check transaction validity
4. Vote approve/reject
```

### 4. Finalization

```
Once ⌈2n/3⌉ votes received:
1. Block is finalized
2. Broadcast to network
3. Update chain state
4. Begin next height
```

## Byzantine Detection

The consensus system actively detects Byzantine behavior:

### Violation Types

1. **Double Voting**: Voting for different blocks at same height
2. **Invalid Proposals**: Proposing blocks that violate rules
3. **Contradictory Votes**: Approving conflicting transactions
4. **Unavailability**: Consistently offline or non-responsive

### Severity Levels

- **Minor**: Single failure (could be network issue)
- **Moderate**: Repeated failures (2-3 occurrences)
- **Severe**: Clear malicious intent (double-voting)
- **Critical**: Coordinated attack pattern

### Response

1. **Detection**: Monitor and log violations
2. **Quarantine**: Reduce weight/influence of suspect nodes
3. **Removal**: Governance vote to remove Byzantine nodes
4. **Slashing**: Penalize stake (if applicable)

## Rate Limiting

Vote spam protection:

- **Per-Peer Limit**: Max 3 votes per peer per height
- **Automatic Cleanup**: Old height data removed
- **Memory Protection**: Prevents DoS via vote spam

## Configuration

### Quorum Settings

```rust
// Automatic 2/3 BFT quorum
let required = quorum::required_for_bft(total_nodes);

// Custom threshold
let required = quorum::calculate_required_votes(
    total_nodes,
    numerator,    // e.g., 2
    denominator   // e.g., 3
);
```

### Rate Limiting

```rust
let config = RateLimitConfig {
    max_votes_per_peer_per_height: 3,
    history_depth: 10,
};
```

### Byzantine Detection

```rust
let detector = ByzantineDetector::new(
    violation_threshold: 3  // moderate violations before considered Byzantine
);
```

## Network Assumptions

1. **Synchrony**: Eventual message delivery (not strictly synchronous)
2. **Connectivity**: > 2/3 of nodes can communicate
3. **Honesty**: ≤ 1/3 of nodes are Byzantine
4. **Time**: Loose time synchronization (NTP recommended)

## Security Considerations

### What We Protect Against

✅ Byzantine nodes (up to 1/3)
✅ Network partitions (with > 2/3 majority)
✅ Vote spam
✅ Double voting
✅ Invalid proposals
✅ Leader grinding
✅ Fork attacks (when synced)

### What We Don't Protect Against

❌ > 1/3 Byzantine nodes (violates BFT assumption)
❌ Complete network partition (no side has 2/3)
❌ All nodes offline
❌ Sybil attacks (requires separate masternode registration)

## Monitoring

Key metrics to track:

1. **Consensus Time**: Time to finalize blocks
2. **Vote Participation**: % of nodes voting
3. **Byzantine Violations**: Count and type
4. **Network Health**: Connectivity between nodes
5. **Leader Distribution**: Fairness of selection

## Recovery Scenarios

### Network Partition Heals

```
1. Sides exchange chain state
2. Longest chain (most cumulative weight) wins
3. Losing side reorganizes to winning chain
4. Consensus resumes on unified chain
```

### Byzantine Node Detected

```
1. Violations logged and broadcast
2. Node quarantined (reduced weight)
3. If violations continue, governance vote for removal
4. Stake slashed (if applicable)
```

### Mass Unavailability

```
1. If < 2/3 nodes available, consensus stalls
2. Emergency mode: lower threshold (requires manual intervention)
3. Wait for nodes to return online
4. Resume normal consensus once 2/3 available
```

## Testing

Comprehensive test coverage includes:

- ✅ BFT quorum calculation
- ✅ VRF determinism and unpredictability
- ✅ Byzantine detection (all violation types)
- ✅ Rate limiting
- ✅ Double vote prevention
- ⏳ Network partition scenarios (TODO)
- ⏳ Byzantine leader scenarios (TODO)
- ⏳ Large network simulation (1000+ nodes) (TODO)

## Future Improvements

1. **Dynamic Quorum**: Adjust based on network conditions
2. **Stake Weighting**: Weight votes by stake amount
3. **Slashing**: Automatic penalty for Byzantine behavior
4. **Reputation System**: Track long-term node reliability
5. **Fast Finality**: Parallel validation for faster consensus
