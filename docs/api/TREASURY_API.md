# Treasury API Endpoints Documentation

This document describes the REST API endpoints for treasury operations in TIME Coin.

## Overview

The Treasury API provides endpoints for creating and managing governance proposals, voting, and querying treasury state. All endpoints follow RESTful conventions and return JSON responses.

## Base URL

```
http://localhost:24101/treasury
```

(Port may vary based on network configuration: testnet uses 24101, mainnet uses 24001)

## Endpoints

### 1. Get Treasury Statistics

Get overall treasury statistics including balance, allocations, and pending proposals.

**Endpoint:** `GET /treasury/stats`

**Response:**
```json
{
  "balance": 500000000000,
  "balance_time": 5000.0,
  "total_allocated": 1000000000000,
  "total_distributed": 500000000000,
  "allocation_count": 150,
  "withdrawal_count": 5,
  "pending_proposals": 3
}
```

**Fields:**
- `balance` (u64): Current treasury balance in satoshis (1 TIME = 100,000,000 satoshis)
- `balance_time` (f64): Current balance in TIME coins
- `total_allocated` (u64): Total amount ever allocated to treasury
- `total_distributed` (u64): Total amount distributed from treasury
- `allocation_count` (usize): Number of allocation events
- `withdrawal_count` (usize): Number of withdrawal events
- `pending_proposals` (usize): Number of approved proposals awaiting execution

**Example:**
```bash
curl http://localhost:24101/treasury/stats
```

---

### 2. Get Treasury Allocations

Get the history of all treasury allocations from block rewards and transaction fees.

**Endpoint:** `GET /treasury/allocations`

**Response:**
```json
[
  {
    "block_number": 100,
    "amount": 500000000,
    "source": "BlockReward",
    "timestamp": 1700000000
  },
  {
    "block_number": 100,
    "amount": 250000000,
    "source": "TransactionFees",
    "timestamp": 1700000000
  }
]
```

**Fields:**
- `block_number` (u64): Block height where allocation occurred
- `amount` (u64): Amount allocated in satoshis
- `source` (string): Source of funds ("BlockReward" or "TransactionFees")
- `timestamp` (i64): Unix timestamp of allocation

**Example:**
```bash
curl http://localhost:24101/treasury/allocations
```

---

### 3. Get Treasury Withdrawals

Get the history of all treasury withdrawals for approved proposals.

**Endpoint:** `GET /treasury/withdrawals`

**Response:**
```json
[
  {
    "proposal_id": "proposal-001",
    "amount": 100000000000,
    "recipient": "TIME1recipient123",
    "block_number": 150,
    "timestamp": 1700100000
  }
]
```

**Fields:**
- `proposal_id` (string): ID of the executed proposal
- `amount` (u64): Amount withdrawn in satoshis
- `recipient` (string): TIME address of recipient
- `block_number` (u64): Block height where withdrawal occurred
- `timestamp` (i64): Unix timestamp of withdrawal

**Example:**
```bash
curl http://localhost:24101/treasury/withdrawals
```

---

### 4. List All Proposals

Get a list of all treasury proposals with summary information.

**Endpoint:** `GET /treasury/proposals`

**Response:**
```json
{
  "proposals": [
    {
      "id": "proposal-001",
      "title": "Development Grant",
      "amount": 100000000000,
      "amount_time": 1000.0,
      "submitter": "TIME1submitter456",
      "submission_time": 1700000000,
      "voting_deadline": 1701209600,
      "status": "Active",
      "vote_count": 5,
      "approval_percentage": 75
    }
  ],
  "count": 1
}
```

**Fields:**
- `proposals` (array): List of proposal summaries
- `count` (usize): Total number of proposals

**Proposal Summary Fields:**
- `id` (string): Unique proposal identifier
- `title` (string): Proposal title
- `amount` (u64): Requested amount in satoshis
- `amount_time` (f64): Requested amount in TIME coins
- `submitter` (string): Address of proposal submitter
- `submission_time` (u64): Unix timestamp when submitted
- `voting_deadline` (u64): Unix timestamp when voting ends
- `status` (string): Proposal status ("Active", "Approved", "Rejected", "Executed", "Expired")
- `vote_count` (usize): Number of votes cast
- `approval_percentage` (u64): Percentage of YES votes (0-100)

**Example:**
```bash
curl http://localhost:24101/treasury/proposals
```

---

### 5. Get Proposal Details

Get detailed information about a specific proposal including all votes and results.

**Endpoint:** `GET /treasury/proposal/{id}`

**Parameters:**
- `id` (path): Proposal ID

