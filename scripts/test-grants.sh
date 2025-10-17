#!/bin/bash

echo "=== Testing Grant System ==="
echo ""

echo "1. Health check..."
curl -w "\n" http://localhost:24101/health
echo ""

echo "2. Check treasury balance..."
curl -w "\n" http://localhost:24101/balance/TIME1treasury00000000000000000000000000
echo ""

echo "3. Apply for grant..."
RESPONSE=$(curl -s http://localhost:24101/grant/apply \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com"}')
echo $RESPONSE | jq
TOKEN=$(echo $RESPONSE | jq -r '.message' | grep -o '[0-9a-f-]\{36\}')
echo "Token: $TOKEN"
echo ""

echo "4. Verify email..."
curl -w "\n" http://localhost:24101/grant/verify/$TOKEN
echo ""

echo "5. Check grant status..."
curl -w "\n" http://localhost:24101/grant/status/test@example.com
echo ""

echo "6. Generate keypair..."
KEYPAIR=$(curl -s -X POST http://localhost:24101/keypair/generate)
echo $KEYPAIR | jq
PUBKEY=$(echo $KEYPAIR | jq -r '.public_key')
ADDRESS=$(echo $KEYPAIR | jq -r '.address')
echo "Public Key: $PUBKEY"
echo "Address: $ADDRESS"
echo ""

echo "7. Activate masternode..."
curl -w "\n" -X POST http://localhost:24101/masternode/activate \
  -H "Content-Type: application/json" \
  -d "{
    \"grant_email\": \"test@example.com\",
    \"public_key\": \"$PUBKEY\",
    \"ip_address\": \"50.28.104.50\",
    \"port\": 24100
  }"
echo ""

echo "8. Check masternode balance..."
curl -w "\n" http://localhost:24101/balance/$ADDRESS
echo ""

echo "9. Export email list..."
curl -w "\n" http://localhost:24101/admin/emails/export
echo ""

echo "=== Test Complete ==="
