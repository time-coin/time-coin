# TIME Coin API Documentation

This directory contains documentation for TIME Coin's API endpoints.

## Overview

TIME Coin provides multiple API interfaces:

### Bitcoin RPC-Compatible API (NEW! ✨)
Bitcoin-compatible JSON RPC endpoints for familiar developer experience.

**Documentation:**
- [Bitcoin RPC API Reference](BITCOIN_RPC_API.md) - Complete endpoint reference with examples
- [RPC Implementation Summary](RPC_IMPLEMENTATION_SUMMARY.md) - Implementation details and design decisions

**Quick Start:**
```bash
# Get blockchain information
curl -X POST http://localhost:24101/rpc/getblockchaininfo \
  -H "Content-Type: application/json" -d '{}'

# Get wallet balance
curl -X POST http://localhost:24101/rpc/getbalance \
  -H "Content-Type: application/json" \
  -d '{"address": "TIME1abc123..."}'
```

**Endpoints:** 14 RPC methods covering blockchain, transactions, wallet, network, and consensus operations.

---

### REST API
Native TIME Coin REST endpoints.

**Base URL:** `http://localhost:24101`

**Key Endpoints:**
- `/blockchain/info` - Blockchain information
- `/balance/{address}` - Get address balance
- `/transaction` - Submit transaction
- `/peers` - Connected peers
- `/mempool/status` - Mempool status

**Documentation:** See [API.md](../../API.md) in the root docs directory.

---

### Governance API
Endpoints for governance proposals and voting.

**Documentation:** [governance-api.md](governance-api.md)

---

### Treasury API
Endpoints for treasury operations and funding.

**Documentation:** [treasury-api.md](treasury-api.md)

---

## Which API Should I Use?

### Use Bitcoin RPC API if:
- You're familiar with Bitcoin's RPC interface
- You want to integrate with existing Bitcoin tools
- You need Bitcoin-compatible responses
- You're building a block explorer or wallet

### Use REST API if:
- You prefer RESTful conventions
- You need TIME-specific features
- You're building a new application from scratch
- You need real-time updates via WebSocket

### Use Governance/Treasury APIs if:
- You're building governance tools
- You need treasury management
- You're integrating voting features

---

## Network Ports

| Network | P2P Port | RPC Port |
|---------|----------|----------|
| Testnet | 24100    | 24101    |
| Mainnet | 24000    | 24001    |

---

## Authentication

Currently no authentication required in testnet/dev mode.

Production will require:
- API keys for write operations
- Rate limiting per IP address
- Optional basic authentication

---

## Rate Limits

**Testnet/Dev:**
- No limits

**Production (planned):**
- 100 read requests/minute per IP
- 10 write requests/minute per IP
- Burst allowance of 20 requests

---

## CORS

**Testnet/Dev:**
- All origins allowed

**Production:**
- Whitelisted domains only
- Configurable in server settings

---

## Error Handling

All APIs use consistent error format:

```json
{
  "error": "error_type",
  "message": "Detailed error message"
}
```

**Common Error Types:**
- `invalid_address` - Address format invalid
- `insufficient_balance` - Not enough balance
- `transaction_not_found` - Transaction doesn't exist
- `invalid_signature` - Signature verification failed
- `internal_error` - Server error
- `bad_request` - Invalid request format
- `unauthorized` - Authentication required

---

## Examples

### JavaScript/Node.js
```javascript
const TIME_RPC = 'http://localhost:24101/rpc';

async function getBlockchainInfo() {
  const response = await fetch(`${TIME_RPC}/getblockchaininfo`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({})
  });
  return response.json();
}
```

### Python
```python
import requests

TIME_RPC = 'http://localhost:24101/rpc'

def get_blockchain_info():
    response = requests.post(
        f'{TIME_RPC}/getblockchaininfo',
        json={}
    )
    return response.json()
```

### Rust
```rust
use reqwest;
use serde_json::json;

const TIME_RPC: &str = "http://localhost:24101/rpc";

async fn get_blockchain_info() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/getblockchaininfo", TIME_RPC))
        .json(&json!({}))
        .send()
        .await?
        .json()
        .await?;
    Ok(response)
}
```

---

## Support

For questions or issues:
- GitHub Issues: https://github.com/time-coin/time-coin/issues
- Telegram: https://t.me/+CaN6EflYM-83OTY0
- Documentation: https://docs.time-coin.io

---

⏰ TIME is money. Make it accessible.
