# Block Rewards Guide

## Overview

TIME Coin implements a consistent block reward system that mints new coins with each block and distributes them to active masternodes. This document explains how block rewards work, how they're calculated, and how they're distributed.

## Block Reward Structure

### Components

Each block's coinbase transaction contains:

1. **Masternode Rewards**: Newly minted coins distributed to masternodes
2. **Transaction Fees**: Fees collected from transactions in the block
3. **Treasury Allocation**: (Currently not pre-allocated; all rewards go to masternodes)

### Reward Formula

```
Total Block Reward = Masternode Rewards + Transaction Fees
```

**Where:**
- **Masternode Rewards**: Calculated using logarithmic scaling based on network size
- **Transaction Fees**: Sum of fees from all transactions in the block

## Masternode Reward Calculation

### Logarithmic Scaling

TIME Coin uses a logarithmic scaling formula to ensure sustainable rewards as the network grows:

```rust
pub fn calculate_total_masternode_reward(counts: &MasternodeCounts) -> u64 {
    const TIME_UNIT: u64 = 100_000_000;
    const BASE_REWARD: f64 = 2000.0; // Base reward
    const SCALE_FACTOR: f64 = 50.0;  // Controls growth speed
    
    let total_nodes = counts.total() as f64;
    
    if total_nodes == 0.0 {
        return 0;
    }
    
    // Logarithmic scaling: BASE * ln(1 + count / SCALE)
    let multiplier = (1.0 + (total_nodes / SCALE_FACTOR)).ln();
    let reward = BASE_REWARD * multiplier * (TIME_UNIT as f64);
    
    reward as u64
}
```

### Why Logarithmic Scaling?

Logarithmic scaling provides:
- ✅ **Sustainable Growth**: Rewards increase with network size but with diminishing returns
- ✅ **Early Incentives**: Attractive rewards for early masternodes
- ✅ **Long-term Stability**: Prevents excessive inflation as network matures
- ✅ **Fair Distribution**: Larger networks share rewards more widely

### Example Reward Progression

| Masternodes | Total Daily Reward | % Increase |
|-------------|-------------------|------------|
| 10          | ~526 TIME         | -          |
| 50          | ~941 TIME         | +79%       |
| 100         | ~1,187 TIME       | +26%       |
| 500         | ~1,864 TIME       | +57%       |
| 1,000       | ~2,160 TIME       | +16%       |

Notice how the percentage increase diminishes as the network grows, ensuring sustainable economics.

## Tier-Based Distribution

### Masternode Tiers

TIME Coin has four masternode tiers, each with different collateral requirements. 
**Reward weights are proportional to collateral** to ensure fair APY (~18%) across all tiers:

| Tier   | Collateral        | Reward Weight | Voting Power | Target APY |
|--------|------------------:|--------------:|-------------:|-----------:|
| Free   | 0 TIME            | 1x            | 0x           | N/A        |
| Bronze | 1,000 TIME        | 1,000x        | 1x           | ~18%       |
| Silver | 10,000 TIME       | 10,000x       | 10x          | ~18%       |
| Gold   | 100,000 TIME      | 100,000x      | 100x         | ~18%       |

### Weight-Based Calculation

Rewards are distributed proportionally based on tier weights:

```rust
pub fn distribute_masternode_rewards(
    active_masternodes: &[(String, MasternodeTier)],
    counts: &MasternodeCounts,
) -> Vec<TxOutput> {
    let mut outputs = Vec::new();
    
    // Calculate total pool and total weight
    let total_pool = calculate_total_masternode_reward(counts);
    let total_weight = counts.total_weight();
    
    if total_weight == 0 || active_masternodes.is_empty() {
        return outputs;
    }
    
    // Calculate reward per weight unit
    let per_weight = total_pool / total_weight;
    
    // Distribute to each masternode based on tier weight
    for (address, tier) in active_masternodes {
        let reward = per_weight * tier.weight();
        if reward > 0 {
            outputs.push(TxOutput::new(reward, address.clone()));
        }
    }
    
    outputs
}
```

### Example Distribution

**Network State:**
- 100 Free tier masternodes
- 50 Bronze tier masternodes
- 20 Silver tier masternodes
- 10 Gold tier masternodes
- Total daily reward pool: 1,187 TIME

**Calculation:**
```
Total Weight = (100 × 1) + (50 × 10) + (20 × 25) + (10 × 50)
             = 100 + 500 + 500 + 500
             = 1,600 weight units

Reward per Weight = 1,187 TIME / 1,600 = ~0.74 TIME

Individual Rewards:
- Free tier:   0.74 × 1  = ~0.74 TIME per node
- Bronze tier: 0.74 × 10 = ~7.4 TIME per node
- Silver tier: 0.74 × 25 = ~18.5 TIME per node
- Gold tier:   0.74 × 50 = ~37 TIME per node
```

