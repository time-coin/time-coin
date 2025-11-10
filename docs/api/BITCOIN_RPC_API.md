# Bitcoin RPC-Compatible API Documentation

This document describes TIME Coin's Bitcoin RPC-compatible JSON API endpoints. These endpoints follow Bitcoin's established RPC interface for maximum developer familiarity and tool compatibility.

## Base URL

```
http://localhost:24101/rpc
```

All RPC endpoints:
- Use POST method
- Accept and return JSON
- Are prefixed with `/rpc/`

## Authentication

Currently no authentication required in testnet/dev mode. Production will require authentication.

---

## Blockchain RPC Methods

### getblockchaininfo

Get blockchain state information.

**Endpoint:** `POST /rpc/getblockchaininfo`

**Request:**
```json
{}
```

**Response:**
```json
{
  "chain": "testnet",
  "blocks": 12345,
  "headers": 12345,
  "bestblockhash": "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f",
  "difficulty": 1.0,
  "mediantime": 1699564800,
  "verificationprogress": 1.0,
  "initialblockdownload": false,
  "chainwork": "0000000000000000000000000000000000000000000000000000000000003039",
  "size_on_disk": 0,
  "pruned": false
}
```

**Notes:**
- TIME uses BFT consensus, not PoW, so `difficulty` is always 1.0
- `chainwork` is simplified for TIME's consensus model

---

### getblockcount

Get the current block height.

**Endpoint:** `POST /rpc/getblockcount`

**Request:**
```json
{}
```

**Response:**
```json
{
  "result": 12345
}
```

---

### getblockhash

Get block hash for a given height.

**Endpoint:** `POST /rpc/getblockhash`

**Request:**
```json
{
  "height": 12345
}
```

**Response:**
```json
{
  "result": "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f"
}
```

---

### getblock

Get block information by hash.

**Endpoint:** `POST /rpc/getblock`

**Request:**
```json
{
  "blockhash": "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f",
  "verbosity": 1
}
```

**Parameters:**
- `blockhash` (string, required): Block hash
- `verbosity` (number, optional): 0=hex, 1=json (default), 2=json with tx details

**Response:**
```json
{
  "hash": "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f",
  "confirmations": 100,
  "height": 12345,
  "version": 1,
  "merkleroot": "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
  "time": 1699564800,
  "mediantime": 1699564800,
  "nonce": 0,
  "bits": "00000000",
  "difficulty": 1.0,
  "chainwork": "0000000000000000000000000000000000000000000000000000000000003039",
  "tx": [
    "e3bf3d07d4b0375638d5f1db5255fe07ba2c4cb067cd81b84ee974b6585fb468",
    "a1075db55d416d3ca199f55b6084e2115b9345e16c5cf302fc80e9d5fbf5d48d"
  ],
  "previousblockhash": "00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048",
  "nextblockhash": "000000006a625f06636b8bb6ac7b960a8d03705d1ace08b1a19da3fdcc99ddbd"
}
```

---

## Transaction RPC Methods

### getrawtransaction

Get transaction details by txid.

**Endpoint:** `POST /rpc/getrawtransaction`

**Request:**
```json
{
  "txid": "e3bf3d07d4b0375638d5f1db5255fe07ba2c4cb067cd81b84ee974b6585fb468",
  "verbose": true
}
```

**Parameters:**
- `txid` (string, required): Transaction ID
- `verbose` (boolean, optional): If true, return JSON object; if false, return hex string

