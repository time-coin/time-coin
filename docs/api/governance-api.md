# Governance API

## Endpoints

### List Active Proposals
```
GET /api/v1/governance/proposals
```

### Get Proposal Details
```
GET /api/v1/governance/proposals/{id}
```

### Submit Proposal
```
POST /api/v1/governance/proposals
```

### Cast Vote
```
POST /api/v1/governance/vote
```

Request:
```json
{
  "proposal_id": "prop-123",
  "choice": "yes",
  "masternode_id": "mn-456",
  "signature": "..."
}
```

### Get Voting Results
```
GET /api/v1/governance/proposals/{id}/results
```

Response:
```json
{
  "proposal_id": "prop-123",
  "yes_votes": 1500,
  "no_votes": 500,
  "abstain": 200,
  "approval_percentage": 75,
  "participation_rate": 65
}
```
