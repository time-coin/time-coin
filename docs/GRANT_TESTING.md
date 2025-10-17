# Testing the Grant System

## Step 1: Verify API is Running

```bash
curl -w "\n" http://localhost:24101/health
```

Expected output:
```json
{"status":"healthy","version":"0.1.0","uptime":123,"dev_mode":true}
```

---

## Step 2: Check Treasury Balance

```bash
curl -w "\n" http://localhost:24101/balance/TIME1treasury00000000000000000000000000
```

Expected output:
```json
{"address":"TIME1treasury00000000000000000000000000","balance":50000000000000,"balance_time":"500000.00 TIME","pending":0}
```

Should show **500,000 TIME** available for grants.

---

## Step 3: Apply for a Grant

```bash
curl -w "\n" -X POST http://localhost:24101/grant/apply \
  -H "Content-Type: application/json" \
  -d '{"email":"alice@example.com"}'
```

Expected output:
```json
{
  "success": true,
  "message": "Grant application submitted! Check your email to verify. Verification link: /grant/verify/{token}",
  "verification_sent": true
}
```

**Important:** Copy the verification token from the message!

---

## Step 4: Verify Email

Use the token from Step 3:

```bash
# Replace {token} with actual token from Step 3
curl -w "\n" http://localhost:24101/grant/verify/YOUR_TOKEN_HERE
```

Expected output:
```json
{
  "success": true,
  "message": "Email verified! You have 30 days to activate your masternode with 1000 TIME",
  "grant_amount": "1000 TIME",
  "expires_in_days": 30
}
```

---

## Step 5: Check Grant Status

```bash
curl -w "\n" http://localhost:24101/grant/status/alice@example.com
```

Expected output:
```json
{
  "email": "alice@example.com",
  "status": "verified",
  "grant_amount": 100000000000,
  "grant_amount_time": "1000 TIME",
  "verified": true,
  "activated": false,
  "masternode_address": null,
  "expires_at": 1734567890,
  "days_remaining": 29
}
```

---

## Step 6: Generate Keypair for Masternode

```bash
curl -w "\n" -X POST http://localhost:24101/keypair/generate
```

Expected output:
```json
{
  "address": "TIME1abc123...",
  "public_key": "049a1b2c3d...",
  "private_key": "a1b2c3d4e5...",
  "warning": "⚠️  NEVER share your private key! Store it securely!"
}
```

**Important:** Save the public_key for Step 7!

---

## Step 7: Activate Masternode

Use the public key from Step 6:

```bash
curl -w "\n" -X POST http://localhost:24101/masternode/activate \
  -H "Content-Type: application/json" \
  -d '{
    "grant_email": "alice@example.com",
    "public_key": "YOUR_PUBLIC_KEY_FROM_STEP_6",
    "ip_address": "50.28.104.50",
    "port": 24100
  }'
```

Expected output:
```json
{
  "success": true,
  "message": "Masternode activated successfully! 1000 TIME locked.",
  "masternode_address": "TIME1abc123...",
  "locked_amount": "1000 TIME",
  "status": "active"
}
```

**Important:** Save the masternode_address!

---

## Step 8: Verify Masternode Balance

Use the masternode address from Step 7:

```bash
curl -w "\n" http://localhost:24101/balance/YOUR_MASTERNODE_ADDRESS
```

Expected output:
```json
{
  "address": "TIME1abc123...",
  "balance": 100000000000,
  "balance_time": "1000.00 TIME",
  "pending": 0
}
```

Should show **1000 TIME** locked!

---

## Step 9: Check Updated Grant Status

```bash
curl -w "\n" http://localhost:24101/grant/status/alice@example.com
```

Expected output:
```json
{
  "email": "alice@example.com",
  "status": "active",
  "grant_amount": 100000000000,
  "grant_amount_time": "1000 TIME",
  "verified": true,
  "activated": true,
  "masternode_address": "TIME1abc123...",
  "expires_at": null,
  "days_remaining": null
}
```

---

## Step 10: Decommission Masternode

```bash
curl -w "\n" -X POST http://localhost:24101/masternode/decommission \
  -H "Content-Type: application/json" \
  -d '{
    "masternode_address": "YOUR_MASTERNODE_ADDRESS"
  }'
```

Expected output:
```json
{
  "success": true,
  "message": "Decommission started. Funds will unlock in 90 days",
  "unlock_date": "2026-01-15T12:00:00Z",
  "days_until_unlock": 90
}
```

