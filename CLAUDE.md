# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Versioning

The version for this workspace is defined in the root **`Cargo.toml`** (`version = "x.y.z"`). All workspace member crates inherit it via `version.workspace = true`. When bumping the version:

1. Update the root `Cargo.toml` here
2. Update **`~/projects/time-website/js/config.js`** — this is the single source of truth for the version numbers displayed on the public website (`walletVersion`, `devNotice`, `progressInfo`). The website does not auto-read Cargo.toml; it must be updated manually.

## Related repositories

- **Masternode**: `~/projects/time-masternode` — the full-node/masternode implementation that the wallet connects to via JSON-RPC (port 24001 mainnet, 24101 testnet) and WebSocket. When tracing wallet↔masternode protocol bugs (e.g. transaction serialization, memo encryption, RPC handlers), always read both repos together.

## Commands

```bash
# Format (enforced in CI)
cargo fmt --all

# Lint (warnings are errors in CI)
cargo clippy --workspace --all-targets -- -D warnings

# Type check (fast, no codegen)
cargo check --workspace

# Run all tests
cargo test --workspace

# Run tests for a single crate
cargo test -p wallet

# Run a specific test
cargo test -p wallet test_address_generation

# Run the GUI
cargo run --release

# Dependency audit
cargo deny check --hide-inclusion-graph
```

Async tests use `#[tokio::test]`.

## Architecture

This is a **Rust desktop GUI wallet** for the TIME Coin blockchain, built with egui/eframe. It is a thin client: key management and transaction signing are local; all blockchain state (UTXOs, blocks, mempool) is managed by remote masternodes.

### Workspace crates

| Crate | Role |
|-------|------|
| `wallet-gui` | Binary — egui/eframe GUI, async service layer, all screens |
| `wallet` | Library — key management, signing, HD derivation, encryption, UTXO selection |
| `time-core` | Library — shared blockchain types, block/tx validation, UTXO set, chain selection, VDF |
| `time-network` | Library — TCP/HTTP hybrid P2P, peer discovery, connection health |
| `time-mempool` | Library — transaction pool with priority ordering |
| `time-crypto` | Library — Ed25519 signatures, SHA-256/SHA-3 hashing |

Dependency flow: `wallet-gui → wallet → time-core → time-crypto`, `wallet-gui → wallet → time-network → time-core`, `wallet-gui → wallet → time-mempool → time-core`.

### wallet-gui internals

- **`service.rs`** — Core async service coordinating all wallet operations. The main bridge between the GUI and all backend crates.
- **`state.rs`** — Application-wide UI state, drives what `app.rs` renders.
- **`app.rs`** — Top-level egui render loop and screen dispatch.
- **`masternode_client.rs`** — JSON-RPC 2.0 (newline-delimited) client for masternode communication over TCP (port 24001 mainnet, 24101 testnet) with HTTP fallback.
- **`ws_client.rs`** — WebSocket client for real-time transaction status updates.
- **`wallet_manager.rs`** — Wallet lifecycle: create, open, close, lock/unlock.
- **`wallet_db.rs`** — Local sled key-value store for contacts, send records, masternode list, and wallet state.
- **`encryption.rs`** — AES-256-GCM encryption + Argon2id key derivation (19 MB memory, 2 iterations) from user password.
- **`memo.rs`** — ECDH-based encrypted memo support.
- **`view/`** — One file per screen: `welcome`, `overview`, `send`, `receive`, `transactions`, `masternodes`, `connections`, `settings`, `tools`.

### Key security properties

- **Ed25519** for all signing; private keys never leave the device.
- Keys encrypted at rest with **AES-256-GCM**; key derived via **Argon2id**.
- `zeroize` crate used for secure memory cleanup of all sensitive data.
- HD wallets follow **BIP39** (12–24 word mnemonics) + **BIP44** derivation paths.
- Address format: `TIME{0|1}{base58(20-byte SHA-256(pubkey) + checksum)}`.

### UTXO model

Transactions are UTXO-based. The wallet handles automatic coin selection, change output generation, and tracks locked UTXOs (masternode collateral separately from spendable balance).

### Database

**sled** is the current embedded KV store for all persistent local state. RocksDB (requires LLVM/clang) is planned for a future phase.

## Conventions

- Commit format: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.
- All `clippy` warnings must be resolved — `-D warnings` is enforced in CI.
- Public API items require `///` rustdoc comments.
- Prefer `thiserror`-based custom error types and `Result<T, E>` throughout; avoid panics.

## Claude behavior

- Run shell commands one at a time — never issue multiple `Bash` tool calls in the same message.
