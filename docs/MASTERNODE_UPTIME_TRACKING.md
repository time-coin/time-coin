# Masternode Uptime Tracking for Block Rewards

## Problem Statement

**Current Issue:** Masternodes can join the network right before a block is created and receive full rewards, even though they didn't contribute to securing the network during the block production period.

**Solution:** Only masternodes that were online for the ENTIRE previous block period qualify for rewards in the current block.

---

## How It Works

### Timeline Example (10-minute blocks)

```
Block N-1 created at 10:00
    ↓
    ├─ 10:00-10:10: Block N production period
    │  ├─ Masternode A: Online since 9:50 ✅ Eligible for Block N rewards
    │  ├─ Masternode B: Online since 10:05 ❌ NOT eligible (joined mid-block)
    │  └─ Masternode C: Offline ❌ NOT eligible
    ↓
Block N created at 10:10
    ↓
    ├─ 10:10-10:20: Block N+1 production period
    │  ├─ Masternode A: Still online ✅ Eligible for Block N+1 rewards
    │  ├─ Masternode B: Still online ✅ NOW eligible (was online for full period)
    │  └─ Masternode C: Still offline ❌ NOT eligible
    ↓
Block N+1 created at 10:20
```

### Key Rules

1. **Join Time Matters**
   - Masternode joins at 10:05
   - Block N created at 10:10
   - Masternode is NOT eligible for Block N (joined mid-production)
   - Masternode IS eligible for Block N+1 (if stays online)

2. **Must Stay Online**
   - Masternode goes offline mid-block → NOT eligible for next block
   - Must be online for ENTIRE block period to qualify

3. **Genesis Bootstrap**
   - All initial masternodes are eligible for first block
   - Ensures network can start producing blocks

---

## Implementation

### 1. Core Module (`masternode_uptime.rs`)

```rust
use time_core::MasternodeUptimeTracker;
use chrono::Utc;

// Create tracker
let mut tracker = MasternodeUptimeTracker::new();

// Bootstrap genesis
let genesis_masternodes = vec!["mn1".to_string(), "mn2".to_string()];
tracker.bootstrap_genesis(Utc::now(), &genesis_masternodes);

// When masternode joins
tracker.register_masternode("mn3".to_string(), Utc::now());

// When masternode leaves
tracker.remove_masternode("mn2");

// When creating block
let current_online = get_current_online_masternodes();
let eligible_for_rewards = tracker.finalize_block(Utc::now(), &current_online);

// Distribute rewards only to eligible masternodes
distribute_rewards(&eligible_for_rewards);
```

### 2. Integration with Block Producer

```rust
pub struct BlockProducer {
    blockchain: Arc<RwLock<Blockchain>>,
    uptime_tracker: Arc<RwLock<MasternodeUptimeTracker>>,
}

impl BlockProducer {
    pub async fn produce_block(&self) -> Result<(), BlockError> {
        // 1. Get current online masternodes
        let current_online = self.consensus.get_online_masternodes().await;
        
        // 2. Get eligible masternodes for THIS block
        let mut tracker = self.uptime_tracker.write().await;
        let eligible = tracker.finalize_block(Utc::now(), &current_online);
        drop(tracker);
        
        log::info!("🎁 {} masternodes eligible for rewards", eligible.len());
        
        // 3. Create coinbase transaction ONLY for eligible masternodes
        let coinbase = self.create_coinbase_for_eligible(&eligible).await?;
        
        // 4. Build and finalize block
        let block = self.build_block(vec![coinbase]).await?;
        
        Ok(())
    }
    
    fn create_coinbase_for_eligible(
        &self,
        eligible: &HashSet<String>,
    ) -> Result<Transaction, BlockError> {
        let mut outputs = vec![];
        
        // Treasury allocation
        outputs.push(TxOutput {
            address: treasury_address(),
            amount: treasury_amount,
        });
        
        // Masternode rewards - ONLY for eligible masternodes
        let reward_per_masternode = calculate_reward(eligible.len());
        
        for masternode_addr in eligible {
            outputs.push(TxOutput {
                address: masternode_addr.clone(),
                amount: reward_per_masternode,
            });
        }
        
        Ok(Transaction::coinbase(outputs))
    }
}
```

