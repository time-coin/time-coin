#!/bin/bash

################################################################################
# TIME Coin Whitepaper Updater (Bash Version)
# Updates whitepaper to reflect three-tier weighted masternode system
# with longevity multiplier and 18-30% ROI range
################################################################################

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration
WHITEPAPER="docs/whitepaper/TIME-Whitepaper.md"
BACKUP_DIR="docs/whitepaper/backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

################################################################################
# Helper Functions
################################################################################

print_header() {
    echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  TIME Coin - Whitepaper Updater${NC}"
    echo -e "${BLUE}  Three-Tier System + Longevity Multiplier${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
    echo ""
}

print_step() {
    echo -e "${CYAN}▶ $1${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

################################################################################
# Main Script
################################################################################

print_header

# Check if whitepaper exists
if [ ! -f "$WHITEPAPER" ]; then
    print_error "Whitepaper not found: $WHITEPAPER"
    echo ""
    echo "Please specify the correct path:"
    echo "  $0 /path/to/whitepaper.md"
    exit 1
fi

# Allow override from command line
if [ ! -z "$1" ]; then
    WHITEPAPER="$1"
    print_step "Using whitepaper: $WHITEPAPER"
fi

# Create backup
print_step "Creating backup..."
mkdir -p "$BACKUP_DIR"
BACKUP_FILE="$BACKUP_DIR/whitepaper-backup-$TIMESTAMP.md"
cp "$WHITEPAPER" "$BACKUP_FILE"
print_success "Backup created: $BACKUP_FILE"
echo ""

# Create temporary work file
TEMP_FILE=$(mktemp)
cp "$WHITEPAPER" "$TEMP_FILE"

################################################################################
# 1. Update Tier System (5 tiers → 3 tiers)
################################################################################

print_step "[1/5] Updating tier system (5 tiers → 3 tiers)..."

# Create new tier section
cat > /tmp/tier_section.txt << 'EOF'
### 8.1 Masternode Tiers

**Three-Tier Weighted System:**

**Bronze Tier:**
- Collateral: 1,000 TIME
- Tier Weight: 1×
- Voting Power: 1×
- Base BFT Weight: 1×
- Entry-level masternode tier

**Silver Tier:**
- Collateral: 10,000 TIME (10× Bronze)
- Tier Weight: 10×
- Voting Power: 10×
- Base BFT Weight: 10×
- Mid-tier commitment

**Gold Tier:**
- Collateral: 100,000 TIME (100× Bronze)
- Tier Weight: 100×
- Voting Power: 100×
- Base BFT Weight: 100×
- Maximum tier and influence

**Design Philosophy:**
- Simple, clear tier structure
- Linear scaling of collateral and power
- Accessible entry point (1,000 TIME)
- Significant commitment at top tier (100,000 TIME)
- Combined with longevity multiplier for fair long-term rewards

EOF

# Remove old tier section (from 8.1 to next ### section)
sed -i '/^### 8\.1 Masternode Tiers/,/^### 8\.2/{
    /^### 8\.1 Masternode Tiers/r /tmp/tier_section.txt
    /^### 8\.2/!d
}' "$TEMP_FILE"

print_success "Tier system updated"

################################################################################
# 2. Add/Update Longevity Multiplier Section
################################################################################

print_step "[2/5] Adding longevity multiplier section..."

cat > /tmp/longevity_section.txt << 'EOF'
### 8.2 Longevity Multiplier System

**Purpose:**
The longevity multiplier rewards masternode operators for long-term commitment and network stability. This system ensures that loyal operators receive increasing rewards over time without disrupting the ability of new participants to join.

**Formula:**
```
Longevity Multiplier = 1 + (Days_Active ÷ 365) × 0.5
Maximum: 3.0× (after 4+ years)
```

**Multiplier Schedule:**

| Duration | Days Active | Multiplier | Bonus | Example: Bronze Daily |
|----------|-------------|-----------|-------|---------------------|
| New Node | 0-30 | 1.0× | 0% | 0.388 TIME |
| 6 Months | ~180 | 1.25× | +25% | 0.485 TIME |
| 1 Year | 365 | 1.5× | +50% | 0.582 TIME |
| 2 Years | 730 | 2.0× | +100% | 0.776 TIME |
| 3 Years | 1,095 | 2.5× | +150% | 0.970 TIME |
| 4+ Years | 1,460+ | 3.0× | +200% | 1.164 TIME |

