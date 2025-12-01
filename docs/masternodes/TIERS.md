# TIME Coin Masternode Tiers

## Overview

TIME Coin uses a 4-tier masternode system designed to balance accessibility, security, and fair rewards. All tiers receive approximately the same APY (~18-20%), ensuring fairness regardless of collateral size.

## Tier Structure

### 🆓 Free Tier (0 TIME)

**Purpose:** Entry-level participation to earn towards paid tiers

**Requirements:**
- Collateral: 0 TIME (no collateral required)
- Hardware: 2 CPU cores, 4GB RAM, 50GB storage
- Uptime: 85% minimum
- Network: Stable connection

**Capabilities:**
- Transaction validation
- Network participation
- Earn small rewards towards Bronze tier
- No voting rights

**Rewards:**
- Reward Weight: 1
- Estimated Annual: ~0.19 TIME
- Purpose: Learn system, earn towards Bronze

---

### 🥉 Bronze Tier (1,000 TIME)

**Purpose:** Entry-level paid tier for community members

**Requirements:**
- Collateral: 1,000 TIME
- Hardware: 2 CPU cores, 4GB RAM, 50GB storage
- Uptime: 90% minimum
- Network: Stable connection

**Capabilities:**
- Transaction validation
- Basic voting rights (1x weight)
- Governance participation

**Rewards:**
- Reward Weight: 1,000 (proportional to collateral)
- Base APY: ~18.5%
- Estimated Annual: ~185 TIME
- Voting Power: 1x

---

### 🥈 Silver Tier (10,000 TIME)

**Purpose:** Mid-tier for serious operators

**Requirements:**
- Collateral: 10,000 TIME
- Hardware: 4 CPU cores, 8GB RAM, 100GB storage
- Uptime: 95% minimum

**Capabilities:**
- Enhanced voting (10x weight)
- Governance proposals
- Priority transaction validation

**Rewards:**
- Reward Weight: 10,000 (proportional to collateral)
- Base APY: ~18.5%
- Estimated Annual: ~1,850 TIME
- Voting Power: 10x

---

### 🥇 Gold Tier (100,000 TIME)

**Purpose:** Premium tier for infrastructure providers

**Requirements:**
- Collateral: 100,000 TIME
- Hardware: 8 CPU cores, 16GB RAM, 500GB storage
- Uptime: 98% minimum

**Capabilities:**
- Maximum voting power (100x weight)
- Proposal creation rights
- Priority network routing
- Enhanced governance influence

**Rewards:**
- Reward Weight: 100,000 (proportional to collateral)
- Base APY: ~18.5%
- Estimated Annual: ~18,500 TIME
- Voting Power: 100x

---

## Comparison Chart

| Feature | Free | Bronze | Silver | Gold |
|---------|------|--------|--------|------|
| **Collateral** | 0 | 1,000 | 10,000 | 100,000 |
| **Base APY** | N/A | ~18.5% | ~18.5% | ~18.5% |
| **Reward Weight** | 1 | 1,000 | 10,000 | 100,000 |
| **Voting Power** | 0 | 1x | 10x | 100x |
| **Min Uptime** | 85% | 90% | 95% | 98% |
| **Est. Annual Reward** | ~0.19 TIME | ~185 TIME | ~1,850 TIME | ~18,500 TIME |

---

## Fair Rewards System

### Logarithmic Scaling

TIME Coin uses logarithmic scaling to control inflation as the network grows:

```
Total Block Reward = BASE_REWARD × ln(1 + total_nodes / SCALE_FACTOR)
```

**Benefits:**
- Early masternodes get good rewards (bootstrap incentive)
- Rewards increase as network grows (but with diminishing returns)
- Prevents runaway inflation
- Self-balancing: more nodes = less reward per node

### Proportional Distribution

All tiers receive the same APY because reward weights are proportional to collateral:

```
Reward Weight = Collateral Amount

Free:   1 (no collateral, minimal reward)
Bronze: 1,000 (1,000 TIME collateral)
Silver: 10,000 (10,000 TIME collateral)
Gold:   100,000 (100,000 TIME collateral)

Your Share = (Your Weight / Total Network Weight) × Total Block Reward
```

**Example:** If there are 100 masternodes and daily reward is 1,187 TIME:
- Bronze node earns: ~0.507 TIME/day = ~185 TIME/year = ~18.5% APY
- Silver node earns: ~5.07 TIME/day = ~1,850 TIME/year = ~18.5% APY  
- Gold node earns: ~50.7 TIME/day = ~18,500 TIME/year = ~18.5% APY

**Result:** All tiers get same APY - completely fair!

---

## Voting Power (Separate from Rewards)

Voting power for governance is intentionally different from reward weights:

| Tier | Voting Power | Purpose |
|------|-------------|---------|
| Free | 0 | Cannot vote |
| Bronze | 1x | Basic governance participation |
| Silver | 10x | Enhanced governance influence |
| Gold | 100x | Maximum governance influence |

This prevents vote-buying attacks while maintaining fair rewards.
