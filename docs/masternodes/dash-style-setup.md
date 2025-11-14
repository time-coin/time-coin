# Dash-Style Masternode Setup Guide

## Overview

TIME Coin now supports a Dash-style masternode setup where:
- Collateral is locked by sending it to yourself (creating a UTXO)
- Masternode configuration is stored in a `masternode.conf` file
- Hot wallet manages collateral and activation
- Remote masternode only needs operational key (no wallet required)

This approach provides better security through hot/cold wallet separation.

## Requirements

### Hardware
- CPU: 2+ cores
- RAM: 4GB+
- Storage: 100GB SSD
- Network: 10 Mbps+
- Public IP address

### Software
- Ubuntu 20.04+ (recommended)
- Rust 1.70+
- Open port 24000 (mainnet) or 24100 (testnet)

### Collateral Tiers

| Tier | Collateral | Voting Power | APY Range |
|------|------------|--------------|-----------|
| Community | 1,000 TIME | 1x | 18% |
| Verified | 10,000 TIME | 10x | 24% |
| Professional | 100,000 TIME | 50x | 30% |

## Setup Process

### Step 1: Install TIME Node

```bash
# Clone repository
git clone https://github.com/time-coin/time-coin.git
cd time-coin

# Build
cargo build --release

# Install binaries
sudo cp target/release/timed /usr/local/bin/
sudo cp target/release/time-cli /usr/local/bin/
```

### Step 2: Generate Masternode Key

Generate a new masternode private key:

```bash
time-cli masternode genkey
```

Output example:
```
🔑 Masternode Private Key
━━━━━━━━━━━━━━━━━━━━━━━━━━━━
MN93HaYBVUCYjEMeeH1Y4sBGLALQZE1Yc1K64xiqgX37tGBDQL8Xg

⚠️  Keep this key secret and secure!
Use this key in your masternode.conf file
```

**Important**: Save this key securely! You'll need it for your `masternode.conf`.

### Step 3: Create Collateral Transaction

Send the required collateral amount to yourself to create a UTXO:

```bash
# Send 10,000 TIME to your own address (for Verified tier)
time-cli wallet send --to YOUR_ADDRESS --amount 10000
```

Wait for the transaction to be confirmed (at least 15 confirmations recommended).

### Step 4: Find Collateral Output

List available collateral outputs:

```bash
time-cli masternode outputs --min-conf 15
```

Output example:
```
💰 Available Collateral Outputs
━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  2bcd3c84c84f87ea:0
    Amount: 10000 TIME (Verified)
    Confirmations: 20
```

Note down the transaction ID (txid) and output index (vout).

### Step 5: Create masternode.conf

Add your masternode to the configuration file:

```bash
time-cli masternode add-conf \
    mn1 \
    YOUR_PUBLIC_IP:24000 \
    MN93HaYBVUCYjEMeeH1Y4sBGLALQZE1Yc1K64xiqgX37tGBDQL8Xg \
    2bcd3c84c84f87eaa86e4e56834c92927a07f9e18718810b92e0d0324456a67c \
    0
```

Parameters:
- `mn1`: Your masternode alias (choose any name)
- `YOUR_PUBLIC_IP:24000`: Your masternode's IP and port
- `MN...`: The masternode private key from Step 2
- `2bcd...`: The collateral transaction ID from Step 4
- `0`: The output index (vout) from Step 4

### Step 6: Verify Configuration

List configured masternodes:

```bash
time-cli masternode list-conf
```

Output example:
```
🔧 Configured Masternodes (1 total)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━

1. mn1
   IP:Port: 192.168.1.100:24000
   Collateral: 2bcd3c84c84f87ea:0
```

### Step 7: Start Masternode (Hot Wallet)

From your hot wallet (where you created the collateral), start the masternode:

```bash
# Start a specific masternode
time-cli masternode start-alias mn1

# Or start all configured masternodes
time-cli masternode start-all
```

Output example:
```
✓ Masternode Started
━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Alias: mn1
Successfully broadcast start message
```

### Step 8: Configure Remote Masternode (Cold Wallet)

On your remote masternode server, you only need the masternode operational key (no wallet):

1. Copy the masternode private key to your remote server
2. Create a configuration file (`masternode.toml`):