**Key Characteristics:**

1. **Continuous Growth:** Multiplier increases smoothly with each day of operation
2. **Fair Start:** All new nodes begin at 1.0× regardless of tier
3. **Maximum Cap:** Caps at 3.0× to prevent excessive concentration
4. **Reset Mechanism:** >72 hours of downtime resets multiplier to 1.0×
5. **Transparent:** All calculations on-chain and publicly verifiable

**Total Weight Calculation:**
```
Total Weight = Tier Weight × Longevity Multiplier

Examples:
- New Bronze: 1 × 1.0 = 1 total weight
- Veteran Bronze (4yr): 1 × 3.0 = 3 total weight
- New Gold: 100 × 1.0 = 100 total weight
- Veteran Gold (4yr): 100 × 3.0 = 300 total weight
```

**Impact on Network:**
- Veteran Gold node (4yr) = equivalent to 300 new Bronze nodes
- Encourages long-term participation and network stability
- New nodes remain competitive with meaningful rewards (18-20% APY)
- Veterans earn premium returns (up to 42%+ APY)

**Reset Conditions:**

⚠️ **Downtime Penalty:**
- **>72 hours offline:** Longevity multiplier resets to 1.0×
- Must rebuild time commitment from scratch
- Encourages reliable operation and uptime
- Prevents gaming the system through intermittent operation

**Strategic Implications:**
- Tier determines base power
- Longevity amplifies that power over time
- Both small long-term operators and large new operators can be competitive
- Balanced system that rewards both capital and commitment

EOF

# Check if 8.2 section exists, if so replace it, otherwise insert after 8.1
if grep -q "^### 8\.2" "$TEMP_FILE"; then
    # Replace existing 8.2
    sed -i '/^### 8\.2/,/^### 8\.3/{
        /^### 8\.2/r /tmp/longevity_section.txt
        /^### 8\.3/!d
    }' "$TEMP_FILE"
else
    # Insert new section after 8.1
    sed -i '/^### 8\.1/,/^###/{
        /^###/!b
        /^### 8\.1/n
        r /tmp/longevity_section.txt
    }' "$TEMP_FILE"
fi

print_success "Longevity multiplier section added"

################################################################################
# 3. Update Rewards Distribution Section
################################################################################

print_step "[3/5] Updating rewards distribution section..."

cat > /tmp/rewards_section.txt << 'EOF'
### 8.3 Rewards Distribution

**Daily Reward Pool:**
```
Base Block Reward: 100 TIME per block (per day)
- Masternode Allocation: 95 TIME (95%)
- Treasury Allocation: 5 TIME (5%)
```

**Individual Reward Calculation:**
```
Node Reward = (Node Total Weight / Network Total Weight) × 95 TIME

Where:
  Node Total Weight = Tier Weight × Longevity Multiplier
  Network Total Weight = Sum of all active masternode weights
```

**Example Network Calculation:**

Assume network with 665 total weight:
- 5 new Bronze (5×1×1.0 = 5 weight)
- 3 Silver 2yr (3×10×2.0 = 60 weight)
- 2 Gold 4yr (2×100×3.0 = 600 weight)
- Total: 665 weight

Daily rewards:
- New Bronze: (1÷665) × 95 = 0.143 TIME/day
- Silver 2yr: (20÷665) × 95 = 2.857 TIME/day
- Gold 4yr: (300÷665) × 95 = 42.857 TIME/day

**Target APY Range: 18-30% (up to 42% for veterans)**

*Bronze Tier (1,000 TIME collateral):*

| Age | Multiplier | Daily Reward | Annual Reward | APY |
|-----|-----------|--------------|---------------|-----|
| New | 1.0× | 0.388 TIME | ~142 TIME | ~18%* |
| 6 months | 1.25× | 0.485 TIME | ~177 TIME | ~22%* |
| 1 year | 1.5× | 0.582 TIME | ~212 TIME | ~26%* |
| 2 years | 2.0× | 0.776 TIME | ~283 TIME | ~28% |
| 4+ years | 3.0× | 1.164 TIME | ~425 TIME | ~42% |

*Silver Tier (10,000 TIME collateral):*

