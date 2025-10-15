#!/bin/bash

##############################################################################
# Fix TIME Coin Tier System
# 
# Corrects all files from incorrect 5-tier system to correct 3-tier system
# 
# CORRECT (from whitepaper):
#   Bronze: 1,000 TIME (1×)
#   Silver: 10,000 TIME (10×)
#   Gold: 100,000 TIME (100×)
#
# INCORRECT (mistakenly created):
#   Bronze: 1,000 TIME (1×)
#   Silver: 5,000 TIME (5×)
#   Gold: 10,000 TIME (10×)
#   Platinum: 50,000 TIME (50×)
#   Diamond: 100,000 TIME (100×)
##############################################################################

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${RED}═══════════════════════════════════════════════════════════${NC}"
echo -e "${RED}  CRITICAL FIX: Correcting Tier System${NC}"
echo -e "${RED}  5-Tier System (WRONG) → 3-Tier System (CORRECT)${NC}"
echo -e "${RED}═══════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${BLUE}Correct Tier System (from whitepaper):${NC}"
echo -e "  Bronze:  1,000 TIME   (1× weight)"
echo -e "  Silver:  10,000 TIME  (10× weight)"
echo -e "  Gold:    100,000 TIME (100× weight)"
echo ""

# Create backup directory
BACKUP_DIR="backups/tier-fix-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP_DIR"
echo -e "${GREEN}✓${NC} Backup directory: $BACKUP_DIR"
echo ""

##############################################################################
# 1. FIX GOVERNANCE MODULE
##############################################################################

echo -e "${BLUE}[1/7] Fixing Governance Module...${NC}"

if [ -f "governance/src/masternode.rs" ]; then
    cp governance/src/masternode.rs "$BACKUP_DIR/"
    
    cat > governance/src/masternode.rs << 'EOF'
//! Masternode tier definitions and voting power

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MasternodeTier {
    Bronze,   // 1,000 TIME
    Silver,   // 10,000 TIME
    Gold,     // 100,000 TIME
}

impl MasternodeTier {
    pub fn from_collateral(amount: u64) -> Option<Self> {
        let time = amount / crate::TIME_UNIT;
        
        match time {
            1_000..=9_999 => Some(MasternodeTier::Bronze),
            10_000..=99_999 => Some(MasternodeTier::Silver),
            100_000.. => Some(MasternodeTier::Gold),
            _ => None,
        }
    }
    
    pub fn voting_power(&self) -> u64 {
        match self {
            MasternodeTier::Bronze => 1,
            MasternodeTier::Silver => 10,
            MasternodeTier::Gold => 100,
        }
    }
    