**Response:**
```json
{
  "id": "proposal-001",
  "title": "Development Grant",
  "description": "Fund development of mobile wallet",
  "recipient": "TIME1recipient123",
  "amount": 100000000000,
  "amount_time": 1000.0,
  "submitter": "TIME1submitter456",
  "submission_time": 1700000000,
  "voting_deadline": 1701209600,
  "execution_deadline": 1703801600,
  "status": "Active",
  "votes": [
    {
      "masternode_id": "mn-gold-1",
      "vote_choice": "Yes",
      "voting_power": 100,
      "timestamp": 1700100000
    }
  ],
  "voting_results": {
    "yes_power": 200,
    "no_power": 50,
    "abstain_power": 25,
    "total_votes": 275,
    "total_possible_power": 300,
    "approval_percentage": 72,
    "participation_rate": 91
  },
  "is_expired": false,
  "has_approval": true
}
```

**Fields:**
- All fields from proposal summary, plus:
- `description` (string): Detailed proposal description
- `execution_deadline` (u64): Unix timestamp when execution window ends
- `votes` (array): List of all votes cast
- `voting_results` (object): Calculated voting statistics
- `is_expired` (bool): Whether approved proposal has expired
- `has_approval` (bool): Whether proposal has 67%+ approval

**Vote Fields:**
- `masternode_id` (string): ID of voting masternode
- `vote_choice` (string): Vote choice ("Yes", "No", "Abstain")
- `voting_power` (u64): Voting power of masternode
- `timestamp` (u64): Unix timestamp of vote

**Voting Results Fields:**
- `yes_power` (u64): Total voting power for YES
- `no_power` (u64): Total voting power for NO
- `abstain_power` (u64): Total voting power for ABSTAIN
- `total_votes` (u64): Total voting power that voted
- `total_possible_power` (u64): Total voting power of all active masternodes
- `approval_percentage` (u64): Percentage of YES votes (0-100)
- `participation_rate` (u64): Percentage of masternodes that voted (0-100)

**Example:**
```bash
curl http://localhost:24101/treasury/proposal/proposal-001
```

**Error Response (404):**
```json
{
  "error": "BadRequest",
  "message": "Proposal proposal-999 not found"
}
```

---

### 6. Create Proposal

Create a new treasury proposal for governance voting.

**Endpoint:** `POST /treasury/proposal`

**Request Body:**
```json
{
  "id": "proposal-001",
  "title": "Development Grant",
  "description": "Fund development of mobile wallet application for iOS and Android",
  "recipient": "TIME1recipient123",
  "amount": 100000000000,
  "submitter": "TIME1submitter456",
  "voting_period_days": 14
}
```

**Request Fields:**
- `id` (string, required): Unique proposal identifier
- `title` (string, required): Short proposal title
- `description` (string, required): Detailed description
- `recipient` (string, required): TIME address to receive funds
- `amount` (u64, required): Amount requested in satoshis
- `submitter` (string, required): TIME address of submitter
- `voting_period_days` (u64, required): Number of days for voting (typically 14)

**Response (200):**
```json
{
  "status": "success",
  "proposal_id": "proposal-001",
  "message": "Treasury proposal created successfully"
}
```

**Error Response (400):**
```json
{
  "error": "BadRequest",
  "message": "Proposal proposal-001 already exists"
}
```

**Example:**
```bash
curl -X POST http://localhost:24101/treasury/proposal \
  -H "Content-Type: application/json" \
  -d '{
    "id": "proposal-001",
    "title": "Development Grant",
    "description": "Fund mobile wallet development",
    "recipient": "TIME1recipient123",
    "amount": 100000000000,
    "submitter": "TIME1submitter456",
    "voting_period_days": 14
  }'
```

---

### 7. Vote on Proposal

Cast a vote on an active proposal (masternodes only).

**Endpoint:** `POST /treasury/vote`

**Request Body:**
```json
{
  "proposal_id": "proposal-001",
  "masternode_id": "mn-gold-1",
  "vote_choice": "yes",
  "voting_power": 100
}
```

**Request Fields:**
- `proposal_id` (string, required): ID of proposal to vote on
- `masternode_id` (string, required): ID of voting masternode
- `vote_choice` (string, required): Vote choice ("yes", "no", or "abstain")
- `voting_power` (u64, required): Voting power of masternode (based on tier)

**Voting Power by Tier:**
- Bronze: 1x (e.g., 1 for single masternode)
- Silver: 5x (e.g., 5 for single masternode)
- Gold: 10x (e.g., 10 for single masternode)

**Response (200):**
```json
{
  "status": "success",
  "proposal_id": "proposal-001",
  "masternode_id": "mn-gold-1",
  "vote": "yes",
  "message": "Vote recorded successfully"
}
```

**Error Responses:**

**Invalid vote choice (400):**
```json
{
  "error": "BadRequest",
  "message": "Invalid vote choice. Must be 'yes', 'no', or 'abstain'"
}
```

**Proposal not found (400):**
```json
{
  "error": "BadRequest",
  "message": "Failed to vote on proposal: Proposal proposal-999 not found"
}
```

**Duplicate vote (400):**
```json
{
  "error": "BadRequest",
  "message": "Failed to vote on proposal: Masternode mn-gold-1 has already voted"
}
```

