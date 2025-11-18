# Fair Masternode Rewards Design

## Current System (Already Implemented)

TIME Coin uses **logarithmic scaling** to control inflation as the network grows:

```rust
pub fn calculate_total_masternode_reward(counts: &MasternodeCounts) -> u64 {
    const BASE_REWARD: f64 = 2000.0;  // Base reward per block
    const SCALE_FACTOR: f64 = 50.0;   // Controls growth curve
    
    let total_nodes = counts.total() as f64;
    let multiplier = (1.0 + (total_nodes / SCALE_FACTOR)).ln();
    let reward = BASE_REWARD * multiplier;
    
    reward as u64
}
```

### How Logarithmic Scaling Works

**Formula**: `Total Reward = BASE × ln(1 + nodes / SCALE)`

**Benefits:**
- ✅ Early masternodes get good rewards (incentive to bootstrap network)
- ✅ Rewards increase as network grows (but with diminishing returns)
- ✅ Prevents runaway inflation with large networks
- ✅ Self-balancing: more nodes = less reward per node = fewer new nodes

### Example Inflation Control

| Masternodes | Daily Reward | Per Node | % Increase |
|-------------|--------------|----------|------------|
| 10 | 526 TIME | 52.6 TIME | - |
| 50 | 941 TIME | 18.8 TIME | +79% total, -64% per node |
| 100 | 1,187 TIME | 11.9 TIME | +26% total, -37% per node |
| 500 | 1,864 TIME | 3.7 TIME | +57% total, -69% per node |
| 1,000 | 2,160 TIME | 2.2 TIME | +16% total, -41% per node |
| 10,000 | 3,815 TIME | 0.38 TIME | +77% total, -83% per node |

**Key Insight**: As network grows, per-node rewards decrease automatically!

## Tunable Parameters

You can adjust inflation control by changing these constants:

### 1. BASE_REWARD (Current: 2000.0)
- **Higher value** = More generous rewards at all network sizes
- **Lower value** = Stricter inflation control
- Recommendation: 1000-3000 range

### 2. SCALE_FACTOR (Current: 50.0)
- **Higher value** = Rewards grow faster with network size
- **Lower value** = Rewards plateau faster (stricter control)
- Recommendation: 30-100 range

### Examples with Different Parameters

#### Conservative (Lower Inflation)
```rust
const BASE_REWARD: f64 = 1500.0;  // 25% less generous
const SCALE_FACTOR: f64 = 30.0;   // Faster plateau
```

| Nodes | Daily Reward | Change from Current |
|-------|--------------|---------------------|
| 100 | 820 TIME | -31% |
| 1,000 | 1,622 TIME | -25% |
| 10,000 | 2,564 TIME | -33% |

#### Generous (Higher Rewards)
```rust
const BASE_REWARD: f64 = 3000.0;  // 50% more generous
const SCALE_FACTOR: f64 = 100.0;  // Slower plateau
```

| Nodes | Daily Reward | Change from Current |
|-------|--------------|---------------------|
| 100 | 1,386 TIME | +17% |
| 1,000 | 2,886 TIME | +34% |
| 10,000 | 5,610 TIME | +47% |

## Fair Distribution Across Tiers

### Problem: Current Weights

Current implementation has inconsistent weights:
- Free: 1x
- Bronze: 10x (should reflect 1,000 TIME collateral)
- Silver: 25x (should reflect 10,000 TIME collateral)
- Gold: 50x (should reflect 100,000 TIME collateral)

This creates **unfair APY distribution**.

### Solution: Proportional Weights

Make weights proportional to collateral for fair APY:

```rust
pub fn weight(&self) -> u64 {
    match self {
        MasternodeTier::Free => 1,
        MasternodeTier::Bronze => 1000,    // 1,000 TIME collateral
        MasternodeTier::Silver => 10000,   // 10,000 TIME collateral
        MasternodeTier::Gold => 100000,    // 100,000 TIME collateral
    }
}
```

### Why This is Fair

**Scenario**: 100 masternodes, 1,187 TIME daily reward pool

#### With Current Weights (Unfair)
```
Total Weight = (10 Free × 1) + (40 Bronze × 10) + (30 Silver × 25) + (20 Gold × 50)
             = 10 + 400 + 750 + 1,000 = 2,160

Per weight = 1,187 / 2,160 = 0.55 TIME

Rewards:
- Free: 0.55 TIME/day → impossible to calculate APY (no collateral)
- Bronze: 5.5 TIME/day → 0.55% daily → 200% APY (!!)
- Silver: 13.75 TIME/day → 0.14% daily → 50% APY
- Gold: 27.5 TIME/day → 0.03% daily → 10% APY

Result: Bronze tier gets 200% APY while Gold gets only 10% - UNFAIR!
```