### 3. Integration with Consensus

```rust
pub struct Consensus {
    uptime_tracker: Arc<RwLock<MasternodeUptimeTracker>>,
}

impl Consensus {
    // When a masternode sends heartbeat
    pub async fn handle_masternode_heartbeat(&self, address: String) {
        let mut tracker = self.uptime_tracker.write().await;
        
        // Check if this is a new masternode
        if !tracker.is_registered(&address) {
            tracker.register_masternode(address, Utc::now());
        }
    }
    
    // When a masternode times out
    pub async fn handle_masternode_timeout(&self, address: &str) {
        let mut tracker = self.uptime_tracker.write().await;
        tracker.remove_masternode(address);
    }
    
    // Get masternodes eligible for current block rewards
    pub async fn get_eligible_masternodes(&self) -> HashSet<String> {
        let tracker = self.uptime_tracker.read().await;
        tracker.get_eligible().clone()
    }
}
```

---

## Example Scenarios

### Scenario 1: New Masternode Joins

```
Time 10:00 - Block 100 created
Time 10:05 - Masternode X joins network
    → tracker.register_masternode("X", 10:05)
    → X is added to join_times
    → X is NOT added to eligible_for_current_block

Time 10:10 - Block 101 being created
    → current_online includes X
    → eligible = tracker.finalize_block(10:10, {X, ...})
    → eligible does NOT include X (joined after previous block)
    → X added to eligible_for_next_block

Time 10:10 - Block 101 rewards distributed
    → X receives NO reward

Time 10:20 - Block 102 being created
    → eligible = tracker.finalize_block(10:20, {X, ...})
    → eligible INCLUDES X (was online for full period)
    → X receives reward! ✅
```

### Scenario 2: Masternode Goes Offline

```
Time 10:00 - Block 100 created
    → Masternode Y is eligible

Time 10:05 - Masternode Y goes offline
    → tracker.remove_masternode("Y")
    → Y removed from join_times
    → Y removed from eligible_for_current_block

Time 10:10 - Block 101 being created
    → current_online does NOT include Y
    → eligible = tracker.finalize_block(10:10, {...})
    → eligible still includes Y (was online at block start)
    → Y gets ONE FINAL reward for Block 101

Time 10:20 - Block 102 being created
    → eligible does NOT include Y (not in current_online)
    → Y receives NO reward
```

### Scenario 3: Masternode Restarts

```
Time 10:00 - Block 100 created
    → Masternode Z is eligible

Time 10:05 - Masternode Z restarts
    → tracker.remove_masternode("Z") (on disconnect)
    → tracker.register_masternode("Z", 10:06) (on reconnect)

Time 10:10 - Block 101 being created
    → eligible does NOT include Z (joined after 10:00)
    → Z receives NO reward

Time 10:20 - Block 102 being created
    → eligible INCLUDES Z (been online since 10:06)
    → Z receives reward again ✅
```

---

## Security Benefits

### 1. Prevents Gaming the System
**Without Uptime Tracking:**
- Masternode operator watches for block production
- Quickly spins up node right before block
- Receives full reward
- Shuts down until next block

**With Uptime Tracking:**
- Masternode must be online for ENTIRE previous block period
- Cannot predict exact block time (especially with VDF)
- Must stay online continuously to receive rewards
- Economic incentive to stay online 24/7

### 2. Encourages True Uptime
- Masternodes must maintain consistent uptime
- Brief disconnections cost one block's worth of rewards
- Aligns incentives with network security

### 3. Fair Reward Distribution
- Only masternodes that contributed to securing the network get rewards
- New masternodes must "earn" their first reward by staying online
- Prevents Sybil attacks (spinning up many short-lived nodes)

