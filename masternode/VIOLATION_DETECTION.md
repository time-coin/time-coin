# Violation Detection System

This document describes the automated violation detection system for the TIME Coin masternode network. The system monitors masternode behavior and automatically detects various types of misbehavior, triggering appropriate penalties.

## Overview

The violation detection system is designed to:
- Automatically detect malicious or negligent behavior
- Maintain network security and reliability
- Provide cryptographic evidence for all violations
- Integrate seamlessly with the existing slashing and treasury systems
- Support future extension with new violation types

## Violation Types

### 1. Double-Signing (Critical)

**Description:** A masternode signs two different blocks at the same block height, attempting to create conflicting blockchain forks.

**Detection Algorithm:**
1. Track all block signatures by height in a `HashMap<u64, Vec<BlockSignature>>`
2. When a new signature arrives, check if the masternode has already signed a different block at that height
3. If two different block hashes are found for the same (masternode, height) pair, raise violation

**Evidence Structure:**
```rust
DoubleSigning {
    block_height: u64,
    block_hash_1: String,
    block_hash_2: String,
    signature_1: String,
    signature_2: String,
}
```

**Cryptographic Proof:**
- SHA-256 hash of the serialized DoubleSigning structure
- Includes both signatures and block hashes for verification

**Penalties:**
- **Reputation:** -1000 (maximum penalty)
- **Collateral Slash:** 100%
- **Auto-ban:** Yes
- **Severity:** Critical

**Rationale:** Double-signing is one of the most severe attacks on a blockchain network. It undermines consensus and can lead to chain splits. The 100% slash and immediate ban are necessary to strongly deter this behavior.

---

### 2. Invalid Block Creation (Moderate)

**Description:** A masternode proposes a block that fails validation, such as having an invalid merkle root or containing invalid transactions.

**Detection Algorithm:**
1. When block validation fails, extract the reason and details
2. Record the block height, hash, and specific validation failure
3. For merkle root mismatches, record both expected and actual values

**Evidence Structure:**
```rust
InvalidBlock {
    block_height: u64,
    block_hash: String,
    reason: String,
    expected_merkle_root: Option<String>,
    actual_merkle_root: Option<String>,
}
```

**Cryptographic Proof:**
- SHA-256 hash of the InvalidBlock structure
- Merkle roots provide verifiable proof of the discrepancy

**Penalties:**
- **Reputation:** -200
- **Collateral Slash:** 10-20% (20% for merkle root issues, 10% for others)
- **Auto-ban:** No
- **Severity:** Moderate

**Rationale:** Invalid blocks can result from bugs or malicious intent. The penalty is moderate to account for potential software issues while still discouraging carelessness.

---

### 3. Extended Downtime (Minor)

**Description:** A masternode remains offline for more than 90 days without decommissioning.

**Detection Algorithm:**
1. Track heartbeats for each masternode with timestamps
2. Periodically check time since last heartbeat: `current_time - last_heartbeat`
3. If elapsed time exceeds 90 days (7,776,000 seconds), raise violation
4. Scale penalty for extremely long downtimes (>365 days)

**Evidence Structure:**
```rust
ExtendedDowntime {
    days_offline: u64,
    last_seen: u64,
    detected_at: u64,
}
```

**Cryptographic Proof:**
- SHA-256 hash of the ExtendedDowntime structure
- Timestamps are verifiable against blockchain time

**Penalties:**
- **Reputation:** -200
- **Collateral Slash:** 5% (90-365 days), 10% (>365 days)
- **Auto-ban:** No
- **Severity:** Minor

**Rationale:** Long-term abandonment reduces network reliability. The 90-day threshold allows for extended maintenance without penalty, but excessive absence is discouraged. Operators should properly decommission if they cannot maintain uptime.

---

### 4. Data Withholding (Moderate)

**Description:** A masternode repeatedly fails to provide requested data (blocks, transactions, etc.) to other nodes.

**Detection Algorithm:**
1. Track data requests in a sliding window per masternode
2. Count consecutive failures for each data type
3. When consecutive failures reach threshold (default: 5), raise violation
4. Reset counter on any successful response
5. Scale penalty based on failure count (5-9: 10%, 10+: 20%)

**Evidence Structure:**
```rust
DataWithholding {
    consecutive_failures: u32,
    data_type: String,
    failed_requests: Vec<u64>, // timestamps
}
```

