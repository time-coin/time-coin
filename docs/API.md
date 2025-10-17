# TIME Coin REST API Documentation

## Base URL

```
http://localhost:24101
```

## Endpoints

### Health Check

**GET** `/health`

Check if the API server is running.

**Response:**
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime": 3600,
  "dev_mode": true
}
```

---

### Create Transaction

**POST** `/transaction/create`

Create and submit a new transaction.

**Request Body:**
```json
{
  "from": "TIME1abc123...",
  "to": "TIME1def456...",
  "amount": 100000000,
  "fee": 1000000,
  "private_key": "your_private_key_hex"
}
```

**Response:**
```json
{
  "txid": "uuid-v4",
  "status": "confirmed",
  "message": "Transaction confirmed (dev mode)"
}
```

**Notes:**
- Amount and fee are in satoshis (100,000,000 = 1 TIME)
- In dev mode, transactions are auto-approved
- In production mode, transactions go through BFT consensus

---

### Get Transaction

**GET** `/transaction/{txid}`

Get transaction status and details.

**Response:**
```json
{
  "txid": "uuid-v4",
  "status": "confirmed",
  "from": "TIME1abc123...",
  "to": "TIME1def456...",
  "amount": 100000000,
  "fee": 1000000,
  "timestamp": 1729123456,
  "confirmations": 1
}
```

---

### Get Balance

**GET** `/balance/{address}`

Get balance for an address.

**Response:**
```json
{
  "address": "TIME1abc123...",
  "balance": 100000000,
  "balance_time": "1.00 TIME",
  "pending": 0
}
```

---

### Get Blockchain Info

**GET** `/blockchain/info`

Get current blockchain information.

**Response:**
```json
{
  "network": "testnet",
  "height": 0,
  "best_block_hash": "00000000839a8e68...",
  "total_supply": 100000000000000,
  "timestamp": 1729123456
}
```

---

### Generate Keypair

**POST** `/keypair/generate`

Generate a new keypair for testing.

**Response:**
```json
{
  "address": "TIME1abc123...",
  "public_key": "hex_encoded_public_key",
  "private_key": "hex_encoded_private_key",
  "warning": "⚠️  NEVER share your private key! Store it securely!"
}
```

**⚠️ Security Warning:**
- This endpoint should only be used in dev/testnet
- NEVER use this in production
- NEVER share private keys

---

## Error Responses

All errors return:

```json
{
  "error": "error_type",
  "message": "Detailed error message"
}
```

**Error Types:**
- `invalid_address` - Address format is invalid
- `insufficient_balance` - Not enough balance
- `transaction_not_found` - TX doesn't exist
- `invalid_signature` - Signature verification failed
- `internal_error` - Server error

---

## Testing Examples

### Using curl

```bash
# Health check
curl http://localhost:24101/health

# Generate keypair
curl -X POST http://localhost:24101/keypair/generate

# Check balance
curl http://localhost:24101/balance/TIME1treasury00000000000000000000000000

# Create transaction
curl -X POST http://localhost:24101/transaction/create \
  -H "Content-Type: application/json" \
  -d '{
    "from": "TIME1treasury00000000000000000000000000",
    "to": "TIME1development0000000000000000000000",
    "amount": 100000000,
    "fee": 1000000,
    "private_key": "test_key"
  }'

# Get blockchain info
curl http://localhost:24101/blockchain/info
```

### Using JavaScript

```javascript
// Check balance
const response = await fetch('http://localhost:24101/balance/TIME1treasury00000000000000000000000000');
const balance = await response.json();
console.log(balance);

// Create transaction
const tx = await fetch('http://localhost:24101/transaction/create', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    from: 'TIME1treasury00000000000000000000000000',
    to: 'TIME1development0000000000000000000000',
    amount: 100000000,
    fee: 1000000,
    private_key: 'your_private_key'
  })
});
const result = await tx.json();
console.log(result);
```

---

## Rate Limits

Currently no rate limits in testnet/dev mode.

Production will have:
- 100 requests/minute per IP
- 10 transaction creations/minute per IP

---

## CORS

All origins allowed in dev mode.

Production will restrict to:
- Official TIME Coin wallet domains
- Whitelisted applications
