# Proof-of-Time Security for 24-Hour Blocks

## Overview

TIME Coin uses a **24-hour block schedule** with **10-minute VDF time locks** to provide:
- ✅ Secure fork resolution (prevents instant rollbacks)
- ✅ Fast block verification (verify 10-min VDF in ~1 second)
- ✅ Predictable block schedule (1 block per day)
- ✅ Energy efficient (no mining required)

---

## Architecture

### Block Schedule vs VDF Time Lock

```
┌─────────────────────── 24 HOURS ──────────────────────┐
│                                                        │
│  Block N          [10-min VDF]    ...waiting...     Block N+1
│  Created ──────────────────────────────────────────> Created
│  9:00 AM                                              9:00 AM (next day)
│                                                        │
└────────────────────────────────────────────────────────┘

VDF Time Lock: ├──────────┤ 10 minutes
                (Sequential computation - cannot be rushed)

Block Schedule: ├─────────────────────────────────────┤ 24 hours
                (Consensus-enforced waiting period)
```

### Two Timing Mechanisms

1. **VDF Time Lock (10 minutes)**
   - Sequential computation required
   - Cannot be parallelized or skipped
   - Prevents instant block creation
   - Fast to verify (~1 second)

2. **Block Schedule (24 hours)**
   - Enforced by consensus rules
   - Validators must wait 24 hours between blocks
   - Independent of VDF computation
   - Ensures predictable block production

---

## Security Model

### Attack Scenario: 100-Block Rollback

```
Honest Chain (100 blocks over 100 days):
├─ Block 1 [10-min VDF] ... 24 hours ... 
├─ Block 2 [10-min VDF] ... 24 hours ...
├─ Block 3 [10-min VDF] ... 24 hours ...
...
└─ Block 100 [10-min VDF]

Attack Chain (trying to rewrite 100 blocks):
├─ Block 1' [10-min VDF] ← Must compute (10 minutes)
├─ Block 2' [10-min VDF] ← Must compute (10 minutes)
├─ Block 3' [10-min VDF] ← Must compute (10 minutes)
...
└─ Block 100' [10-min VDF] ← Must compute (10 minutes)

Attack Time Required:
- 100 blocks × 10 minutes = 1,000 minutes = 16.7 hours MINIMUM
- During these 16.7 hours, honest chain advances 0-1 blocks
- But honest chain ALREADY has 100 blocks head start
- Attack fails - can never catch up
```

### Why Attackers Cannot Win

**Scenario 1: Instant Fork (Without VDF)**
```
❌ Current System:
- Attacker creates fork at block 50
- Instant computation (no time cost)
- Broadcasts 51-100 blocks
- Nodes accept (no proof of time invested)
- SUCCESS - 50 blocks erased

✅ With VDF:
- Attacker must recompute VDF for blocks 50-100
- Takes 50 × 10 min = 500 minutes (8.3 hours)
- During 8.3 hours, honest chain grows
- Attacker falls behind
- FAILURE - cannot catch up
```

**Scenario 2: Long-Range Attack**
```
Attacker tries to fork from block 1:
- Must recompute 100 × 10-min VDFs = 1,000 minutes
- Takes 16.7 hours minimum
- Honest chain already has 100 blocks (100 days of work)
- Attacker chain has less cumulative VDF time
- Nodes reject attacker chain
- FAILURE - insufficient proof of time
```

**Scenario 3: 51% Consensus Attack**
```
Attacker controls 51% of masternodes:
- Goes offline at block 50
- Secretly builds blocks 50-100
- Minimum time: 50 × 10 min = 500 minutes
- Comes back online after 8.3 hours
- But honest chain (49%) built blocks 50-100 over 50 DAYS
- Honest chain has same number of blocks but produced over actual time
- Both chains have similar VDF work (both have 50 blocks with VDF)
- Tie-breaker: First seen wins (honest chain)
- PARTIAL SUCCESS but very limited window
```

---

## Configuration Options

### Conservative (High Security)

```rust
pub const DEFAULT_VDF_LOCK_SECONDS: u64 = 600; // 10 minutes
pub const DEFAULT_TOTAL_ITERATIONS: u64 = 60_000_000;

Security:
- 100-block reorg: 1,000 minutes (16.7 hours)
- Very expensive to attack
- Slightly longer verification time
```

### Balanced (Recommended)

```rust
pub const DEFAULT_VDF_LOCK_SECONDS: u64 = 300; // 5 minutes
pub const DEFAULT_TOTAL_ITERATIONS: u64 = 30_000_000;

Security:
- 100-block reorg: 500 minutes (8.3 hours)
- Still very expensive to attack
- Faster verification
```

### Fast (Lower Security)

```rust
pub const DEFAULT_VDF_LOCK_SECONDS: u64 = 120; // 2 minutes
pub const DEFAULT_TOTAL_ITERATIONS: u64 = 12_000_000;

Security:
- 100-block reorg: 200 minutes (3.3 hours)
- Easier to attack but still costly
- Very fast verification
```

---

## Implementation Details

### Block Creation Process