## Coinbase Transaction Creation

### Process Flow

1. **Block Producer Selected**: Deterministic selection based on block height
2. **Active Masternodes Retrieved**: From blockchain state
3. **Rewards Calculated**: Using current masternode counts
4. **Fees Aggregated**: From mempool transactions
5. **Coinbase Created**: With all rewards and fees
6. **Block Assembled**: Coinbase + regular transactions
7. **Consensus Reached**: Masternodes vote on the block
8. **Block Finalized**: Rewards distributed

### Implementation

```rust
pub fn create_coinbase_transaction(
    block_number: u64,
    active_masternodes: &[(String, MasternodeTier)],
    counts: &MasternodeCounts,
    transaction_fees: u64,
    block_timestamp: i64,
) -> Transaction {
    let mut outputs = Vec::new();
    
    // Masternode rewards - sorted by address for determinism
    let mut masternode_list: Vec<(String, MasternodeTier)> = active_masternodes.to_vec();
    masternode_list.sort_by(|a, b| a.0.cmp(&b.0));
    
    let masternode_outputs = distribute_masternode_rewards(&masternode_list, counts);
    outputs.extend(masternode_outputs);
    
    // Transaction fees go to block producer
    if transaction_fees > 0 && !masternode_list.is_empty() {
        if let Some((producer_address, _)) = masternode_list.first() {
            outputs.push(TxOutput::new(
                transaction_fees,
                producer_address.clone(),
            ));
        }
    }
    
    // Create deterministic coinbase transaction
    Transaction {
        txid: format!("coinbase_{}", block_number),
        version: 1,
        inputs: vec![],  // No inputs = minting new coins
        outputs,
        lock_time: 0,
        timestamp: block_timestamp,
    }
}
```

## Reward-Only Blocks

### Purpose

Reward-only blocks are created when the mempool is empty, ensuring:
- ✅ **Consistent Rewards**: Masternodes always receive rewards
- ✅ **Deterministic Creation**: All nodes create identical blocks
- ✅ **Fast Consensus**: Instant agreement due to determinism
- ✅ **Network Health**: Regular block production maintained

### Creation

```rust
pub fn create_reward_only_block(
    block_number: u64,
    previous_hash: String,
    validator_address: String,
    active_masternodes: &[(String, MasternodeTier)],
    counts: &MasternodeCounts,
) -> Block {
    // Use normalized timestamp for determinism
    let normalized_timestamp = (block_number * 86400) as i64;
    
    // Sort masternodes for determinism
    let mut sorted_masternodes = active_masternodes.to_vec();
    sorted_masternodes.sort_by(|a, b| a.0.cmp(&b.0));
    
    // Create deterministic coinbase
    let coinbase_tx = create_coinbase_transaction(
        block_number,
        &sorted_masternodes,
        counts,
        0,  // No transaction fees
        normalized_timestamp,
    );
    
    // Create block
    let mut block = Block {
        header: BlockHeader {
            block_number,
            timestamp: DateTime::from_timestamp(normalized_timestamp, 0).unwrap(),
            previous_hash,
            merkle_root: String::new(),
            validator_signature: String::new(),
            validator_address,
        },
        transactions: vec![coinbase_tx],
        hash: String::new(),
    };
    
    // Calculate merkle root and hash
    block.header.merkle_root = block.calculate_merkle_root();
    block.hash = block.calculate_hash();
    
    block
}
```

## Annual Percentage Yield (APY)

### Calculation

APY depends on:
- Network size (total masternodes)
- Your masternode tier
- Daily block production rate

**Formula:**
```
APY = (Daily Reward × 365 / Collateral) × 100%
```

### Example APY Calculations

**Network: 200 total masternodes**

| Tier   | Collateral | Daily Reward | Annual Reward | APY    |
|--------|------------|--------------|---------------|--------|
| Bronze | 1,000      | ~11.5 TIME   | ~4,198 TIME   | ~420%  |
| Silver | 10,000     | ~28.7 TIME   | ~10,496 TIME  | ~105%  |
| Gold   | 100,000    | ~57.5 TIME   | ~20,988 TIME  | ~21%   |

**Network: 1,000 total masternodes**

| Tier   | Collateral | Daily Reward | Annual Reward | APY    |
|--------|------------|--------------|---------------|--------|
| Bronze | 1,000      | ~19.6 TIME   | ~7,154 TIME   | ~715%  |
| Silver | 10,000     | ~49 TIME     | ~17,885 TIME  | ~179%  |
| Gold   | 100,000    | ~98 TIME     | ~35,770 TIME  | ~36%   |

### Dynamic APY

