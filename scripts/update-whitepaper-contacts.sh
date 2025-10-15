#!/bin/bash

##############################################################################
# Update TIME Coin Whitepaper Contact Information
# 
# This script updates contact details and branding in the whitepaper
##############################################################################

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

WHITEPAPER="docs/whitepaper/TIME-Whitepaper-v1.1.md"

echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Updating TIME Coin Whitepaper Contact Information${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo ""

# Check if whitepaper exists
if [ ! -f "$WHITEPAPER" ]; then
    echo -e "${RED}Error: Whitepaper not found at $WHITEPAPER${NC}"
    echo -e "${YELLOW}Please ensure the whitepaper exists before running this script.${NC}"
    exit 1
fi

echo -e "${YELLOW}Whitepaper found: $WHITEPAPER${NC}"
echo ""

# Create backup
BACKUP="${WHITEPAPER}.backup-$(date +%Y%m%d-%H%M%S)"
cp "$WHITEPAPER" "$BACKUP"
echo -e "${GREEN}✓${NC} Backup created: $BACKUP"
echo ""

echo -e "${BLUE}Applying updates...${NC}"
echo ""

# Update Twitter handle
echo -e "  ${YELLOW}→${NC} Updating Twitter handle..."
sed -i.tmp 's/@TIMEcoinOfficial/@TIMEcoin515010/g' "$WHITEPAPER"
echo -e "    ${GREEN}✓${NC} Twitter: @TIMEcoinOfficial → @TIMEcoin515010"

# Update Telegram link
echo -e "  ${YELLOW}→${NC} Updating Telegram link..."
sed -i.tmp 's|https://t\.me/timecoin|https://t.me/+CaN6EflYM-83OTY0|g' "$WHITEPAPER"
sed -i.tmp 's|https://t\.co/ISNmAW8gMV|https://t.me/+CaN6EflYM-83OTY0|g' "$WHITEPAPER"
sed -i.tmp 's|Telegram: https://t\.me/timecoin|Telegram: https://t.me/+CaN6EflYM-83OTY0|g' "$WHITEPAPER"
echo -e "    ${GREEN}✓${NC} Telegram: https://t.me/timecoin → https://t.me/+CaN6EflYM-83OTY0"

# Update GitHub repository
echo -e "  ${YELLOW}→${NC} Updating GitHub repository..."
sed -i.tmp 's|https://github\.com/time-coin/time-coin|https://github.com/time-coin/time-coin|g' "$WHITEPAPER"
echo -e "    ${GREEN}✓${NC} GitHub: https://github.com/time-coin/time-coin (already correct)"

# Update tagline/motto
echo -e "  ${YELLOW}→${NC} Updating tagline..."
sed -i.tmp 's/Your TIME is valuable\. Spend it wisely\./TIME is money./g' "$WHITEPAPER"
sed -i.tmp 's/\*\*⏰ Your TIME is valuable\. Spend it wisely\.\*\*/\*\*⏰ TIME is money.\*\*/g' "$WHITEPAPER"
sed -i.tmp 's/⏰ Your TIME is valuable\. Spend it wisely\./⏰ TIME is money./g' "$WHITEPAPER"
echo -e "    ${GREEN}✓${NC} Tagline: \"Your TIME is valuable. Spend it wisely.\" → \"TIME is money.\""

# Clean up temporary files
rm -f "${WHITEPAPER}.tmp"

echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  Updates Complete! ✓${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${BLUE}Summary of Changes:${NC}"
echo -e "  ${GREEN}✓${NC} Twitter:  @TIMEcoinOfficial → @TIMEcoin515010"
echo -e "  ${GREEN}✓${NC} Telegram: https://t.me/timecoin → https://t.me/+CaN6EflYM-83OTY0"
echo -e "  ${GREEN}✓${NC} GitHub:   https://github.com/time-coin/time-coin (unchanged)"
echo -e "  ${GREEN}✓${NC} Tagline:  \"Your TIME is valuable. Spend it wisely.\" → \"TIME is money.\""
echo ""

echo -e "${BLUE}Files:${NC}"
echo -e "  Updated: ${YELLOW}$WHITEPAPER${NC}"
echo -e "  Backup:  ${YELLOW}$BACKUP${NC}"
echo ""

# Show statistics
CHANGES=$(diff -u "$BACKUP" "$WHITEPAPER" | grep -c "^[-+]" || true)
echo -e "${BLUE}Statistics:${NC}"
echo -e "  Total lines changed: ${YELLOW}$CHANGES${NC}"
echo ""

# Verification
echo -e "${BLUE}Verification:${NC}"
echo -e "  Checking updated Twitter handle..."
if grep -q "@TIMEcoin515010" "$WHITEPAPER"; then
    echo -e "    ${GREEN}✓${NC} Twitter handle updated successfully"
else
    echo -e "    ${RED}✗${NC} Twitter handle not found (may not have existed)"
fi

echo -e "  Checking updated Telegram link..."
if grep -q "https://t.me/+CaN6EflYM-83OTY0" "$WHITEPAPER"; then
    echo -e "    ${GREEN}✓${NC} Telegram link updated successfully"
else
    echo -e "    ${RED}✗${NC} Telegram link not found (may not have existed)"
fi

echo -e "  Checking updated tagline..."
if grep -q "TIME is money" "$WHITEPAPER"; then
    echo -e "    ${GREEN}✓${NC} Tagline updated successfully"
else
    echo -e "    ${RED}✗${NC} Tagline not found (may not have existed)"
fi

echo ""
echo -e "${YELLOW}Note: If you need to revert changes, restore from: $BACKUP${NC}"
echo ""
echo -e "${GREEN}Done! 🎉${NC}"
