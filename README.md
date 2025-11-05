# TIME Coin ⏰

A next-generation cryptocurrency featuring 24-hour time blocks, instant transaction finality, and masternode-powered consensus.

## Key features

- ⚡ Instant finality — sub-3 second transaction confirmation
- 🕐 24-hour blocks — daily checkpoints for immutable settlement
- 🔗 Masternode network — BFT-style consensus for instant validation
- 💰 Tiered staking — competitive APY across collateral tiers
- 🛡️ Community treasury — on-chain governance and funding
- 🚀 Fair launch — no pre-mine, purchase-based minting model

## Architecture overview

TIME separates transaction finality from block production:

1. Instant transactions validated by masternodes in real time
2. Daily blocks used as periodic immutable checkpoints
3. BFT consensus: validators must reach a 67% agreement threshold
4. Masternode rewards: block rewards distributed to masternodes by tier
5. Treasury funding: a portion of each block funds ecosystem development

## Masternode tiers

| Tier   | Collateral   | APY   | Voting power |
|--------|--------------:|------:|-------------:|
| Bronze | 1,000 TIME    | 18%   | 1x |
| Silver | 10,000 TIME   | 19.8% | 5x |
| Gold   | 100,000 TIME  | 22.5% | 10x |

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
cargo run --bin time-node --release
```

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

### Step 2 — Install and configure the masternode

Example (from your home directory):
```bash
cd ~/time-coin
chmod +x scripts/setup-masternode.sh
sudo ./scripts/setup-masternode.sh
```

Typical post-install actions performed by the installer:
- Build in release mode and install `time-node`
- Create default configuration files (e.g., config/testnet.toml and genesis file)
- Set up a systemd service and management scripts
- Start the masternode service

### Post-installation management

```bash
# Check service status
sudo systemctl status time-node

# Follow logs
sudo journalctl -u time-node -f

# Restart the service
sudo systemctl restart time-node
```

### Updating your masternode

```bash
cd ~/time-coin
git pull origin main
sudo bash ./scripts/setup-masternode.sh
sudo systemctl restart time-node
```

### Network ports

TIME Coin uses themed ports for P2P and RPC access:

| Network | P2P port | RPC port |
|---------|---------:|---------:|
| Testnet | 24100    | 24101    |
| Mainnet | 24000    | 24001    |

Defaults can be overridden in your configuration file (e.g., config/testnet.toml). The setup scripts and node will respect values set in your config.

If using a firewall, allow the P2P port (example for testnet):
```bash
sudo ufw allow 24100/tcp comment 'TIME Coin Testnet P2P'
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
time-node -c ~/time-coin-node/config/testnet.toml
```

The node expands `$HOME` and `~` in config paths.

## Documentation

- docs/whitepaper-technical.md — Technical whitepaper
- docs/masternodes/setup-guide.md — Masternode setup guide
- docs/api/README.md — API documentation
- docs/architecture/README.md — Architecture overview

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