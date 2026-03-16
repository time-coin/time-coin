//! Event types for communication between UI and service task.
//!
//! These two enums are the *only* interface between the synchronous egui render
//! loop and the asynchronous service task. No shared state, no Arc, no Mutex.

use crate::masternode_client::{Balance, HealthStatus, TransactionRecord, Utxo};
use crate::state::AddressInfo;
use crate::ws_client::TxNotification;

/// A payment request displayed in the wallet UI.
#[derive(Debug, Clone)]
pub struct PaymentRequest {
    pub id: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: u64,
    /// Short label / subject for the request (e.g. "Invoice #42").
    pub label: String,
    pub memo: String,
    pub pubkey_hex: String,
    pub signature_hex: String,
    pub timestamp: i64,
    pub expires: i64,
}

// ============================================================================
// UI → Service
// ============================================================================

/// Commands sent from the UI thread to the background service task.
#[derive(Debug)]
pub enum UiEvent {
    /// Load an existing wallet (optionally with a password for encrypted wallets).
    LoadWallet {
        password: Option<String>,
    },

    /// Create a new wallet from a mnemonic phrase.
    CreateWallet {
        mnemonic: String,
        password: Option<String>,
    },

    /// Prepare for new wallet creation — backup existing wallet file if present.
    PrepareNewWallet,

    /// Request a balance refresh from the masternode.
    RefreshBalance,

    /// Request the full transaction history for all wallet addresses.
    RefreshTransactions,

    /// Request UTXOs for all wallet addresses.
    RefreshUtxos,

    /// Submit a signed transaction to the masternode.
    SendTransaction {
        to: String,
        amount: u64,
        fee: u64,
        memo: String,
    },

    /// The user navigated to a new screen — the service may prefetch data.
    NavigatedTo(Screen),

    /// Request a masternode health check.
    CheckHealth,

    /// Switch network (mainnet / testnet). Requires wallet reload.
    SwitchNetwork {
        network: String,
    },

    /// Select network on first run (before any wallet is created).
    SelectNetwork {
        network: String,
    },

    /// Update the label for a wallet address (persisted to local db).
    UpdateAddressLabel {
        index: usize,
        label: String,
    },

    /// Generate a new receive address from the HD wallet.
    GenerateAddress,

    /// Save an external contact (send address book).
    SaveContact {
        name: String,
        address: String,
    },

    /// Delete an external contact.
    DeleteContact {
        address: String,
    },

    /// Update the number of decimal places for amount display.
    UpdateDecimalPlaces(usize),

    /// Erase cached data and resync all transactions from masternodes.
    ResyncWallet,

    /// Repair the wallet database — backs up corrupt db, recreates, and resyncs.
    RepairDatabase,

    /// Open a config file in the system's default text editor.
    OpenConfigFile {
        path: std::path::PathBuf,
    },

    /// Clean shutdown.
    Shutdown,

    /// Encrypt an unencrypted wallet with the given password.
    EncryptWallet {
        password: String,
    },

    /// Set the external editor command (None = OS default).
    SetEditor {
        editor: Option<String>,
    },
    SetMaxConnections(usize),

    /// Save a masternode entry to the database.
    SaveMasternodeEntry(crate::wallet_db::MasternodeEntry),

    /// Delete a masternode entry by alias.
    DeleteMasternodeEntry {
        alias: String,
    },

    /// Import masternode entries from a masternode.conf file.
    ImportMasternodeConf {
        path: std::path::PathBuf,
    },

    /// Consolidate many small UTXOs into fewer large ones.
    ConsolidateUtxos,

    /// Register a masternode on-chain via a special transaction.
    RegisterMasternode {
        alias: String,
        ip: String,
        port: u16,
        collateral_txid: String,
        collateral_vout: u32,
        payout_address: String,
    },

    /// Update a masternode's payout address on-chain.
    UpdateMasternodePayout {
        masternode_id: String,
        new_payout_address: String,
    },

    /// Update a masternode entry in the DB, replacing the old alias key.
    UpdateMasternodeEntry {
        old_alias: String,
        new_entry: crate::wallet_db::MasternodeEntry,
    },

    /// Persist updated send records to the database.
    PersistSendRecords(Vec<TransactionRecord>),

    /// Manually switch the active masternode to a specific peer endpoint.
    SwitchPeer {
        endpoint: String,
    },

    /// Send a payment request to another wallet via the masternode P2P network.
    SendPaymentRequest {
        to_address: String,
        amount: u64,
        label: String,
        memo: String,
    },

    /// Pay a received payment request (auto-fills and sends a transaction).
    PayRequest {
        request_id: String,
    },