**Response:**
```json
{
  "hex": "0100000001...",
  "txid": "e3bf3d07d4b0375638d5f1db5255fe07ba2c4cb067cd81b84ee974b6585fb468",
  "hash": "e3bf3d07d4b0375638d5f1db5255fe07ba2c4cb067cd81b84ee974b6585fb468",
  "size": 250,
  "vsize": 250,
  "version": 1,
  "locktime": 0,
  "vin": [
    {
      "txid": "a1075db55d416d3ca199f55b6084e2115b9345e16c5cf302fc80e9d5fbf5d48d",
      "vout": 0,
      "scriptSig": {
        "asm": "3045022100...",
        "hex": "483045022100..."
      },
      "sequence": 4294967295
    }
  ],
  "vout": [
    {
      "value": 1.5,
      "n": 0,
      "scriptPubKey": {
        "asm": "OP_DUP OP_HASH160 ... OP_EQUALVERIFY OP_CHECKSIG",
        "hex": "76a914...",
        "type": "pubkeyhash",
        "address": "TIME1abc123..."
      }
    }
  ],
  "blockhash": "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f",
  "confirmations": 100,
  "time": 1699564800,
  "blocktime": 1699564800
}
```

---

### sendrawtransaction

Broadcast a raw transaction to the network.

**Endpoint:** `POST /rpc/sendrawtransaction`

**Request:**
```json
{
  "hexstring": "0100000001...",
  "maxfeerate": 0.1
}
```

**Parameters:**
- `hexstring` (string, required): Raw transaction in hex format
- `maxfeerate` (number, optional): Maximum fee rate in TIME/KB

**Response:**
```json
{
  "result": "e3bf3d07d4b0375638d5f1db5255fe07ba2c4cb067cd81b84ee974b6585fb468"
}
```

---

## Wallet RPC Methods

### getwalletinfo

Get wallet information.

**Endpoint:** `POST /rpc/getwalletinfo`

**Request:**
```json
{}
```

**Response:**
```json
{
  "walletname": "time-wallet",
  "walletversion": 1,
  "balance": 100.5,
  "unconfirmed_balance": 0.0,
  "immature_balance": 0.0,
  "txcount": 150,
  "keypoololdest": 1699564800,
  "keypoolsize": 100,
  "paytxfee": 0.00001,
  "hdseedid": null,
  "private_keys_enabled": true
}
```

---

### getbalance

Get wallet balance.

**Endpoint:** `POST /rpc/getbalance`

**Request:**
```json
{
  "address": "TIME1abc123..."
}
```

**Parameters:**
- `address` (string, optional): Address to check balance for. If omitted, uses node's wallet address.

**Response:**
```json
{
  "result": 100.5
}
```

**Notes:**
- Balance is returned in TIME coins (not satoshis)
- 1 TIME = 100,000,000 satoshis

---

### getnewaddress

Generate a new TIME address.

**Endpoint:** `POST /rpc/getnewaddress`

**Request:**
```json
{}
```

**Response:**
```json
{
  "result": "TIME1abc123def456..."
}
```

---

### validateaddress

Validate a TIME address.

**Endpoint:** `POST /rpc/validateaddress`

**Request:**
```json
{
  "address": "TIME1abc123..."
}
```

**Response:**
```json
{
  "isvalid": true,
  "address": "TIME1abc123...",
  "scriptPubKey": "76a914...",
  "isscript": false,
  "iswitness": false
}
```

---

### listunspent

List unspent transaction outputs (UTXOs).

**Endpoint:** `POST /rpc/listunspent`

**Request:**
```json
{
  "minconf": 1,
  "maxconf": 9999999,
  "addresses": ["TIME1abc123..."]
}
```

**Parameters:**
- `minconf` (number, optional): Minimum confirmations (default: 0)
- `maxconf` (number, optional): Maximum confirmations (default: 9999999)
- `addresses` (array, optional): Filter by addresses. If empty, uses node's wallet.

**Response:**
```json
[
  {
    "txid": "e3bf3d07d4b0375638d5f1db5255fe07ba2c4cb067cd81b84ee974b6585fb468",
    "vout": 0,
    "address": "TIME1abc123...",
    "account": "",
    "scriptPubKey": "76a914...",
    "amount": 1.5,
    "confirmations": 100,
    "spendable": true,
    "solvable": true
  }
]
```

---

### listtransactions

List recent transactions.

