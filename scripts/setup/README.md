# TIME Coin Node Setup Scripts

## Testnet Node Setup (Ubuntu/Debian)

Quick setup script for Ubuntu 20.04+ or Debian 11+.

### Download and Run
```bash
wget https://raw.githubusercontent.com/time-coin/time-coin/main/scripts/setup/setup-testnet-node.sh
chmod +x setup-testnet-node.sh
./setup-testnet-node.sh
```

### What It Does

- Installs dependencies (Rust, build tools)
- Builds TIME node from source
- Configures testnet masternode (any tier)
- Sets up systemd service
- Creates management commands
- Configures firewall

### Requirements

- Ubuntu 20.04+ or Debian 11+
- 2+ CPU cores
- 4+ GB RAM
- 50+ GB disk space
- Root/sudo access

### After Installation

Manage your node:
```bash
~/time-coin-node/manage.sh start      # Start node
~/time-coin-node/manage.sh status     # Check status
~/time-coin-node/manage.sh logs       # View logs
```