| Age | Multiplier | Daily Reward | Annual Reward | APY |
|-----|-----------|--------------|---------------|-----|
| New | 1.0× | 3.878 TIME | ~1,415 TIME | ~18%* |
| 6 months | 1.25× | 4.847 TIME | ~1,769 TIME | ~22%* |
| 1 year | 1.5× | 5.816 TIME | ~2,123 TIME | ~26%* |
| 2 years | 2.0× | 7.755 TIME | ~2,831 TIME | ~28% |
| 4+ years | 3.0× | 11.633 TIME | ~4,246 TIME | ~42% |

*Gold Tier (100,000 TIME collateral):*

| Age | Multiplier | Daily Reward | Annual Reward | APY |
|-----|-----------|--------------|---------------|-----|
| New | 1.0× | 38.78 TIME | ~14,155 TIME | ~18%* |
| 6 months | 1.25× | 48.47 TIME | ~17,693 TIME | ~22%* |
| 1 year | 1.5× | 58.16 TIME | ~21,232 TIME | ~26%* |
| 2 years | 2.0× | 77.55 TIME | ~28,309 TIME | ~28% |
| 4+ years | 3.0× | 116.33 TIME | ~42,464 TIME | ~42% |

*Note: Calculations assume stable network with 245 base total weight.*

**APY Summary by Node Age:**

| Node Age | Target APY Range |
|----------|------------------|
| New (0-30 days) | **18-20%** |
| 6 months | **22-24%** |
| 1 year | **26-28%** |
| 2 years | **28-30%** |
| 4+ years (Veteran) | **Up to 42%+** |

**Additional Revenue: Transaction Fees**

**Fee Distribution:**
```
Transaction Fee Pool: 50% of all network fees
Distribution: Proportional to total weight (same as block rewards)
```

**Fee Impact Examples:**

| Daily Txns | Avg Fee | Fee Pool | MN Share (50%) | Bronze (New) Extra | Gold (4yr) Extra |
|-----------|---------|----------|----------------|-------------------|------------------|
| 10,000 | 0.001 TIME | 10 TIME | 5 TIME | +0.02 TIME/day | +6 TIME/day |
| 100,000 | 0.001 TIME | 100 TIME | 50 TIME | +0.2 TIME/day | +60 TIME/day |
| 1,000,000 | 0.001 TIME | 1,000 TIME | 500 TIME | +2 TIME/day | +600 TIME/day |

**Fee Impact on APY:**
- At 100k daily transactions: +2-4% additional APY
- At 1M daily transactions: +20-40% additional APY
- High network adoption significantly increases operator returns

**Network Equilibrium:**

The system naturally balances through market forces:
- More nodes joining → Lower individual APY → Some nodes exit
- Nodes leaving → Higher APY for remaining → Attracts new nodes
- Transaction fee growth → Additional revenue → More attractive
- Natural equilibrium at 18-30% APY range (targeting sustainable returns)

**Example Weight Comparison:**

| Node Type | Total Weight | Equivalent To |
|-----------|--------------|---------------|
| New Bronze | 1 | 1× new Bronze |
| Veteran Bronze (4yr) | 3 | 3× new Bronze |
| New Silver | 10 | 10× new Bronze |
| Veteran Silver (4yr) | 30 | 30× new Bronze |
| New Gold | 100 | 100× new Bronze |
| Veteran Gold (4yr) | 300 | **300× new Bronze** |

*A single veteran Gold node has the same reward weight as 300 brand new Bronze nodes!*

EOF

# Find and replace rewards distribution section
# Look for section 8.3 or "Rewards Distribution" header
sed -i '/^### 8\.3 Rewards Distribution/,/^### 8\.4/{
    /^### 8\.3 Rewards Distribution/r /tmp/rewards_section.txt
    /^### 8\.4/!d
}' "$TEMP_FILE"

print_success "Rewards distribution section updated"

################################################################################
# 4. Update Section 5.4 Masternode Economics
################################################################################

print_step "[4/6] Updating Section 5.4 Masternode Economics..."

cat > /tmp/economics_section.txt << 'EOF'
### 5.4 Masternode Economics

**Daily Reward Distribution:**

```
Base Daily Pool: 95 TIME (95% of 100 TIME block reward)
Treasury Pool: 5 TIME (5% of 100 TIME block reward)

Distribution Formula:
  Node Reward = (Node Total Weight / Total Network Weight) × 95 TIME
  
Where:
  Node Total Weight = Tier Weight × Longevity Multiplier
```

**Three-Tier System with Longevity:**

