#!/bin/bash
# Send TIME coins from treasury to a wallet address

set -e

# Colors for output
GREEN="\033[0;32m"
YELLOW="\033[1;33m"
RED="\033[0;31m"
CYAN="\033[0;36m"
NC="\033[0m" # No Color

# Configuration
API_URL="http://localhost:24101"
TREASURY_ADDRESS="TIME1treasury00000000000000000000000000"

# Check if recipient address is provided
if [ -z "$1" ]; then
    echo -e "${RED}Error: Recipient address required${NC}"
    echo -e "${CYAN}Usage: $0 <recipient_address> <amount_in_TIME> [private_key]${NC}"
    echo -e "Example: $0 TIME1mqEkCrHaJyHu6yY8m2tf4nYe1zed2aCfQE 100.0"
    exit 1
fi

if [ -z "$2" ]; then
    echo -e "${RED}Error: Amount required${NC}"
    echo -e "${CYAN}Usage: $0 <recipient_address> <amount_in_TIME> [private_key]${NC}"
    echo -e "Example: $0 TIME1mqEkCrHaJyHu6yY8m2tf4nYe1zed2aCfQE 100.0"
    exit 1
fi

RECIPIENT_ADDRESS="$1"
AMOUNT_TIME="$2"
PRIVATE_KEY="${3:-treasury_private_key_placeholder}"

# Convert TIME to satoshis (1 TIME = 100,000,000 satoshis)
AMOUNT_SATOSHIS=$(echo "$AMOUNT_TIME * 100000000" | bc | cut -d'.' -f1)
FEE_SATOSHIS=1000000  # 0.01 TIME fee

echo -e "${CYAN}════════════════════════════════════════════════${NC}"
echo -e "${CYAN}   TIME Coin Treasury Transfer${NC}"
echo -e "${CYAN}════════════════════════════════════════════════${NC}\n"

echo -e "${YELLOW}From:${NC}       $TREASURY_ADDRESS"
echo -e "${YELLOW}To:${NC}         $RECIPIENT_ADDRESS"
echo -e "${YELLOW}Amount:${NC}     $AMOUNT_TIME TIME ($AMOUNT_SATOSHIS satoshis)"
echo -e "${YELLOW}Fee:${NC}        0.01 TIME ($FEE_SATOSHIS satoshis)"
echo -e ""

# Check treasury balance
echo -e "${CYAN}Checking treasury balance...${NC}"
TREASURY_BALANCE=$(curl -s "$API_URL/balance/$TREASURY_ADDRESS" | jq -r '.balance')

if [ "$TREASURY_BALANCE" == "null" ] || [ -z "$TREASURY_BALANCE" ]; then
    echo -e "${RED}✗ Failed to fetch treasury balance${NC}"
    exit 1
fi

TREASURY_TIME=$(echo "scale=8; $TREASURY_BALANCE / 100000000" | bc)
echo -e "${GREEN}✓ Treasury balance: $TREASURY_TIME TIME${NC}"

# Check if sufficient balance
TOTAL_NEEDED=$((AMOUNT_SATOSHIS + FEE_SATOSHIS))
if [ "$TREASURY_BALANCE" -lt "$TOTAL_NEEDED" ]; then
    echo -e "${RED}✗ Insufficient treasury balance${NC}"
    echo -e "   Have: $TREASURY_TIME TIME"
    echo -e "   Need: $(echo "scale=8; $TOTAL_NEEDED / 100000000" | bc) TIME"
    exit 1
fi

echo -e "${GREEN}✓ Sufficient balance available${NC}\n"

# Confirm transaction
read -p "$(echo -e ${YELLOW}Proceed with transaction? [y/N]:${NC} )" -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}Transaction cancelled${NC}"
    exit 0
fi

# Create transaction
echo -e "\n${CYAN}Creating transaction...${NC}"

TRANSACTION_JSON=$(cat <<EOF
{
  "from": "$TREASURY_ADDRESS",
  "to": "$RECIPIENT_ADDRESS",
  "amount": $AMOUNT_SATOSHIS,
  "fee": $FEE_SATOSHIS,
  "private_key": "$PRIVATE_KEY"
}
EOF
)

RESPONSE=$(curl -s -X POST "$API_URL/transaction/create" \
  -H "Content-Type: application/json" \
  -d "$TRANSACTION_JSON")

# Check response
TXID=$(echo "$RESPONSE" | jq -r '.txid')
STATUS=$(echo "$RESPONSE" | jq -r '.status')
MESSAGE=$(echo "$RESPONSE" | jq -r '.message')

if [ "$TXID" == "null" ] || [ -z "$TXID" ]; then
    echo -e "${RED}✗ Transaction failed${NC}"
    echo -e "${RED}Error: $MESSAGE${NC}"
    echo -e "\nFull response:"
    echo "$RESPONSE" | jq .
    exit 1
fi

# Success
echo -e "${GREEN}✓ Transaction successful!${NC}\n"
echo -e "${CYAN}═══════════════════════════════════════════${NC}"
echo -e "${GREEN}Transaction ID:${NC} $TXID"
echo -e "${GREEN}Status:${NC}         $STATUS"
echo -e "${GREEN}Message:${NC}        $MESSAGE"
echo -e "${CYAN}═══════════════════════════════════════════${NC}\n"

# Check new balances
echo -e "${CYAN}Checking updated balances...${NC}"

NEW_TREASURY=$(curl -s "$API_URL/balance/$TREASURY_ADDRESS" | jq -r '.balance')
NEW_RECIPIENT=$(curl -s "$API_URL/balance/$RECIPIENT_ADDRESS" | jq -r '.balance')

NEW_TREASURY_TIME=$(echo "scale=8; $NEW_TREASURY / 100000000" | bc)
NEW_RECIPIENT_TIME=$(echo "scale=8; $NEW_RECIPIENT / 100000000" | bc)

echo -e "${GREEN}✓ Treasury balance:${NC}  $NEW_TREASURY_TIME TIME"
echo -e "${GREEN}✓ Recipient balance:${NC} $NEW_RECIPIENT_TIME TIME"

echo -e "\n${GREEN}Transfer complete!${NC} 🎉"
