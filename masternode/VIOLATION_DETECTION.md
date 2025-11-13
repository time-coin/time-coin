# Automatic Violation Detection for Masternodes

This document describes the automated violation detection system implemented in the TIME Coin masternode network. The system monitors masternode behavior and automatically identifies and penalizes violations to ensure network security and integrity.

## Overview

The violation detection system is designed to protect the TIME Coin network from Byzantine faults and malicious masternode behavior. It operates continuously, monitoring masternode activities and triggering penalties when violations are detected.

### Design Principles

1. **Automated Detection**: All violations are detected automatically without manual intervention
2. **Modular Architecture**: Each violation type has its own detection logic for easy extension
3. **Evidence-Based**: Every violation is recorded with detailed evidence for audit trails
4. **Graduated Penalties**: Penalties are proportional to violation severity
5. **Real-Time Response**: Critical violations trigger immediate actions (auto-ban, slashing)

## Violation Types

The system detects five categories of violations:

### 1. Double-Signing (Critical)

**Description**: A masternode signs two different blocks at the same block height.

**Why It's Critical**: Double-signing is a fundamental Byzantine fault that can lead to blockchain forks and undermine consensus safety. In BFT consensus, if more than 1/3 of masternodes double-sign, the network could experience safety violations.

**Detection Algorithm**:
```
For each block signature received:
1. Extract (masternode_id, block_height, block_hash, signature)
2. Check if masternode_id has already signed a block at block_height
3. If yes AND the block_hash differs:
   - Record violation with both signatures as evidence
   - Flag for immediate action
4. Store signature for future checks
```

**Evidence Collected**:
- Block height where double-signing occurred
- Both block hashes
- Both signatures
- Timestamp of detection

**Penalties**:
- **Reputation**: -1000 (maximum penalty)
- **Collateral Slash**: 100% (total loss of stake)
- **Auto-Ban**: Yes (immediate removal from active validator set)

**Implementation**: `ViolationDetector::check_double_signing()`

### 2. Invalid Block Creation (High Severity)

**Description**: A masternode creates a block that fails validation rules.

**Why It Matters**: Invalid blocks waste network resources and can indicate malicious intent or serious implementation bugs. Examples include:
- Invalid merkle roots
- Invalid transactions (double-spends, invalid signatures)
- Timestamp manipulation
- Incorrect block rewards

**Detection Algorithm**:
```
When a block fails validation:
1. Identify the masternode that created the block
2. Record the specific validation failure
3. Store block hash and failure reason as evidence
4. Track repeat offenses for pattern analysis
```

**Evidence Collected**:
- Block height and hash
- Specific validation failure reason
- Timestamp of detection

**Penalties**:
- **Reputation**: -500
- **Collateral Slash**: 10%
- **Auto-Ban**: No (but multiple violations may trigger ban)

**Implementation**: `ViolationDetector::check_invalid_block()`

### 3. Extended Downtime (Medium Severity)

**Description**: A masternode is offline for more than 90 consecutive days.

**Why It Matters**: Extended downtime indicates masternode abandonment. Inactive masternodes still hold collateral but don't contribute to network security, effectively reducing the active validator set and weakening BFT guarantees.

**Detection Algorithm**:
```
For each masternode in periodic health check:
1. Calculate: time_offline = current_time - last_heartbeat
2. If time_offline > max_downtime_threshold (default: 90 days):
   - Calculate days_offline = time_offline / (24 * 60 * 60)
   - Record violation with downtime evidence
```

**Evidence Collected**:
- Last seen timestamp
- Detection timestamp
- Total days offline

**Penalties**:
- **Reputation**: -200
- **Collateral Slash**: 5%
- **Auto-Ban**: No (masternode can recover by coming online)

**Configuration**: Threshold is configurable via `DetectorConfig.max_downtime_seconds`

**Implementation**: `ViolationDetector::check_extended_downtime()`

### 4. Data Withholding (High Severity)

**Description**: A masternode repeatedly fails to respond to valid data requests.

**Why It Matters**: Masternodes must serve blockchain data to other nodes. Withholding data is a form of censorship attack that can prevent nodes from syncing or validating transactions. This can also indicate:
- Selective censorship
- Availability attacks
- Storage failures