| Tier | Collateral | Base Weight | Longevity Range | Weight Range |
|------|-----------|-------------|-----------------|--------------|
| Bronze | 1,000 TIME | 1× | 1.0× - 3.0× | 1 - 3 |
| Silver | 10,000 TIME | 10× | 1.0× - 3.0× | 10 - 30 |
| Gold | 100,000 TIME | 100× | 1.0× - 3.0× | 100 - 300 |

**Example Network Calculation:**

Assume network with 665 total weight:
- 80 new Bronze (80 × 1 × 1.0 = 80 weight)
- 15 Silver 1yr (15 × 10 × 1.5 = 225 weight)
- 4 Gold 2yr (4 × 100 × 2.0 = 800 weight)

Total Network Weight: 1,105

Daily rewards:
- New Bronze: (1 ÷ 1,105) × 95 = 0.086 TIME/day = 31.4 TIME/year = 3.1% APY
- Silver 1yr: (15 ÷ 1,105) × 95 = 1.289 TIME/day = 470.5 TIME/year = 4.7% APY
- Gold 2yr: (200 ÷ 1,105) × 95 = 17.194 TIME/day = 6,276 TIME/year = 6.3% APY

*Note: APY varies significantly based on network competition. Above example shows high competition scenario. Target equilibrium maintains 18-30% APY range.*

**Target Network Equilibrium (245 base weight):**

With more balanced network achieving target 18-30% APY:

| Tier | Age | Weight | Daily Reward | Annual Reward | APY |
|------|-----|--------|--------------|---------------|-----|
| **Bronze (1,000 TIME)** |
| | New | 1.0× | 0.388 TIME | ~142 TIME | ~18%* |
| | 1 year | 1.5× | 0.582 TIME | ~212 TIME | ~26%* |
| | 2 years | 2.0× | 0.776 TIME | ~283 TIME | ~28% |
| | 4+ years | 3.0× | 1.164 TIME | ~425 TIME | ~42% |
| **Silver (10,000 TIME)** |
| | New | 10.0× | 3.878 TIME | ~1,415 TIME | ~18%* |
| | 1 year | 15.0× | 5.816 TIME | ~2,123 TIME | ~26%* |
| | 2 years | 20.0× | 7.755 TIME | ~2,831 TIME | ~28% |
| | 4+ years | 30.0× | 11.633 TIME | ~4,246 TIME | ~42% |
| **Gold (100,000 TIME)** |
| | New | 100.0× | 38.78 TIME | ~14,155 TIME | ~18%* |
| | 1 year | 150.0× | 58.16 TIME | ~21,232 TIME | ~26%* |
| | 2 years | 200.0× | 77.55 TIME | ~28,309 TIME | ~28% |
| | 4+ years | 300.0× | 116.33 TIME | ~42,464 TIME | ~42% |

*Assumes stable network with 245 base total weight achieving target equilibrium.*

**ROI Summary:**

| Node Age | Target APY Range | Typical Annual Return |
|----------|------------------|---------------------|
| New (0-30 days) | 18-20% | Bronze: ~142 TIME |
| 6 months | 22-24% | Bronze: ~177 TIME |
| 1 year | 26-28% | Bronze: ~212 TIME |
| 2 years | 28-30% | Bronze: ~283 TIME |
| 4+ years | Up to 42%+ | Bronze: ~425 TIME |

**Market Dynamics:**

The system self-balances through economic incentives:

1. **High APY → More nodes join** → Increased competition → APY decreases
2. **Low APY → Nodes leave** → Reduced competition → APY increases  
3. **Transaction fees add revenue** → Higher adoption = higher returns
4. **Longevity rewards loyalty** → Long-term stability encouraged
5. **Natural equilibrium at 18-30%** → Sustainable for network growth

**Additional Revenue: Transaction Fees**

```
Fee Pool: 50% of all transaction fees
Distribution: Proportional to total weight (same as block rewards)

Example at 100,000 daily transactions (0.001 TIME avg fee):
  Total daily fees: 100 TIME
  Masternode share: 50 TIME
  
  New Bronze (1/245 weight): +0.204 TIME/day = +74.5 TIME/year (+7.5% APY)
  Gold 4yr (300/245 weight): +61.2 TIME/day = +22,338 TIME/year (+22% APY)
```

High network adoption significantly boosts masternode returns through fee sharing.

EOF

# Find and replace section 5.4
sed -i '/^### 5\.4 Masternode Economics/,/^### 5\.5\|^## 6/{
    /^### 5\.4 Masternode Economics/r /tmp/economics_section.txt
    /^### 5\.5\|^## 6/!d
}' "$TEMP_FILE"

