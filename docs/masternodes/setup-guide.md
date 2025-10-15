# Masternode Setup Guide

## Overview

Run a TIME Coin masternode to earn rewards (18-30% APY) and participate in governance.

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
- Open port 9999 (default)

### Collateral

Choose your tier:

| Tier | Collateral | Voting Power | APY Range |
|------|------------|--------------|-----------|
| Bronze | 1,000 TIME | 1x | 18-22% |
| Silver | 5,000 TIME | 5x | 20-24% |
| Gold | 10,000 TIME | 10x | 22-26% |
| Platinum | 50,000 TIME | 50x | 24-28% |
| Diamond | 100,000 TIME | 100x | 26-30% |

## Installation

### 1. Install TIME Node

```bash
# Clone repository
git clone https://github.com/time-coin/time-coin.git
cd time-coin

# Build
cargo build --release

# Install
sudo cp target/release/time-node /usr/local/bin/
sudo cp target/release/time-cli /usr/local/bin/
```

### 2. Create Wallet

```bash
time-cli wallet create --name masternode
time-cli wallet address --name masternode
```

Save your address and backup your wallet!

### 3. Lock Collateral

```bash
# Send collateral to your address
# Then lock it for masternode
time-cli masternode lock-collateral \
    --amount 10000 \
    --address <your_address>
```

### 4. Register Masternode

```bash
time-cli masternode register \
    --ip <your_public_ip> \
    --port 9999 \
    --collateral-tx <tx_hash>
```

### 5. Start Masternode

```bash
time-node masternode start \
    --config config/masternode.toml
```

### 6. Verify Status

```bash
time-cli masternode status
```

## Monitoring

```bash
# Check status
time-cli masternode status

# Check rewards
time-cli masternode rewards

# Check reputation
time-cli masternode reputation
```

## Maintenance

- Monitor uptime (affects reputation)
- Keep node updated
- Participate in governance voting
- Monitor logs: `tail -f ~/.time/logs/masternode.log`

## Troubleshooting

**Masternode offline:**
- Check internet connection
- Verify port 9999 is open
- Check logs for errors

**Low reputation:**
- Improve uptime
- Vote on governance proposals
- Avoid missed blocks

## Support

- Forum: https://forum.time-coin.io
- Telegram: https://t.co/ISNmAW8gMV
- Discord: https://discord.gg/timecoin