**Endpoint:** `POST /rpc/listtransactions`

**Request:**
```json
{
  "count": 10,
  "skip": 0
}
```

**Parameters:**
- `count` (number, optional): Number of transactions to return (default: 10)
- `skip` (number, optional): Number of transactions to skip (default: 0)

**Response:**
```json
[
  {
    "address": "TIME1abc123...",
    "category": "receive",
    "amount": 1.5,
    "vout": 0,
    "confirmations": 100,
    "blockhash": "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f",
    "blockheight": 12345,
    "blocktime": 1699564800,
    "txid": "e3bf3d07d4b0375638d5f1db5255fe07ba2c4cb067cd81b84ee974b6585fb468",
    "time": 1699564800,
    "timereceived": 1699564800
  }
]
```

---

## Network RPC Methods

### getpeerinfo

Get information about connected peers.

**Endpoint:** `POST /rpc/getpeerinfo`

**Request:**
```json
{}
```

**Response:**
```json
[
  {
    "id": 0,
    "addr": "192.168.1.100:24100",
    "addrlocal": null,
    "services": "0000000000000001",
    "relaytxes": true,
    "lastsend": 1699564800,
    "lastrecv": 1699564800,
    "bytessent": 1024000,
    "bytesrecv": 2048000,
    "conntime": 1699564700,
    "timeoffset": 0,
    "pingtime": 0.05,
    "version": "1.0.0",
    "subver": "/TIME:1.0.0/",
    "inbound": false,
    "startingheight": 12000,
    "banscore": 0,
    "synced_headers": 12345,
    "synced_blocks": 12345
  }
]
```

---

### getnetworkinfo

Get network information.

**Endpoint:** `POST /rpc/getnetworkinfo`

**Request:**
```json
{}
```

**Response:**
```json
{
  "version": 1000000,
  "subversion": "/TIME:1.0.0/",
  "protocolversion": 1,
  "localservices": "0000000000000001",
  "localrelay": true,
  "timeoffset": 0,
  "networkactive": true,
  "connections": 8,
  "networks": [
    {
      "name": "ipv4",
      "limited": false,
      "reachable": true,
      "proxy": "",
      "proxy_randomize_credentials": false
    }
  ],
  "relayfee": 0.00001,
  "incrementalfee": 0.00001,
  "localaddresses": [],
  "warnings": ""
}
```

---

## Mining/Consensus RPC Methods

### getmininginfo

Get mining/consensus information.

**Endpoint:** `POST /rpc/getmininginfo`

**Request:**
```json
{}
```

**Response:**
```json
{
  "blocks": 12345,
  "currentblockweight": 0,
  "currentblocktx": 0,
  "difficulty": 1.0,
  "networkhashps": 0.0,
  "pooledtx": 25,
  "chain": "testnet",
  "warnings": "TIME Coin uses BFT consensus, not Proof-of-Work mining"
}
```

**Notes:**
- TIME uses BFT consensus instead of PoW mining
- `networkhashps` is not applicable and returns 0.0
- `pooledtx` shows the number of transactions in the mempool

---

### estimatefee

Estimate transaction fee.

**Endpoint:** `POST /rpc/estimatefee`

**Request:**
```json
{
  "conf_target": 6
}
```

**Parameters:**
- `conf_target` (number, required): Confirmation target (blocks)

**Response:**
```json
{
  "feerate": 0.00001,
  "blocks": 1
}
```

**Notes:**
- TIME has instant finality, so fees are constant
- Fee rate is in TIME per KB

---

## Error Responses

All errors return in this format:

```json
{
  "error": "error_type",
  "message": "Detailed error message"
}
```

**Common Error Types:**
- `invalid_address` - Address format is invalid
- `insufficient_balance` - Not enough balance
- `transaction_not_found` - Transaction doesn't exist
- `invalid_signature` - Signature verification failed
- `internal_error` - Server error
- `bad_request` - Invalid request format

---

