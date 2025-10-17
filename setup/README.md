# TIME Coin Node Setup Scripts

Setup scripts for deploying TIME Coin nodes on various platforms.

## Testnet Node

### Ubuntu/Debian (20.04+)
```bash
wget https://raw.githubusercontent.com/time-coin/time-coin/main/scripts/setup/setup-testnet-node.sh
chmod +x setup-testnet-node.sh
./setup-testnet-node.sh
```

**What it does:**
- Installs dependencies (Rust, build tools)
- Builds TIME node from source
- Configures testnet masternode
- Sets up systemd service
- Creates management commands

**Requirements:**
- Ubuntu 20.04+ or Debian 11+
- 2+ CPU cores
- 4+ GB RAM
- 50+ GB disk space

## Mainnet Node (Coming Soon)

Mainnet setup will require actual TIME collateral:
- Community: 1,000 TIME
- Verified: 10,000 TIME
- Professional: 100,000 TIME
