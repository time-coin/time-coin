# Changelog

All notable changes to the TIME Coin Wallet will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.1] - 2026-06-10

### Fixed
- **Incoming message sender address** — Messages no longer show the wallet's own address as the sender. The sender address is now correctly derived from the `sender_pubkey` embedded in every envelope; the broken fallback that returned `msg.recipient_addr` (the local receiving address) has been removed
- **Delete message** — Right-click any message bubble to delete it from local storage. Deleted messages can be re-fetched from the relay with the correct sender attribution
- **Delete conversation** — Right-click any conversation row to delete all locally stored messages for that peer at once, clearing the entry from the Recent list
- **Delete contact from Messages** — Right-click any conversation or contact row in the Messages sidebar to delete the contact (previously only available in the Send tab)

### Added
- **From-address selector in message compose** — Wallets with multiple addresses now show a "From:" dropdown when composing a message, so the sender address can be chosen explicitly
- **Pubkey-based contact identity** — When a message or payment request arrives from an address not yet in the address book, the wallet checks whether any existing contact owns that pubkey and links the address as a secondary address, keeping conversations grouped correctly across per-transaction address changes

## [0.7.0] - 2026-06-09

### Added
- **Secure messaging (TIME-MSG)** — End-to-end encrypted wallet-to-wallet chat using XChaCha20-Poly1305 + X25519 ECDH. Messages are relayed through masternodes and decrypted locally; private keys never leave the device
- **Message Requests inbox** — Messages from unknown senders land in a quarantine "Requests" tab instead of the main inbox. Users can Accept (move to Chats) or Block; blocked senders' messages are silently discarded at fetch time and the decision persists across restarts
- **Block list** — Addresses can be blocked from the message request banner; blocks are stored in sled and loaded on wallet startup
- **Pubkey registration and lookup** — Wallet registers its Ed25519 public key with masternodes on startup, peer switch, and new address generation so other wallets can encrypt messages to it. Three-source lookup chain: masternode contacts book → blockchain scriptSig extraction → wallet-to-wallet flag-0x04 key-request envelope
- **"Request Key" button** — Shown in the chat header when the contact's pubkey is unknown; triggers the three-source lookup and falls back to a wallet-to-wallet key request
- **Chat header action buttons** — "💸 Send", "💰 Request", "➕ Add Contact" (when not yet a contact), and "📋 Copy Address" buttons in the chat header for quick actions without leaving the conversation
- **Add contact button in messages panel** — Phosphor USER_PLUS icon button in the left panel search bar navigates to the Send screen with the add-contact form pre-filled
- **Email and phone fields on contacts** — Contacts now store optional email address and phone number; fields appear in the Add Contact and Edit Contact forms
- **Message timestamps in local time** — Message bubbles and date separators now show times in the device's local timezone instead of UTC
- **Pubkey persistence across masternode restarts** — Masternodes now write registered pubkeys to a sled contacts book so they survive restarts and are propagated via P2P to other nodes
- **Relay store on all masternode tiers** — Bronze-tier masternodes now store message envelopes locally (previously only Silver/Gold did), fixing message delivery on networks without Silver/Gold nodes
- **Send page scrollable** — The Send screen now wraps its full content in a scroll area

### Fixed
- **Chat row click detection** — Clicking anywhere in a conversation row now selects it; previously only clicking the avatar circle registered due to child widgets consuming the click sense in egui
- **Send / Request / Add Contact navigation** — Header buttons now correctly switch the active screen (`state.screen` was missing alongside `NavigatedTo` event)
- **Contact inline edit missing email/phone** — Editing an existing contact now shows and saves email/phone fields; entering edit mode pre-populates them from the stored contact

## [0.6.9] - 2026-06-07

### Added
- **macOS universal DMG** — GitHub Releases now ships a drag-to-install `.dmg` containing a universal binary (Intel x86_64 + Apple Silicon arm64); minimum supported macOS is 11.0
- **Windows installer in CI** — GitHub Releases now ships an Inno Setup `.exe` installer alongside the existing zip archive; includes Start Menu shortcut and optional desktop icon