APY is dynamic and changes with:
- ✅ Network growth (more masternodes = lower APY per node)
- ✅ Tier distribution (affects weight-based calculations)
- ✅ Your masternode tier (higher tiers = lower % but higher absolute rewards)
- ✅ Network activity (transaction fees provide additional income)

## Validation

### Block Reward Validation

During block validation, the system ensures:

```rust
pub fn validate_and_apply(
    &self,
    utxo_set: &mut UTXOSet,
    masternode_counts: &MasternodeCounts,
) -> Result<(), BlockError> {
    // Validate structure first
    self.validate_structure()?;
    
    // Calculate expected rewards
    let total_masternode_reward = calculate_total_masternode_reward(masternode_counts);
    
    // Validate coinbase reward
    let coinbase = self.coinbase().ok_or(BlockError::InvalidCoinbase)?;
    let coinbase_total: u64 = coinbase.outputs.iter().map(|o| o.amount).sum();
    
    // Calculate total fees from regular transactions
    let mut total_fees = 0u64;
    for tx in self.regular_transactions() {
        let fee = tx.fee(utxo_set.utxos())?;
        total_fees += fee;
    }
    
    // Coinbase should be masternode rewards + fees
    let max_coinbase = total_masternode_reward + total_fees;
    if coinbase_total > max_coinbase {
        return Err(BlockError::InvalidCoinbase);
    }
    
    // Apply transactions...
    Ok(())
}
```

## Monitoring Rewards

### Via Node Logs

Block production logs show reward details:

```
💰 Distributing rewards to 150 masternodes:
   Total reward pool: 118700000000 satoshis (1187 TIME)
   Total weight: 1600
   Per weight unit: 74187500 satoshis

   - Bronze tier (10 weight): wallet_addr_1 → 741875000 satoshis (7.42 TIME)
   - Silver tier (25 weight): wallet_addr_2 → 1854687500 satoshis (18.55 TIME)
   - Gold tier (50 weight): wallet_addr_3 → 3709375000 satoshis (37.09 TIME)
   ...

📊 Reward Summary:
   Bronze: 50 nodes, 37093750000 satoshis total (7.42 TIME each)
   Silver: 20 nodes, 37093750000 satoshis total (18.55 TIME each)
   Gold: 10 nodes, 37093750000 satoshis total (37.09 TIME each)
```

### Via API

```bash
# Get time block rewards
curl -X POST http://localhost:24101/rpc/gettimeblockrewards \
  -H "Content-Type: application/json" \
  -d '{}'

# Response
{
  "total_pool": 118700000000,
  "per_tier": {
    "free": 74187500,
    "bronze": 741875000,
    "silver": 1854687500,
    "gold": 3709375000
  },
  "distribution": [
    {
      "address": "wallet_addr_1",
      "tier": "Bronze",
      "amount": 741875000
    },
    ...
  ]
}
```

### Via Blockchain Explorer

Check your masternode's balance and reward history using a blockchain explorer or wallet.

## Best Practices

### For Masternode Operators

1. **Choose Appropriate Tier**: Consider your collateral and desired APY
2. **Maintain Uptime**: Stay online to receive rewards
3. **Monitor Network**: Watch for changes in masternode counts
4. **Track Rewards**: Keep records of daily reward income
5. **Reinvest Wisely**: Consider upgrading tiers with earned rewards

### For Network Health

1. **Diverse Tiers**: Mix of all tiers creates balanced network
2. **Geographic Distribution**: Spread masternodes globally
3. **Regular Updates**: Keep software up to date
4. **Active Participation**: Vote on governance proposals
5. **Community Engagement**: Share knowledge and help others

## Frequently Asked Questions

### Q: When do I receive rewards?

**A:** Rewards are distributed in the coinbase transaction of each block (every 24 hours on TIME Coin).

### Q: What if my masternode is offline?

**A:** You won't receive rewards for blocks produced while offline. Maintain high uptime for maximum rewards.

### Q: Can I change my masternode tier?

**A:** Yes, but you'll need to unregister, adjust your collateral, and re-register.

### Q: How are rewards affected by network growth?

**A:** As more masternodes join, total rewards increase logarithmically, but per-node rewards decrease proportionally.

### Q: Do I need to claim rewards?

**A:** No, rewards are automatically sent to your masternode's wallet address in each block.

## Related Documentation

- [Transaction Fees Guide](transaction-fees.md) - How fees add to rewards
- [Masternode Setup](masternodes/setup-guide.md) - Setting up a masternode
- [Economics Overview](economics/overview.md) - Full economic model
- [Treasury Proposals](treasury-proposals.md) - Governance-based treasury grants

## References

- **Issue #131**: Consistent Block Reward Creation and Masternode Distribution
- **Implementation**: `core/src/block.rs`
- **Tests**: `core/src/block.rs` (test suite with 29 passing tests)
- **Consensus**: `consensus/src/` (consensus mechanisms)