print_success "Section 5.4 Masternode Economics updated"

################################################################################
# 5. Update Any Old ROI Tables
################################################################################

print_step "[5/6] Removing old 5-tier ROI tables..."

# Remove any tables that mention Platinum or Diamond tiers
sed -i '/Platinum.*50,000 TIME/d' "$TEMP_FILE"
sed -i '/Diamond.*100,000 TIME/d' "$TEMP_FILE"
sed -i '/\| Platinum \|/d' "$TEMP_FILE"
sed -i '/\| Diamond \|/d' "$TEMP_FILE"

# Remove old ROI calculation sections that don't match our new format
sed -i '/ROI Calculations (assuming stable network):/,/Plus Transaction Fees:/{
    /ROI Calculations (assuming stable network):/d
    /Plus Transaction Fees:/!d
}' "$TEMP_FILE"

print_success "Old ROI tables removed"

################################################################################
# 6. Update Section Numbers (if needed)
################################################################################

print_step "[6/6] Updating section numbers..."

# If sections got renumbered, fix them
# 8.3 → 8.4 (Masternode Functions becomes 8.4)
# 8.4 → 8.5 (Setup Requirements)
# 8.5 → 8.6 (Slashing & Penalties)

sed -i 's/^### 8\.3 Masternode Functions/### 8.4 Masternode Functions/' "$TEMP_FILE"
sed -i 's/^### 8\.4 Setup Requirements/### 8.5 Setup Requirements/' "$TEMP_FILE"
sed -i 's/^### 8\.5 Slashing/### 8.6 Slashing/' "$TEMP_FILE"

print_success "Section numbers updated"

################################################################################
# Save Results
################################################################################

echo ""
print_step "Saving updated whitepaper..."

# Copy temp file to original location
cp "$TEMP_FILE" "$WHITEPAPER"
rm "$TEMP_FILE"

# Clean up temp files
rm -f /tmp/tier_section.txt /tmp/longevity_section.txt /tmp/rewards_section.txt /tmp/economics_section.txt

print_success "Whitepaper updated successfully!"

################################################################################
# Summary
################################################################################

echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  ✓ Update Complete!${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${CYAN}Changes Made:${NC}"
echo -e "  ${GREEN}✓${NC} Updated tier system: 5 tiers → 3 tiers (Bronze, Silver, Gold)"
echo -e "  ${GREEN}✓${NC} Added longevity multiplier system (1.0× to 3.0×)"
echo -e "  ${GREEN}✓${NC} Updated Section 5.4 Masternode Economics"
echo -e "  ${GREEN}✓${NC} Updated Section 8.x ROI calculations (18-30% APY range, up to 42% for veterans)"
echo -e "  ${GREEN}✓${NC} Removed Platinum and Diamond tier references"
echo -e "  ${GREEN}✓${NC} Updated section numbering"
echo ""

echo -e "${CYAN}Files:${NC}"
echo -e "  ${YELLOW}Updated:${NC} $WHITEPAPER"
echo -e "  ${YELLOW}Backup:${NC}  $BACKUP_FILE"
echo ""

echo -e "${CYAN}New Tier Structure:${NC}"
echo -e "  • Bronze:  1,000 TIME (1× weight)"
echo -e "  • Silver:  10,000 TIME (10× weight)"
echo -e "  • Gold:    100,000 TIME (100× weight)"
echo ""

echo -e "${CYAN}Longevity Multiplier:${NC}"
echo -e "  • New nodes:    1.0× (18-20% APY)"
echo -e "  • 6 months:     1.25× (22-24% APY)"
echo -e "  • 1 year:       1.5× (26-28% APY)"
echo -e "  • 2 years:      2.0× (28-30% APY)"
echo -e "  • 4+ years:     3.0× (up to 42%+ APY)"
echo -e "  • Reset:        >72 hours downtime"
echo ""

echo -e "${BLUE}Review the changes:${NC}"
echo -e "  cat $WHITEPAPER"
echo ""

echo -e "${BLUE}Compare with backup:${NC}"
echo -e "  diff $BACKUP_FILE $WHITEPAPER"
echo ""

echo -e "${BLUE}Restore from backup if needed:${NC}"
echo -e "  cp $BACKUP_FILE $WHITEPAPER"
echo ""

echo -e "⏰ ${CYAN}TIME is money.${NC}"
echo ""