### Fixed
- **Fee convergence** — Fee calculation now uses output-based iteration, preventing over/under-fee on sends that produce multiple change outputs
- **Windows taskbar icon** — Logo is now correctly embedded in the `.exe` resource table via winresource, so the correct icon appears in the taskbar and Alt+Tab switcher

## [0.6.7] - 2026-05-15

### Added
- **Collateral lock audit** — Tools screen now shows a collateral audit panel (behind a collapsing header) listing all locked UTXOs with their masternode assignment and lock status
- **Collateral locked badge** — Masternode list entries show a locked badge; Register On-Chain pre-fills the node IP from the stored entry

### Changed
- **Incremental startup sync** — Eliminated full chain rescan on startup; wallet now performs an incremental sync so the UI is responsive within seconds
- **Parallel peer probing** — Peer probe sub-checks are now parallelised and deduplicate TLS handshakes, cutting connection setup time significantly
- **Parallel UTXO and transaction scans** — UTXO and transaction syncs are now parallelised after peer connect, eliminating the post-connect delay

### Fixed
- **Consolidation balance inflation after UTXO finalization** — Consolidation outputs are now correctly excluded until they reach masternode finality, preventing double-counting
- **Broken transactions (v2 signing)** — Upgraded to v2 transaction signing format; previously signed transactions were rejected by the masternode due to format mismatch
- **Live fee schedule for consolidation** — Consolidation now fetches the current fee schedule from the masternode instead of using a hardcoded value
- **Deregistration used wrong key** — Deregistration now signs with the collateral owner key; the Masternodes screen correctly shows a "Deregister" button for locked entries
- **Tools screen not scrollable** — Tools screen is now scrollable; collateral audit is hidden behind a collapsing header to keep the layout clean
- **Peer list flicker and verified badge** — Fixed flicker on peer list refresh, stale connecting state, slow peer switching, and incorrect verified badge display
- **Peers rejected on genesis chain mismatch** — Peers reporting an incompatible genesis chain are now rejected early instead of causing downstream sync errors

## [0.6.4] - 2026-04-09

### Added
- **Single-instance lock** — A second wallet instance targeting the same network now shows a native error dialog ("Already Running") and exits cleanly instead of corrupting the sled database. Uses an OS advisory file lock (auto-released on crash) per network directory

### Changed
- **Overview status bar** — Block height, peer count, Mainnet/Testnet badge, and version label are now rendered at 13 px instead of the previous small (~10 px) size for improved readability

### Added
- **Payment Requests screen** — Send payment requests to other wallets via the masternode P2P network. Incoming requests show amount, sender, and expiry timer; approve to pre-fill the Send form or decline to reject
- **Incoming payment request persistence** — Received payment requests are saved to the local sled database and restored on startup so they survive restarts
- **Sent payment request persistence** — Sent requests are saved locally before the RPC call; they appear immediately in the Sent section and show a red "Failed" badge if the network call does not succeed
- **"Request Payment" button on Requests page** — Replaced the non-functional unicode toggle with a plain button; form opens by default when the page loads

### Changed
- **Payment request acknowledgement deferred until send** — Clicking "Approve" on an incoming request no longer immediately fires `acknowledged = paid` on the masternode; the acknowledgement is sent only after the transaction is successfully broadcast, preventing the sender from seeing "Paid" when the payer navigated away without confirming
- **Transaction status: Approved on block inclusion** — Transactions transition to `✅ Approved` once included in a block (`blockhash` present), or when the masternode RPC returns `finalized: true`. Block rewards (`generate` category) are always Approved since they cannot exist outside a block
- **Payment request amount wire format** — Amount is now sent as float TIME (e.g. `1.0`) in the `sendpaymentrequest` RPC call; previously raw satoshis were sent (e.g. `100000`) which the masternode rejected. Incoming amounts from the poll RPC are now correctly converted from float TIME to satoshis