---

## Step 11: Export Email List (Admin)

```bash
curl -w "\n" http://localhost:24101/admin/emails/export
```

Expected output:
```json
{
  "total_emails": 1,
  "verified_emails": 1,
  "active_masternodes": 1,
  "emails": [
    {
      "email": "alice@example.com",
      "verified": true,
      "status": "active",
      "applied_at": "2025-10-17T20:30:00Z"
    }
  ]
}
```

---

## Complete Test Script

Save this as `test-grants.sh` on your server:

```bash
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
```

Run it:
```bash
bash test-grants.sh
```

---

## Quick Manual Test (3 Steps)

```bash
# 1. Apply for grant
curl -X POST http://localhost:24101/grant/apply \
  -H "Content-Type: application/json" \
  -d '{"email":"bob@test.com"}'

# 2. Copy token from response, then verify
curl http://localhost:24101/grant/verify/PASTE_TOKEN_HERE

# 3. Check status
curl -w "\n" http://localhost:24101/grant/status/bob@test.com
```

---

## Full Flow Test (7 Steps)

```bash
# 1. Apply
curl -X POST http://localhost:24101/grant/apply \
  -H "Content-Type: application/json" \
  -d '{"email":"charlie@test.com"}'

# 2. Verify (use token from step 1)
curl http://localhost:24101/grant/verify/YOUR_TOKEN

# 3. Generate keypair
curl -X POST http://localhost:24101/keypair/generate

# 4. Activate masternode (use public_key from step 3)
curl -X POST http://localhost:24101/masternode/activate \
  -H "Content-Type: application/json" \
  -d '{
    "grant_email":"charlie@test.com",
    "public_key":"YOUR_PUBLIC_KEY",
    "ip_address":"1.2.3.4",
    "port":24100
  }'

# 5. Check masternode balance (use address from step 4)
curl -w "\n" http://localhost:24101/balance/YOUR_MASTERNODE_ADDRESS

# 6. Check grant status
curl -w "\n" http://localhost:24101/grant/status/charlie@test.com

# 7. View all grants
curl -w "\n" http://localhost:24101/admin/emails/export
```

---

## What to Watch For

✅ **Success Indicators:**
- Treasury balance decreases by 1000 TIME per grant
- Masternode address has 1000 TIME
- Grant status shows "active"
- Email appears in export list
- Server logs show grant activity

❌ **Common Issues:**
- "Email already applied" → Use different email
- "Invalid verification token" → Token expired or wrong
- "Grant has expired" → Apply again with new email
- "Insufficient balance" → Treasury empty (500 grants max)

---

## Check Server Logs

```bash
# Watch node logs for grant activity
tail -f ~/time-coin-node/logs/node.log

# Look for lines like:
# "Grant application: alice@example.com - Verification token: ..."
# "Grant verified: alice@example.com"
# "Masternode activated: TIME1abc123... for grant alice@example.com"
```

---

## Test Results Checklist

After running tests, verify:

- [ ] Health endpoint returns healthy status
- [ ] Treasury balance is 500,000 TIME
- [ ] Grant application creates verification token
- [ ] Email verification works
- [ ] Grant status shows correct information
- [ ] Keypair generation works
- [ ] Masternode activation locks 1000 TIME
- [ ] Masternode balance is correct
- [ ] Grant status updates to "active"
- [ ] Email export shows all grants
- [ ] Decommission starts 90-day cooldown
- [ ] Server logs show all activities

---

## Performance Testing

Test with multiple grants:

```bash
# Apply for 10 grants
for i in {1..10}; do
  curl -X POST http://localhost:24101/grant/apply \
    -H "Content-Type: application/json" \
    -d "{\"email\":\"test$i@example.com\"}"
  echo ""
done

# Check email list
curl -w "\n" http://localhost:24101/admin/emails/export | jq
```

---

## Production Checklist

Before going live:

- [ ] Email verification sends actual emails
- [ ] Rate limiting enabled (prevent spam)
- [ ] Admin endpoints require authentication
- [ ] Database persistence (not just in-memory)
- [ ] Automatic grant expiration job (30 days)
- [ ] Decommission unlock automation (90 days)
- [ ] Monitoring and alerts
- [ ] Backup system for grant data
- [ ] Terms of service acceptance
- [ ] KYC/AML compliance (if required)