## Usage Examples

### Using curl

```bash
# Get blockchain info
curl -X POST http://localhost:24101/rpc/getblockchaininfo \
  -H "Content-Type: application/json" \
  -d '{}'

# Get block count
curl -X POST http://localhost:24101/rpc/getblockcount \
  -H "Content-Type: application/json" \
  -d '{}'

# Get block hash
curl -X POST http://localhost:24101/rpc/getblockhash \
  -H "Content-Type: application/json" \
  -d '{"height": 100}'

# Get balance
curl -X POST http://localhost:24101/rpc/getbalance \
  -H "Content-Type: application/json" \
  -d '{"address": "TIME1abc123..."}'

# Generate new address
curl -X POST http://localhost:24101/rpc/getnewaddress \
  -H "Content-Type: application/json" \
  -d '{}'

# List unspent outputs
curl -X POST http://localhost:24101/rpc/listunspent \
  -H "Content-Type: application/json" \
  -d '{"addresses": ["TIME1abc123..."]}'
```

### Using JavaScript

```javascript
async function getBlockchainInfo() {
  const response = await fetch('http://localhost:24101/rpc/getblockchaininfo', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({})
  });
  return await response.json();
}

async function getBalance(address) {
  const response = await fetch('http://localhost:24101/rpc/getbalance', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ address })
  });
  return await response.json();
}

async function generateNewAddress() {
  const response = await fetch('http://localhost:24101/rpc/getnewaddress', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({})
  });
  return await response.json();
}
```

### Using Python

```python
import requests

def get_blockchain_info():
    response = requests.post(
        'http://localhost:24101/rpc/getblockchaininfo',
        json={}
    )
    return response.json()

def get_balance(address):
    response = requests.post(
        'http://localhost:24101/rpc/getbalance',
        json={'address': address}
    )
    return response.json()

def generate_new_address():
    response = requests.post(
        'http://localhost:24101/rpc/getnewaddress',
        json={}
    )
    return response.json()
```

---

## Differences from Bitcoin RPC

While TIME Coin's RPC API closely follows Bitcoin's interface, there are some key differences:

1. **Consensus Model**: TIME uses BFT consensus instead of PoW
   - No mining/hashrate concepts
   - Instant transaction finality (no need for multiple confirmations)
   - Fixed difficulty (always 1.0)

2. **Block Production**: TIME uses 24-hour time blocks
   - Blocks serve as periodic checkpoints, not transaction containers
   - Transactions are validated instantly by masternodes

3. **Address Format**: TIME addresses use `TIME1` prefix
   - Example: `TIME1abc123def456...`

4. **Fee Structure**: Fixed, low fees due to instant finality
   - No need for dynamic fee estimation
   - Fees are constant at ~0.00001 TIME per KB

5. **Masternode System**: TIME has tiered masternodes (Bronze/Silver/Gold)
   - Not present in Bitcoin
   - See masternode-specific endpoints for more details

---

## Compatibility with Bitcoin Tools

The Bitcoin RPC-compatible API allows TIME Coin to work with many existing Bitcoin tools and libraries:

- **bitcoin-cli**: Can be adapted to work with TIME RPC endpoints
- **Electrum**: With modifications, can connect to TIME nodes
- **Block explorers**: Bitcoin explorer codebases can be adapted
- **Wallets**: Bitcoin wallet libraries can integrate TIME with minimal changes
- **Trading platforms**: Existing Bitcoin integration code can be reused

---

## Rate Limits

**Testnet/Dev Mode:**
- No rate limits

**Production (future):**
- 100 requests/minute per IP for read operations
- 10 requests/minute per IP for write operations (sendrawtransaction)

---

## Support

For questions, issues, or feature requests:
- GitHub: https://github.com/time-coin/time-coin
- Telegram: https://t.me/+CaN6EflYM-83OTY0
- Documentation: https://docs.time-coin.io

---

⏰ TIME is money. Make it accessible.
