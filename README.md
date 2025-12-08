# TIME Coin ⏰

[![CI](https://github.com/time-coin/time-coin/actions/workflows/ci.yml/badge.svg)](https://github.com/time-coin/time-coin/actions/workflows/ci.yml)
[![GitHub language count](https://img.shields.io/github/languages/top/time-coin/time-coin)](https://github.com/time-coin/time-coin)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/time-coin/time-coin/blob/main/LICENSE)

A next-generation cryptocurrency featuring 24-hour time blocks, instant transaction finality, and masternode-powered consensus.

**Powered by the [TIME Coin Protocol](TIME_COIN_PROTOCOL.md)** - UTXO-based instant finality for real-world adoption.

## Key features

- ⚡ **Instant finality** — sub-3 second transaction confirmation via TIME Coin Protocol
- ⏱️ **Time-based validation** — blocks validated against real-world time to prevent manipulation
- 🔐 **Proof-of-Time security** — VDF-based rollback protection (testnet: 10-min blocks, mainnet: 1-hour blocks)
- 🔷 **Deterministic consensus** — all nodes generate identical blocks, no single point of failure
- 🏦 **UTXO model** — Bitcoin-compatible accounting with instant finality
- 🔒 **Fork-resistant** — objective time-based chain selection prevents 51% rollback attacks
- 💰 **Tiered staking** — competitive APY across collateral tiers
- 🛡️ **Community treasury** — on-chain governance and funding
- 🚀 **Fair launch** — no pre-mine, purchase-based minting model

## The TIME Coin Protocol

TIME Coin's unique protocol combines Bitcoin's proven UTXO model with instant finality:

```
Transaction → UTXO Locked → Masternode Voting → 67%+ Consensus 
    → INSTANT FINALITY (<3 sec) → Block Inclusion → Confirmed
```

**Read more**: [TIME Coin Protocol Documentation](TIME_COIN_PROTOCOL.md)

## Architecture overview

TIME separates transaction finality from block production with time-based validation:

1. **Instant transactions** validated by masternodes in real time via TIME Coin Protocol
2. **UTXO state tracking** prevents double-spends through immediate locking
3. **Time-based validation** ensures blocks match elapsed time since genesis
   - Expected block height = (current_time - genesis_time) / block_time
   - Testnet: 10-minute blocks (144 blocks/day expected)
   - Mainnet: 1-hour blocks (24 blocks/day expected)
   - Prevents nodes from claiming future blocks by manipulating system clock
   - Auto-detects when node needs catch-up sync
4. **Proof-of-Time blocks** provide objective immutable checkpoints
   - Testnet: 10-minute blocks with 2-minute VDF locks
   - Mainnet: 1-hour blocks with 5-minute VDF locks
   - Verifiable Delay Functions (VDF) prevent instant rollback attacks
5. **Deterministic consensus**: all nodes independently generate identical blocks
   - No leader election or single point of failure
   - All nodes compare blocks with peers (<10 seconds)
   - 67% agreement threshold for instant finalization
   - Automatic reconciliation if differences detected
6. **Fork resolution**: cumulative VDF time + block height determines the canonical chain
   - Even 51% malicious masternodes cannot instant-rewrite history
   - Must invest actual time to create alternative chains
   - Cannot exceed time-based maximum block height
7. **Masternode rewards**: block rewards distributed to masternodes by tier (with uptime requirements)
8. **Treasury funding**: a portion of each block funds ecosystem development

## Masternode tiers

| Tier   | Collateral   | Reward Weight | Voting Power | Est. APY* |
|--------|--------------:|--------------:|-------------:|----------:|
| Free   | 0 TIME        | 1x            | 0x           | N/A       |
| Bronze | 1,000 TIME    | 10x           | 1x           | 35-180%   |
| Silver | 10,000 TIME   | 100x          | 10x          | 35-180%   |
| Gold   | 100,000 TIME  | 1000x         | 100x         | 35-180%   |

*APY varies based on network size. Smaller networks = higher APY.

## Project structure

```
time-coin/
├── api/            # API server
├── cli/            # Command-line interface
├── core/           # Core blockchain logic
├── crypto/         # Cryptographic primitives
├── masternode/     # Masternode management
├── network/        # P2P networking
├── purchase/       # Fiat/crypto purchases
├── storage/        # Database layer
├── treasury/       # Community treasury
├── wallet/         # Wallet implementation
├── wallet-gui/     # GUI hot wallet application
└── docs/           # Documentation and guides
```

## Getting started

### Prerequisites

- Rust 1.75 or higher
- Git
- Ubuntu 20.04+ (recommended for masternode deployments) or Debian 11+

### Build from source

```bash
# Clone the repository
git clone https://github.com/time-coin/time-coin.git
cd time-coin

# Build all components
cargo build --release

# Run tests
cargo test --all

# Run a node (development)
cargo run --bin timed --release

# Run the GUI wallet
cargo run --bin wallet-gui --release
```

## Usage

### Starting the Node (Daemon)
```bash
# Start the TIME Coin daemon
timed --config config/testnet.toml

# Or with default config
timed
```

### Using the CLI
All user operations should use `time-cli`:
```bash
# Check node status
time-cli status

# Validate blockchain
time-cli validate-chain --verbose

# Create treasury grant proposal (requires masternode voting)
time-cli proposal create --address TIME1... --amount 1000 --reason "Development funding"

# Vote on proposal (masternodes only)
time-cli proposal vote --id proposal_xxx --approve

# RPC operations
time-cli rpc get-blockchain-info
```

The daemon (`timed`) runs in the background and exposes RPC endpoints.
The CLI tool (`time-cli`) connects to those RPC endpoints to perform operations.

## GUI Wallet

TIME Coin includes a cross-platform GUI hot wallet with Bitcoin-style wallet.dat file support:

```bash
# Run the GUI wallet
cargo run --bin wallet-gui --release
```

See [wallet-gui/README.md](wallet-gui/README.md) for detailed documentation on:
- Wallet creation and key management
- Sending and receiving TIME coins
- Backup and restore procedures
- Security considerations

## Running a masternode

Setting up a TIME Coin masternode is a two-step process: system setup and masternode installation.

### Step 1 — System setup (one-time)

There are two convenient entry points for the documented installer:

- The canonical installer: scripts/setup/setup-testnet-node.sh
- A small wrapper used for docs compatibility: scripts/dev-setup.sh (it invokes the canonical script if present)

Download and run the installer (choose one):

Using the canonical path (recommended when present in the repo):
```bash
bash scripts/setup/setup-testnet-node.sh
```

Using the convenience wrapper (this will run the canonical installer if available):
```bash
bash scripts/dev-setup.sh
```

Or fetch the canonical installer from the raw GitHub URL:
```bash
wget https://raw.githubusercontent.com/time-coin/time-coin/main/scripts/setup/setup-testnet-node.sh
chmod +x setup-testnet-node.sh
sudo ./setup-testnet-node.sh
```

What these scripts typically do:
- Install build tools and dependencies, including the Rust toolchain
- Build the TIME node from source
- Configure a testnet masternode and systemd service
- Configure firewall rules and management helpers

> **⚠️ Important**: Do NOT use `apt install cargo` or `apt install rust`. The scripts install Rust via `rustup`, which provides the latest stable version and easy updates. Ubuntu's apt packages are outdated and incompatible with TIME Coin's dependencies.

### Step 2 — Install and configure the masternode

Example (from your home directory):
```bash
cd ~/time-coin
chmod +x scripts/setup-masternode.sh
sudo ./scripts/setup-masternode.sh
```

Typical post-install actions performed by the installer:
- Build in release mode and install `timed`
- Create default configuration files (e.g., config/testnet.toml and genesis file)
- Set up a systemd service and management scripts
- Start the masternode service

### Post-installation management

```bash
# Check service status
sudo systemctl status timed

# Follow logs
sudo journalctl -u timed -f

# Restart the service
sudo systemctl restart timed
```

### Updating your masternode

```bash
cd ~/time-coin
git pull origin main
sudo bash ./scripts/setup-masternode.sh
sudo systemctl restart timed
```

### Network ports

TIME Coin uses themed ports for P2P, RPC, and WebSocket access:

| Network | P2P port | RPC port | WebSocket port |
|---------|----------|----------|----------------|
| Testnet | 24100    | 24101    | 24102          |
| Mainnet | 24000    | 24001    | 24002          |

- **P2P port**: Node-to-node blockchain communication
- **RPC port**: HTTP API for wallet/client requests  
- **WebSocket port**: TIME Coin Protocol real-time UTXO state notifications

Defaults can be overridden in your configuration file (e.g., config/testnet.toml). The setup scripts and node will respect values set in your config.

If using a firewall, allow the P2P and WebSocket ports (example for testnet):
```bash
sudo ufw allow 24100/tcp comment 'TIME Coin Testnet P2P'
sudo ufw allow 24102/tcp comment 'TIME Coin Protocol WebSocket'
```

## Configuration

For testnet, copy the example config and genesis files, then edit paths as needed:

```bash
mkdir -p ~/time-coin-node/config
cp config/testnet.toml ~/time-coin-node/config/
cp config/genesis-testnet.json ~/time-coin-node/config/
# edit testnet.toml to point to the correct genesis path (use $HOME or absolute path)
```

Start a node with your config:
```bash
timed -c ~/time-coin-node/config/testnet.toml
```

The node expands `$HOME` and `~` in config paths.

## Documentation

Comprehensive documentation is available in the `/docs` directory:

**Core Protocol:**
- **[PROTOCOL_INDEX.md](docs/PROTOCOL_INDEX.md)** - Complete documentation index
- **[TIME_COIN_UTXO_PROTOCOL_SUMMARY.md](docs/TIME_COIN_UTXO_PROTOCOL_SUMMARY.md)** - UTXO protocol summary
- **[TIME_BASED_VALIDATION.md](docs/TIME_BASED_VALIDATION.md)** - Time-based validation system ⭐ **NEW**
- **[PROOF_OF_TIME.md](docs/PROOF_OF_TIME.md)** - VDF Proof-of-Time system
- **[TIME-COIN-TECHNICAL-SPECIFICATION.md](docs/TIME-COIN-TECHNICAL-SPECIFICATION.md)** - Technical specification v3.0

**For Developers:**
- [Building from Source](docs/BUILDING.md)
- [Running a Masternode](docs/RUNNING_MASTERNODE.md)
- [Wallet Integration](docs/WALLET_PROTOCOL_INTEGRATION.md)
- [API Documentation](docs/api/README.md)
- [Architecture Overview](docs/architecture/README.md)

**Academic:**
- [Technical Whitepaper](docs/whitepaper/Technical-Whitepaper-v3.0.md)
- [Security Whitepaper](docs/whitepaper/Security-Whitepaper-V3.0.md)

## Contributing

We welcome contributions. Please see CONTRIBUTING.md in the project root for guidelines.

## Community

- Website: https://time-coin.io
- Telegram: https://t.me/+CaN6EflYM-83OTY0
- Twitter: [@TIMEcoin515010](https://twitter.com/TIMEcoin515010)
- GitHub: https://github.com/time-coin/time-coin

## License

MIT License — see LICENSE for details.

## Security

See SECURITY.md for reporting vulnerabilities.

---

⏰ TIME is money. Make it accessible.