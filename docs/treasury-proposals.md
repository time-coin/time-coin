# Treasury Grant Proposals

## Overview

TIME Coin implements a decentralized governance system for treasury grants through masternode voting. This replaces the old testnet-mint system with a democratic, consensus-based approach that requires 2/3+ masternode approval.

## Key Features

✅ **BFT Consensus**: Requires 2/3+ masternode approval  
✅ **Transparent Voting**: All votes are recorded and auditable  
✅ **Proper Treasury Grants**: Creates legitimate treasury grant transactions  
✅ **Persistent History**: Proposals saved to proposals.json  
✅ **Automatic Funding**: Approved proposals are funded in the next block  

## Usage

### Create a Proposal

Any masternode can create a treasury grant proposal:

```bash
time-cli proposal create \
  --address TIME1mkLKMVtAbyBefLRcn8cLqjaxbVuBYheeYB \
  --amount 1000 \
  --reason "Development funding for Q4 2025"
```

**Parameters:**
- `--address` / `-a`: Recipient wallet address (required)
- `--amount` / `-m`: Amount in TIME (e.g., 1000 means 1000 TIME)
- `--reason` / `-r`: Reason for the grant (required)

**Example Output:**
```
📜 Creating Treasury Grant Proposal
═══════════════════════════════════════
Recipient: TIME1mkLKMVtAbyBefLRcn8cLqjaxbVuBYheeYB
Amount: 1000 TIME (100000000000 satoshis)
Reason: Development funding for Q4 2025

📡 Submitting proposal...

✅ Proposal Created!
ID: proposal_1731708456123456789

Masternodes can now vote with:
  time-cli proposal vote --id proposal_1731708456123456789 --approve
```

### Vote on a Proposal

**Only masternodes can vote.** Each masternode gets one vote per proposal:

```bash
# Approve a proposal
time-cli proposal vote --id proposal_1731708456123456789 --approve

# Reject a proposal (don't include --approve flag to reject)
time-cli proposal vote --id proposal_1731708456123456789
```

**Example Output:**
```
🗳️  Voting on Proposal
═══════════════════════════════
Proposal ID: proposal_1731708456123456789
Vote: ✅ APPROVE

📡 Submitting vote...

✅ Vote Recorded!
Proposal Status: Pending
```

### List Proposals

View all proposals or filter by status:

```bash
# List all proposals
time-cli proposal list

# List only pending proposals
time-cli proposal list --pending
```

**Example Output:**
```
📋 Treasury Grant Proposals
═══════════════════════════════════════

🆔 ID: proposal_1731708456123456789
   Recipient: TIME1mkLKMVtAbyBefLRcn8cLqjaxbVuBYheeYB
   Amount: 1000 TIME
   Status: Approved
   Votes: 3 for, 0 against
   Reason: Development funding for Q4 2025

🆔 ID: proposal_1731708456987654321
   Recipient: TIME1abc123xyz789...
   Amount: 500 TIME
   Status: Pending
   Votes: 2 for, 1 against
   Reason: Marketing campaign
```

### Get Proposal Details

View detailed information about a specific proposal:

```bash
time-cli proposal get proposal_1731708456123456789
```

**Example Output:**
```
📜 Proposal Details
═══════════════════════════════════════
ID: proposal_1731708456123456789
Proposer: 134.199.175.106
Recipient: TIME1mkLKMVtAbyBefLRcn8cLqjaxbVuBYheeYB
Amount: 1000 TIME
Status: Approved
Reason: Development funding for Q4 2025

Votes For (3):
  ✅ 134.199.175.106
  ✅ 161.35.129.70
  ✅ 50.28.104.50

Votes Against (0):
```

## How It Works

### 1. Proposal Creation
- Any masternode can create a proposal
- Proposal includes recipient address, amount, and reason
- Assigned unique ID (proposal_TIMESTAMP)
- Status set to "Pending"

### 2. Voting Process
- Masternodes review the proposal
- Each masternode votes once (approve or reject)
- Votes are recorded with the masternode's IP
- Changing vote overwrites previous vote

### 3. Consensus Threshold
- Requires **2/3+ of masternodes** to approve
- Example: With 4 masternodes, need 3 votes to approve
- If 1/3+ reject, proposal is dead (status: Rejected)
- Until threshold is met, status remains "Pending"

### 4. Automatic Funding
When a proposal reaches 2/3+ approval:
- Status changes to "Approved"
- Next block creation checks for approved proposals
- Treasury grant transaction is automatically created
- Funds distributed to recipient address
- Proposal marked as "Executed"

### 5. Transaction Format
Treasury grants are created as:
```
txid: "treasury_grant_<proposal_id>"
inputs: [] (no inputs = treasury grant)
outputs: [
  { amount: <approved_amount>, address: <recipient> }
]
```

## API Endpoints

### POST /proposals/create

Create a new proposal.

**Request:**
```json
{
  "recipient": "TIME1mkLKMVtAbyBefLRcn8cLqjaxbVuBYheeYB",
  "amount": 100000000000,
  "reason": "Development funding"
}
```

