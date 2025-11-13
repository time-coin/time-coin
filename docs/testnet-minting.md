# Testnet Minting Guide

## Overview

TIME Coin provides a testnet-only minting mechanism to create coins for development and testing purposes. This feature allows developers and testers to mint coins without affecting mainnet supply or distribution.

## Safety Features

🔒 **Critical Safety Measures:**
- ✅ Only works in testnet mode
- ✅ Automatically rejects all mainnet requests
- ✅ Network type checked on every request
- ✅ No possibility of accidental mainnet minting

## Usage

### Method 1: CLI Command (Recommended)

The easiest way to mint testnet coins is using the CLI:

```bash
# Basic usage
timed testnet-mint --address <ADDRESS> --amount <AMOUNT_IN_TIME>

# With optional reason
timed testnet-mint \
  --address wallet_addr_123 \
  --amount 1000 \
  --reason "Testing transaction flow"

# Custom RPC endpoint
timed testnet-mint \
  --address wallet_addr_123 \
  --amount 500.5 \
  --rpc-url http://192.168.1.100:24101
```

**Parameters:**
- `--address` / `-a`: Wallet address to receive the minted coins (required)
- `--amount` / `-a`: Amount to mint in TIME (e.g., 100.5 means 100.5 TIME)
- `--reason` / `-r`: Optional description for why coins are being minted
- `--rpc-url`: RPC endpoint (default: http://127.0.0.1:24101)

**Example Output:**
```
🪙 TIME Coin Testnet Minter
═══════════════════════════════════

📋 Mint Request:
   Address: wallet_addr_123
   Amount: 1000 TIME (100000000000 satoshis)
   Reason: Testing transaction flow
   RPC: http://127.0.0.1:24101

📡 Sending mint request...

✅ SUCCESS!
═══════════════════════════════════
Successfully minted 100000000000 satoshis (1000 TIME) to address wallet_addr_123. 
Transaction will be included in the next block.

Transaction Details:
  • TX ID: testnet_mint_1699564832123456789
  • Amount: 100000000000 satoshis (1000 TIME)
  • Recipient: wallet_addr_123

💡 The minted coins will appear in the next block.
```

### Method 2: API Endpoint

You can also mint coins directly via the API:

#### POST /testnet/mint

Mint new coins in testnet mode.

**Request:**
```json
{
  "address": "wallet_addr_123",
  "amount": 100000000000,
  "reason": "Testing transaction flow"
}
```

**Response:**
```json
{
  "success": true,
  "message": "Successfully minted 100000000000 satoshis (1000 TIME) to address wallet_addr_123. Transaction will be included in the next block.",
  "txid": "testnet_mint_1699564832123456789",
  "amount": 100000000000,
  "address": "wallet_addr_123"
}
```

**Error Response (Mainnet):**
```json
{
  "error": "Minting is only allowed in testnet mode. Mainnet minting is prohibited."
}
```

#### GET /testnet/mint/info

Get information about testnet minting capabilities.

**Response (Testnet):**
```json
{
  "network": "testnet",
  "minting_enabled": true,
  "message": "Testnet minting is enabled. Use POST /testnet/mint to create test coins."
}
```

**Response (Mainnet):**
```json
{
  "network": "mainnet",
  "minting_enabled": false,
  "message": "Minting is disabled in mainnet mode for security reasons."
}
```

### Method 3: cURL

You can use cURL to mint coins:

```bash
# Mint 1000 TIME to an address
curl -X POST http://127.0.0.1:24101/testnet/mint \
  -H "Content-Type: application/json" \
  -d '{
    "address": "wallet_addr_123",
    "amount": 100000000000,
    "reason": "Testing"
  }'

# Check if minting is enabled
curl http://127.0.0.1:24101/testnet/mint/info
```

## How It Works

1. **Minting Request**: A request is made with the target address and amount
2. **Safety Check**: The system verifies it's running in testnet mode
3. **Transaction Creation**: A special coinbase-style transaction is created with no inputs (minting new coins)
4. **Mempool Addition**: The minting transaction is added to the mempool
5. **Network Broadcast**: The transaction is broadcast to other nodes
6. **Block Inclusion**: The next block will include the minting transaction
7. **Balance Update**: After the block is finalized, the address will have the minted coins

## Amount Format

Amounts are specified in **satoshis** (the smallest unit):
- 1 TIME = 100,000,000 satoshis
- 0.5 TIME = 50,000,000 satoshis
- 10 TIME = 1,000,000,000 satoshis

When using the CLI, you can specify amounts in TIME (e.g., 100.5), and it will be automatically converted to satoshis.

## Common Use Cases

### 1. Testing Wallet Functionality
```bash
# Mint coins to test sending transactions
timed testnet-mint --address test_wallet_1 --amount 100 --reason "Wallet testing"
```

### 2. Testing Masternode Registration
```bash
# Mint collateral for Bronze tier (1,000 TIME)
timed testnet-mint --address masternode_wallet --amount 1000 --reason "Bronze masternode setup"

# Mint collateral for Silver tier (10,000 TIME)
timed testnet-mint --address masternode_wallet --amount 10000 --reason "Silver masternode setup"

# Mint collateral for Gold tier (100,000 TIME)
timed testnet-mint --address masternode_wallet --amount 100000 --reason "Gold masternode setup"
```

### 3. Testing Transaction Fees
```bash
# Mint coins to multiple addresses to test fee calculations
timed testnet-mint --address sender_wallet --amount 100 --reason "Fee testing"
timed testnet-mint --address recipient_wallet --amount 50 --reason "Fee testing"
```

### 4. Stress Testing
```bash
# Mint large amounts for stress testing
timed testnet-mint --address stress_test_addr --amount 1000000 --reason "Network stress test"
```

## Security Considerations

### Testnet Only
The minting functionality is **strictly limited to testnet**. The code includes multiple safety checks:

1. **Network Type Verification**: Every request checks `state.network != "testnet"`
2. **Automatic Rejection**: Mainnet requests are immediately rejected with an error
3. **No Bypass Possible**: There is no way to override or bypass this check

### Transaction Validation
Minted transactions:
- Are treated like coinbase transactions (no inputs)
- Go through standard mempool validation
- Are included in blocks via normal consensus
- Are broadcast to all nodes on the network

### Best Practices
- ✅ Use descriptive reasons for audit trails
- ✅ Mint only what you need for testing
- ✅ Keep track of minted amounts for test case consistency
- ❌ Don't abuse minting - it's for testing, not for fun
- ❌ Don't try to use testnet coins on mainnet (they're incompatible)

