# Changelog

All notable changes to the TIME Coin Wallet will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Masternode tier display** — Bronze / Silver / Gold badges with colored text (no emoji) based on collateral amount
- **Locked balance display** — Overview shows Available (large, green), Locked (orange), and Total on a secondary row; locked row only shown when collateral is present
- **"Use as Masternode Collateral" button** — Click any confirmed received transaction to pre-fill the masternode add form and navigate to Masternodes tab
- **Auto-name suggestion** — Add form pre-fills name as `mn1`, `mn2`, etc. based on existing entries
- **Optimistic masternode updates** — Save / edit / delete apply immediately to UI state without waiting for async confirmation
- **Locked UTXO tracking** — `listunspentmulti` now returns locked collateral UTXOs alongside spendable ones; `spendable` field propagated to avoid including them in sends or consolidation
- **Collateral amount persistence** — On each UTXO sync, `collateral_amount` is backfilled on masternode entries and saved to the sled database; amount and tier are available immediately on next startup
- **Instant startup data** — Heavy data (balance, transactions, UTXOs) is fetched on the very first poll tick (5 s) instead of waiting for the 3rd tick (15 s)

### Changed
- **Masternode form simplified** — IP address and masternode key fields removed; the masternode daemon reads the IP from `externalip=` in its own `time.conf`; the wallet only stores alias, collateral TXID, and vout
- **masternode.conf format** — Entries now use 3-field format: `alias txid vout` (old 4–6 field format still accepted for backward compatibility)
- **Masternode entry storage** — Switched from `bincode` to `serde_json`; old bincode entries are auto-migrated on first read
- **Overview balance layout** — Available is now the primary (large) number; Locked and Total appear on a smaller secondary row below
- **Tier requirements table** — Reward Weight column removed; only Tier, Collateral Required shown
- **Per-address balance in Receive tab** — Now shows only spendable balance (excludes locked collateral UTXOs)

### Fixed
- **Peer discovery count** — Gossip-discovered peers are now added to the peer list instead of replacing existing ones; wallet correctly shows all reachable peers (was showing 6 instead of 8)
- **Locked balance for all tiers** — Gold and Bronze entries now register correctly; previously only Silver was counted because locked UTXOs were filtered out before reaching state
- **Tier detection on startup** — `collateral_amount` is loaded from disk and tier badge resolves without waiting for a UTXO sync

## [0.1.0] - 2026-02-25

### Added
- Cross-platform GUI wallet built with egui/eframe
- HD wallet support with BIP39 mnemonic seed and BIP32 key derivation
- Send and receive TIME coins via UTXO-based transactions
- AES-256-GCM encrypted wallet storage with Argon2 key derivation
- QR code generation for receiving addresses
- Bitcoin-style wallet.dat backup and restore
- PDF export for mnemonic seed backup
- P2P network connectivity with peer discovery
- Address book with contact management
- Transaction history view
