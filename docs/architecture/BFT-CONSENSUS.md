# Byzantine Fault Tolerant Consensus

## Overview

TIME Coin uses BFT consensus for instant transaction finality with masternode voting.

## How It Works

### 1. Transaction Submitted
```
User → Transaction → Network
```

### 2. Quorum Selection
- Calculate quorum size (7-50 nodes)
- Use VRF with transaction hash as seed
- Select masternodes weighted by:
  - Tier (1x, 10x, 100x)
  - Longevity (up to 3x)
  - Reputation (up to 2x)

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
- Calculate voting power:
  - Approve power
  - Reject power
- Check threshold (67% = 2/3+)

### 5. Consensus Result
- **Approved**: Transaction confirmed instantly
- **Rejected**: Transaction invalid
- **Timeout**: Try again with new quorum

## Voting Power

### Base Weight by Tier
- Community: 1x
- Verified: 10x  
- Professional: 100x

### Longevity Multiplier
```
multiplier = 1.0 + (years_active × 0.5)
max = 3.0

Examples:
- 1 year:  1.5x
- 2 years: 2.0x
- 4+ years: 3.0x
```

### Reputation Multiplier
```
multiplier = reputation / 100
min = 0.5
max = 2.0

Examples:
- 50 reputation:  0.5x
- 100 reputation: 1.0x
- 200 reputation: 2.0x
```

### Total Power
```
voting_power = base_weight × longevity × reputation

Professional (4 years, 150 rep):
100 × 3.0 × 1.5 = 450 power

Verified (1 year, 100 rep):
10 × 1.5 × 1.0 = 15 power

Community (6 months, 80 rep):
1 × 1.0 × 0.8 = 0.8 power
```

## Quorum Size

Dynamic based on network size:

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
- Minimum 7 nodes (f=3)
- Weighted voting reduces attack surface

### Sybil Resistance
- High collateral requirements
- Weighted by tier
- Long-term reputation matters

### Deterministic Selection
- VRF ensures fairness
- Same transaction = same quorum
- No favoritism possible

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
