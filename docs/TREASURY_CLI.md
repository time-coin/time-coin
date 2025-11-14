# Treasury CLI Commands

This document describes the CLI commands for interacting with the TIME Coin treasury system.

## Overview

The treasury CLI provides commands for:
- Viewing treasury information and statistics
- Listing and viewing treasury proposals
- Submitting new treasury proposals
- Voting on treasury proposals

All commands support JSON output via the `--json` flag for scripting and automation.

## Commands

### 1. Get Treasury Information

View current treasury balance and statistics:

```bash
time-cli treasury info
```

**Output:**
```
💰 Treasury Information
━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Balance: 12345.67 TIME
Total Allocated: 500000000000 satoshis
Total Distributed: 300000000000 satoshis
Allocations: 45
Withdrawals: 23
Pending Proposals: 3
```

**JSON Output:**
```bash
time-cli treasury info --json
```

### 2. List All Proposals

View all treasury proposals:

```bash
time-cli treasury list-proposals
```

**Output:**
```
📋 Treasury Proposals (3 total)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━

1. Development Grant (prop-123)
   Amount: 1000.0 TIME
   Status: Active
   Votes: 5 Yes / 2 No

2. Marketing Campaign (prop-124)
   Amount: 500.0 TIME
   Status: Approved
   Votes: 8 Yes / 1 No
```

**JSON Output:**
```bash
time-cli treasury list-proposals --json
```

### 3. Get Specific Proposal

View detailed information about a specific proposal:

```bash
time-cli treasury get-proposal prop-123
```

**Output:**
```
📄 Proposal Details
━━━━━━━━━━━━━━━━━━━━━━━━━━━━
ID: prop-123
Title: Development Grant
Description: Fund development of new features
Recipient: TIME1dev000000000000000000000000000000
Amount: 1000.0 TIME
Status: Active
Submitter: node-1

Votes (7):
  192.168.1.100 -> Yes
  192.168.1.101 -> Yes
  192.168.1.102 -> No
```

**JSON Output:**
```bash
time-cli treasury get-proposal prop-123 --json
```

### 4. Submit a New Proposal

Create a new treasury proposal:

```bash
time-cli treasury propose \
  --title "Development Grant" \
  --description "Fund development of new wallet features" \
  --recipient TIME1dev000000000000000000000000000000 \
  --amount 1000.0 \
  --voting-period 14
```

**Arguments:**
- `--title` (required): Short title for the proposal
- `--description` (required): Detailed description of the proposal
- `--recipient` (required): TIME address to receive the funds
- `--amount` (required): Amount in TIME (e.g., 1000.0)
- `--voting-period` (optional): Voting period in days (default: 14)

**Output:**
```
📝 Submitting Treasury Proposal
━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Title: Development Grant
Description: Fund development of new wallet features
Recipient: TIME1dev000000000000000000000000000000
Amount: 1000.0 TIME (100000000000 satoshis)
Voting Period: 14 days

📡 Submitting proposal...

✅ SUCCESS!
Proposal ID: prop-1634567890
Proposal submitted successfully and is now open for voting
```

**JSON Output:**
```bash
time-cli treasury propose \
  --title "Development Grant" \
  --description "Fund development" \
  --recipient TIME1dev000000000000000000000000000000 \
  --amount 1000.0 \
  --json
```

### 5. Vote on a Proposal

Cast a vote on a treasury proposal:

```bash
time-cli treasury vote prop-123 yes
```

**Arguments:**
- `<PROPOSAL_ID>` (required): The proposal ID to vote on
- `<VOTE>` (required): Vote choice - one of: `yes`, `no`, `abstain`
- `--masternode-id` (optional): Masternode ID (defaults to local node IP)

**Vote Choices:**
- `yes` - Vote in favor of the proposal
- `no` - Vote against the proposal
- `abstain` - Abstain from voting

**Examples:**

Vote yes:
```bash
time-cli treasury vote prop-123 yes
```

Vote no:
```bash
time-cli treasury vote prop-123 no
```

Abstain from voting:
```bash
time-cli treasury vote prop-123 abstain
```

Vote with specific masternode:
```bash
time-cli treasury vote prop-123 yes --masternode-id 192.168.1.100
```

**Output:**
```
🗳️  Casting Vote
━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Proposal ID: prop-123
Vote: Yes
Masternode: 192.168.1.100

📡 Submitting vote...

✅ SUCCESS!
Vote 'Yes' recorded successfully
```

**JSON Output:**
```bash
time-cli treasury vote prop-123 yes --json
```

## API Configuration

All commands support specifying a custom API endpoint:

```bash
time-cli --api http://example.com:24101 treasury info
```

Or via short flag:
```bash
time-cli -a http://example.com:24101 treasury info
```

Default API endpoint: `http://localhost:24101`

## JSON Output

All commands support JSON output for scripting and automation:

```bash
time-cli treasury info --json
time-cli treasury list-proposals --json
time-cli treasury get-proposal prop-123 --json
time-cli treasury propose --title "..." --description "..." --recipient "..." --amount 1000 --json
time-cli treasury vote prop-123 yes --json
```

## Security Considerations

1. **On-chain Validation**: All proposals and votes are validated on-chain
2. **Masternode Only**: Only active masternodes can vote on proposals
3. **Consensus Required**: Proposals require 2/3+ masternode approval
4. **One Vote Per Node**: Each masternode can only vote once per proposal
5. **Time-locked Execution**: Approved proposals have a 30-day execution window

## Testing

Run the treasury CLI tests:

```bash
cargo test --test treasury_cli_test
```

## Examples

### Complete Workflow

1. Check treasury balance:
```bash
time-cli treasury info
```

2. Submit a proposal:
```bash
time-cli treasury propose \
  --title "Q4 Marketing Campaign" \
  --description "Marketing campaign for Q4 2024" \
  --recipient TIME1marketing00000000000000000000000 \
  --amount 5000.0 \
  --voting-period 14
```

3. List proposals to get the ID:
```bash
time-cli treasury list-proposals
```

4. Vote on the proposal:
```bash
time-cli treasury vote prop-1634567890 yes
```

5. Check proposal status:
```bash
time-cli treasury get-proposal prop-1634567890
```

### Scripting Example

Using JSON output for automation:

```bash
#!/bin/bash

# Get treasury balance
BALANCE=$(time-cli treasury info --json | jq -r '.balance_time')
echo "Current treasury balance: $BALANCE TIME"

# Submit proposal only if balance is sufficient
if (( $(echo "$BALANCE > 1000" | bc -l) )); then
  PROPOSAL_ID=$(time-cli treasury propose \
    --title "Automated Proposal" \
    --description "Auto-generated proposal" \
    --recipient TIME1auto000000000000000000000000000000 \
    --amount 1000.0 \
    --json | jq -r '.proposal_id')
  
  echo "Submitted proposal: $PROPOSAL_ID"
  
  # Automatically vote yes
  time-cli treasury vote "$PROPOSAL_ID" yes --json
fi
```