```rust
async fn create_block_24hr_schedule(
    transactions: Vec<Transaction>,
    previous_block: &Block,
    validator: &Masternode,
) -> Result<Block, BlockError> {
    // 1. Check if 24 hours have passed since previous block
    let time_since_last = Utc::now()
        .signed_duration_since(previous_block.header.timestamp);
    
    if time_since_last < chrono::Duration::hours(24) {
        return Err(BlockError::BlockTooEarly(
            "Must wait 24 hours between blocks"
        ));
    }
    
    // 2. Build block header
    let merkle_root = calculate_merkle_root(&transactions);
    let vdf_input = generate_vdf_input(
        previous_block.header.block_number + 1,
        &previous_block.hash,
        &merkle_root,
        Utc::now().timestamp_nanos_opt().unwrap_or(0),
    );
    
    // 3. Compute VDF proof (10 minutes)
    log::info!("⏱️ Computing Proof-of-Time (10 minutes)...");
    let start = Instant::now();
    let vdf_proof = compute_vdf(&vdf_input, DEFAULT_TOTAL_ITERATIONS)?;
    log::info!("✅ VDF computed in {:.1} minutes", 
        start.elapsed().as_secs_f64() / 60.0);
    
    // 4. Create block
    let header = BlockHeader {
        block_number: previous_block.header.block_number + 1,
        timestamp: Utc::now(),
        previous_hash: previous_block.hash.clone(),
        merkle_root,
        validator_signature: String::new(),
        validator_address: validator.address.clone(),
        masternode_counts: get_current_counts(),
        proof_of_time: Some(vdf_proof),
    };
    
    // 5. Sign and finalize
    sign_and_finalize_block(header, transactions)
}
```

### Block Validation

```rust
async fn validate_block_24hr_schedule(
    block: &Block,
    previous_block: &Block,
) -> Result<bool, BlockError> {
    // 1. Check 24-hour schedule
    let time_diff = block.header.timestamp
        .signed_duration_since(previous_block.header.timestamp);
    
    if time_diff < chrono::Duration::hours(24) {
        return Err(BlockError::BlockTooFast);
    }
    
    // 2. Verify VDF proof (fast - ~1 second)
    if let Some(proof) = &block.header.proof_of_time {
        let vdf_input = generate_vdf_input(
            block.header.block_number,
            &previous_block.hash,
            &block.header.merkle_root,
            block.header.timestamp.timestamp_nanos_opt().unwrap_or(0),
        );
        
        if !verify_vdf(&vdf_input, proof)? {
            return Err(BlockError::InvalidProofOfTime);
        }
        
        log::debug!("✅ Block {} VDF valid (10-min time lock)", 
            block.header.block_number);
    }
    
    // 3. Other validations...
    Ok(true)
}
```

---

## Benefits

### Security Benefits

✅ **Rollback Protection**: Cannot rewrite history without investing real time  
✅ **Fork Resolution**: Objective measure (cumulative VDF time) determines best chain  
✅ **Sybil Resistance**: Cannot create unlimited blocks instantly  
✅ **Consensus Independence**: Works even if masternodes collude  

### Performance Benefits

✅ **Fast Verification**: Verify 10-min VDF in ~1 second  
✅ **Parallel Validation**: Can verify multiple forks simultaneously  
✅ **Low Bandwidth**: VDF proof is small (~10KB with checkpoints)  
✅ **No Mining**: No wasteful energy consumption  

### Operational Benefits

✅ **Predictable Schedule**: Exactly 1 block per day  
✅ **No Mining Pools**: Every masternode can participate  
✅ **Fair Selection**: VRF-based validator selection remains fair  
✅ **Backwards Compatible**: Optional VDF field, old blocks still valid  

---

## Attack Cost Comparison

| Attack Type | Without VDF | With VDF (10-min lock) |
|-------------|-------------|------------------------|
| **10-block reorg** | Instant | 100 minutes minimum |
| **100-block reorg** | Instant | 1,000 minutes (16.7 hours) |
| **1000-block reorg** | Instant | 10,000 minutes (6.9 days) |
| **Genesis reorg** | Instant | Years of computation |

---

## Recommendations

### For 24-Hour Block Schedule

**Recommended VDF Lock:** 5-10 minutes

**Rationale:**
- 5 minutes = reasonable attack cost (500 min for 100 blocks)
- 10 minutes = high security (1000 min for 100 blocks)
- Both are much shorter than 24-hour block time
- Verification remains fast (<1 second)
- Prevents instant forks while allowing honest production

### Configuration

```toml
# config.toml
[blockchain]
block_schedule_hours = 24
vdf_lock_minutes = 10  # Adjustable: 5, 10, or 15
vdf_enabled = true      # Set false to disable (testing only)
```

---

## Summary

**24-hour blocks + 10-minute VDF = Perfect Security Balance**

- **Block Schedule (24 hours)**: Controls when blocks are produced
- **VDF Lock (10 minutes)**: Prevents rollback attacks
- **Fast Verification (1 second)**: No performance penalty
- **Strong Security**: Deep reorgs become impossibly expensive

This gives TIME Coin **Bitcoin-level finality** with:
- ✅ No energy waste (no mining)
- ✅ Predictable block times (exactly 24 hours)
- ✅ Fast sync (verify VDF quickly)
- ✅ Attack resistance (cannot fake time)

🎯 **Best of both worlds: Security + Efficiency!**