    /// Decline a received payment request.
    DeclineRequest {
        request_id: String,
    },
}

/// Screens the wallet can display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Welcome,
    NetworkSelect,
    MnemonicSetup,
    MnemonicConfirm,
    Overview,
    Send,
    Receive,
    Transactions,
    Utxos,
    Masternodes,
    Connections,
    Settings,
    Tools,
    Charts,
}

// ============================================================================
// Service → UI
// ============================================================================

/// Events sent from the service task back to the UI thread.
#[derive(Debug)]
pub enum ServiceEvent {
    /// Wallet loaded successfully.
    WalletLoaded {
        addresses: Vec<AddressInfo>,
        is_testnet: bool,
        is_encrypted: bool,
    },

    /// New wallet created — pass mnemonic back for confirmation screen.
    WalletCreated {
        mnemonic: String,
    },

    /// Updated balance from masternode.
    BalanceUpdated(Balance),

    /// Updated transaction list.
    TransactionsUpdated(Vec<TransactionRecord>),

    /// Updated UTXO set.
    UtxosUpdated(Vec<Utxo>),

    /// Transaction broadcast succeeded.
    TransactionSent {
        txid: String,
    },

    /// Real-time transaction notification from WebSocket.
    TransactionReceived(TxNotification),

    /// A single transaction should be inserted (from WS notification or finality update).
    TransactionInserted(TransactionRecord),

    /// Transaction finality status updated.
    TransactionFinalityUpdated {
        txid: String,
        finalized: bool,
    },

    /// Masternode health status.
    HealthUpdated(HealthStatus),

    /// WebSocket connection state changed.
    WsConnected,
    WsDisconnected,
    /// Masternode WebSocket was at capacity — wallet should failover to another peer.
    WsCapacityFull(String),

    /// The wallet is encrypted and a password is needed to unlock it.
    PasswordRequired,

    /// Existing wallet was backed up (or none existed). Ready for mnemonic input.
    ReadyForMnemonic {
        backed_up_path: Option<String>,
    },

    /// A new address was generated.
    AddressGenerated(AddressInfo),

    /// External contacts list updated.
    ContactsUpdated(Vec<crate::state::ContactInfo>),

    /// Peer discovery results with health/ping info.
    PeersDiscovered(Vec<crate::state::PeerInfo>),

    /// Lightweight per-peer block height update (endpoint → height).
    PeerHeightsUpdated(std::collections::HashMap<String, u64>),

    /// Non-fatal error to display in the UI.
    Error(String),

    /// Send failed because the transaction would be too large.
    /// Prompt the user to consolidate UTXOs first.
    SendTooLarge,

    /// Network selected on first run — config saved, service reinitialized.
    NetworkConfigured {
        is_testnet: bool,
    },

    /// Wallet was successfully encrypted with a password.
    WalletEncrypted,

    /// Resync completed — cache cleared, fresh data loaded.
    ResyncComplete,

    /// Database repair completed.
    DatabaseRepaired {
        message: String,
    },

    /// Initial network sync completed (first successful poll).
    SyncComplete,

    /// Decimal places preference loaded from database.
    DecimalPlacesLoaded(usize),

    /// Editor command loaded from config.
    EditorLoaded(Option<String>),

    /// Whether a wallet file exists on disk.
    WalletExists(bool),

    /// Persisted send records loaded from database.
    SendRecordsLoaded(std::collections::HashMap<String, TransactionRecord>),

    /// Masternode entries loaded from database.
    MasternodeEntriesLoaded(Vec<crate::wallet_db::MasternodeEntry>),

    /// Masternode registration transaction broadcast successfully.
    MasternodeRegistered {
        alias: String,
        txid: String,
    },

    /// Masternode payout update transaction broadcast successfully.
    MasternodePayoutUpdated {
        masternode_id: String,
        txid: String,
    },

    /// UTXO consolidation progress update.
    ConsolidationProgress {
        batch: usize,
        total_batches: usize,
        message: String,
    },

    /// UTXO consolidation completed.
    ConsolidationComplete {
        message: String,
    },

    /// Block height polled from active peer.
    BlockHeightUpdated(u64),

    /// Max connections setting updated.
    MaxConnectionsUpdated(usize),

    /// Payment requests received (from poll or WS).
    PaymentRequestsUpdated(Vec<PaymentRequest>),

    /// A single payment request arrived via WebSocket.
    PaymentRequestReceived(PaymentRequest),

    /// Payment request sent successfully.
    PaymentRequestSent {
        id: String,
    },
}
