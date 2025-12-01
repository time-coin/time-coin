# Masternode Tier Summary

**Last Updated:** December 1, 2025  
**Status:** Current Official Specification

## Quick Reference

| Tier | Collateral | Reward Weight | Voting Power | Conflict Weight | Est. APY |
|------|------------|---------------|--------------|-----------------|----------|
| Free | 0 TIME | 1 | 0 | 1 | N/A |
| Bronze | 1,000 TIME | 1,000 | 1x | 5 | ~18.5% |
| Silver | 10,000 TIME | 10,000 | 10x | 10 | ~18.5% |
| Gold | 100,000 TIME | 100,000 | 100x | 20 | ~18.5% |

## Three Types of Weights

TIME Coin uses **three different weight systems** for different purposes:

### 1. Reward Weights (Proportional to Collateral)
**Purpose:** Fair distribution of block rewards  
**Formula:** Weight = Collateral Amount

| Tier | Weight | Why |
|------|--------|-----|
| Free | 1 | No collateral, minimal reward |
| Bronze | 1,000 | Matches 1,000 TIME collateral |
| Silver | 10,000 | Matches 10,000 TIME collateral |
| Gold | 100,000 | Matches 100,000 TIME collateral |

**Result:** All tiers earn same APY (~18.5%)

### 2. Voting Power (Governance)
**Purpose:** Prevent vote-buying while giving higher tiers more influence  
**Formula:** Logarithmic scaling, not proportional

| Tier | Voting Power | Why |
|------|--------------|-----|
| Free | 0 | Cannot participate in governance |
| Bronze | 1x | Basic governance participation |
| Silver | 10x | 10x more influence than Bronze |
| Gold | 100x | 100x more influence than Bronze |

**Result:** Higher tiers have more governance say, but not proportional to collateral (prevents plutocracy)

### 3. Conflict Resolution Weights
**Purpose:** Breaking ties in transaction conflicts  
**Formula:** Moderate scaling between reward and voting

| Tier | Conflict Weight | Why |
|------|-----------------|-----|
| Free | 1 | Minimal influence |
| Bronze | 5 | Basic dispute resolution power |
| Silver | 10 | Enhanced dispute resolution |
| Gold | 20 | Maximum dispute resolution |

**Result:** Higher tiers have more say in conflicts, but not overwhelming

## Why Three Different Systems?

**Each system serves a different purpose:**

1. **Reward Weights** must be proportional to ensure fairness (same APY)
2. **Voting Power** uses logarithmic scaling to prevent plutocracy
3. **Conflict Weights** balance between fairness and preventing Sybil attacks

## Example Scenarios

### Scenario 1: Block Rewards (100 nodes total)
- Network: 10 Free, 40 Bronze, 30 Silver, 20 Gold
- Daily reward pool: 1,187 TIME
- Each tier earns ~18.5% APY on their collateral
- Fair because weights are proportional

### Scenario 2: Governance Vote
- Proposal to change block time
- Network: Same 100 nodes
- Total voting power: 0 + 40 + 300 + 2000 = 2,340
- Gold nodes control most votes, but need Bronze/Silver support
- Prevents plutocracy while respecting investment

### Scenario 3: Transaction Conflict
- Two transactions spending same UTXO
- Both received by 50 nodes at same time
- Bronze side: 25 nodes × 5 = 125 weight
- Gold side: 25 nodes × 20 = 500 weight
- Gold side wins (more skin in the game)
- Fair because higher collateral = higher stake in network health

## Implementation Status

✅ **Reward weights** - Implemented in `core/src/block.rs`  
✅ **Voting power** - Implemented in governance system  
✅ **Conflict weights** - Documented in transaction approval protocol  
⏳ **Transaction approval** - In progress

## References

- [TIERS.md](TIERS.md) - Detailed tier information
- [FAIR_REWARDS_DESIGN.md](FAIR_REWARDS_DESIGN.md) - Economic model explanation
- [collateral-tiers.md](collateral-tiers.md) - ROI calculator
- [../analysis/transaction-approval-protocol.md](../../analysis/transaction-approval-protocol.md) - Full protocol spec
