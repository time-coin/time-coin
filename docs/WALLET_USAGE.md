# Wallet Usage Guide

## First Launch

On first launch, the wallet presents a network selection screen:

1. **🌐 Mainnet** — Connect to the production TIME Coin network
2. **🧪 Testnet** — Connect to the test network for development

After selecting a network, you can:

1. **Create New Wallet** — generates a new BIP39 mnemonic phrase
2. **Open Wallet** — unlock an existing wallet with your password
3. **Restore Wallet** — enter an existing mnemonic phrase to recover

### Creating a New Wallet

1. Choose word count (12 or 24 words recommended)
2. Optionally click "Generate Random Phrase" or enter your own
3. **Write down your mnemonic phrase** — this is the only way to recover your wallet
4. Use "Copy to Clipboard" or "🖨 Print PDF" to save a backup
5. Set a password to encrypt your wallet file
6. The wallet generates your first receiving address and begins syncing

### Restoring a Wallet

1. Enter your mnemonic phrase (12–24 words)
2. Set a new password for the local wallet file
3. The wallet derives your addresses and syncs with masternodes

---

## Configuration

The wallet uses two configuration files in `~/.time-wallet/`:

| File | Purpose |
|------|---------|
| `time.toml` | Startup preference — which network to launch on |
| `time.conf` | Mainnet settings (peers, RPC credentials, etc.) |
| `testnet/time.conf` | Testnet settings |

**`time.toml`** — edit to change the default startup network:
```toml
# "mainnet" or "testnet"
network = "mainnet"
```

**`time.conf`** (and `testnet/time.conf`) — Bitcoin-style `key=value` format:
```
# Masternode peers (IP, IP:port, or http://IP:port). Repeat for multiple.
addnode=64.91.241.10:24001

# RPC credentials (from the masternode's time.conf)
rpcuser=timecoinrpc
rpcpassword=

# Maximum peer connections (0 = unlimited)
maxconnections=0
```

| Key | Default | Description |
|-----|---------|-------------|
| `addnode` | — | Masternode peer address. Repeatable. |
| `rpcuser` | — | RPC username for masternode auth |
| `rpcpassword` | — | RPC password for masternode auth |
| `maxconnections` | `0` (unlimited) | Cap the peer list size |
| `wsendpoint` | — | Override the WebSocket URL |
| `editor` | system default | Editor used to open config files |

### Network Ports

| Network | P2P Port | RPC Port | WS Port |
|---------|----------|----------|---------|
| Testnet | 24100 | 24101 | 24102 |
| Mainnet | 24000 | 24001 | 24002 |

### Data Directory