**Detection Algorithm**:
```
Track data request success/failure per masternode:
1. For each data request to a masternode:
   - Record request type and outcome
   - Increment failed_count if request fails
   - Increment total_count
2. Periodically check:
   - If failed_count >= max_failed_requests (default: 10):
     - Record violation
     - Reset counters
```

**Evidence Collected**:
- Request type (e.g., "block_data", "transaction_data")
- Number of failed responses
- Timestamp of detection

**Penalties**:
- **Reputation**: -400
- **Collateral Slash**: 20%
- **Auto-Ban**: No (but repeated violations may trigger ban)

**Configuration**: Threshold is configurable via `DetectorConfig.max_failed_requests`

**Implementation**: `ViolationDetector::record_data_request()` and `ViolationDetector::check_data_withholding()`

### 5. Network Manipulation (Critical)

**Description**: Attempts to subvert consensus through coordinated attacks.

**Why It's Critical**: Network manipulation attacks can compromise the entire blockchain if successful. Examples include:
- **Coordinated Voting**: Multiple masternodes coordinating to manipulate governance votes
- **Sybil Attacks**: Single entity controlling multiple masternodes to gain outsized influence
- **Consensus Attacks**: Coordinated actions to prevent consensus or force specific outcomes

**Detection Algorithm**:
```
Requires higher-level analysis:
1. Pattern Detection:
   - Analyze voting patterns for unusual coordination
   - Monitor IP addresses for Sybil detection
   - Track consensus participation for manipulation attempts
2. When suspicious pattern detected:
   - Flag involved masternodes
   - Record evidence of coordination
   - Trigger investigation/automatic action
```

**Evidence Collected**:
- Type of manipulation detected
- Detailed evidence (varies by attack type)
- Timestamp and involved parties

**Penalties**:
- **Reputation**: -1000 (maximum penalty)
- **Collateral Slash**: 100%
- **Auto-Ban**: Yes (immediate removal)

**Implementation**: `ViolationDetector::check_network_manipulation()`

## Penalty System

### Reputation Penalties

All violations affect masternode reputation scores (range: -1000 to +1000):
- Scores below -100: Masternode becomes ineligible for rewards
- Scores below -500: Marked as "Very Poor" reputation
- Minimum score: -1000 (complete loss of trust)

Reputation can be slowly recovered through good behavior, except for masternodes that are auto-banned.

### Collateral Slashing

Violations result in partial or complete loss of staked collateral:

| Violation Type | Slash Percentage |
|----------------|-----------------|
| Double-Signing | 100% |
| Network Manipulation | 100% |
| Data Withholding | 20% |
| Invalid Block | 10% |
| Extended Downtime | 5% |

Slashed collateral is either:
1. Burned (removed from circulation)
2. Redistributed to the treasury
3. Distributed to honest masternodes

The specific distribution is determined by network governance.

### Auto-Ban

Critical violations (double-signing, network manipulation) trigger automatic ban:
1. Masternode is immediately removed from active validator set
2. Cannot participate in consensus
3. Cannot earn rewards
4. Collateral is fully slashed

Auto-banned masternodes must be manually reviewed before being allowed to rejoin (if ever).

## Configuration

The detection system is configurable through `DetectorConfig`:

```rust
pub struct DetectorConfig {
    /// Maximum downtime in seconds (default: 90 days)
    pub max_downtime_seconds: u64,
    
    /// Failed data requests threshold (default: 10)
    pub max_failed_requests: u32,
    
    /// Enable/disable individual detectors
    pub enable_double_sign_detection: bool,
    pub enable_invalid_block_detection: bool,
    pub enable_downtime_detection: bool,
    pub enable_data_withholding_detection: bool,
    pub enable_network_manipulation_detection: bool,
}
```

### Example Configuration

```rust
// Strict configuration for production
let config = DetectorConfig {
    max_downtime_seconds: 60 * 24 * 60 * 60, // 60 days
    max_failed_requests: 5,                   // Lower threshold
    enable_double_sign_detection: true,
    enable_invalid_block_detection: true,
    enable_downtime_detection: true,
    enable_data_withholding_detection: true,
    enable_network_manipulation_detection: true,
};

let detector = ViolationDetector::new(config);
```