---

## Configuration

### Testnet (10-minute blocks)

```rust
// In block producer initialization
let mut uptime_tracker = MasternodeUptimeTracker::new();

// Bootstrap with initial masternodes
uptime_tracker.bootstrap_genesis(
    genesis_block.header.timestamp,
    &initial_masternodes,
);
```

### Mainnet (1-hour blocks)

Same code - the uptime tracker works with any block time!

The key is that masternodes must be online for the ENTIRE previous block period, regardless of how long that period is.

---

## Migration Strategy

### Phase 1: Deploy Code
```rust
// Add uptime tracker to block producer
let uptime_tracker = Arc::new(RwLock::new(MasternodeUptimeTracker::new()));
```

### Phase 2: Bootstrap
```rust
// On first run, bootstrap with current masternodes
if !tracker_initialized {
    uptime_tracker.bootstrap_genesis(Utc::now(), &current_masternodes);
}
```

### Phase 3: Integrate with Block Production
```rust
// Modify coinbase creation to use eligible set
let eligible = uptime_tracker.finalize_block(...);
create_coinbase_for_eligible(&eligible);
```

### Phase 4: Monitor
```
- Log when masternodes join/leave
- Log eligible counts
- Verify rewards distributed correctly
```

---

## Testing

### Test 1: New Masternode Not Eligible
```rust
#[test]
fn test_new_masternode_not_eligible() {
    let mut tracker = MasternodeUptimeTracker::new();
    tracker.register_masternode("new_mn".to_string(), Utc::now());
    
    let current = ["new_mn".to_string()].iter().cloned().collect();
    let eligible = tracker.finalize_block(Utc::now(), &current);
    
    assert_eq!(eligible.len(), 0); // NOT eligible immediately
}
```

### Test 2: Masternode Eligible After One Block
```rust
#[test]
fn test_eligible_after_one_block() {
    let mut tracker = MasternodeUptimeTracker::new();
    let t0 = Utc::now();
    let t1 = t0 + Duration::minutes(10);
    
    tracker.register_masternode("mn1".to_string(), t0);
    let current = ["mn1".to_string()].iter().cloned().collect();
    
    // Block 1: Not eligible
    let eligible1 = tracker.finalize_block(t0, &current);
    assert_eq!(eligible1.len(), 0);
    
    // Block 2: NOW eligible
    let eligible2 = tracker.finalize_block(t1, &current);
    assert_eq!(eligible2.len(), 1);
}
```

---

## Monitoring & Debugging

### Log Messages

```
📋 Masternode masternode1 joined at 2025-12-01 18:30:00 UTC
⏰ Masternode masternode1 joined after previous block - NOT eligible for current block

🔄 Finalizing block at 18:40:00
   Current online: 5 masternodes
   Previously eligible: 4 masternodes
   ⏰ Masternode masternode1 joined at 18:30:00 (after previous block at 18:20:00) - not eligible yet
   ✅ Eligible for current block: 4 masternodes
   📋 Eligible for next block: 5 masternodes

👋 Masternode masternode2 went offline
```

### Metrics to Track

- **Join Rate**: How often new masternodes join
- **Leave Rate**: How often masternodes go offline
- **Eligibility Rate**: % of online masternodes that are eligible
- **Average Uptime**: How long masternodes stay online

---

## Summary

**Problem:** Masternodes gaming block rewards by joining right before blocks

**Solution:** 3-part system
1. **Track Join Time** - Record when each masternode joins
2. **Enforce Time Lock** - Must be online for full block period
3. **Delayed Eligibility** - First reward comes NEXT block, not current

**Result:** Fair reward distribution + Strong uptime incentives! 🎯

**Implementation:** Simple API:
- `register_masternode()` - When MN joins
- `remove_masternode()` - When MN leaves
- `finalize_block()` - Get eligible set for rewards

**Next Steps:** Integrate with block producer and consensus module
