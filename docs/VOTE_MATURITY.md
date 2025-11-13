# Vote Maturity Delays for Masternodes

## Overview

This feature implements vote maturity periods for newly registered masternodes to prevent instant takeover attacks by coordinated malicious actors. Masternodes must wait a tier-specific number of blocks after registration before they can participate in consensus voting.

## Maturity Periods by Tier

| Tier | Maturity Period | Rationale |
|------|-----------------|-----------|
| Community | 1 block | Lower voting power (1x), minimal risk |
| Verified | 3 blocks | Medium voting power (10x), moderate waiting period |
| Professional | 10 blocks | Highest voting power (50x), longest waiting period for maximum security |

The maturity periods are inversely proportional to the tier's voting power - higher voting power requires a longer maturity period to prevent attacks.

## Usage

### Checking Vote Eligibility

Before allowing a masternode to vote, you must check both its basic eligibility (active, synced, online) AND its maturity status:

```rust
use time_masternode::{CollateralTier, status::MasternodeStatus};

// Create or retrieve masternode status
let mut mn_status = MasternodeStatus::new(
    "pubkey123".to_string(),
    "192.168.1.100".to_string(),
    9000,
    100, // registration_block
);

// Ensure masternode is active and synced
mn_status.is_active = true;
mn_status.sync_status = SyncStatus::Synced;

// Determine the tier
let tier = CollateralTier::Verified;

// Check if masternode can vote at current block height
let current_block = 103;
if mn_status.can_vote_at_height(current_block, &tier) {
    // Masternode has reached maturity and is eligible to vote
    consensus.vote_on_block(block_hash, voter, approve).await?;
} else {
    // Masternode is not yet mature or not eligible
    println!("Masternode cannot vote yet");
}
```

### Consensus Integration

When implementing consensus voting, always validate maturity before accepting votes:

```rust
use time_consensus::block_consensus::BlockVote;

// Receive vote request
let vote = BlockVote {
    block_height: current_height,
    block_hash: block_hash.clone(),
    voter: masternode_address.clone(),
    approve: true,
    timestamp: chrono::Utc::now().timestamp(),
};

// IMPORTANT: Check maturity before voting
let mn_status = get_masternode_status(&vote.voter)?;
let tier = get_masternode_tier(&vote.voter)?;

if !mn_status.can_vote_at_height(vote.block_height, &tier) {
    return Err("Masternode has not reached vote maturity".to_string());
}

// Now safe to submit vote
block_consensus.vote_on_block(vote).await?;
```

## Admin Functions

### VoteMaturityConfig

The system provides a configuration structure for runtime adjustment of maturity periods:

```rust
use time_masternode::status::VoteMaturityConfig;

let mut config = VoteMaturityConfig::new();

// Default values
assert_eq!(config.community_maturity_blocks, 1);
assert_eq!(config.verified_maturity_blocks, 3);
assert_eq!(config.professional_maturity_blocks, 10);

// Admin: Update individual tier
config.set_professional_maturity(15);

// Admin: Emergency disable all maturity checks (use with caution!)
config.emergency_disable_maturity();

// Admin: Set uniform maturity for all tiers
config.emergency_set_all_maturity(5);
```

### Emergency Override Scenarios

**When to use emergency_disable_maturity():**
- Network is completely stalled due to insufficient mature masternodes
- Critical security patch requires immediate participation from all nodes
- Testing or development environments

**When to use emergency_set_all_maturity():**
- Suspected coordinated attack in progress - increase maturity for all tiers
- Network stabilization after crisis - temporarily reduce maturity
- Transitioning between security policies

**⚠️ IMPORTANT:** Emergency functions should only be used by authorized administrators and should be logged/audited.

## Security Considerations

### Attack Scenarios Prevented

1. **Instant Takeover Attack**: An attacker cannot register multiple masternodes and immediately vote to control consensus. They must wait the maturity period.

2. **Sybil Attack Mitigation**: Combined with collateral requirements, maturity periods make it expensive and time-consuming to create attack nodes.

3. **Coordinated Attack**: Multiple attackers coordinating to register nodes simultaneously cannot immediately take control.

### Example Attack Prevention

```rust
// Scenario: Attacker registers 10 Professional tier masternodes
let registration_block = 1000;
let current_block = 1000; // Attack happens immediately

for i in 0..10 {
    let mut attacker_node = MasternodeStatus::new(
        format!("attacker_{}", i),
        format!("10.0.0.{}", i),
        9000,
        registration_block,
    );
    attacker_node.is_active = true;
    attacker_node.sync_status = SyncStatus::Synced;
    
    let tier = CollateralTier::Professional;
    
    // BLOCKED: Cannot vote immediately
    assert!(!attacker_node.can_vote_at_height(current_block, &tier));
}

// Only after 10 blocks can they vote
let future_block = registration_block + 10;
// Now they can vote, but the network has time to detect suspicious activity
```

### Best Practices

1. **Always check maturity**: Never skip the `can_vote_at_height()` check in production code.

2. **Log maturity denials**: Log when votes are rejected due to maturity to detect attack attempts:
```rust
if !mn_status.can_vote_at_height(current_block, &tier) {
    log::warn!("Vote denied for {} - not yet mature (registered at block {}, current {})",
               voter, mn_status.registration_block, current_block);
    return Err("Vote maturity not reached");
}
```

3. **Monitor maturity settings**: Track changes to VoteMaturityConfig to detect unauthorized modifications.

4. **Combine with other security**: Maturity periods work best alongside:
   - Collateral requirements
   - KYC verification
   - Reputation systems
   - Network analysis for suspicious patterns

## Implementation Details

### Code Locations

- **Maturity configuration**: `masternode/src/status.rs` - `VoteMaturityConfig`
- **Tier maturity periods**: `masternode/src/lib.rs` - `CollateralTier::vote_maturity_blocks()`
- **Maturity checking**: `masternode/src/status.rs` - `MasternodeStatus::can_vote_at_height()`
- **Integration tests**: `masternode/tests/vote_maturity.rs`
- **Consensus documentation**: `consensus/src/lib.rs` - voting function docs

### Testing

Run the vote maturity tests:
```bash
# Run all masternode tests (includes maturity tests)
cargo test --package time-masternode

# Run only vote maturity integration tests
cargo test --package time-masternode --test vote_maturity

# Run specific test
cargo test --package time-masternode test_coordinated_attack_prevention_scenario
```

## Future Enhancements

Potential improvements to the maturity system:

1. **Dynamic maturity periods**: Adjust based on network conditions or threat level
2. **Reputation-based maturity**: Reduce maturity for masternodes with proven good behavior
3. **Progressive maturity**: Limited voting power that increases over time
4. **Cross-chain verification**: Validate masternode history on other chains
5. **ML-based attack detection**: Automatically increase maturity when suspicious patterns detected

## References

- Issue: "Enforce Vote Maturity Delays for Newly Registered Masternodes"
- Acceptance Criteria: ✅ All met
  - Maturity period checked before votes
  - Configurable delay duration via VoteMaturityConfig
  - Comprehensive code and test coverage
  - Admin functions for emergency updates
