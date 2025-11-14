# Treasury API

## Overview

The Treasury API provides endpoints for managing the TIME Coin protocol-managed treasury, including proposal creation, voting, and fund distribution. All operations are governed by masternode consensus requiring 67% approval.

## Endpoints

### Get Treasury Statistics
```
GET /treasury/stats
```

Returns current treasury balance and statistics.

**Response:**
```json
{
  "balance": 1000000000,
  "balance_time": 10.0,
  "total_allocated": 5000000000,
  "total_distributed": 1000000000,
  "allocation_count": 100,
  "withdrawal_count": 10,
  "pending_proposals": 5
}
```

### Get Treasury Allocations
```
GET /treasury/allocations
```

Returns history of treasury fund allocations from block rewards and fees.

**Response:**
```json
[
  {
    "block_number": 100,
    "amount": 500000000,
    "source": "BlockReward",
    "timestamp": 1699564800
  },
  {
    "block_number": 100,
    "amount": 250000000,
    "source": "TransactionFees",
    "timestamp": 1699564800
  }
]
```

### Get Treasury Withdrawals
```
GET /treasury/withdrawals
```

Returns history of treasury fund distributions.

**Response:**
```json
[
  {
    "proposal_id": "dev-grant-001",
    "amount": 10000000000,
    "recipient": "TIME1recipient...",
    "block_number": 150,
    "timestamp": 1699651200
  }
]
```

### List All Proposals
```
GET /treasury/proposals
```

Returns all treasury proposals with their current status and voting results.

**Response:**
```json
[
  {
    "id": "dev-grant-001",
    "title": "Mobile Wallet Development",
    "description": "Funding for iOS and Android wallet development",
    "recipient": "TIME1recipient...",
    "amount": 10000000000,
    "amount_time": 100.0,
    "submitter": "TIME1submitter...",
    "submission_time": 1699564800,
    "voting_deadline": 1700774400,
    "execution_deadline": 1703366400,
    "status": "Active",
    "yes_votes": 1500,
    "no_votes": 200,
    "total_votes": 1700,
    "approval_percentage": 88.2
  }
]
```

### Get Specific Proposal
```
GET /treasury/proposal/:id
```

Returns detailed information about a specific proposal.

**Parameters:**
- `id` - Proposal ID

**Response:**
```json
{
  "id": "dev-grant-001",
  "title": "Mobile Wallet Development",
  "description": "Funding for iOS and Android wallet development",
  "recipient": "TIME1recipient...",
  "amount": 10000000000,
  "amount_time": 100.0,
  "submitter": "TIME1submitter...",
  "submission_time": 1699564800,
  "voting_deadline": 1700774400,
  "execution_deadline": 1703366400,
  "status": "Active",
  "yes_votes": 1500,
  "no_votes": 200,
  "total_votes": 1700,
  "approval_percentage": 88.2
}
```

### Create Proposal
```
POST /treasury/proposal
```

Creates a new treasury spending proposal.

**Request:**
```json
{
  "id": "dev-grant-001",
  "title": "Mobile Wallet Development",
  "description": "Funding for iOS and Android wallet development",
  "recipient": "TIME1recipient...",
  "amount": 10000000000,
  "submitter": "TIME1submitter...",
  "voting_period_days": 14
}
```

**Response:**
```json
{
  "status": "success",
  "proposal_id": "dev-grant-001",
  "message": "Proposal created successfully"
}
```

### Vote on Proposal
```
POST /treasury/vote
```

Records a masternode vote on a proposal.

**Request:**
```json
{
  "proposal_id": "dev-grant-001",
  "masternode_id": "masternode-gold-1",
  "vote_choice": "yes",
  "voting_power": 100
}
```

**Parameters:**
- `vote_choice` - One of: "yes", "no", "abstain"
- `voting_power` - Voting power based on masternode tier (Bronze: 1, Silver: 5, Gold: 10)

**Response:**
```json
{
  "status": "success",
  "proposal_id": "dev-grant-001",
  "masternode_id": "masternode-gold-1",
  "vote": "yes",
  "message": "Vote recorded successfully"
}
```

### Approve Proposal (Legacy)
```
POST /treasury/approve
```

Approve a proposal for spending. Note: This endpoint is for backward compatibility. Use the voting system for proper governance.

**Request:**
```json
{
  "proposal_id": "dev-grant-001",
  "amount": 10000000000
}
```

### Distribute Funds
```
POST /treasury/distribute
```

Distribute funds for an approved proposal.

**Request:**
```json
{
  "proposal_id": "dev-grant-001",
  "recipient": "TIME1recipient...",
  "amount": 10000000000
}
```

**Response:**
```json
{
  "status": "success",
  "proposal_id": "dev-grant-001",
  "recipient": "TIME1recipient...",
  "amount": 10000000000,
  "message": "Treasury funds distributed successfully"
}
```

## RPC Endpoints

### Get Treasury Info
```
POST /rpc/gettreasury
```

**Request:**
```json
{}
```

**Response:**
```json
{
  "balance": 10.0,
  "total_allocated": 50.0,
  "pending_proposals": 5,
  "monthly_budget": 150.0
}
```

### List Proposals
```
POST /rpc/listproposals
```

**Request:**
```json
{}
```

**Response:**
```json
[
  {
    "id": "dev-grant-001",
    "title": "Mobile Wallet Development",
    "amount": 100.0,
    "votes_yes": 1500,
    "votes_no": 200,
    "status": "Active"
  }
]
```

## Proposal Lifecycle

1. **Draft** - Proposal is created but not yet submitted
2. **Active** - Proposal is open for voting (14 days by default)
3. **Approved** - Proposal received 67%+ yes votes
4. **Rejected** - Proposal did not receive sufficient votes
5. **Executed** - Funds have been distributed
6. **Expired** - Approved proposal was not executed within deadline

## Voting Power

Masternode voting power is based on tier:
- Bronze: 1x voting power
- Silver: 5x voting power  
- Gold: 10x voting power

## Approval Threshold

Proposals require 67% (2/3+) of cast votes to be "Yes" votes for approval.

## Authentication

Currently, proposal creation and voting endpoints accept requests without authentication for testnet. In production:
- Proposal creation should require submitter signature
- Voting should require masternode signature
- Only approved proposals can be executed
- Distribution requires governance consensus validation