### Fixed
- **UTXO consolidation balance inflation** — Consolidation send records are now marked `is_consolidation: true` so they are excluded from `computed_balance()`. Consolidation output receive entries are now treated as change (not income) during transaction list reconstruction, preventing the consolidated amount from being double-counted alongside the original input receive entries
- **Transactions not appearing on receiving wallet** — The transaction hash (`txid()`) now excludes `encrypted_memo` before hashing. Previously, the memo was attached to the transaction *after* signing, causing the masternode to fail signature verification (hash mismatch) and reject the transaction

## [0.2.0] - 2026-03-11

### Added
- **Consensus column on Connections page** — each peer shows ✔ (green) or ✗ (red) indicating whether it is within 3 blocks of the best known height; hover for exact lag
- **Transaction detail enrichment** — detail view now shows Block Height, Confirmations, and Block Hash (copyable) in addition to existing fields
- **Consensus-based peer filtering** — masternodes more than 3 blocks behind the best peer are automatically dropped from the pool and trigger failover to an in-consensus peer
- **Masternode tier display** — Bronze / Silver / Gold badges with colored text (no emoji) based on collateral amount
- **Locked balance display** — Overview shows Available (large, green), Locked (orange), and Total on a secondary row; locked row only shown when collateral is present
- **"Use as Masternode Collateral" button** — Click any confirmed received transaction to pre-fill the masternode add form and navigate to Masternodes tab
- **Auto-name suggestion** — Add form pre-fills name as `mn1`, `mn2`, etc. based on existing entries
- **Optimistic masternode updates** — Save / edit / delete apply immediately to UI state without waiting for async confirmation
- **Locked UTXO tracking** — `listunspentmulti` now returns locked collateral UTXOs alongside spendable ones; `spendable` field propagated to avoid including them in sends or consolidation
- **Collateral amount persistence** — On each UTXO sync, `collateral_amount` is backfilled on masternode entries and saved to the sled database; amount and tier are available immediately on next startup
- **Instant startup data** — Heavy data (balance, transactions, UTXOs) is fetched on the very first poll tick (5 s) instead of waiting for the 3rd tick (15 s)

### Changed
- **UTXO consolidation order** — Consolidation now processes smallest UTXOs first (dust first), leaving larger UTXOs intact if the run is interrupted
- **Consolidation dismiss** — Dismissing the consolidation banner suppresses it until the next consolidation completes (previously it reappeared within seconds)
- **Settings page** — "Version" label renamed to "Network"; now shows actual daemon version (e.g. `testnet (timed:0.1.0)`) and real peer count from `getnetworkinfo`
- **Masternode form simplified** — IP address, masternode key, and payout address fields removed; the wallet only stores alias, collateral TXID, and vout
- **masternode.conf removed from Tools** — The `masternode.conf` button and template have been removed; masternode configuration lives on the daemon
- **masternode.conf format** — Entries now use 3-field format: `alias txid vout` (old 4–6 field format still accepted for backward compatibility)
- **Masternode entry storage** — Switched from `bincode` to `serde_json`; old bincode entries are auto-migrated on first read
- **Overview balance layout** — Available is now the primary (large) number; Locked and Total appear on a smaller secondary row below
- **Tier requirements table** — Reward Weight column removed; only Tier, Collateral Required shown
- **Per-address balance in Receive tab** — Now shows only spendable balance (excludes locked collateral UTXOs)
- **Send form** — Recipient name field now clears after a successful send alongside address and amount

### Fixed
- **Zero-amount received transactions** — Scientific notation amounts (e.g. `1e-8`) now parse correctly; staking-input-only entries are filtered at the masternode and wallet layers
- **HTTP endpoint scheme** — Bare IP addresses and hostnames now use `http://` (masternodes do not use TLS on ports 24001/24101)
- **Peer discovery count** — Gossip-discovered peers are now added to the peer list instead of replacing existing ones; wallet correctly shows all reachable peers
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
