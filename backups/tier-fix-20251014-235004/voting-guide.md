# Masternode Voting Guide

## Overview

As a TIME Coin masternode operator, you have the responsibility and privilege to vote on treasury proposals that shape the ecosystem.

## Voting Power

Your voting power is determined by your masternode tier and operational longevity:

### Base Voting Power (Tier Weight)

| Tier | Collateral | Base Weight |
|------|------------|-------------|
| Bronze | 1,000 TIME | 1× |
| Silver | 10,000 TIME | 10× |
| Gold | 100,000 TIME | 100× |

### Longevity Multiplier

Your total voting power increases with continuous operation:

**Formula:** `1 + (Days Active ÷ 365) × 0.5`

**Maximum:** 3.0× (after 4 years)

| Time Active | Multiplier | Bronze Total | Silver Total | Gold Total |
|-------------|-----------|--------------|--------------|------------|
| 0-30 days | 1.0× | 1 | 10 | 100 |
| 6 months | 1.25× | 1.25 | 12.5 | 125 |
| 1 year | 1.5× | 1.5 | 15 | 150 |
| 2 years | 2.0× | 2 | 20 | 200 |
| 4+ years | 3.0× | 3 | 30 | 300 |

**Example:** A Gold tier masternode running for 4 years has weight of **300**, equivalent to **300 new Bronze masternodes**!

## How to Vote

### Via Web Interface
1. Go to time-coin.io/governance
2. Connect your masternode wallet
3. Browse active proposals
4. Review details and community discussion
5. Cast your vote (Yes/No/Abstain)

### Via CLI
```bash
time-cli governance vote <proposal-id> <yes|no|abstain>
```

## Voting Process

1. **Submission** - Proposal submitted with 10 TIME deposit
2. **Discussion Period** - 14 days for community feedback
3. **Voting Period** - 7 days for masternode voting
4. **Threshold** - 51% approval required (weighted by voting power)
5. **Execution** - Approved proposals funded via milestone payments

## Best Practices

✓ Read the full proposal carefully  
✓ Review team qualifications  
✓ Check budget and timeline  
✓ Participate in community discussion  
✓ Vote on every proposal (5% reward bonus)  
✓ Consider long-term ecosystem impact  

## Voting Incentives

Masternodes that actively vote receive:
- **+5% reward multiplier** for participation
- Recognition in governance dashboard
- Influence over treasury spending

## Questions?

- Governance Forum: forum.time-coin.io/governance
- Telegram: https://t.me/+CaN6EflYM-83OTY0
- Discord: discord.gg/timecoin
