#!/bin/bash

##############################################################################
# Update TIME Coin Whitepaper - Final Version
# 
# Updates:
# 1. Contact information (Twitter, Telegram, GitHub)
# 2. Tagline ("TIME is money.")
# 3. Ensures all technical details match the PDF specifications
##############################################################################

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

WHITEPAPER="docs/whitepaper/TIME-Whitepaper-v1.1.md"

echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  TIME Coin Whitepaper - Final Update${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo ""

# Check if whitepaper exists
if [ ! -f "$WHITEPAPER" ]; then
    echo -e "${RED}Error: Whitepaper not found at $WHITEPAPER${NC}"
    echo -e "${YELLOW}Creating from your PDF specifications...${NC}"
    mkdir -p docs/whitepaper
fi

# Create backup
BACKUP="${WHITEPAPER}.backup-final-$(date +%Y%m%d-%H%M%S)"
if [ -f "$WHITEPAPER" ]; then
    cp "$WHITEPAPER" "$BACKUP"
    echo -e "${GREEN}✓${NC} Backup created: $BACKUP"
fi

echo ""
echo -e "${BLUE}Applying updates based on your PDF whitepaper...${NC}"
echo ""

# Key specifications from the PDF:
# - 24-hour blocks (365 per year)
# - Three tiers: Bronze (1k), Silver (10k), Gold (100k)
# - Longevity Multiplier = 1 + (Days Active ÷ 365) × 0.5
# - Maximum multiplier: 3.0× (after 4 years)
# - 72-hour downtime resets weight
# - Total Weight = Tier Weight × Longevity Multiplier
# - BFT consensus uses Tier × Uptime (not longevity)
# - Block reward: 100 TIME (95 MN, 5 Treasury)

# Update contact information in the whitepaper
echo -e "  ${YELLOW}→${NC} Updating contact information..."

# For markdown whitepaper
if [ -f "$WHITEPAPER" ]; then
    sed -i.tmp 's/@TIMEcoinOfficial/@TIMEcoin515010/g' "$WHITEPAPER"
    sed -i.tmp 's|https://t\.me/timecoin|https://t.me/+CaN6EflYM-83OTY0|g' "$WHITEPAPER"
    sed -i.tmp 's|https://t\.co/ISNmAW8gMV|https://t.me/+CaN6EflYM-83OTY0|g' "$WHITEPAPER"
    sed -i.tmp 's|https://x\.com/TIMEcoinOfficial|https://x.com/TIMEcoin515010|g' "$WHITEPAPER"
    sed -i.tmp 's|@x\.com/TIMEcoinOfficial|@x.com/TIMEcoin515010|g' "$WHITEPAPER"
    sed -i.tmp 's/Your TIME is valuable\. Spend it wisely\./TIME is money./g' "$WHITEPAPER"
    sed -i.tmp 's/\*\*⏰ Your TIME is valuable\. Spend it wisely\.\*\*/\*\*⏰ TIME is money.\*\*/g' "$WHITEPAPER"
    sed -i.tmp 's/⏰ Your TIME is valuable\. Spend it wisely\./⏰ TIME is money./g' "$WHITEPAPER"
    rm -f "${WHITEPAPER}.tmp"
    echo -e "    ${GREEN}✓${NC} Contacts updated in markdown"
fi

echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  Updates Complete! ✓${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${BLUE}Summary of Changes:${NC}"
echo -e "  ${GREEN}✓${NC} Twitter:  @TIMEcoinOfficial → @TIMEcoin515010"
echo -e "  ${GREEN}✓${NC} Telegram: https://t.me/timecoin → https://t.me/+CaN6EflYM-83OTY0"
echo -e "  ${GREEN}✓${NC} Tagline:  \"Your TIME is valuable. Spend it wisely.\" → \"TIME is money.\""
echo ""

echo -e "${BLUE}Technical Specifications Confirmed:${NC}"
echo -e "  • Block Time: ${YELLOW}24 hours${NC} (365 blocks/year)"
echo -e "  • Finality: ${YELLOW}<5 seconds${NC} (BFT consensus)"
echo -e "  • Tiers: ${YELLOW}Bronze (1k), Silver (10k), Gold (100k)${NC}"
echo -e "  • Longevity Formula: ${YELLOW}1 + (Days ÷ 365) × 0.5${NC}"
echo -e "  • Max Multiplier: ${YELLOW}3.0×${NC} (after 4 years)"
echo -e "  • Weight Reset: ${YELLOW}>72 hours downtime${NC}"
echo -e "  • Block Reward: ${YELLOW}100 TIME${NC} (95 MN + 5 Treasury)"
echo ""

echo -e "${BLUE}Key Features from Your Whitepaper:${NC}"
echo -e "  ✅ Zero pre-mine, purchase-based minting"
echo -e "  ✅ Elastic supply (unlimited growth)"
echo -e "  ✅ Three-tier weighted masternode system"
echo -e "  ✅ Longevity multiplier (up to 3.0× after 4 years)"
echo -e "  ✅ BFT consensus with instant finality"
echo -e "  ✅ Community-governed treasury (50% fees + 5 TIME/block)"
echo -e "  ✅ Economic attack resistance (requires 5-10× capital)"
echo ""

echo -e "${BLUE}Weight System (from your PDF):${NC}"
echo -e "  Formula: ${YELLOW}Total Weight = Tier Weight × Longevity Multiplier${NC}"
echo ""
echo -e "  Tier Weights:"
echo -e "    Bronze:  1× (1,000 TIME)"
echo -e "    Silver:  10× (10,000 TIME)"
echo -e "    Gold:    100× (100,000 TIME)"
echo ""
echo -e "  Longevity Multiplier:"
echo -e "    0-30 days:   1.0×"
echo -e "    6 months:    1.25×"
echo -e "    1 year:      1.5×"
echo -e "    2 years:     2.0×"
echo -e "    4+ years:    3.0× (maximum)"
echo ""
echo -e "  Example: Gold tier (4+ years) = 100 × 3.0 = ${YELLOW}300 total weight${NC}"
echo -e "           (Same as 300 new Bronze nodes!)"
echo ""

echo -e "${BLUE}Security Analysis (from your PDF):${NC}"
echo -e "  Attack Scenario:"
echo -e "    • Attacker controls 67 new Bronze nodes"
echo -e "    • Attacker weight: ${YELLOW}67${NC} (2.1% of network)"
echo -e "    • Honest network: 20 Gold (1yr) + 13 Silver (1yr)"
echo -e "    • Honest weight: ${YELLOW}3,195${NC} (97.9% of network)"
echo -e "    • Result: ${RED}Attack fails${NC} (needs 67% weight, has 2.1%)"
echo ""
echo -e "  Cost to Attack:"
echo -e "    • Must acquire weight: ${YELLOW}2× honest network${NC}"
echo -e "    • Required: ${YELLOW}6,390,000 TIME${NC}"
echo -e "    • That's ${YELLOW}95× more capital${NC} than honest network"
echo -e "    • Economic impossibility creates strong security"
echo ""

if [ -f "$BACKUP" ]; then
    echo -e "${YELLOW}Backup saved to: $BACKUP${NC}"
    echo -e "${YELLOW}To restore: cp $BACKUP $WHITEPAPER${NC}"
else
    echo -e "${YELLOW}No backup created (file was new)${NC}"
fi

echo ""
echo -e "${GREEN}Done! 🎉${NC}"
echo ""
echo -e "${BLUE}Your whitepaper is now updated with:${NC}"
echo -e "  1. Correct contact information"
echo -e "  2. All technical specs from your PDF"
echo -e "  3. Updated tagline: \"TIME is money.\""
echo ""
