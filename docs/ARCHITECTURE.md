# TIME Coin Wallet — Architecture

## Overview

The TIME Coin Wallet is a desktop GUI application built with Rust and [egui](https://github.com/emilk/egui). It is a thin client — all blockchain state is managed by remote masternodes. Key management, transaction signing, and HD derivation happen entirely locally.

## Workspace Structure

```
time-wallet/
├── Cargo.toml                  # Workspace root
├── wallet-gui/                 # Binary — egui/eframe GUI + async service
├── wallet/                     # Library — keys, signing, HD derivation, encryption
├── time-core/                  # Library — shared blockchain types, validation, VDF
├── time-network/               # Library — TCP/HTTP hybrid P2P, peer discovery
├── time-mempool/               # Library — transaction pool with priority ordering
└── time-crypto/                # Library — Ed25519 signatures, SHA-256/SHA-3 hashing
```

## Crate Dependency Graph

```
wallet-gui
├── wallet
│   ├── time-core
│   │   └── time-crypto
│   ├── time-network
│   │   └── time-core
│   └── time-mempool
│       └── time-core
└── (egui / eframe)
```

## wallet-gui internals

The GUI binary is structured as an event-driven pipeline: the egui render loop sends `UiEvent`s to an async service task, which sends `ServiceEvent`s back.

### Key modules

| Module | Purpose |
|--------|---------|
| `app.rs` | Top-level egui render loop and screen dispatch |
| `state.rs` | All UI-visible application state; applies `ServiceEvent`s |
| `service.rs` | Core async service — coordinates masternode polling, WS, wallet ops |
| `events.rs` | `UiEvent` (UI→service) and `ServiceEvent` (service→UI) enums |
| `masternode_client.rs` | JSON-RPC 2.0 (newline-delimited) client over TCP with HTTP fallback |
| `ws_client.rs` | WebSocket client for real-time transaction and payment request updates |
| `wallet_manager.rs` | Wallet lifecycle: create, open, close, lock/unlock |
| `wallet_db.rs` | Local sled key-value store — contacts, send records, masternodes, payment requests |
| `encryption.rs` | AES-256-GCM encryption + Argon2id key derivation from user password |
| `memo.rs` | ECDH-based encrypted memo (Ed25519→X25519 + AES-256-GCM) |
| `view/` | One file per screen (see below) |

### Screens (`view/`)

| File | Screen |
|------|--------|
| `welcome.rs` | Network selection, wallet create/open/restore |
| `overview.rs` | Balance display, recent transactions, income chart |
| `send.rs` | Send form, address book |
| `receive.rs` | Address list with QR codes and editable labels |
| `payment_requests.rs` | Send/receive/manage payment requests |
| `transactions.rs` | Full transaction history with search and detail view |
| `masternodes.rs` | Masternode collateral management |
| `connections.rs` | Peer health table |
| `settings.rs` | Connection info, editor config |
| `tools.rs` | Resync, DB repair, UTXO consolidation |

### Data flow

```
egui render loop  ──UiEvent──►  service task  ──ServiceEvent──►  AppState
     ▲                                                               │
     └──────────────────── state fields read at render time ◄───────┘
```

The service task runs entirely on a `tokio` runtime. It owns a `select!` loop over:
- A 5-second poll tick (balance, transactions, UTXOs every 3rd tick)
- `UiEvent` receiver (commands from the UI)
- WebSocket event stream (real-time notifications)

## Security Architecture

| Concern | Implementation |
|---------|---------------|
| Signing | Ed25519 (ed25519-dalek); all signing is local |
| Key storage | AES-256-GCM; key derived via Argon2id (19 MB memory, 2 iterations) |
| HD wallet | BIP39 mnemonics (12–24 words) + BIP44 derivation paths |
| Secure memory | `zeroize` crate wipes sensitive data from memory |
| Memos | ECDH (Ed25519→X25519) + AES-256-GCM; only sender and recipient can decrypt |
| Transaction hash | `encrypted_memo` excluded from the hash so signatures are stable when memos are attached after signing |

## UTXO Model

Transactions are UTXO-based. The wallet:
- Selects coins automatically (locked collateral UTXOs excluded)
- Generates change outputs back to the same address
- Tracks locked UTXOs (masternode collateral) separately from spendable balance
- Consolidates dust UTXOs on demand (Tools → Consolidate UTXOs)

## Local Database (sled)

All persistent local state lives in a sled key-value store under `~/.time-wallet/`:

| Key prefix | Contents |
|------------|----------|
| `contact:` | Address book entries |
| `send_record:` | Locally-inserted send transaction records |
| `masternode:` | Masternode collateral entries |
| `sent_pr:` | Sent payment requests |
| `incoming_pr:` | Received payment requests |
| `utxo:` | Cached UTXO set (fast startup) |
| `cached_balance` | Last known balance (fast startup) |
| `cached_txs:` | Last known transaction list (fast startup) |

## Network Protocol

The wallet communicates with masternodes over two channels:

1. **JSON-RPC 2.0** (newline-delimited) over TCP port 24001 (mainnet) / 24101 (testnet) with HTTP fallback. Used for: balance queries, transaction history, UTXO set, broadcast, payment requests.

2. **WebSocket** on port 24002 (mainnet) / 24102 (testnet). Used for: real-time transaction notifications, UTXO finality events, and payment request relay.

See `docs/RPC_PROTOCOL.md` for the full RPC method reference.
