# Contributing to TIME Coin

Thank you for your interest in contributing to TIME Coin! This document provides guidelines for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Testing Guidelines](#testing-guidelines)
- [Documentation](#documentation)
- [Community](#community)

## Code of Conduct

We are committed to providing a welcoming and inclusive environment. Please be respectful and constructive in all interactions.

## Getting Started

### Prerequisites

- Rust 1.75 or higher
- Git
- Familiarity with blockchain concepts and Rust development

### Building the Project

```bash
# Clone the repository
git clone https://github.com/time-coin/time-coin.git
cd time-coin

# Build all components
cargo build --release

# Run tests
cargo test --all

# Run linting
cargo clippy --all-targets --all-features
```

### Repository Structure

- `core/` - Core blockchain logic
- `wallet/` - Wallet implementation
- `network/` - P2P networking
- `consensus/` - Consensus mechanisms
- `masternode/` - Masternode management
- `api/` - API server
- `cli/` - Command-line interface
- `wallet-gui/` - GUI wallet application
- `docs/` - Documentation

## Development Workflow

1. **Fork the repository** to your GitHub account
2. **Clone your fork** locally
3. **Create a feature branch** from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```
4. **Make your changes** following our coding standards
5. **Test your changes** thoroughly
6. **Commit your changes** with clear, descriptive messages
7. **Push to your fork** and create a pull request

### Branch Naming Convention

- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation updates
- `refactor/` - Code refactoring
- `test/` - Test additions or modifications

## Pull Request Process

1. **Update documentation** if your changes affect user-facing behavior
2. **Add tests** for new functionality
3. **Ensure all tests pass**: `cargo test --all`
4. **Run clippy**: `cargo clippy --all-targets --all-features -- -D warnings`
5. **Format your code**: `cargo fmt --all`
6. **Update CHANGELOG.md** if applicable
7. **Write a clear PR description** explaining:
   - What problem does this solve?
   - How does it solve it?
   - Any breaking changes?
   - Related issues (use `Fixes #123` to auto-close issues)

### PR Review Process

- At least one maintainer approval is required
- All CI checks must pass
- Address review feedback promptly
- Keep PRs focused and reasonably sized

## Coding Standards

### Rust Code Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for consistent formatting
- Use `cargo clippy` to catch common mistakes
- Write idiomatic Rust code
- Prefer explicit error handling over panics
- Document public APIs with rustdoc comments

### Code Quality

- **DRY (Don't Repeat Yourself)**: Extract common logic
- **KISS (Keep It Simple)**: Prefer simple, clear solutions
- **YAGNI (You Aren't Gonna Need It)**: Don't add speculative features
- **Single Responsibility**: Functions/modules should do one thing well

### Error Handling

- Use `Result<T, E>` for recoverable errors
- Use custom error types with `thiserror`
- Provide meaningful error messages
- Document error conditions in function docs

### Comments

- Write self-documenting code when possible
- Add comments for complex algorithms or non-obvious logic
- Use rustdoc for public APIs:
  ```rust
  /// Validates a transaction against the UTXO set.
  ///
  /// # Arguments
  ///
  /// * `tx` - The transaction to validate
  /// * `utxo_set` - The current UTXO set
  ///
  /// # Returns
  ///
  /// Returns `Ok(())` if valid, or an error describing the validation failure.
  pub fn validate_transaction(tx: &Transaction, utxo_set: &UtxoSet) -> Result<()> {
      // ...
  }
  ```

## Testing Guidelines

### Test Categories

- **Unit tests**: Test individual functions/modules in isolation
- **Integration tests**: Test component interactions
- **End-to-end tests**: Test complete workflows

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_validation() {
        // Arrange
        let tx = create_test_transaction();
        let utxo_set = create_test_utxo_set();

        // Act
        let result = validate_transaction(&tx, &utxo_set);

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_async_operation() {
        // Test async code
    }
}
```

### Test Coverage

- Aim for meaningful test coverage, not just high percentages
- Test edge cases and error conditions
- Test concurrent operations where applicable
- Use property-based testing for complex logic (e.g., with `proptest`)

## Documentation

### Types of Documentation

1. **Code documentation** (rustdoc)
   - Public APIs must be documented
   - Include examples where helpful

2. **README files**
   - Each major component should have a README
   - Explain purpose, usage, and architecture

3. **Technical documentation** (`docs/`)
   - Architecture decisions
   - Protocol specifications
   - Setup guides

### Writing Documentation

- Be clear and concise
- Include code examples
- Keep documentation up to date with code changes
- Use diagrams where helpful

## Commit Messages

Use clear, descriptive commit messages following this format:

```
<type>: <short summary>

<optional detailed description>

<optional footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `refactor`: Code refactoring
- `test`: Test additions/modifications
- `chore`: Maintenance tasks

**Examples:**
```
feat: implement instant transaction finality in masternode consensus

fix: resolve UTXO double-spend race condition

docs: update masternode setup guide with Ubuntu 22.04 instructions

refactor: simplify block validation logic
```

## Issue Reporting

When reporting issues, please include:

- **Clear title** describing the problem
- **Description** of what you expected vs. what happened
- **Steps to reproduce** the issue
- **Environment details** (OS, Rust version, TIME Coin version)
- **Logs or error messages** if applicable
- **Possible solution** if you have ideas

## Feature Requests

For feature requests:

- Check if it's already been requested
- Explain the use case and benefits
- Consider implementation complexity
- Be open to discussion and alternatives

## Community

- **GitHub Discussions**: For questions and general discussion
- **Issues**: For bug reports and feature requests
- **Telegram**: https://t.me/+CaN6EflYM-83OTY0
- **Twitter**: [@TIMEcoin515010](https://twitter.com/TIMEcoin515010)

## License

By contributing to TIME Coin, you agree that your contributions will be licensed under the MIT License.

---

Thank you for contributing to TIME Coin! ⏰
