# Contributing to TIME Coin

Thank you for your interest in contributing to TIME Coin!

## Ways to Contribute

- 🐛 Report bugs
- 💡 Suggest features
- 📝 Improve documentation
- 🔧 Submit pull requests
- 🗳️ Participate in governance

## Development Setup

1. Install Rust: https://rustup.rs/
2. Clone the repository
3. Run `cargo build`
4. Run tests: `cargo test --all`

## Pull Request Process

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run `cargo fmt` and `cargo clippy`
6. Submit a pull request

## Code Style

- Follow Rust conventions
- Use `cargo fmt` for formatting
- Pass `cargo clippy` without warnings
- Add documentation for public APIs
- Include tests for new features

## Testing

```bash
# Run all tests
cargo test --all

# Run specific module tests
cargo test --package treasury

# Run with coverage
cargo tarpaulin --out Html
```

## Governance Contributions

Want to propose ecosystem improvements? See:
- [Proposal Template](docs/governance/proposal-template.md)
- [Treasury Guidelines](docs/treasury/treasury-overview.md)

## Questions?

- Forum: https://forum.time-coin.io
- Telegram: https://t.me/+CaN6EflYM-83OTY0
- Discord: https://discord.gg/timecoin

## Code of Conduct

Be respectful, inclusive, and professional in all interactions.
