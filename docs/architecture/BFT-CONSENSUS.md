# Byzantine Fault Tolerant Consensus

## Overview

TIME Coin uses BFT consensus for instant transaction finality with masternode voting.

**Current Implementation**: The system uses VRF-based (Verifiable Random Function) leader selection with SHA256-based weighted random selection. Each block's leader is selected unpredictably but verifiably using a cryptographic seed derived from the block height and previous block hash. Weighted voting based on tier, longevity, and reputation is implemented in the phased protocol module.

## How It Works

### 1. Transaction Submitted
```
User → Transaction → Network
```

### 2. Quorum Selection
- Calculate quorum size (minimum 3 nodes)
- **Current**: VRF-based weighted selection using SHA256 for unpredictable but verifiable randomness
- Seed generated from: block height + previous block hash
- Selection includes cryptographic proof for verification

### 3. Voting Round
- Transaction broadcast to quorum
- Each masternode validates:
  - Sufficient balance
  - Valid signature
  - No double-spend
  - Proper format
- Cast vote (approve/reject)

### 4. Vote Collection
- Collect votes from quorum
- **Current**: Simple one-node-one-vote counting
- **Future**: Weighted voting power calculation:
  - Approve power
  - Reject power
- Check threshold (67% = 2/3+)

### 5. Consensus Result
- **Approved**: Transaction confirmed instantly
- **Rejected**: Transaction invalid
- **Timeout**: Try again with new quorum

## Voting Power

**Note**: The following weighted voting system is planned for future implementation. Current implementation uses simple one-node-one-vote counting.

### Base Weight by Tier (Planned)
- Free: 1x
- Bronze: 1x  
- Silver: 10x
- Gold: 100x

### Longevity Multiplier (Planned)
```
multiplier = 1.0 + (years_active × 0.5)
max = 3.0

Examples:
- 1 year:  1.5x
- 2 years: 2.0x
- 4+ years: 3.0x
```

### Reputation Multiplier (Planned)
```
multiplier = reputation / 100
min = 0.5
max = 2.0

Examples:
- 50 reputation:  0.5x
- 100 reputation: 1.0x
- 200 reputation: 2.0x
```

### Total Power (Planned)
```
voting_power = base_weight × longevity × reputation

Gold (4 years, 150 rep):
100 × 3.0 × 1.5 = 450 power

Silver (1 year, 100 rep):
10 × 1.5 × 1.0 = 15 power

Bronze (6 months, 80 rep):
1 × 1.0 × 0.8 = 0.8 power
```

## Quorum Size

**Current Implementation**: Quorum size is dynamically calculated based on network size: `(total_nodes * 2 / 3) + 1`

**Minimum Requirements**:
```
min = 3 nodes (tolerates 0 Byzantine failures)

Recommended for production:
- 4+ nodes (tolerates 1 Byzantine failure)
- 7+ nodes (tolerates 2 Byzantine failures)
- 10+ nodes (tolerates 3 Byzantine failures)

Examples:
- 3 nodes:   3 quorum (100%)
- 10 nodes:  7 quorum (70%)
- 100 nodes: 67 quorum (67%)
```

**Future Enhancement**: Dynamic logarithmic scaling
```
size = log2(total_nodes) × 7
min = 7 (for f=3 Byzantine faults)
max = 50 (efficiency cap)

Examples:
- 10 nodes:   7 quorum
- 100 nodes:  49 quorum
- 1000 nodes: 50 quorum (capped)
```

## Security Guarantees

### Byzantine Fault Tolerance
- Tolerates up to f < n/3 malicious nodes
- **Minimum 3 nodes** (tolerates 0 Byzantine failures - basic consensus)
- **Recommended 4+ nodes** for production (tolerates 1 Byzantine failure)
- **Recommended 7+ nodes** for high security (tolerates 2 Byzantine failures)
- **Future**: Weighted voting will reduce attack surface further

### Sybil Resistance
- High collateral requirements
- Weighted by tier (implemented in phased protocol)
- Long-term reputation matters (implemented in phased protocol)

### VRF Security Properties
- **Unpredictability**: Attacker cannot predict which nodes will be selected in future rounds
- **Verifiability**: Any node can verify that leader selection was done correctly
- **Uniqueness**: Given same inputs, VRF produces same output (deterministic)
- **Pseudo-randomness**: Output is indistinguishable from true random
- **Manipulation resistance**: Cannot manipulate selection without controlling previous block hash
- **Fair distribution**: All eligible nodes have fair chance proportional to their weight

### Attack Resistance
- **Targeted attacks**: VRF prevents attackers from predicting and targeting future validators
- **Grinding attacks**: Using previous block hash prevents attackers from grinding for favorable selections
- **Eclipse attacks**: Random selection makes it harder to isolate specific validators
- **Collusion**: Distributed randomness source (previous block) prevents coordinated manipulation

### Deterministic Yet Unpredictable Selection
- **VRF-based selection**: Uses SHA256-based Verifiable Random Function
- Same inputs (block height + previous hash) = same leader (deterministic)
- Cannot predict future leaders without knowing future block hashes (unpredictable)
- Selection includes cryptographic proof for verification
- No favoritism possible - selection is mathematically fair

## Performance

### Latency
- Quorum selection: <1ms
- Vote collection: 100-500ms
- Total finality: <1 second

### Throughput
- Parallel quorums
- Thousands of TPS possible
- Limited by network, not consensus

## Implementation Example

```rust
use time_consensus::{ConsensusEngine, Transaction};

let engine = ConsensusEngine::new();

let tx = Transaction {
    txid: "tx123".to_string(),
    from: "addr1".to_string(),
    to: "addr2".to_string(),
    amount: 100_00000000,
    fee: 1_00000000,
    timestamp: Utc::now().timestamp(),
    nonce: 0,
};

// Validate with BFT
let result = engine.validate_transaction(&tx, &all_masternodes)?;

if result.approved {
    // Transaction confirmed!
    state.confirm_transaction(&tx.txid);
}
```

## Recovery

If consensus fails:
1. Wait random backoff
2. Select new quorum
3. Retry validation
4. Maximum 3 attempts
5. Then queue for next block

## Future Enhancements

- Async voting (don't wait for all votes)
- Threshold signatures
- Cross-shard consensus
- Optimistic execution
