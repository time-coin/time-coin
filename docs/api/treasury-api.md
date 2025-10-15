# Treasury API

## Endpoints

### Get Treasury Balance
```
GET /api/v1/treasury/balance
```

Response:
```json
{
  "balance": 1000000,
  "balance_time": 10000.0,
  "last_updated": 1234567890
}
```

### Get Treasury Statistics
```
GET /api/v1/treasury/stats
```

Response:
```json
{
  "total_deposits": 5000000,
  "total_withdrawals": 1000000,
  "transaction_count": 1234,
  "active_proposals": 5
}
```

### Submit Withdrawal Request
```
POST /api/v1/treasury/withdraw
```

Request:
```json
{
  "proposal_id": "prop-123",
  "milestone_id": "milestone-1",
  "amount": 10000,
  "recipient": "TIME_address_here",
  "signature": "..."
}
```

## Authentication

All write operations require masternode authentication.