| OS | Path |
|----|------|
| Windows | `%USERPROFILE%\.time-wallet\` |
| macOS | `~/.time-wallet/` |
| Linux | `~/.time-wallet/` |

Testnet data is stored in a `testnet/` subdirectory. The wallet stores its sled database, wallet file, and masternode configuration here.

---

## Overview Screen

The overview shows:

- **Available** — Large green number showing your spendable balance
- **Locked / Total** — Shown below Available when masternode collateral is locked; Locked is the total collateral amount, Total is the full wallet balance
- **Balance verification** — Shows ✅ Verified when UTXO total matches masternode-reported balance, or ⏳ Pending when transactions are unconfirmed
- **Notifications** — Real-time transaction notifications from WebSocket
- **Recent Transactions** — Last 10 transactions in a table with Type, Amount, Address, Date, and Status columns

> **Note:** Locked funds are your masternode collateral. They remain in your wallet and are returned when you stop running a masternode.

---

## Sending Coins

1. Navigate to the **📤 Send** tab
2. Enter the recipient address (`TIME0...` for testnet, `TIME1...` for mainnet)
3. Or click **📷 Scan** to use your webcam to scan a QR code — plays an audible beep on success
4. Optionally enter a **Recipient Name** — saved to your address book automatically
5. Enter the amount in TIME
6. Review the transaction details and fee (automatically calculated)
7. Click **Send Transaction** to sign and broadcast

The wallet automatically selects spendable UTXOs (locked collateral is excluded) and creates change outputs. If UTXOs are temporarily locked pending finalization, the wallet retries up to 5 times with a 2-second wait.

### Address Book

Below the send form, the **Address Book** section shows saved contacts:

- Search contacts by name or address
- Click a contact to auto-fill the recipient address and name
- Edit or delete existing contacts
- Contacts are automatically saved when you send to a named recipient

### Self-Sends

When sending to one of your own addresses, the wallet immediately shows all three transaction entries: Sent, Fee, and Received — all as Pending until finalized.

---

## Receiving Coins

1. Navigate to the **📥 Receive** tab
2. Select an address from your address list — the balance shown per address is spendable funds only (locked collateral is excluded)
3. Copy the address to clipboard or share the displayed QR code
4. The wallet derives new addresses from your HD key chain as needed

---

## Payment Requests

The **📋 Requests** tab lets you send payment requests to other wallets and respond to incoming ones.

### Sending a Request

1. Click **Request Payment** to open the request form
2. Enter:
   - **Payer Address** — the TIME address of the wallet you want to pay you
   - **Amount (TIME)** — the amount you are requesting
   - **Label** — a short description (e.g. "Invoice #42", "Rent")
   - **Memo** — optional note visible to the payer
3. Click **Send Request**

The request is saved locally immediately and appears in the **Sent** section. If the masternode cannot be reached, the status shows **Failed** in red.

### Incoming Requests

Pending requests from other wallets appear in the **Incoming** section with:
- Sender address, amount (including estimated fee), and expiry timer
- An editable memo field so you can annotate before approving
- **Approve** — pre-fills the Send screen with the payer's address, amount, and memo; the masternode is notified of payment only after the transaction actually broadcasts
- **Decline** — rejects the request and notifies the sender

### Sent Request Statuses

| Status | Meaning |
|--------|---------|
| **Pending** | Awaiting response from the payer |
| **Paid** | Payer approved and transaction was broadcast |
| **Declined** | Payer rejected the request |
| **Cancelled** | You withdrew the request before the payer responded |
| **Failed** | Masternode could not be reached when sending; request was not relayed |

> **Note:** Incoming requests are saved to the local database and restored on startup. They remain visible until they expire or are acted upon.

---

## Transaction History

The **📋 Transactions** tab shows all transactions in a table format:

| Column | Description |
|--------|-------------|
| Type | 📤 Sent, 📥 Received, or 💸 Fee |
| Amount | Signed amount in TIME |
| Address | Full recipient/sender address with contact name if saved |
| Date | Local timestamp |
| Status | ✅ Approved, ⏳ Pending, or ❌ Declined |
| TxID | Abbreviated transaction ID |

- Click any row to view full transaction details
- Search transactions by address, amount, contact name, or transaction ID
- Striped rows for readability

### Transaction Detail

Click a transaction row to see full details including:

- **Status** — Approved, Pending, or Declined
- **Transaction ID** — full TxID (click to copy)
- **To / From** — address with contact name if saved (click to copy)
- **Fee** — network fee paid (send transactions only)
- **Date** — local timestamp
- **Block Height** — block number the transaction was included in
- **Confirmations** — number of confirmations
- **Block Hash** — block hash (click to copy; shown truncated)

For confirmed **received** transactions, a **"Use as Masternode Collateral"** button appears at the bottom of the detail view. Clicking it:
1. Pre-fills the masternode add form with the TXID and Vout
2. Suggests the next available name (e.g. `mn1`, `mn2`)
3. Navigates directly to the Masternodes tab

### Transaction Statuses

- **⏳ Pending** — Transaction broadcast but not yet included in a block
- **✅ Approved** — Transaction is included in a block (synchronized). Block rewards are always Approved
- **❌ Declined** — Rejected by the network (insufficient funds, invalid signature, etc.)

---

## Masternodes

The **🖥 Masternodes** tab lets you register and manage masternode collateral entries.

### Masternode Tiers

| Tier | Collateral Required |
|------|-------------------|
| **Gold** | 100,000 TIME |
| **Silver** | 10,000 TIME |
| **Bronze** | 1,000 TIME |

The wallet automatically detects the tier from the collateral amount once the UTXO is seen on the network.

### Adding a Masternode Entry

1. Click **+ Add Masternode** (or use **"Use as Masternode Collateral"** from a transaction detail — this pre-fills the form)
2. Enter:
   - **Name** — a local label (e.g. `mn1`). The wallet suggests the next available name automatically.
   - **Collateral TXID** — the transaction ID of your collateral deposit
   - **Vout** — the output index (usually `0`)
3. Click **Save**

The entry appears immediately. The tier badge updates within a few seconds once the wallet confirms the collateral amount from the network.

> **Note:** The IP address and masternode key are **not** stored in the wallet. The masternode daemon reads its own IP from `externalip=` in its `time.conf` file.

### masternode.conf Format

The wallet can copy a conf line for your masternode daemon. The format is:

```
alias  collateral_txid  vout
```

Example:
```
mn1  048fa7a49a3eea905581fa803460a22f6f49c790e0a37adeaab1e5cfa7929a73  0
mn2  61853e9b...e7524489  0
```

Click **Copy Conf** on any masternode entry to copy its conf line to the clipboard. Paste this into `masternode.conf` on the masternode daemon.

### Editing and Deleting

- Click the **Edit** button on an entry to modify the name or collateral details
- Click **Delete** to remove an entry — this releases the funds from the locked balance

### Locked Balance

When masternode entries are saved, the wallet locks the corresponding UTXOs. The locked amount is shown on the Overview screen. Locked funds cannot be spent while the masternode entry exists.

---

## Connections

The **🔗 Connections** tab shows all discovered masternode peers in a table:

| Column | Description |
|--------|-------------|
| # | Row number |
| (dot) | Health indicator — green (<100 ms), yellow (<500 ms), red (unhealthy) |
| IP Address | Peer IP extracted from endpoint |
| Status | Active (currently in use) or Healthy |
| WS | WebSocket availability |
| Ping | TCP round-trip latency |
| Block | Current block height reported by this peer |
| Consensus | ✔ green = within 3 blocks of best height; ✗ red = lagging behind |

Peer discovery runs automatically every 5–15 seconds. Peers more than 3 blocks behind the best known height are excluded from the active pool — the wallet only connects to in-consensus nodes.

---

## Settings

The **⚙ Settings** tab provides:

- **Connection** — Shows WebSocket status, block height, peer count, and Network (daemon version + network type e.g. `testnet (timed:0.1.0)`)
- **Text Editor** — Configure which editor opens config files (text input + Browse… button)
- Editor is auto-detected on first run (checks for Notepad++, then Notepad on Windows)

---

## Tools

The **🔧 Tools** tab offers:

- **Resync Wallet** — Re-fetches all transactions and UTXOs from masternodes
- **Repair Database** — Backs up a corrupted database and creates a fresh one; re-fetches all data
- **Consolidate UTXOs** — Merges many small UTXOs into fewer large ones; processes smallest UTXOs first. Runs automatically in the background; a banner prompts when 50+ UTXOs are detected
- **Open time.toml** — Edit the startup network preference (`mainnet` or `testnet`)
- **Open time.conf (mainnet/testnet)** — Edit peers, RPC credentials, and other settings for the active network

---

## Security

### Password Protection

Your wallet file is encrypted with AES-256-GCM. The encryption key is derived from your password using Argon2id with:
- 19 MB memory cost
- 2 time iterations
- 12-byte random nonce per encryption

### Password Strength

The wallet enforces minimum requirements:
- At least 8 characters
- Strength indicator shows: Very Weak → Weak → Fair → Strong → Very Strong
- Uses character diversity (uppercase, lowercase, digits, special characters)

### Mnemonic Phrase

Your 12–24 word mnemonic phrase is the master backup for your wallet. Anyone with this phrase can access your funds. Store it securely offline.

### Key Isolation

Private keys never leave your device. Transaction signing is performed locally, and only the signed transaction is broadcast to the masternode.

### Secure Memory

Private keys and mnemonic phrases are zeroed from memory using the `zeroize` crate when no longer needed.

---

## Troubleshooting

### Cannot connect to masternode

1. Check your internet connection
2. The wallet discovers peers automatically — wait 10–15 seconds for discovery to complete
3. Verify `addnode=` entries in `time.conf` (or `testnet/time.conf`) contain reachable addresses if automatic discovery fails
4. Ensure firewall allows outbound TCP on port 24101 (testnet) or 24001 (mainnet)
5. Check the **Connections** tab for peer health status

### Wallet shows zero balance

1. Confirm you are on the correct network (testnet vs mainnet)
2. Wait for sync to complete — the overview shows a spinner while syncing
3. If restored from mnemonic, ensure the phrase was entered correctly
4. Use **Resync Wallet** in the Tools tab to force a full refresh

### Balance differs on startup

The wallet loads cached UTXOs from the last session immediately, then fetches fresh data from masternodes within 5 seconds. A brief discrepancy on startup is normal and self-corrects.

### Locked balance shows 0 on startup

If this is the first time running after adding masternode entries, the wallet needs one UTXO sync to detect the collateral amounts and persist them. After the first sync the amounts are cached and will show immediately on all future startups.

### Tier shows "Tier pending"

The tier is determined from the collateral UTXO amount. It resolves within 5 seconds of startup once the wallet fetches UTXOs from the network. After that, the tier is cached and loads instantly.

### Forgot password

The wallet password cannot be recovered. If you have your mnemonic phrase, create a new wallet and restore from the mnemonic.