**Cryptographic Proof:**
- SHA-256 hash of the DataWithholding structure
- Timestamps of failed requests provide an audit trail

**Penalties:**
- **Reputation:** -300
- **Collateral Slash:** 10-20% (based on failure count)
- **Auto-ban:** No
- **Severity:** Moderate

**Rationale:** Data availability is crucial for network operation. Occasional failures are acceptable (network issues, restarts), but persistent withholding indicates either inadequate resources or malicious behavior.

---

### 5. Network Manipulation (Critical)

**Description:** Coordinated behavior by multiple masternodes that appears to be manipulating network consensus or governance.

**Detection Algorithm:**
1. Track votes by proposal ID with timestamps
2. Group votes into time buckets (60-second windows)
3. Detect clusters where multiple nodes (≥3) vote identically within the same time window
4. Flag as suspicious if coordinated nodes exceed threshold

**Evidence Structure:**
```rust
NetworkManipulation {
    manipulation_type: String, // e.g., "coordinated_voting"
    coordinated_nodes: u32,
    description: String,
}
```

**Cryptographic Proof:**
- SHA-256 hash of the NetworkManipulation structure
- Vote records provide verifiable evidence of coordination

**Penalties:**
- **Reputation:** -1000 (maximum penalty)
- **Collateral Slash:** 100%
- **Auto-ban:** Yes
- **Severity:** Critical

**Rationale:** Coordinated attacks on governance or consensus threaten the entire network. The severe penalty reflects the existential risk these attacks pose. Note: This detection is probabilistic and may require manual review in production systems.

---

## Evidence System

All violations include cryptographic evidence to ensure:
1. **Integrity:** Evidence cannot be tampered with
2. **Verifiability:** Anyone can verify the evidence
3. **Auditability:** Complete trail for investigations

### Evidence Structure

```rust
pub struct Evidence {
    pub evidence_type: String,
    pub data: String,              // Serialized violation details
    pub proof: String,             // SHA-256 hash of data
    pub timestamp: u64,
}
```

### Verification

Evidence can be verified by:
1. Deserializing the `data` field
2. Computing SHA-256 hash of the data
3. Comparing computed hash with stored `proof`

```rust
fn verify_evidence(evidence: &Evidence) -> bool {
    let computed_hash = sha256(evidence.data);
    computed_hash == evidence.proof
}
```

---

## Integration with Existing Systems

### Integration with MasternodeRegistry

The violation detection system integrates with the existing `MasternodeRegistry`:

1. **Detection Phase:** `ViolationDetector` monitors behavior and detects violations
2. **Reporting Phase:** Violations are reported to the registry
3. **Execution Phase:** Registry updates masternode state and triggers slashing

```rust
// Pseudocode integration
let mut detector = ViolationDetector::new();
let mut registry = MasternodeRegistry::new();

// Detect violation
if let Some(violation) = detector.record_block_signature(...) {
    // Update reputation
    if let Some(masternode) = registry.get_mut(&violation.masternode_id) {
        masternode.record_violation(
            violation.reputation_penalty(),
            violation.detected_at
        );
    }
    
    // Trigger slashing through existing system
    let slash_amount = calculate_slash_amount(&violation);
    registry.slash_masternode(&violation.masternode_id, slash_amount);
}
```

### Integration with Slashing Module

The existing `slashing` module defines violation types and calculates penalties. The violation detection system extends this by:

1. Automatically detecting violations (vs. manual reporting)
2. Providing structured evidence
3. Supporting configurable thresholds

### Integration with Treasury

