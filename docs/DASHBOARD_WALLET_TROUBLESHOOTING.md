# Dashboard Wallet Balance Troubleshooting

## Issue

The TIME Coin dashboard shows:
```
💰 Wallet Balance:
   ⚠️  Unable to fetch wallet balance
```

## Root Cause

The dashboard tries to detect the node's wallet address using:
1. `WALLET_ADDRESS` environment variable
2. API endpoint: `/node/wallet` (redirects to `/masternode/wallet`)
3. API endpoint: `/masternode/wallet`

The issue can occur when:
- The node is not running
- The API endpoint returns an empty wallet address
- The redirect is not followed properly
- Network connectivity issues between dashboard and node

## Solution Applied

### Code Changes

Updated `tools/masternode-dashboard/src/main.rs`:

1. **Improved address detection logic**:
   - Check environment variable for non-empty value
   - Try `/masternode/wallet` endpoint first (direct)
   - Fallback to `/node/wallet` (with redirect)
   - Check for empty strings in responses

2. **Better error reporting**:
   - Show which detection methods are being tried
   - Display helpful message when wallet not found
   - Continue running dashboard even without wallet address

### How to Use

#### Method 1: Set Environment Variable (Recommended)
```bash
# Linux/Mac
export WALLET_ADDRESS="TIME0your_wallet_address_here"
./target/release/time-dashboard

# Windows PowerShell
$env:WALLET_ADDRESS = "TIME0your_wallet_address_here"
.\target\release\time-dashboard.exe
```

#### Method 2: Let Dashboard Auto-Detect

The dashboard will automatically detect the wallet address from the running node if:
- The node is running on `http://localhost:24101`
- The node has loaded its wallet successfully
- The API endpoints are responding

#### Method 3: Check Node Wallet

Query the node directly to see its wallet address:
```bash
# Check if endpoint responds
curl http://localhost:24101/masternode/wallet

# Expected response:
{
  "wallet_address": "TIME0abc123...",
  "status": "success"
}
```

## Verification Steps

1. **Check if node is running**:
   ```bash
   curl http://localhost:24101/health
   ```

2. **Check wallet endpoint**:
   ```bash
   curl http://localhost:24101/masternode/wallet
   ```

3. **Check node wallet file**:
   ```bash
   # Linux/Mac
   ls -la ~/.timecoin/data/wallets/

   # Windows
   dir %APPDATA%\timecoin\data\wallets\
   ```

4. **Run dashboard**:
   ```bash
   ./target/release/time-dashboard
   ```

## Expected Behavior

### Success Case
```
✓ Found masternode wallet: TIME0abc123def456...

[Dashboard shows wallet balance]
```

### Warning Case (No wallet found)
```
⚠️  No wallet address found. Checking:
   1. WALLET_ADDRESS environment variable
   2. http://localhost:24101/masternode/wallet endpoint
   3. http://localhost:24101/node/wallet endpoint

ℹ️  Wallet balance will not be displayed.

[Dashboard continues without wallet section]
```

## Common Issues

### Issue: "Unable to fetch wallet balance"

**Cause**: Node not running or API not responding

**Fix**: 
```bash
# Start the node
cargo run --release --bin timed

# Or if installed as service
sudo systemctl start timed
```

### Issue: Empty wallet address in API response

**Cause**: Node wallet not initialized

**Fix**: Check node startup logs for wallet loading errors:
```bash
# Linux
sudo journalctl -u timed -f

# Or check log file
tail -f ~/.timecoin/logs/node.log
```

### Issue: Connection refused

**Cause**: Node API not bound to localhost:24101

**Fix**: Check node configuration (`~/.timecoin/config/testnet.toml`):
```toml
[rpc]
enabled = true
bind = "0.0.0.0"
port = 24101
```

## API Endpoint Reference

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/masternode/wallet` | GET | Get node wallet address |
| `/node/wallet` | GET | Legacy endpoint (redirects) |
| `/balance/{address}` | GET | Get balance for address |

## Debug Mode

To see more detailed information, modify the dashboard code to print API responses:

```rust
if let Ok(response) = reqwest::blocking::get(&wallet_url) {
    println!("API Response: {:?}", response);
    if let Ok(json) = response.json::<serde_json::Value>() {
        println!("JSON: {}", json);
        // ... rest of code
    }
}
```

## Support

If issues persist:
1. Check node logs: `~/.timecoin/logs/node.log`
2. Verify API is responding: `curl http://localhost:24101/health`
3. Check wallet file exists: `~/.timecoin/data/wallets/node.json`
4. Open an issue on GitHub with logs and error messages