**Response:**
```json
{
  "success": true,
  "id": "proposal_1731708456123456789",
  "message": "Proposal created successfully. Masternodes can now vote."
}
```

### POST /proposals/vote

Vote on a proposal (masternodes only).

**Request:**
```json
{
  "proposal_id": "proposal_1731708456123456789",
  "approve": true
}
```

**Response:**
```json
{
  "success": true,
  "status": "Approved",
  "message": "Vote recorded. Votes: 3 for, 0 against"
}
```

### GET /proposals/list

List all proposals.

**Query Parameters:**
- `pending` (optional): If true, only show pending proposals

**Response:**
```json
{
  "success": true,
  "count": 2,
  "proposals": [
    {
      "id": "proposal_1731708456123456789",
      "proposer": "134.199.175.106",
      "recipient": "TIME1mkLKMVtAbyBefLRcn8cLqjaxbVuBYheeYB",
      "amount": 100000000000,
      "reason": "Development funding",
      "created_at": 1731708456,
      "status": "Approved",
      "votes_for": ["134.199.175.106", "161.35.129.70", "50.28.104.50"],
      "votes_against": [],
      "executed_at": 1731708500,
      "tx_id": "treasury_grant_proposal_1731708456123456789"
    }
  ]
}
```

### GET /proposals/:id

Get details of a specific proposal.

**Response:**
```json
{
  "success": true,
  "proposal": {
    "id": "proposal_1731708456123456789",
    "proposer": "134.199.175.106",
    "recipient": "TIME1mkLKMVtAbyBefLRcn8cLqjaxbVuBYheeYB",
    "amount": 100000000000,
    "reason": "Development funding",
    "created_at": 1731708456,
    "status": "Executed",
    "votes_for": ["134.199.175.106", "161.35.129.70", "50.28.104.50"],
    "votes_against": [],
    "executed_at": 1731708500,
    "tx_id": "treasury_grant_proposal_1731708456123456789"
  }
}
```

## Proposal States

| State | Description |
|-------|-------------|
| **Pending** | Waiting for votes, has not reached threshold |
| **Approved** | Has 2/3+ votes, waiting for execution |
| **Rejected** | More than 1/3 voted against, proposal is dead |
| **Executed** | Approved and included in a block, funds distributed |

## Amount Format

Amounts are specified in **satoshis**:
- 1 TIME = 100,000,000 satoshis
- 0.5 TIME = 50,000,000 satoshis
- 1000 TIME = 100,000,000,000 satoshis

When using the CLI, specify amounts in TIME (e.g., 1000), and it will be automatically converted to satoshis.

## Common Use Cases

### 1. Development Funding
```bash
time-cli proposal create \
  --address TIME1devWallet \
  --amount 5000 \
  --reason "Q4 2025 core development"
```

### 2. Marketing Campaign
```bash
time-cli proposal create \
  --address TIME1marketingWallet \
  --amount 2000 \
  --reason "Social media campaign for Q1 2026"
```

### 3. Community Initiatives
```bash
time-cli proposal create \
  --address TIME1communityWallet \
  --amount 1000 \
  --reason "Community meetup funding"
```

### 4. Testing Purposes
```bash
time-cli proposal create \
  --address TIME1testWallet \
  --amount 100 \
  --reason "Testing proposal system"
```

## Security & Best Practices

### Voting Guidelines
- ✅ Review proposal details carefully before voting
- ✅ Consider the reason and benefit to the network
- ✅ Verify the recipient address is legitimate
- ✅ Check the requested amount is reasonable
- ❌ Don't approve proposals without due diligence

### Proposal Guidelines
- ✅ Provide clear, detailed reasons
- ✅ Request only what's needed
- ✅ Include timeline or milestones if applicable
- ✅ Be transparent about use of funds
- ❌ Don't create spam proposals

### Storage & Persistence
- Proposals are saved to `/var/lib/time-coin/proposals.json`
- Automatically loaded on node startup
- Votes are persistent across restarts
- Executed proposals remain in history for auditing

## Troubleshooting

### Error: "Only masternodes can vote on proposals"
**Cause**: Your node is not registered as a masternode  
**Solution**: Register your node as a masternode first

### Error: "Proposal not found"
**Cause**: Invalid proposal ID or proposal doesn't exist  
**Solution**: Check the proposal ID with `time-cli proposal list`

### Proposal stuck in "Approved" status
**Cause**: Waiting for next block to execute  
**Solution**: Wait for block production cycle (up to 24 hours, or sooner with catch-up blocks)

### Vote not counting
**Cause**: Node may not be in consensus masternode list  
**Solution**: Ensure your node is properly connected and in BFT mode

## Related Documentation

- [Block Rewards Guide](block-rewards.md) - Understanding block reward distribution
- [Masternode Setup](masternodes/setup-guide.md) - Setting up masternodes
- [API Documentation](api/README.md) - Complete API reference

## Support

If you encounter issues with proposals:
1. Check node status: `systemctl status timed`
2. View node logs: `journalctl -u timed -f`
3. Check masternode registration: `time-cli masternodes list`
4. Ask in the community Telegram: https://t.me/+CaN6EflYM-83OTY0