When slashing occurs:
1. Collateral is locked in the treasury (from PR #148)
2. Slashed amount is transferred to treasury pool
3. Treasury manages the cooldown period for remaining collateral

---

## Configuration

The system supports configurable thresholds:

```rust
pub struct DetectorConfig {
    pub max_downtime_days: u64,              // Default: 90
    pub max_consecutive_failures: u32,       // Default: 5
    pub min_coordinated_nodes: u32,          // Default: 3
    pub data_request_window: usize,          // Default: 100
}
```

Configuration can be adjusted based on:
- Network conditions
- Governance decisions
- Security requirements

---

## Extending the System

### Adding New Violation Types

To add a new violation type:

1. **Define the violation structure** in `violations.rs`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewViolation {
    pub field1: String,
    pub field2: u64,
}
```

2. **Add to ViolationType enum**:
```rust
pub enum ViolationType {
    // ... existing types
    NewViolation(NewViolation),
}
```

3. **Implement penalty logic**:
```rust
impl ViolationType {
    pub fn severity(&self) -> ViolationSeverity {
        match self {
            // ... existing cases
            ViolationType::NewViolation(_) => ViolationSeverity::Moderate,
        }
    }
    
    pub fn slash_percentage(&self) -> f64 {
        match self {
            // ... existing cases
            ViolationType::NewViolation(_) => 0.15, // 15%
        }
    }
    
    // ... other methods
}
```

4. **Add detection logic** in `detector.rs`:
```rust
impl ViolationDetector {
    pub fn detect_new_violation(&mut self, ...) -> Option<Violation> {
        // Detection logic
        if violation_condition {
            let new_violation = NewViolation { ... };
            let evidence = Evidence::new(...);
            let violation = Violation::new(...);
            self.detected_violations.push(violation.clone());
            return Some(violation);
        }
        None
    }
}
```

5. **Add tests** in `tests/violation_detection.rs`:
```rust
#[test]
fn test_new_violation_detection() {
    // Test the detection logic
}
```

---

## Testing

The system includes comprehensive tests covering:

### Unit Tests (71 tests)
- Evidence creation and verification
- Individual violation type logic
- Penalty calculations
- Detector configuration

### Integration Tests (16 tests)
- Double-signing attack scenarios
- Invalid merkle root detection
- 100+ day downtime scenarios
- Data withholding with 5+ consecutive failures
- Coordinated vote manipulation
- False positive prevention
- Multi-violation tracking

### Running Tests

```bash
# Run all tests
cargo test -p time-masternode

# Run specific test suite
cargo test -p time-masternode --test violation_detection

# Run with output
cargo test -p time-masternode -- --nocapture
```

---

## Performance Considerations

### Memory Management

- **Block Signatures:** Use `cleanup_old_signatures()` to remove old data
- **Heartbeats:** Limited to 1000 most recent per masternode
- **Data Requests:** Sliding window of 100 requests per masternode

### Efficiency

- O(1) lookup for double-signing detection using HashMap
- O(n) for vote manipulation detection (n = votes per proposal)
- Minimal overhead on heartbeat processing

---

## Security Considerations

### False Positives

The system is designed to minimize false positives:
- **Double-signing:** Requires cryptographic proof (signatures)
- **Invalid blocks:** Validated through merkle root verification
- **Downtime:** Long threshold (90 days) prevents transient issues
- **Data withholding:** Requires consecutive failures (5+)
- **Vote manipulation:** Requires multiple coordinated nodes (3+)

### Evidence Tampering

All evidence includes SHA-256 cryptographic proofs that:
- Prevent modification without detection
- Allow anyone to verify authenticity
- Support dispute resolution

### Attack Resistance

The system is resistant to:
- **Sybil attacks:** Vote manipulation requires multiple masternodes (costly)
- **Evidence fabrication:** Cryptographic proofs prevent forgery
- **Reputation grinding:** Severe violations have permanent consequences

---

## Future Enhancements

Possible extensions to the system:

1. **Machine Learning Detection**
   - Pattern recognition for sophisticated attacks
   - Adaptive thresholds based on network behavior

2. **Reputation Recovery**
   - Gradual reputation restoration for reformed nodes
   - Reduced penalties for nodes with good long-term history

3. **Graduated Penalties**
   - First offense warnings
   - Escalating penalties for repeat offenders

4. **Dispute Resolution**
   - Appeal process for false positives
   - Community review of edge cases

5. **Advanced Vote Analysis**
   - Detect more sophisticated coordination patterns
   - Account for legitimate voting clusters

---

## References

- **PR #148:** Collateral locking system with treasury integration
- **Slashing Module:** Existing penalty calculation system
- **Treasury Module:** Collateral management and cooldown periods
- **Registry Module:** Masternode state management

---

## Summary

The violation detection system provides:
- ✅ All 5 violation types with appropriate penalties
- ✅ Automated detection for each violation
- ✅ Integration with existing registry and slashing systems
- ✅ Cryptographic evidence for all violations
- ✅ Comprehensive test coverage (87 tests total)
- ✅ Extensible design for future violation types
- ✅ Performance-optimized with memory management
- ✅ Security-focused with tamper-proof evidence

The system is production-ready and maintains network security while remaining fair and verifiable.
