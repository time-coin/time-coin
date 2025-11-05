```markdown
# TIME Coin ⏰

A next-generation cryptocurrency featuring 24-hour time blocks, instant transaction finality, and masternode-powered consensus.

## Key Features

- ⚡ **Instant Finality**: <3 second transaction confirmation
- 🕐 **24-Hour Blocks**: Daily settlement for immutable checkpoints
- 🔗 **Masternode Network**: Byzantine Fault Tolerant consensus
- 💰 **Tiered Staking**: 18-30% APY across 5 collateral tiers
- 🛡️ **Community Treasury**: Decentralized governance
- 🚀 **Fair Launch**: No pre-mine, purchase-based minting

## Architecture

TIME Coin separates transaction finality from block production:

1. **Instant Transactions**: Validated by masternodes in real-time
2. **Daily Blocks**: Periodic checkpoints every 24 hours
3. **BFT Consensus**: 67% validator agreement required
4. **Masternode Rewards**: 95 TIME per block distributed based on tier
5. **Treasury Funding**: 5 TIME per block for ecosystem development

## Masternode Tiers

| Tier | Collateral | APY | Voting Power |
|------|-----------:|-----:|--------------|
| Bronze | 1,000 TIME | 18% | 1x |
| Silver | 10,000 TIME | 19.8% | 5x |
| Gold   | 100,000 TIME | 22.5% | 10x |

## Project Structure

```
time-coin/
├── core/           # Core blockchain logic
├── masternode/     # Masternode management
├── treasury/       # Community treasury
├── network/        # P2P networking
├── purchase/       # Fiat/crypto purchases
├── wallet/         # Wallet implementation
├── api/            # API server
├── storage/        # Database layer
├── crypto/         # Cryptographic primitives
└── cli/            # Command-line interface
```

## Getting Started

### Prerequisites

- Rust 1.75 or higher
- Git
- Ubuntu 20.04+ (for masternode deployment)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/time-coin/time-coin.git
cd time-coin

# Build all components
cargo build --release

# Run tests
cargo test --all

# Run a node
cargo run --bin time-node --release
```

## Running a Masternode

### Quick Installation (Ubuntu)

Setting up a TIME Coin masternode is a two-step process:

#### Step 1: System Setup (One-time)

Prepare your Ubuntu server with all dependencies:

```bash
# download helper script from repository
curl -O https://raw.githubusercontent.com/time-coin/time-coin/main/scripts/setup/setup-testnet-node.sh
chmod +x setup-testnet-node.sh
sudo ./setup-testnet-node.sh
```

This installs:
- Build tools and dependencies
- Rust toolchain
- TIME Coin repository

#### Step 2: Install Masternode

Build and deploy the masternode:

```bash
cd ~/time-coin
# installer is located under scripts/setup/
chmod +x scripts/setup-masternode.sh
sudo ./scripts/setup-masternode.sh
```

This will:
- Build TIME Coin in release mode
- Install the `time-node` binary
- Create configuration (ports 24100/24101 for testnet)
- Set up systemd service
- Start your masternode

### Post-Installation

Monitor your masternode:

```bash
# Check status
sudo systemctl status time-node

# View live logs
sudo journalctl -u time-node -f

# Restart if needed
sudo systemctl restart time-node
```

### Updating Your Masternode

To update to the latest version:

```bash
cd ~/time-coin
git pull origin main
sudo bash ./scripts/setup-masternode.sh
systemctl restart time-node
```

### Network Ports

TIME Coin uses 24-hour themed ports:

| Network | P2P Port | RPC Port |
|---------|---------:|---------:|
| Testnet | 24100 | 24101 |
| Mainnet | 24000 | 24001 |

Note: the canonical default ports for Testnet are P2P=24100 and RPC=24101. These defaults can be overridden in config/testnet.toml — the setup scripts and node will respect the config values if changed.

If using a firewall, allow the P2P port:

```bash
sudo ufw allow 24100/tcp comment 'TIME Coin Testnet P2P'
```

## Documentation

- [Technical Whitepaper](docs/whitepaper-technical.md)
- [Masternode Setup Guide](docs/masternodes/setup-guide.md)
- [API Documentation](docs/api/README.md)
- [Architecture Overview](docs/architecture/README.md)

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Community

- Website: https://time-coin.io
- Telegram: https://t.me/+CaN6EflYM-83OTY0
- Twitter: @TIMEcoin515010
- GitHub: https://github.com/time-coin/time-coin

## License

MIT License - see [LICENSE](LICENSE) for details

## Security

See [SECURITY.md](SECURITY.md) for reporting security vulnerabilities.

---

**⏰ TIME is money. Make it accessible.**
```