#### With Proportional Weights (Fair)
```
Total Weight = (10 Free × 1) + (40 Bronze × 1000) + (30 Silver × 10000) + (20 Gold × 100000)
             = 10 + 40,000 + 300,000 + 2,000,000 = 2,340,010

Per weight = 1,187 / 2,340,010 = 0.000507 TIME

Rewards:
- Free: 0.000507 TIME/day → ~0.19 TIME/year (no collateral needed)
- Bronze: 0.507 TIME/day → ~185 TIME/year → 18.5% APY
- Silver: 5.07 TIME/day → ~1,850 TIME/year → 18.5% APY
- Gold: 50.7 TIME/day → ~18,500 TIME/year → 18.5% APY

Result: All tiers get ~18.5% APY - FAIR!
```

## Recommended Implementation

### Step 1: Update Weights in core/src/block.rs

```rust
pub fn weight(&self) -> u64 {
    match self {
        MasternodeTier::Free => 1,
        MasternodeTier::Bronze => 1000,
        MasternodeTier::Silver => 10000,
        MasternodeTier::Gold => 100000,
    }
}
```

### Step 2: Keep Voting Power Separate

Voting power should remain 1x/10x/100x for governance (not proportional to collateral):

```rust
pub fn voting_power(&self) -> u64 {
    match self {
        MasternodeTier::Free => 0,     // Cannot vote
        MasternodeTier::Bronze => 1,
        MasternodeTier::Silver => 10,
        MasternodeTier::Gold => 100,
    }
}
```

### Step 3: Adjust Base Reward if Needed

If you want to target specific APY values (e.g., 20% instead of 18.5%), adjust BASE_REWARD:

```rust
// For ~20% APY across all tiers
const BASE_REWARD: f64 = 2200.0;

// For ~15% APY across all tiers (more conservative)
const BASE_REWARD: f64 = 1650.0;
```

## Inflation Control Strategies

### Strategy 1: Fixed Parameters (Current)
- BASE_REWARD and SCALE_FACTOR are constants
- Simple and predictable
- May need adjustment as network matures

### Strategy 2: Halving Events (Bitcoin-style)
```rust
pub fn calculate_base_reward(block_height: u64) -> f64 {
    const INITIAL_REWARD: f64 = 2000.0;
    const HALVING_INTERVAL: u64 = 210_000; // ~2 years at 1 block/day
    
    let halvings = block_height / HALVING_INTERVAL;
    INITIAL_REWARD / (2_u64.pow(halvings as u32) as f64)
}
```

### Strategy 3: Difficulty Adjustment (Dynamic)
```rust
pub fn calculate_base_reward(total_supply: u64, target_supply: u64) -> f64 {
    const MAX_REWARD: f64 = 3000.0;
    const MIN_REWARD: f64 = 1000.0;
    
    // Reduce rewards as we approach target supply
    let supply_ratio = total_supply as f64 / target_supply as f64;
    let reward = MAX_REWARD * (1.0 - supply_ratio).max(MIN_REWARD / MAX_REWARD);
    
    reward
}
```

### Strategy 4: Adaptive (Market-Based)
```rust
pub fn calculate_base_reward(node_count: u64, target_nodes: u64) -> f64 {
    const IDEAL_REWARD: f64 = 2000.0;
    
    // Increase rewards if too few nodes, decrease if too many
    if node_count < target_nodes {
        IDEAL_REWARD * 1.2  // Incentivize joining
    } else {
        IDEAL_REWARD * 0.8  // Reduce inflation
    }
}
```

## Recommended Configuration

For a **fair and sustainable** system:

```rust
// In core/src/block.rs

// Inflation control parameters
const BASE_REWARD: f64 = 2000.0;    // Adjust for target APY
const SCALE_FACTOR: f64 = 50.0;     // Moderate growth curve

// Fair reward weights (proportional to collateral)
impl MasternodeTier {
    pub fn weight(&self) -> u64 {
        match self {
            MasternodeTier::Free => 1,
            MasternodeTier::Bronze => 1000,
            MasternodeTier::Silver => 10000,
            MasternodeTier::Gold => 100000,
        }
    }
    
    pub fn voting_power(&self) -> u64 {
        match self {
            MasternodeTier::Free => 0,
            MasternodeTier::Bronze => 1,
            MasternodeTier::Silver => 10,
            MasternodeTier::Gold => 100,
        }
    }
}
```

## Monitoring and Adjustment

Track these metrics to ensure system health:

1. **Effective APY per tier** - Should be similar across tiers
2. **Total daily inflation** - Should decrease as network grows
3. **Node growth rate** - Healthy growth indicates good rewards
4. **Supply velocity** - How quickly new coins enter circulation

Adjust BASE_REWARD quarterly based on:
- Actual vs target APY
- Network growth trends
- Market conditions
- Community feedback

## Summary

✅ **Logarithmic scaling already controls inflation**
✅ **Update weights to be proportional to collateral for fairness**
✅ **Keep voting power separate (1x/10x/100x)**
✅ **Tune BASE_REWARD and SCALE_FACTOR as needed**

This design ensures:
- Fair APY for all participants
- Controlled inflation as network grows
- Strong incentives for early adopters
- Long-term sustainability