    pub fn required_collateral(&self) -> u64 {
        match self {
            MasternodeTier::Bronze => 1_000 * crate::TIME_UNIT,
            MasternodeTier::Silver => 10_000 * crate::TIME_UNIT,
            MasternodeTier::Gold => 100_000 * crate::TIME_UNIT,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            MasternodeTier::Bronze => "Bronze",
            MasternodeTier::Silver => "Silver",
            MasternodeTier::Gold => "Gold",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Masternode {
    pub id: String,
    pub address: String,
    pub tier: MasternodeTier,
    pub collateral: u64,
    pub active: bool,
    pub registration_time: u64,
    pub last_active: u64,
}

impl Masternode {
    pub fn voting_power(&self) -> u64 {
        if self.active {
            self.tier.voting_power()
        } else {
            0
        }
    }
    
    pub fn weighted_voting_power(&self, current_time: u64) -> u64 {
        if !self.active {
            return 0;
        }
        
        let base_power = self.tier.voting_power();
        let longevity_multiplier = self.calculate_longevity_multiplier(current_time);
        
        // Total Weight = Tier Weight × Longevity Multiplier
        (base_power as f64 * longevity_multiplier) as u64
    }
    
    pub fn calculate_longevity_multiplier(&self, current_time: u64) -> f64 {
        let days_active = (current_time - self.registration_time) / 86400;
        
        // Formula: 1 + (Days Active ÷ 365) × 0.5
        // Maximum: 3.0× (after 4 years = 1460 days)
        let multiplier = 1.0 + ((days_active as f64) / 365.0) * 0.5;
        
        // Cap at 3.0×
        multiplier.min(3.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_from_collateral() {
        assert_eq!(
            MasternodeTier::from_collateral(1_000 * crate::TIME_UNIT),
            Some(MasternodeTier::Bronze)
        );
        
        assert_eq!(
            MasternodeTier::from_collateral(10_000 * crate::TIME_UNIT),
            Some(MasternodeTier::Silver)
        );
        
        assert_eq!(
            MasternodeTier::from_collateral(100_000 * crate::TIME_UNIT),
            Some(MasternodeTier::Gold)
        );
    }

    #[test]
    fn test_voting_power() {
        assert_eq!(MasternodeTier::Bronze.voting_power(), 1);
        assert_eq!(MasternodeTier::Silver.voting_power(), 10);
        assert_eq!(MasternodeTier::Gold.voting_power(), 100);
    }
    
    #[test]
    fn test_longevity_multiplier() {
        let mn = Masternode {
            id: "test".to_string(),
            address: "addr".to_string(),
            tier: MasternodeTier::Gold,
            collateral: 100_000 * crate::TIME_UNIT,
            active: true,
            registration_time: 0,
            last_active: 0,
        };
        
        // New node (0 days)
        assert!((mn.calculate_longevity_multiplier(0) - 1.0).abs() < 0.01);
        
        // 6 months (180 days)
        let six_months = 180 * 86400;
        assert!((mn.calculate_longevity_multiplier(six_months) - 1.25).abs() < 0.01);
        
        // 1 year (365 days)
        let one_year = 365 * 86400;
        assert!((mn.calculate_longevity_multiplier(one_year) - 1.5).abs() < 0.01);
        
        // 2 years (730 days)
        let two_years = 730 * 86400;
        assert!((mn.calculate_longevity_multiplier(two_years) - 2.0).abs() < 0.01);
        
        // 4 years (1460 days) - maximum
        let four_years = 1460 * 86400;
        assert!((mn.calculate_longevity_multiplier(four_years) - 3.0).abs() < 0.01);
        
        // 5 years - should still be capped at 3.0
        let five_years = 1825 * 86400;
        assert!((mn.calculate_longevity_multiplier(five_years) - 3.0).abs() < 0.01);
    }
    
    #[test]
    fn test_weighted_voting_power() {
        let mut mn = Masternode {
            id: "test".to_string(),
            address: "addr".to_string(),
            tier: MasternodeTier::Gold,
            collateral: 100_000 * crate::TIME_UNIT,
            active: true,
            registration_time: 0,
            last_active: 0,
        };
        
        // New Gold node: 100 × 1.0 = 100
        assert_eq!(mn.weighted_voting_power(0), 100);
        
        // Gold node after 1 year: 100 × 1.5 = 150
        let one_year = 365 * 86400;
        assert_eq!(mn.weighted_voting_power(one_year), 150);
        
        // Gold node after 4 years: 100 × 3.0 = 300 (maximum)
        let four_years = 1460 * 86400;
        assert_eq!(mn.weighted_voting_power(four_years), 300);
        
        // Inactive node has 0 power
        mn.active = false;
        assert_eq!(mn.weighted_voting_power(four_years), 0);
    }
}
EOF
    
    echo -e "  ${GREEN}✓${NC} governance/src/masternode.rs"
fi

##############################################################################
# 2. FIX GOVERNANCE CONFIG
##############################################################################

echo -e "${BLUE}[2/7] Fixing Governance Configuration...${NC}"

if [ -f "config/governance.toml" ]; then
    cp config/governance.toml "$BACKUP_DIR/"
    
    cat > config/governance.toml << 'EOF'
# TIME Coin Governance Configuration

[voting]
# Masternode voting weights by tier (3-tier system)
[voting.tiers]
bronze = 1      # 1,000 TIME
silver = 10     # 10,000 TIME
gold = 100      # 100,000 TIME

[longevity]
# Longevity multiplier formula: 1 + (Days Active ÷ 365) × 0.5
# Maximum multiplier after 4 years
max_multiplier = 3.0
formula = "1 + (days / 365) * 0.5"

[incentives]
# Reward multiplier for active voters
voting_bonus_percent = 5

# Proposal bounty for successful proposals (in TIME)
proposal_bounty = 1000

[committees]
# Enable elected committees
enabled = true

# Committee term length (in months)
term_length_months = 12

# Number of committee members
committee_size = 5

# Committee compensation (in TIME per month)
monthly_compensation = 500

[parameters]
# Which protocol parameters can be adjusted via governance
adjustable = [
    "transaction_fee_rate",
    "masternode_collateral",
    "daily_reward_pool",
    "slashing_penalties",
]

[upgrades]
# Soft fork approval threshold
soft_fork_threshold = 75

# Hard fork approval threshold
hard_fork_threshold = 90

# Emergency upgrade authority
emergency_upgrades_enabled = true

# Days to ratify emergency upgrades
emergency_ratification_days = 7
EOF
    
    echo -e "  ${GREEN}✓${NC} config/governance.toml"
fi

##############################################################################
# 3. FIX DOCUMENTATION
##############################################################################

echo -e "${BLUE}[3/7] Fixing Documentation...${NC}"

# Fix voting guide
if [ -f "docs/governance/voting-guide.md" ]; then
    cp docs/governance/voting-guide.md "$BACKUP_DIR/"
    
    cat > docs/governance/voting-guide.md << 'EOF'
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
EOF
    
    echo -e "  ${GREEN}✓${NC} docs/governance/voting-guide.md"
fi

##############################################################################
# 4. FIX ECONOMICS MODULE
##############################################################################

echo -e "${BLUE}[4/7] Fixing Economics Module...${NC}"

if [ -f "economics/src/rewards.rs" ]; then
    cp economics/src/rewards.rs "$BACKUP_DIR/"
    
    # Update rewards.rs to reflect 3-tier system
    sed -i.tmp 's/5 tiers/3 tiers/g' economics/src/rewards.rs
    sed -i.tmp 's/Bronze → Diamond/Bronze → Gold/g' economics/src/rewards.rs
    sed -i.tmp '/Platinum/d' economics/src/rewards.rs
    sed -i.tmp '/Diamond/d' economics/src/rewards.rs
    rm -f economics/src/rewards.rs.tmp
    
    echo -e "  ${GREEN}✓${NC} economics/src/rewards.rs"
fi

##############################################################################
# 5. FIX README FILES
##############################################################################

echo -e "${BLUE}[5/7] Fixing README files...${NC}"

for readme in README.md docs/README.md; do
    if [ -f "$readme" ]; then
        cp "$readme" "$BACKUP_DIR/"
        
        # Fix tier references
        sed -i.tmp 's/5-tier masternode/3-tier masternode/g' "$readme"
        sed -i.tmp 's/Bronze → Diamond/Bronze → Gold/g' "$readme"
        sed -i.tmp 's/five-tier/three-tier/g' "$readme"
        sed -i.tmp 's/5 tiers/3 tiers/g' "$readme"
        
        # Remove Platinum and Diamond references if they exist in lists
        sed -i.tmp '/Platinum.*50,000/d' "$readme"
        sed -i.tmp '/Diamond.*100,000/d' "$readme"
        
        rm -f "$readme.tmp"
        echo -e "  ${GREEN}✓${NC} $readme"
    fi
done

##############################################################################
# 6. FIX TEST FILES
##############################################################################

echo -e "${BLUE}[6/7] Fixing Test Files...${NC}"

if [ -f "governance/tests/integration.rs" ]; then
    cp governance/tests/integration.rs "$BACKUP_DIR/"
    
    # Remove Platinum and Diamond test cases
    sed -i.tmp '/Platinum/d' governance/tests/integration.rs
    sed -i.tmp '/Diamond/d' governance/tests/integration.rs
    
    # Update any test expectations
    sed -i.tmp 's/5_000 \* TIME_UNIT/10_000 * TIME_UNIT/g' governance/tests/integration.rs
    sed -i.tmp 's/50_000 \* TIME_UNIT/100_000 * TIME_UNIT/g' governance/tests/integration.rs
    
    rm -f governance/tests/integration.rs.tmp
    echo -e "  ${GREEN}✓${NC} governance/tests/integration.rs"
fi

##############################################################################
# 7. CREATE SUMMARY DOCUMENT
##############################################################################

echo -e "${BLUE}[7/7] Creating Summary Document...${NC}"

cat > "$BACKUP_DIR/CHANGES.md" << 'EOF'
# Tier System Correction Summary

## Issue
The codebase incorrectly implemented a 5-tier masternode system instead of the 3-tier system specified in the whitepaper.

## Incorrect (5-tier):
- Bronze: 1,000 TIME (1×)
- Silver: 5,000 TIME (5×)
- Gold: 10,000 TIME (10×)
- Platinum: 50,000 TIME (50×)
- Diamond: 100,000 TIME (100×)

## Correct (3-tier):
- Bronze: 1,000 TIME (1×)
- Silver: 10,000 TIME (10×)
- Gold: 100,000 TIME (100×)

## Files Modified

### Governance Module
- governance/src/masternode.rs - Complete rewrite with 3 tiers
- governance/tests/integration.rs - Updated tests

### Configuration
- config/governance.toml - Updated tier definitions

### Documentation
- docs/governance/voting-guide.md - Updated tier tables
- README.md - Updated tier references
- docs/README.md - Updated tier references

### Economics Module
- economics/src/rewards.rs - Updated tier references

## Longevity System (Unchanged)
The time-based longevity multiplier remains the same:
- Formula: 1 + (Days Active ÷ 365) × 0.5
- Maximum: 3.0× after 4 years
- Total Weight = Tier Weight × Longevity Multiplier

## Testing Required
After applying these changes:
1. cargo test --package governance
2. cargo test --package economics
3. cargo test --workspace
4. Review all documentation for consistency

## Backup Location
All original files backed up to: $(pwd)
EOF

echo -e "  ${GREEN}✓${NC} $BACKUP_DIR/CHANGES.md"

##############################################################################
# SUMMARY
##############################################################################

echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  Tier System Correction Complete! ✓${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${BLUE}Files Modified:${NC}"
echo -e "  ${GREEN}✓${NC} governance/src/masternode.rs (complete rewrite)"
echo -e "  ${GREEN}✓${NC} config/governance.toml"
echo -e "  ${GREEN}✓${NC} docs/governance/voting-guide.md"
echo -e "  ${GREEN}✓${NC} economics/src/rewards.rs"
echo -e "  ${GREEN}✓${NC} README.md and docs/README.md"
echo -e "  ${GREEN}✓${NC} Test files updated"
echo ""

echo -e "${BLUE}Correct 3-Tier System Now Implemented:${NC}"
echo -e "  Bronze:  1,000 TIME    (1× weight)"
echo -e "  Silver:  10,000 TIME   (10× weight)"
echo -e "  Gold:    100,000 TIME  (100× weight)"
echo ""

echo -e "${BLUE}Longevity Multiplier (unchanged):${NC}"
echo -e "  Formula: 1 + (Days ÷ 365) × 0.5"
echo -e "  Maximum: 3.0× (after 4 years)"
echo ""

echo -e "${BLUE}Example Weights:${NC}"
echo -e "  New Bronze:        1 × 1.0 = ${YELLOW}1${NC}"
echo -e "  Bronze (4yr):      1 × 3.0 = ${YELLOW}3${NC}"
echo -e "  New Silver:        10 × 1.0 = ${YELLOW}10${NC}"
echo -e "  Silver (4yr):      10 × 3.0 = ${YELLOW}30${NC}"
echo -e "  New Gold:          100 × 1.0 = ${YELLOW}100${NC}"
echo -e "  Gold (4yr):        100 × 3.0 = ${YELLOW}300${NC}"
echo ""

echo -e "${YELLOW}⚠️  Next Steps:${NC}"
echo -e "  1. Review changes: cat $BACKUP_DIR/CHANGES.md"
echo -e "  2. Test governance: cargo test --package governance"
echo -e "  3. Test economics: cargo test --package economics"
echo -e "  4. Test all: cargo test --workspace"
echo -e "  5. Review documentation for any remaining 5-tier references"
echo ""

echo -e "${BLUE}Backups saved to:${NC} ${YELLOW}$BACKUP_DIR${NC}"
echo ""
echo -e "${GREEN}Done! 🎉${NC}"
