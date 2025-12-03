#!/bin/bash
# TIME Coin Masternode Setup Script
# This script applies for a grant and configures your masternode

set -e

GREEN="\033[0;32m"
BLUE="\033[0;34m"
YELLOW="\033[1;33m"
RED="\033[0;31m"
NC="\033[0m"

API="http://localhost:24101"
# Use Bitcoin-style data directory
CONFIG_FILE="$HOME/.timecoin/config/testnet.toml"
CREDS_FILE="$HOME/.timecoin/masternode-credentials.txt"

echo -e "${BLUE}════════════════════════════════════════════════${NC}"
echo -e "${BLUE}   TIME Coin Masternode Setup${NC}"
echo -e "${BLUE}════════════════════════════════════════════════${NC}\n"

# Check if API is running
echo -e "${BLUE}Checking if TIME node is running...${NC}"
if ! curl -s $API/health > /dev/null 2>&1; then
    echo -e "${RED}✗ TIME node API is not responding${NC}"
    echo "Please ensure the TIME node is running first:"
    echo "  sudo systemctl start timed"
    exit 1
fi
echo -e "${GREEN}✓ TIME node is running${NC}\n"

# Get user email
echo -e "${YELLOW}Enter your email address for the grant:${NC}"
read -p "Email: " EMAIL

if [ -z "$EMAIL" ]; then
    echo -e "${RED}✗ Email is required${NC}"
    exit 1
fi

echo ""

# STEP 1: Apply for grant
echo -e "${BLUE}Step 1/7: Applying for grant...${NC}"
APPLY_RESPONSE=$(curl -s -X POST $API/grant/apply \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$EMAIL\"}")

if echo "$APPLY_RESPONSE" | grep -q '"success":true'; then
    echo -e "${GREEN}✓ Grant application submitted${NC}"
else
    echo -e "${RED}✗ Grant application failed${NC}"
    echo "$APPLY_RESPONSE" | jq -r '.message' 2>/dev/null || echo "$APPLY_RESPONSE"
    exit 1
fi

# STEP 2: Extract token
echo -e "\n${BLUE}Step 2/7: Extracting verification token...${NC}"
sleep 2
TOKEN=$(tail -50 ~/.timecoin/logs/node.log | grep "Grant application: $EMAIL" | grep -o '[0-9a-f]\{8\}-[0-9a-f]\{4\}-[0-9a-f]\{4\}-[0-9a-f]\{4\}-[0-9a-f]\{12\}' | tail -1)

if [ -z "$TOKEN" ]; then
    echo -e "${RED}✗ Could not find verification token${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Token extracted${NC}"

# STEP 3: Verify email
echo -e "\n${BLUE}Step 3/7: Verifying email...${NC}"
VERIFY_RESPONSE=$(curl -s $API/grant/verify/$TOKEN)

if echo "$VERIFY_RESPONSE" | grep -q '"success":true'; then
    echo -e "${GREEN}✓ Email verified${NC}"
else
    echo -e "${RED}✗ Email verification failed${NC}"
    exit 1
fi

# STEP 4: Generate keypair
echo -e "\n${BLUE}Step 4/7: Generating masternode keypair...${NC}"
KEYPAIR=$(curl -s -X POST $API/keypair/generate)
ADDRESS=$(echo "$KEYPAIR" | jq -r '.address')
PUBLIC_KEY=$(echo "$KEYPAIR" | jq -r '.public_key')
PRIVATE_KEY=$(echo "$KEYPAIR" | jq -r '.private_key')

if [ -z "$ADDRESS" ] || [ "$ADDRESS" == "null" ]; then
    echo -e "${RED}✗ Failed to generate keypair${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Keypair generated${NC}"
echo -e "   Address: ${YELLOW}$ADDRESS${NC}"

# STEP 5: Activate masternode
echo -e "\n${BLUE}Step 5/7: Activating masternode...${NC}"
SERVER_IP=$(curl -s ifconfig.me 2>/dev/null || echo "0.0.0.0")

