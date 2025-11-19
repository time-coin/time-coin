# TIME Coin Masternode

This directory contains the TIME Coin masternode implementation with a 3-tier collateral system.

## Quick Start

Most operators should use the **simple setup** with the installation script:

```bash
sudo ./install-masternode.sh
```

This creates a masternode with:
- Simple private key in `masternode.conf` for signing
- Rewards sent to your hot wallet (wallet-gui) address
- No funds stored on the masternode server

See [install-masternode.sh](../install-masternode.sh) for details.

## Wallet Options

### 1. Simple Setup (Recommended)

**What you get:**
- Private key in `masternode.conf` for signing messages
- Rewards sent to your hot wallet address
- Masternode never holds funds

**Security:** ✅ Best - no funds on server

This is created automatically by the installation script.

### 2. Full HD Wallet (Optional)

**What you get:**
- BIP-39 mnemonic-based HD wallet
- Multiple address derivation
- Funds managed on the masternode

**Security:** ⚠️ Lower - funds stored on public server

Only use this if you need advanced HD wallet features. See [WALLET.md](WALLET.md) for details.

## Masternode Tiers

- **Community**: 1,000 TIME, 18% APY, 90% uptime
- **Verified**: 10,000 TIME, 24% APY, 95% uptime  
- **Professional**: 100,000 TIME, 30% APY, 98% uptime

## Configuration

### masternode.conf Format

```
# Format: alias IP:port masternodeprivkey collateral_txid collateral_output_index
mn1 192.168.1.100:24000 93HaYBVUCYjEMeeH1Y4sBGLALQZE1Yc1K64xiqgX37tGBDQL8Xg 2bcd3c84c84f87eaa86e4e56834c92927a07f9e18718810b92e0d0324456a67c 0
```

### testnet.toml

The installation script creates this for you:

```toml
[node]
network = "testnet"
mode = "masternode"

[masternode]
enabled = true
address = "TIME0your_hot_wallet_address"  # From wallet-gui
```

## Documentation

- [WALLET.md](WALLET.md) - Optional full HD wallet (advanced)
- [SECURITY_ANALYSIS.md](SECURITY_ANALYSIS.md) - Security model
- [SLASHING.md](SLASHING.md) - Slashing rules
- [VIOLATION_DETECTION.md](VIOLATION_DETECTION.md) - Violation detection

## Development

Build the masternode:

```bash
cargo build -p time-masternode --release
```

Run tests:

```bash
cargo test -p time-masternode
```

## Support

For issues or questions, see the main [README.md](../README.md).