```toml
[masternode]
# Your masternode private key (from genkey)
privkey = "MN93HaYBVUCYjEMeeH1Y4sBGLALQZE1Yc1K64xiqgX37tGBDQL8Xg"

[network]
listen_addr = "0.0.0.0:24000"
external_addr = "YOUR_PUBLIC_IP:24000"

[rpc]
bind = "0.0.0.0"
port = 24001
```

3. Start the node:

```bash
timed --config masternode.toml
```

### Step 9: Verify Status

Check masternode status:

```bash
# From hot wallet
time-cli masternode info --address YOUR_PUBLIC_IP

# List all masternodes
time-cli masternode list

# Get count by tier
time-cli masternode count
```

## masternode.conf Format

The `masternode.conf` file follows this format:

```
# Comment lines start with #
alias IP:port masternodeprivkey collateral_txid collateral_output_index

# Example:
mn1 192.168.1.100:24000 MN93HaYBVUCYjEMeeH1Y4sBGLALQZE1Yc1K64xiqgX37tGBDQL8Xg 2bcd3c84c84f87eaa86e4e56834c92927a07f9e18718810b92e0d0324456a67c 0
mn2 192.168.1.101:24000 MN83HaYBVUCYjEMeeH1Y4sBGLALQZE1Yc1K64xiqgX37tGBDQL8Xh 3bcd3c84c84f87eaa86e4e56834c92927a07f9e18718810b92e0d0324456a67d 1
```

## CLI Command Reference

### Key Generation
```bash
time-cli masternode genkey
```

### List Collateral Outputs
```bash
time-cli masternode outputs [--min-conf 15]
```

### Configuration Management
```bash
# List configured masternodes
time-cli masternode list-conf [--config masternode.conf]

# Add masternode to config
time-cli masternode add-conf <alias> <ip:port> <privkey> <txid> <vout> [--config masternode.conf]

# Remove masternode from config
time-cli masternode remove-conf <alias> [--config masternode.conf]
```

### Start Masternodes
```bash
# Start specific masternode
time-cli masternode start-alias <alias> [--config masternode.conf]

# Start all masternodes
time-cli masternode start-all [--config masternode.conf]
```

### Status Commands
```bash
# Get masternode info
time-cli masternode info [--address IP]

# List all masternodes
time-cli masternode list

# Get masternode count
time-cli masternode count
```

## Security Best Practices

1. **Keep Private Keys Secret**: Never share your masternode private key or wallet private key
2. **Use Cold Storage**: Keep collateral in a secure hot wallet, run masternode on remote server
3. **Backup Configuration**: Keep secure backups of `masternode.conf`
4. **Firewall**: Only open necessary ports (24000 for P2P, 24001 for RPC if needed)
5. **Regular Updates**: Keep your node software up to date
6. **Monitor Uptime**: Maintain required uptime for your tier

## Troubleshooting

### Masternode not starting
- Verify collateral transaction has enough confirmations (15+ recommended)
- Check that collateral amount meets tier requirements
- Ensure IP address is correct and accessible
- Verify masternode private key is valid

### Cannot find collateral output
- Wait for more confirmations (transaction needs to be confirmed)
- Check that the transaction is sending to your own address
- Use `time-cli wallet list-utxos` to see all UTXOs

### Connection issues
- Verify port 24000 is open and forwarded
- Check firewall rules
- Ensure public IP address is correct
- Verify network connectivity

## Differences from Legacy Setup

| Feature | Legacy | Dash-Style |
|---------|--------|------------|
| Collateral | Single registration transaction | Send-to-self UTXO |
| Configuration | On-chain registration | masternode.conf file |
| Wallet on MN | Required | Not required |
| Hot/Cold Split | No | Yes |
| Key Management | Single key | Separate operational key |
| Activation | Automatic | Manual start message |

## Support

For help with masternode setup:
- Discord: https://discord.gg/timecoin
- Forum: https://forum.timecoin.org
- Documentation: https://docs.timecoin.org

## See Also

- [Collateral Tiers Documentation](./collateral-tiers.md)
- [Legacy Setup Guide](./setup-guide.md)
- [Masternode Economics](./TIERS.md)