ACTIVATE_RESPONSE=$(curl -s -X POST $API/masternode/activate \
  -H "Content-Type: application/json" \
  -d "{\"grant_email\":\"$EMAIL\",\"public_key\":\"$PUBLIC_KEY\",\"ip_address\":\"$SERVER_IP\",\"port\":24100}")

if echo "$ACTIVATE_RESPONSE" | grep -q '"success":true'; then
    echo -e "${GREEN}✓ Masternode activated with 1000 TIME${NC}"
else
    echo -e "${RED}✗ Masternode activation failed${NC}"
    exit 1
fi

# STEP 6: Update config
echo -e "\n${BLUE}Step 6/7: Updating node configuration...${NC}"
if [ -f "$CONFIG_FILE" ]; then
    cp "$CONFIG_FILE" "$CONFIG_FILE.backup"
fi

cat > "$CONFIG_FILE" << CONFIG_EOF
# TIME Coin Testnet Node Configuration

[node]
mode = "dev"
name = "masternode-$ADDRESS"
data_dir = "$HOME/.timecoin/data"
log_dir = "$HOME/.timecoin/logs"

[network]
port = 24100
max_peers = 50

[masternode]
enabled = true
tier = "entry"
collateral = 100000000000
address = "$ADDRESS"
public_key = "$PUBLIC_KEY"
private_key = "$PRIVATE_KEY"
public_ip = "$SERVER_IP"
public_port = 24100
testnet_mode = true

[rpc]
enabled = true
bind = "127.0.0.1"
port = 24101

[logging]
level = "info"
file = "$HOME/.timecoin/logs/node.log"

[blockchain]
genesis_file = "\$HOME/.timecoin/data/genesis-testnet.json"

[consensus]
dev_mode = true
auto_approve = true
CONFIG_EOF

echo -e "${GREEN}✓ Configuration updated${NC}"

# STEP 7: Save credentials
echo -e "\n${BLUE}Step 7/7: Saving credentials...${NC}"

cat > "$CREDS_FILE" << CREDS_EOF
TIME COIN MASTERNODE CREDENTIALS
Generated: $(date)

Email: $EMAIL
Address: $ADDRESS
Public Key: $PUBLIC_KEY
Private Key: $PRIVATE_KEY
IP: $SERVER_IP:24100
Status: Active

⚠️  NEVER SHARE YOUR PRIVATE KEY!
CREDS_EOF

chmod 600 "$CREDS_FILE"
echo -e "${GREEN}✓ Credentials saved to: $CREDS_FILE${NC}"

# Restart node
echo -e "\n${BLUE}Restarting TIME node...${NC}"
if sudo systemctl restart timed 2>/dev/null; then
    echo -e "${GREEN}✓ Node restarted${NC}"
fi

sleep 3

# Verify
BALANCE_RESPONSE=$(curl -s $API/balance/$ADDRESS 2>/dev/null)
BALANCE=$(echo "$BALANCE_RESPONSE" | jq -r '.balance_time' 2>/dev/null || echo "Unknown")

echo -e "\n${GREEN}════════════════════════════════════════════════${NC}"
echo -e "${GREEN}   🎉 MASTERNODE SETUP COMPLETE! 🎉${NC}"
echo -e "${GREEN}════════════════════════════════════════════════${NC}\n"
echo -e "${YELLOW}Details:${NC}"
echo -e "  Email:    $EMAIL"
echo -e "  Address:  $ADDRESS"
echo -e "  Balance:  ${GREEN}$BALANCE${NC}"
echo -e "  IP:       $SERVER_IP:24100"
echo ""
echo -e "${BLUE}View credentials: ${YELLOW}cat $CREDS_FILE${NC}"
echo -e "${GREEN}Your masternode is now running! 🚀${NC}\n"