**Voting period ended (400):**
```json
{
  "error": "BadRequest",
  "message": "Failed to vote on proposal: Voting period has ended"
}
```

**Example:**
```bash
curl -X POST http://localhost:24101/treasury/vote \
  -H "Content-Type: application/json" \
  -d '{
    "proposal_id": "proposal-001",
    "masternode_id": "mn-gold-1",
    "vote_choice": "yes",
    "voting_power": 100
  }'
```

---

### 8. Approve Proposal (Internal)

Manually approve a proposal for treasury spending. This is typically called after governance consensus is reached.

**Endpoint:** `POST /treasury/approve`

**Request Body:**
```json
{
  "proposal_id": "proposal-001",
  "amount": 100000000000
}
```

**Note:** This endpoint is for internal use and may require admin authentication in production.

---

### 9. Distribute Funds (Internal)

Distribute treasury funds for an approved proposal.

**Endpoint:** `POST /treasury/distribute`

**Request Body:**
```json
{
  "proposal_id": "proposal-001",
  "recipient": "TIME1recipient123",
  "amount": 100000000000
}
```

**Note:** This endpoint is for internal use and should only be called after proposal execution.

---

## Proposal Lifecycle

1. **Creation** → Status: `Active`
   - Proposal is created via POST /treasury/proposal
   - Voting period begins immediately

2. **Voting** → Status: `Active`
   - Masternodes vote via POST /treasury/vote
   - Each masternode can vote once
   - Votes are weighted by tier (Bronze: 1x, Silver: 5x, Gold: 10x)

3. **Voting Ends** → Status: `Approved` or `Rejected`
   - After voting deadline passes
   - Requires 67%+ YES votes to be approved
   - Less than 67% YES votes → rejected

4. **Execution** → Status: `Executed`
   - Approved proposals can be executed
   - Funds are distributed to recipient
   - Proposal marked as executed

5. **Expiration** → Status: `Expired`
   - If approved proposal not executed before execution_deadline
   - Funds remain in treasury

## Approval Rules

- **Threshold:** 67% (2/3+) of voting power must vote YES
- **Calculation:** `(yes_power * 100) / total_votes >= 67`
- **Abstain votes** count toward total_votes but not yes_power
- **No minimum participation** required (any number of votes can approve)

## Error Codes

- `200` - Success
- `400` - Bad Request (invalid input, validation error)
- `404` - Not Found (proposal doesn't exist)
- `500` - Internal Server Error

## Security Considerations

1. **On-chain Governance:** All treasury operations are protocol-managed, no private keys
2. **One Vote Per Masternode:** Each masternode can only vote once per proposal
3. **Time Bounds:** Proposals have voting and execution deadlines
4. **Approval Threshold:** 67% supermajority required
5. **Immutable Votes:** Votes cannot be changed once cast
6. **Unique Proposal IDs:** Each proposal must have a unique identifier

## Best Practices

1. **Proposal IDs:** Use descriptive, unique IDs (e.g., "dev-grant-001", "marketing-q1-2024")
2. **Voting Period:** 14 days is standard for adequate deliberation
3. **Amount Precision:** Always specify amounts in satoshis (1 TIME = 100,000,000 satoshis)
4. **Validation:** Check proposal status before attempting to vote
5. **Monitor Deadlines:** Track voting_deadline and execution_deadline
6. **Check Approval:** Use `has_approval` field to determine if proposal passed

## Examples

### Complete Workflow

```bash
# 1. Check treasury balance
curl http://localhost:24101/treasury/stats

# 2. Create a proposal
curl -X POST http://localhost:24101/treasury/proposal \
  -H "Content-Type: application/json" \
  -d '{
    "id": "dev-grant-001",
    "title": "Mobile Wallet Development",
    "description": "Develop iOS and Android wallets",
    "recipient": "TIME1recipient123",
    "amount": 100000000000,
    "submitter": "TIME1submitter456",
    "voting_period_days": 14
  }'

# 3. Vote on the proposal (as masternode)
curl -X POST http://localhost:24101/treasury/vote \
  -H "Content-Type: application/json" \
  -d '{
    "proposal_id": "dev-grant-001",
    "masternode_id": "mn-gold-1",
    "vote_choice": "yes",
    "voting_power": 100
  }'

# 4. Check proposal details
curl http://localhost:24101/treasury/proposal/dev-grant-001

# 5. List all proposals
curl http://localhost:24101/treasury/proposals
```

## Rate Limiting

Currently no rate limiting is implemented. Consider implementing rate limiting for production deployments, especially for proposal creation endpoints.

## Authentication

Currently, endpoints are open. For production deployments, consider:
- Requiring masternode authentication for voting
- Admin authentication for manual approval/distribution
- Rate limiting per IP or masternode

## Support

For issues or questions about the Treasury API, please refer to:
- GitHub: https://github.com/time-coin/time-coin
- Documentation: docs/TREASURY_ARCHITECTURE.md
- Community: https://t.me/+CaN6EflYM-83OTY0