## Troubleshooting

### Error: "Minting is only allowed in testnet mode"
**Cause**: You're trying to mint on mainnet  
**Solution**: This is expected behavior. Minting is not allowed on mainnet. Use testnet for testing.

### Error: "Connection failed"
**Cause**: The TIME Coin node is not running or not accessible  
**Solution**: 
1. Check if the node is running: `systemctl status timed`
2. Verify the RPC port is open: `netstat -tlnp | grep 24101`
3. Check firewall rules if accessing remotely

### Error: "Amount must be greater than 0"
**Cause**: You specified an amount of 0  
**Solution**: Specify a positive amount (minimum 1 satoshi)

### Minted coins not appearing
**Cause**: The transaction is still in the mempool  
**Solution**: Wait for the next block to be produced (up to 24 hours for TIME Coin, but catch-up blocks are created faster). Check the mempool status at `/mempool/status`.

## API Reference

### Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/testnet/mint` | Mint new coins (testnet only) |
| GET | `/testnet/mint/info` | Get minting information |

### Request Schema

```typescript
interface MintCoinsRequest {
  address: string;      // Wallet address to receive coins
  amount: number;       // Amount in satoshis (1 TIME = 100,000,000)
  reason?: string;      // Optional description
}
```

### Response Schema

```typescript
interface MintCoinsResponse {
  success: boolean;     // Whether the minting succeeded
  message: string;      // Human-readable message
  txid: string;        // Transaction ID
  amount: number;      // Amount minted in satoshis
  address: string;     // Recipient address
}
```

## Examples

### Full Example: Setting Up a Test Environment

```bash
#!/bin/bash
# setup-testnet-environment.sh

# Mint coins for testing wallets
echo "Setting up testnet environment..."

# Create master wallet
timed testnet-mint --address master_wallet --amount 100000 --reason "Master test wallet"

# Create user wallets for testing
timed testnet-mint --address user1_wallet --amount 1000 --reason "User 1 testing"
timed testnet-mint --address user2_wallet --amount 1000 --reason "User 2 testing"
timed testnet-mint --address user3_wallet --amount 1000 --reason "User 3 testing"

# Create masternode wallets
timed testnet-mint --address mn_bronze_1 --amount 1000 --reason "Bronze MN 1"
timed testnet-mint --address mn_bronze_2 --amount 1000 --reason "Bronze MN 2"
timed testnet-mint --address mn_silver_1 --amount 10000 --reason "Silver MN 1"
timed testnet-mint --address mn_gold_1 --amount 100000 --reason "Gold MN 1"

echo "Testnet environment setup complete!"
echo "Wait for the next block to see the minted coins."
```

## Related Documentation

- [Block Rewards Guide](../block-rewards.md) - Understanding block reward distribution
- [Transaction Fees Guide](../transaction-fees.md) - How transaction fees work
- [Masternode Setup](../masternodes/setup-guide.md) - Setting up masternodes
- [API Documentation](../api/README.md) - Complete API reference

## Support

If you encounter issues with testnet minting:
1. Check this documentation
2. Verify you're running in testnet mode
3. Check the node logs: `journalctl -u timed -f`
4. Ask in the community Telegram: https://t.me/+CaN6EflYM-83OTY0
