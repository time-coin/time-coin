# TIME Coin Masternode Grant System

## Overview

The TIME Coin grant system provides a permission-less way for users to obtain 1000 TIME to run a masternode, while building an email list and bootstrapping the network.

## How It Works

### 1. Grant Application
- Users apply with their email address
- Each email can receive **1000 TIME**
- Treasury holds **500,000 TIME** for grants (500 possible masternodes)

### 2. Email Verification
- User receives verification email
- Must verify within 30 days
- Once verified, grant is approved

### 3. Masternode Activation
- User has **30 days** to activate after verification
- Must provide:
  - Public key
  - IP address and port
  - Grant email
- Funds are locked to masternode address

### 4. Running Masternode
- Masternode runs and earns rewards
- Funds remain locked while active
- Can run indefinitely

### 5. Decommissioning
- User can request decommission
- **3-month waiting period** begins
- After 3 months, funds unlock
- Can then withdraw the 1000 TIME

### 6. Forfeiture
- If masternode not activated within 30 days
- Grant is forfeited
- Funds return to treasury for another applicant

## API Endpoints

### Apply for Grant
```bash
curl -X POST http://localhost:24101/grant/apply \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com"}'
```

### Verify Email
```bash
curl http://localhost:24101/grant/verify/{token}
```

### Check Grant Status
```bash
curl http://localhost:24101/grant/status/user@example.com
```

### Activate Masternode
```bash
curl -X POST http://localhost:24101/masternode/activate \
  -H "Content-Type: application/json" \
  -d '{
    "grant_email": "user@example.com",
    "public_key": "your_public_key_hex",
    "ip_address": "50.28.104.50",
    "port": 24100
  }'
```

### Decommission Masternode
```bash
curl -X POST http://localhost:24101/masternode/decommission \
  -H "Content-Type: application/json" \
  -d '{
    "masternode_address": "TIME1abc123..."
  }'
```

### Export Email List (Admin)
```bash
curl http://localhost:24101/admin/emails/export
```

## Treasury Allocation

**Total Supply:** 1,000,000 TIME

- **500,000 TIME** - Masternode Grants (50%)
- **300,000 TIME** - Reward Pool (30%)
- **100,000 TIME** - Development (10%)
- **100,000 TIME** - Operations/Marketing (10%)

## Grant Timeline

1. **Day 0:** User applies with email
2. **Day 0-1:** Email verification
3. **Day 1-30:** Activation window
4. **Day 30+:** Masternode running
5. **Decommission:** 90-day cooldown
6. **Withdrawal:** Available after cooldown

## Benefits

✅ **For Users:**
- Free 1000 TIME to start
- Run a masternode
- Earn rewards
- Exit after 3 months if desired

✅ **For TIME Coin:**
- Build email list
- Bootstrap network with 500 masternodes
- Permission-less onboarding
- Engaged community

## Email List

All emails are stored and can be exported for:
- Marketing campaigns
- Network updates
- Community building
- Future airdrops

## Security

- Email verification required
- 30-day activation window prevents abuse
- 3-month lock prevents quick dumps
- One grant per email
- Funds stay in treasury until activated

## Future Enhancements

- Email notification system
- Automatic forfeiture of expired grants
- Masternode performance tracking
- Reputation system
- Tiered grants (1000/5000/10000 TIME)
