# TIME Coin Wallet ⏰

[![CI](https://github.com/time-coin/time-coin/actions/workflows/ci.yml/badge.svg)](https://github.com/time-coin/time-coin/actions/workflows/ci.yml)
[![License: BUSL-1.1](https://img.shields.io/badge/License-BUSL--1.1-blue.svg)](https://github.com/time-coin/time-coin/blob/main/LICENSE)

A cross-platform GUI wallet for the TIME Coin network. Built with Rust and [egui](https://github.com/emilk/egui).
If you are looking for the masternode repository go to [time-masternode](https://github.com/time-coin/time-masternode.git)

## Features

- 🔑 **HD wallet** — BIP39 mnemonic seed (12–24 words) with BIP44 key derivation (Ed25519)
- 💸 **Send & receive** — UTXO-based transactions with automatic coin selection and change outputs
- 📇 **Address book** — Save recipient names, auto-fill from contacts, search/edit/delete
- 📷 **QR code scanner** — Scan recipient addresses via webcam with audible confirmation
- 📱 **QR code generation** — Display QR codes for your receiving addresses
- ⚡ **Instant finality** — Real-time transaction status via WebSocket (Pending → Approved)
- 🔒 **Encrypted storage** — AES-256-GCM encryption with Argon2id key derivation
- 💾 **Persistent state** — UTXO cache, contacts, send records, and masternodes saved to sled DB
- 🌐 **Hybrid connectivity** — TCP-first with HTTP fallback, automatic peer discovery and health checks
- 🖥️ **Masternode management** — Register, import, and manage masternodes from the wallet
- 📝 **Config file editor** — Open masternode.conf in your preferred text editor
- 🔄 **Balance verification** — Cross-checks UTXO total against masternode-reported balance
- 📄 **PDF mnemonic backup** — Printable seed phrase backup with QR code
- 🚪 **Clean shutdown** — Flushes all pending data on exit (X button or Exit menu)

## Getting started

### Prerequisites

- [Rust](https://rustup.rs/) 1.75 or higher

### Build and run

```bash
git clone https://github.com/time-coin/time-coin.git
cd time-coin

# Run the wallet
cargo run --release

# Or build first, then run
cargo build --release
./target/release/wallet-gui
```

### Run tests

```bash
# All tests
cargo test --workspace

# Tests for a single crate
cargo test -p wallet

# A specific test
cargo test -p wallet test_address_generation
```

### Lint

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Project structure

```
time-wallet/
├── wallet-gui/      # GUI application (egui/eframe)
├── wallet/          # Wallet logic, key management, signing
├── core/            # Blockchain types (blocks, transactions, UTXO)
├── crypto/          # Ed25519 signatures, SHA-256 hashing
├── network/         # P2P networking and peer discovery
├── mempool/         # Transaction pool
├── docs/            # Documentation
├── Cargo.toml       # Workspace configuration
└── deny.toml        # Dependency audit rules
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Community

- Website: https://time-coin.io
- Telegram: https://t.me/+CaN6EflYM-83OTY0
- Twitter: [@TIMEcoin515010](https://twitter.com/TIMEcoin515010)
- GitHub: https://github.com/time-coin/time-coin

## License

BUSL-1.1 — see [LICENSE](LICENSE) for details.