## Memory Management

To prevent memory bloat from storing historical signatures:

### Signature Cleanup

The detector automatically cleans old block signatures:

```rust
// Clean up signatures older than 1000 blocks
detector.cleanup_old_signatures(current_height, 1000);
```

This should be called periodically (e.g., every 100 blocks) to maintain reasonable memory usage.

## Monitoring and Statistics

### Get Detection Statistics

```rust
let stats = detector.get_stats();
println!("Total violations: {}", stats.total);
println!("Double-signing: {}", stats.double_signing);
println!("Invalid blocks: {}", stats.invalid_blocks);
```

### Query Violations

```rust
// Get all violations
let all_violations = detector.get_violations();

// Get violations for specific masternode
let mn_violations = detector.get_violations_for_masternode("mn_id");
```

## Integration Example

```rust
use time_masternode::detector::{ViolationDetector, DetectorConfig, BlockSignature};
use time_masternode::violations::ViolationType;

// Initialize detector
let mut detector = ViolationDetector::default();

// Check for double-signing
let signature = BlockSignature {
    block_height: 1000,
    block_hash: "abc123...".to_string(),
    signature: "sig456...".to_string(),
    masternode_id: "mn_001".to_string(),
    timestamp: current_timestamp(),
};

if let Some(violation) = detector.check_double_signing(signature, current_timestamp())? {
    if violation.should_auto_ban() {
        // Immediately ban the masternode
        ban_masternode(&violation.masternode_id);
    }
    
    // Apply penalties
    let collateral = get_masternode_collateral(&violation.masternode_id);
    let mut violation = violation;
    violation.apply_penalty(collateral)?;
    
    // Slash collateral
    slash_collateral(&violation.masternode_id, violation.collateral_slashed);
    
    // Update reputation
    update_reputation(&violation.masternode_id, violation.reputation_penalty);
}
```

## Security Considerations

### False Positive Prevention

1. **Evidence Collection**: All violations include detailed evidence for manual review if needed
2. **Thresholds**: Configurable thresholds prevent temporary issues from triggering penalties
3. **Graduated Response**: Most violations don't result in auto-ban, allowing recovery

### Resistance to Gaming

1. **Cryptographic Evidence**: Double-signing violations include cryptographic signatures
2. **Timestamp Verification**: All evidence includes timestamps to prevent replay attacks
3. **Pattern Analysis**: Network manipulation detection uses multiple signals

### Disaster Recovery

If the detection system incorrectly penalizes masternodes:
1. Evidence can be reviewed by governance
2. Penalties can be reversed through governance vote
3. Collateral can be restored if appropriate

## Future Extensions

The modular design allows for easy addition of new violation types:

### Potential Future Violations

1. **Latency Violations**: Consistently slow response times
2. **Selective Processing**: Processing only profitable transactions
3. **Version Non-compliance**: Running outdated or modified software
4. **Spam Behavior**: Flooding network with invalid messages
5. **Coordination Patterns**: More sophisticated analysis of collusion

### Adding New Violations

To add a new violation type:

1. Add variant to `ViolationType` enum in `violations.rs`
2. Define severity, penalties, and auto-ban behavior
3. Implement detection logic in `detector.rs`
4. Add comprehensive tests in `tests/violation_detection.rs`
5. Update this documentation

## Testing

Comprehensive test suite covers:
- All violation types with simulated attacks
- Edge cases and threshold boundaries
- Configuration variations
- False positive scenarios
- Memory management (signature cleanup)

Run tests:
```bash
cargo test --package time-masternode
```

Specific violation tests:
```bash
cargo test --package time-masternode --test violation_detection
```

## Conclusion

The automatic violation detection system provides robust protection against malicious masternode behavior without requiring manual intervention. By detecting and penalizing violations automatically, the system ensures that:

1. **Security**: Network remains secure even under attack
2. **Incentive Alignment**: Bad actors are economically punished
3. **Decentralization**: No trusted party needed to enforce rules
4. **Transparency**: All violations and penalties are recorded with evidence

This system is critical for maintaining TIME Coin's security guarantees, especially as the network scales and the value at stake increases.
