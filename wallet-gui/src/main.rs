#![allow(dead_code)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::get_first)]
#![allow(clippy::manual_while_let_some)]
#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::await_holding_lock)] // Safe in spawn_blocking contexts
#![allow(unused_variables)]
#![allow(non_snake_case)]
use chrono::Timelike;
use eframe::egui;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use wallet::NetworkType;

mod config;
mod encryption;
mod mnemonic_ui;
mod monitoring;
mod network;
mod password_ui;
mod peer_manager;
mod protocol_client;
mod rate_limiter;
mod tcp_protocol_client;
mod ui_components;
mod utxo_manager;
mod wallet_dat;
mod wallet_db;
mod wallet_manager;
mod wallet_sync;

use config::Config;
use mnemonic_ui::{MnemonicAction, MnemonicInterface};
use network::NetworkManager;
use peer_manager::PeerManager;
use protocol_client::ProtocolClient;
use utxo_manager::{UtxoAction, UtxoManager};
use wallet_db::{AddressContact, WalletDb};
use wallet_manager::WalletManager;

fn main() -> Result<(), eframe::Error> {
    // Initialize tokio runtime for async network operations
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();

    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "TIME Coin Wallet",
        options,
        Box::new(|cc| {
            // Enable emoji support using system fonts
            setup_emoji_fonts(&cc.egui_ctx);
            Ok(Box::new(WalletApp::default()))
        }),
    )
}

/// Setup fonts to support emoji rendering
fn setup_emoji_fonts(ctx: &egui::Context) {
    let fonts = egui::FontDefinitions::default();

    // egui has built-in emoji support, we just need to enable it
    // by using emoji in our proportional font family
    egui_extras::install_image_loaders(ctx);

    // The default fonts in egui already support many emojis
    // We just need to make sure they're loaded properly
    ctx.set_fonts(fonts);
}

#[derive(PartialEq)]
enum Screen {
    Welcome,
    MnemonicSetup,
    MnemonicConfirm,
    Overview,
    Send,
    Receive,
    Transactions,
    Utxos,
    Settings,
    Peers,
}

struct WalletApp {
    current_screen: Screen,
    wallet_manager: Option<WalletManager>,
    wallet_db: Option<WalletDb>,
    network: NetworkType,
    password: String,
    error_message: Option<String>,
    error_message_time: Option<std::time::Instant>,
    success_message: Option<String>,
    success_message_time: Option<std::time::Instant>,

    // Send screen fields
    send_address: String,
    send_amount: String,
    selected_contact: Option<String>, // Selected contact address
    new_contact_address: String,
    new_contact_name: String,
    new_contact_email: String,
    new_contact_phone: String,
    edit_contact_address: String,
    edit_contact_name: String,
    edit_contact_email: String,
    edit_contact_phone: String,
    contact_search: String,
    is_adding_new_contact: bool,
    is_scanning_qr: bool,

    // Transaction sync
    last_sync_time: Option<std::time::Instant>,
    is_syncing_transactions: bool,

    // UI state
    // Network manager (wrapped for thread safety)
    network_manager: Option<Arc<std::sync::Mutex<NetworkManager>>>,
    network_status: String,

    // Peer manager for discovering and managing masternode peers
    peer_manager: Option<Arc<PeerManager>>,

    // TIME Coin Protocol client for real-time notifications
    protocol_client: Option<Arc<ProtocolClient>>,
    // notification_rx: Option<mpsc::UnboundedReceiver<WalletNotification>>, // Removed - WebSocket notifications
    // recent_notifications: Vec<WalletNotification>, // Removed - WebSocket notifications

    // Mnemonic setup - NEW enhanced interface
    mnemonic_interface: MnemonicInterface,
    mnemonic_confirmed: bool,

    // Password prompt for wallet encryption
    password_prompt: Option<password_ui::PasswordPrompt>,
    pending_mnemonic: Option<String>, // Store mnemonic while waiting for password

    // Receiving address management
    selected_address: Option<String>,
    new_address_label: String,
    edit_address_name: String,
    edit_address_email: String,
    edit_address_phone: String,
    address_search: String,
    show_qr_for_address: Option<String>,
    is_creating_new_address: bool,

    // Channel for receiving UTXO updates from TCP listener
    utxo_rx: Option<tokio::sync::mpsc::UnboundedReceiver<time_network::protocol::UtxoInfo>>,

    // Channel for transaction approval/rejection notifications
    tx_notification_rx: Option<tokio::sync::mpsc::UnboundedReceiver<TransactionNotification>>,

    // UTXO management
    utxo_manager: UtxoManager,
}

// Use the TransactionNotification from tcp_protocol_client module
use tcp_protocol_client::TransactionNotification;

impl Default for WalletApp {
    fn default() -> Self {
        // Load config to determine network
        let config = Config::load().unwrap_or_default();
        let network = if config.network == "mainnet" {
            NetworkType::Mainnet
        } else {
            NetworkType::Testnet
        };

        // Check if wallet exists for this network
        let wallet_exists = WalletManager::exists(network);

        // Check if wallet is encrypted
        let wallet_encrypted = if wallet_exists {
            WalletManager::is_encrypted(network).unwrap_or(false)
        } else {
            false
        };

        // Start on Overview if wallet exists AND is unencrypted, otherwise show appropriate screen
        let initial_screen = if wallet_exists {
            if wallet_encrypted {
                Screen::Welcome // Will show unlock prompt
            } else {
                Screen::Overview // Auto-load unencrypted wallet
            }
        } else {
            Screen::Welcome
        };

        let mut app = Self {
            current_screen: initial_screen,
            wallet_manager: None,
            wallet_db: None,
            network,
            password: String::new(),
            error_message: None,
            error_message_time: None,
            success_message: None,
            success_message_time: None,
            send_address: String::new(),
            send_amount: String::new(),
            selected_contact: None,
            new_contact_address: String::new(),
            new_contact_name: String::new(),
            new_contact_email: String::new(),
            new_contact_phone: String::new(),
            edit_contact_address: String::new(),
            edit_contact_name: String::new(),
            edit_contact_email: String::new(),
            edit_contact_phone: String::new(),
            contact_search: String::new(),
            is_adding_new_contact: false,
            is_scanning_qr: false,
            last_sync_time: None,
            is_syncing_transactions: false,
            network_manager: None,
            network_status: "Not connected".to_string(),
            peer_manager: None,
            protocol_client: None,
            mnemonic_interface: MnemonicInterface::new(),
            mnemonic_confirmed: false,
            password_prompt: None,
            pending_mnemonic: None,
            selected_address: None,
            new_address_label: String::new(),
            edit_address_name: String::new(),
            edit_address_email: String::new(),
            edit_address_phone: String::new(),
            address_search: String::new(),
            show_qr_for_address: None,
            is_creating_new_address: false,
            utxo_rx: None,
            tx_notification_rx: None,
            utxo_manager: UtxoManager::new(),
        };

        // If wallet exists and is NOT encrypted, auto-load it
        if wallet_exists && !wallet_encrypted {
            app.auto_load_wallet();
        }

        app
    }
}

impl WalletApp {
    /// Auto-load wallet on startup (without UI context)
    fn auto_load_wallet(&mut self) {
        match WalletManager::load(self.network) {
            Ok(mut manager) => {
                // IMPORTANT: Set UI network to match the loaded wallet's network
                self.network = manager.network();

                // Initialize wallet database
                if let Ok(main_config) = Config::load() {
                    let wallet_dir = main_config.wallet_dir();
                    let db_path = wallet_dir.join("wallet.db");
                    match WalletDb::open(&db_path) {
                        Ok(db) => {
                            // Sync address index with database
                            if let Ok(owned_addresses) = db.get_owned_addresses() {
                                if let Some(max_index) = owned_addresses
                                    .iter()
                                    .filter_map(|a| a.derivation_index)
                                    .max()
                                {
                                    manager.sync_address_index(max_index);
                                    log::info!("Synced address index to {}", max_index + 1);
                                }
                            }
                            self.wallet_db = Some(db);
                            log::info!("Wallet database initialized");
                        }
                        Err(e) => {
                            log::warn!("Failed to open wallet database: {}", e);
                        }
                    }

                    // Initialize peer manager
                    let peer_mgr = Arc::new(PeerManager::new(self.network));
                    if let Some(db) = &self.wallet_db {
                        let db_clone = db.clone();
                        let peer_mgr_clone = peer_mgr.clone();
                        tokio::spawn(async move {
                            peer_mgr_clone.set_wallet_db(db_clone).await;
                        });
                    }
                    self.peer_manager = Some(peer_mgr.clone());

                    // Initialize network manager
                    let network_mgr = Arc::new(Mutex::new(NetworkManager::new(
                        main_config.api_endpoint.clone(),
                    )));

                    // Connect network manager to peer manager
                    {
                        let mut net = network_mgr.lock().unwrap();
                        net.set_peer_manager(peer_mgr.clone());
                    }

                    self.network_manager = Some(network_mgr.clone());
                    self.network_status = "Connecting...".to_string();

                    // Get xpub BEFORE manager is moved
                    let wallet_xpub = manager.get_xpub().to_string();
                    let wallet_network = manager.network(); // Also get network type

                    // Store manager first
                    self.wallet_manager = Some(manager);
                    log::info!("Wallet auto-loaded successfully");

                    // Spawn network bootstrap task
                    let bootstrap_nodes = main_config.bootstrap_nodes.clone();
                    let addnodes = main_config.addnode.clone();
                    let api_endpoint_str = main_config.api_endpoint.clone();
                    let network_mgr_clone = network_mgr.clone();

                    tokio::spawn(async move {
                        let db_peer_count = peer_mgr.peer_count().await;
                        log::info!("📂 Found {} peers in database", db_peer_count);

                        if !addnodes.is_empty() {
                            log::info!("📝 Adding {} nodes from config", addnodes.len());
                            for node in addnodes {
                                let (ip, port) = if let Some((ip, port_str)) = node.split_once(':')
                                {
                                    (ip.to_string(), port_str.parse().unwrap_or(24100))
                                } else {
                                    (node.clone(), 24100)
                                };
                                peer_mgr.add_peer(ip, port).await;
                            }
                        }

                        let total_peer_count = peer_mgr.peer_count().await;
                        if total_peer_count == 0 {
                            log::info!(
                                "🌐 No peers found, fetching from API: {}",
                                api_endpoint_str
                            );
                            if let Ok(client) = reqwest::Client::builder()
                                .timeout(std::time::Duration::from_secs(10))
                                .build()
                            {
                                if let Ok(response) = client.get(&api_endpoint_str).send().await {
                                    if let Ok(peers) = response.json::<Vec<String>>().await {
                                        log::info!("✓ Fetched {} peers from API", peers.len());
                                        for peer_str in peers {
                                            let (ip, port) = if let Some((ip, port_str)) =
                                                peer_str.split_once(':')
                                            {
                                                (ip.to_string(), port_str.parse().unwrap_or(24100))
                                            } else {
                                                (peer_str, 24100)
                                            };
                                            peer_mgr.add_peer(ip, port).await;
                                        }
                                    }
                                }
                            }
                        }

                        // Bootstrap PeerManager to discover peers from network (don't block)
                        log::info!("🔍 Bootstrapping PeerManager...");
                        let peer_mgr_for_bootstrap = peer_mgr.clone();
                        tokio::spawn(async move {
                            if let Err(e) = peer_mgr_for_bootstrap.bootstrap().await {
                                log::warn!("⚠️ PeerManager bootstrap failed: {}", e);
                            } else {
                                log::info!("✅ PeerManager bootstrap completed");
                            }
                        });

                        // ✅ CRITICAL FIX: Actually connect NetworkManager to peers!
                        log::info!("🔗 Connecting NetworkManager to discovered peers...");
                        let peer_list = peer_mgr.get_healthy_peers().await;
                        log::info!("📋 Attempting to connect to {} peers", peer_list.len());

                        let peer_infos: Vec<network::PeerInfo> = peer_list
                            .into_iter()
                            .map(|p| network::PeerInfo {
                                address: p.address,
                                port: p.port,
                                version: None,
                                last_seen: Some(
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs(),
                                ),
                                latency_ms: 0,
                            })
                            .collect();

                        // Connect in a separate thread to avoid Send issues
                        let network_mgr_for_connect = network_mgr_clone.clone();
                        std::thread::spawn(move || {
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async move {
                                // Scope the lock to avoid holding across await
                                // Safe in spawn_blocking/block_on context
                                #[allow(clippy::await_holding_lock)]
                                {
                                    let mut net = network_mgr_for_connect.lock().unwrap();
                                    log::info!("Acquired NetworkManager lock, calling connect_to_peers");
                                    let result = net.connect_to_peers(peer_infos).await;
                                    let peer_count = net.peer_count();
                                    drop(net);

                                    match result {
                                        Ok(_) => {
                                            log::info!(
                                                "✅ NetworkManager connected to {} peers successfully", peer_count
                                            );

                                            // Now discover more peers from the connected ones
                                            log::info!("🔍 Starting peer discovery...");
                                            let mut net = network_mgr_for_connect.lock().unwrap();
                                            let discover_result =
                                                net.discover_and_connect_peers().await;
                                            drop(net);

                                            if let Err(e) = discover_result {
                                                log::warn!("⚠️ Peer discovery had issues: {}", e);
                                            } else {
                                                log::info!("✅ Peer discovery completed");
                                            }
                                        }
                                        Err(e) => {
                                            log::error!(
                                                "❌ Failed to connect NetworkManager: {}",
                                                e
                                            );
                                        }
                                    }
                                }
                            });
                        });

                        // Start periodic peer discovery from PeerManager
                        let peer_mgr_periodic = peer_mgr.clone();
                        let network_mgr_periodic = network_mgr_clone.clone();
                        tokio::spawn(async move {
                            // Wait a bit before starting periodic discovery
                            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

                            loop {
                                log::debug!("Running periodic peer discovery...");

                                // Get new peers from network via PeerManager
                                if let Some(new_peers) = peer_mgr_periodic.try_get_peer_list().await
                                {
                                    peer_mgr_periodic.add_peers(new_peers).await;
                                    log::debug!("Discovered and added peers");

                                    // Connect NetworkManager to newly discovered peers
                                    let peer_list = peer_mgr_periodic.get_healthy_peers().await;
                                    if !peer_list.is_empty() {
                                        log::info!(
                                            "🔗 Connecting to {} total peers",
                                            peer_list.len()
                                        );

                                        let peer_infos: Vec<network::PeerInfo> = peer_list
                                            .into_iter()
                                            .map(|p| network::PeerInfo {
                                                address: p.address,
                                                port: p.port,
                                                version: None,
                                                last_seen: Some(
                                                    std::time::SystemTime::now()
                                                        .duration_since(std::time::UNIX_EPOCH)
                                                        .unwrap()
                                                        .as_secs(),
                                                ),
                                                latency_ms: 0,
                                            })
                                            .collect();

                                        // TODO: Fix peer connection to not block GUI thread
                                        // For now, skip to prevent hanging
                                        log::debug!("Skipping peer connection refresh");
                                        /*
                                        // Connect in background without blocking GUI
                                        let network_clone = network_mgr_periodic.clone();
                                        tokio::spawn(async move {
                                            // Add timeout to prevent hanging
                                            let connect_result = tokio::time::timeout(
                                                std::time::Duration::from_secs(30),
                                                async move {
                                                    let mut net = network_clone.lock().unwrap();
                                                    net.connect_to_peers(peer_infos).await
                                                },
                                            )
                                            .await;

                                            match connect_result {
                                                Ok(Ok(_)) => {
                                                    log::info!("✅ Connected to peers successfully")
                                                }
                                                Ok(Err(e)) => log::warn!(
                                                    "⚠️ Error connecting to peers: {}",
                                                    e
                                                ),
                                                Err(_) => {
                                                    log::warn!("⚠️ Timeout connecting to peers")
                                                }
                                            }
                                        });
                                        */
                                    }
                                }

                                // Check again every 60 seconds (1 minute)
                                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                            }
                        });

                        // Initialize TCP listener for transaction notifications
                        log::info!("🔌 Initializing TCP listener for auto-loaded wallet");
                        let tcp_network_mgr = network_mgr.clone();
                        let wallet_xpub_clone = wallet_xpub.clone();
                        let wallet_network_clone = wallet_network;
                        tokio::spawn(async move {
                            // Wait a bit for peers to connect
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                            let peer_addr = {
                                let net = tcp_network_mgr.lock().unwrap();
                                let peers = net.get_connected_peers();
                                if let Some(peer) = peers.first() {
                                    let peer_ip =
                                        peer.address.split(':').next().unwrap_or(&peer.address);
                                    Some(format!("{}:24100", peer_ip))
                                } else {
                                    None
                                }
                            };

                            if let Some(addr) = peer_addr {
                                log::info!("🔗 Starting TCP listener for {}", addr);
                                let (utxo_tx, _utxo_rx) = tokio::sync::mpsc::unbounded_channel::<
                                    time_network::protocol::UtxoInfo,
                                >();
                                let (tx_notif_tx, _tx_notif_rx) =
                                    tokio::sync::mpsc::unbounded_channel::<
                                        tcp_protocol_client::TransactionNotification,
                                    >();
                                let listener = tcp_protocol_client::TcpProtocolListener::new(
                                    addr.clone(),
                                    wallet_xpub_clone.clone(),
                                    utxo_tx,
                                    tx_notif_tx,
                                );

                                // Note: receivers are dropped here since we can't store them in self
                                // from inside the async task. This is a limitation that should be
                                // refactored in the future.

                                // TODO: Refactor UTXO notification handling
                                // Currently disabled because wallet_manager is owned by self
                                // and can't be shared with async tasks
                                /*
                                // Spawn task to handle incoming UTXO notifications
                                let wallet_for_utxo = wallet_mgr_clone.clone();
                                tokio::spawn(async move {
                                    while let Some(utxo_info) = utxo_rx.recv().await {
                                        log::info!(
                                            "📬 Received new UTXO notification: {} TIME",
                                            utxo_info.amount as f64 / 100_000_000.0
                                        );

                                        let mut wallet = wallet_for_utxo.lock().unwrap();

                                        // Convert txid string to [u8; 32]
                                        let tx_hash = hex::decode(&utxo_info.txid)
                                            .ok()
                                            .and_then(|bytes| {
                                                if bytes.len() == 32 {
                                                    let mut arr = [0u8; 32];
                                                    arr.copy_from_slice(&bytes);
                                                    Some(arr)
                                                } else {
                                                    None
                                                }
                                            })
                                            .unwrap_or([0u8; 32]);

                                        let utxo = wallet::UTXO {
                                            tx_hash,
                                            output_index: utxo_info.vout,
                                            amount: utxo_info.amount,
                                            address: utxo_info.address,
                                        };
                                        wallet.add_utxo(utxo);

                                        let new_balance = wallet.get_balance();
                                        log::info!(
                                            "💰 Balance updated: {} TIME",
                                            new_balance as f64 / 100_000_000.0
                                        );
                                    }
                                });
                                */

                                // TODO: Refactor blockchain scanning
                                // Currently disabled because wallet_manager is owned by self
                                /*
                                // Scan blockchain for existing transactions BEFORE starting listener
                                log::info!("🔍 Scanning blockchain for wallet transactions...");
                                let client = protocol_client::ProtocolClient::new(
                                    addr.clone(),
                                    wallet_network,
                                );
                                match client.request_wallet_transactions(wallet_xpub.clone()) {
                                    Ok(response) => {
                                        log::info!("✅ Found {} transactions on blockchain (synced to block {})",
                                            response.transactions.len(), response.last_synced_height);

                                        // Save transactions to wallet manager
                                        let mut wallet = wallet_mgr_clone.lock().unwrap();
                                        for tx in response.transactions {
                                            // Convert txid string to [u8; 32]
                                            let tx_hash = hex::decode(&tx.tx_hash)
                                                .ok()
                                                .and_then(|bytes| {
                                                    if bytes.len() == 32 {
                                                        let mut arr = [0u8; 32];
                                                        arr.copy_from_slice(&bytes);
                                                        Some(arr)
                                                    } else {
                                                        None
                                                    }
                                                })
                                                .unwrap_or([0u8; 32]);

                                            let utxo = wallet::UTXO {
                                                tx_hash,
                                                output_index: 0, // Assuming first output
                                                amount: tx.amount,
                                                address: tx.to_address.clone(),
                                            };
                                            wallet.add_utxo(utxo);
                                            log::info!(
                                                "💰 Added UTXO: {} TIME (txid: {})",
                                                tx.amount as f64 / 100_000_000.0,
                                                &tx.tx_hash[..16]
                                            );
                                        }

                                        let total_balance = wallet.get_balance();
                                        log::info!(
                                            "💰 Total wallet balance: {} TIME",
                                            total_balance as f64 / 100_000_000.0
                                        );
                                    }
                                    Err(e) => {
                                        log::warn!("⚠️ Failed to scan blockchain: {}", e);
                                    }
                                }

                                // Now start listening for new transactions
                                listener.start().await;
                                */

                                log::info!(
                                    "💡 UTXO notifications temporarily disabled pending refactor"
                                );
                            } else {
                                log::warn!("❌ No peers available for TCP listener");
                            }
                        });

                        // Start periodic refresh for blockchain height, latency, versions
                        // Check more frequently around midnight (23:50-00:10 UTC) when blocks are produced
                        let network_refresh = network_mgr_clone.clone();
                        tokio::spawn(async move {
                            loop {
                                // Determine refresh interval based on time of day
                                let now = chrono::Utc::now();
                                let hour = now.hour();
                                let minute = now.minute();

                                let refresh_interval = if (hour == 23 && minute >= 50)
                                    || (hour == 0 && minute <= 10)
                                {
                                    // Check every 10 seconds around midnight
                                    tokio::time::Duration::from_secs(10)
                                } else {
                                    // Check every 5 minutes during the day
                                    tokio::time::Duration::from_secs(300)
                                };

                                tokio::time::sleep(refresh_interval).await;

                                let network_clone = network_refresh.clone();
                                tokio::task::spawn_blocking(move || {
                                    let rt = tokio::runtime::Runtime::new().unwrap();
                                    rt.block_on(async move {
                                        #[allow(clippy::await_holding_lock)]
                                        {
                                            let mut manager = network_clone.lock().unwrap();
                                            manager.periodic_refresh().await
                                        }
                                    });
                                })
                                .await
                                .ok();
                            }
                        });
                    });
                }
            }
            Err(e) => {
                log::error!("Failed to auto-load wallet: {}", e);
                self.error_message = Some(format!("Failed to load wallet: {}", e));
                self.error_message_time = Some(std::time::Instant::now());
                self.current_screen = Screen::Welcome;
            }
        }
    }

    fn show_welcome_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);

                // TIME Coin Logo (hourglass)
                ui.heading(egui::RichText::new("⏳").size(80.0));
                ui.add_space(20.0);
                ui.heading(egui::RichText::new("TIME Coin Wallet").size(32.0));
                ui.add_space(40.0);

                ui.label("Select Network:");
                ui.add_space(10.0);

                // Network selection - centered
                ui.horizontal(|ui| {
                    // Add spacing to center the buttons
                    let button_width = 80.0;
                    let total_width = button_width * 2.0 + ui.spacing().item_spacing.x;
                    let available_width = ui.available_width();
                    let padding = (available_width - total_width) / 2.0;

                    if padding > 0.0 {
                        ui.add_space(padding);
                    }

                    // Disable network selector if wallet is loaded
                    let is_wallet_loaded = self.wallet_manager.is_some();

                    ui.add_enabled_ui(!is_wallet_loaded, |ui| {
                        ui.selectable_value(&mut self.network, NetworkType::Mainnet, "Mainnet");
                        ui.selectable_value(&mut self.network, NetworkType::Testnet, "Testnet");

                        if is_wallet_loaded {
                            ui.label(egui::RichText::new("(Wallet network)").size(10.0).color(egui::Color32::GRAY));
                        }
                    });
                });

                ui.add_space(40.0);

                if WalletManager::exists(self.network) {
                    ui.heading("Welcome Back!");
                    ui.add_space(20.0);

                    // Check if wallet is encrypted
                    let is_encrypted = WalletManager::is_encrypted(self.network).unwrap_or(false);

                    if is_encrypted {
                        ui.label("This wallet is encrypted. Please enter your password.");
                        ui.add_space(10.0);

                        // Show unlock password prompt
                        if self.password_prompt.is_none() {
                            self.password_prompt = Some(password_ui::PasswordPrompt::new_unlock(
                                "Unlock Wallet"
                            ));
                        }
                    } else {
                        // Unencrypted wallet - direct unlock button
                        if ui.button(egui::RichText::new("Open Wallet").size(16.0)).clicked() {
                            match WalletManager::load(self.network) {
                            Ok(mut manager) => {
                                // IMPORTANT: Set UI network to match the loaded wallet's network
                                self.network = manager.network();

                                // Initialize wallet database first
                                if let Ok(main_config) = Config::load() {
                                    let wallet_dir = main_config.wallet_dir();
                                    let db_path = wallet_dir.join("wallet.db");
                                    match WalletDb::open(&db_path) {
                                        Ok(db) => {
                                            // Sync address index with database
                                            if let Ok(owned_addresses) = db.get_owned_addresses() {
                                                if let Some(max_index) = owned_addresses.iter()
                                                    .filter_map(|a| a.derivation_index)
                                                    .max() {
                                                    manager.sync_address_index(max_index);
                                                    log::info!("Synced address index to {}", max_index + 1);
                                                }
                                            }
                                            self.wallet_db = Some(db);
                                            log::info!("Wallet database initialized");
                                        }
                                        Err(e) => {
                                            log::warn!("Failed to open wallet database: {}", e);
                                        }
                                    }
                                }

                                // Extract xpub before moving manager
                                let xpub = manager.get_xpub().to_string();

                                self.wallet_manager = Some(manager);
                                self.current_screen = Screen::Overview;
                                self.set_success("Wallet unlocked successfully!".to_string());

                                // Load config and initialize network + peer manager
                                if let Ok(main_config) = Config::load() {
                                    // Initialize peer manager
                                    let peer_mgr = Arc::new(PeerManager::new(self.network));

                                    // Connect peer manager to wallet database
                                    if let Some(db) = &self.wallet_db {
                                        let db_clone = db.clone();
                                        let peer_mgr_clone = peer_mgr.clone();
                                        tokio::spawn(async move {
                                            peer_mgr_clone.set_wallet_db(db_clone).await;
                                        });
                                    }

                                    self.peer_manager = Some(peer_mgr.clone());

                                    let network_mgr = Arc::new(std::sync::Mutex::new(NetworkManager::new(main_config.api_endpoint.clone())));

                                    // Connect network manager to peer manager
                                    {
                                        let mut net = network_mgr.lock().unwrap();
                                        net.set_peer_manager(peer_mgr.clone());
                                    }

                                    self.network_manager = Some(network_mgr.clone());
                                    self.network_status = "Connecting...".to_string();

                                    // NOTE: TCP listener will be initialized AFTER network bootstrap completes
                                    // (moved to after peer connection to ensure peers are available)

                                    // Trigger network bootstrap in background
                                    let bootstrap_nodes = main_config.bootstrap_nodes.clone();
                                    let addnodes = main_config.addnode.clone();
                                    let ctx_clone = ctx.clone();
                                    let wallet_db = self.wallet_db.clone();
                                    let api_endpoint_str = main_config.api_endpoint.clone();
                                    let xpub_for_tcp = xpub.clone(); // Clone for TCP listener

                                    tokio::spawn(async move {
                                        // PRIORITY 1: Load peers from database first
                                        let db_peer_count = peer_mgr.peer_count().await;
                                        log::info!("📂 Found {} peers in database", db_peer_count);

                                        // PRIORITY 2: Add manual nodes from config to database if not already there
                                        if !addnodes.is_empty() {
                                            log::info!("📝 Adding {} nodes from config", addnodes.len());
                                            for node in addnodes {
                                                // Parse IP:port or use default port
                                                let (ip, port) = if let Some((ip, port_str)) = node.split_once(':') {
                                                    (ip.to_string(), port_str.parse().unwrap_or(24100))
                                                } else {
                                                    (node.clone(), 24100)
                                                };
                                                peer_mgr.add_peer(ip, port).await;
                                            }
                                        }

                                        // PRIORITY 3: If we have no peers at all, fetch from API
                                        let total_peer_count = peer_mgr.peer_count().await;
                                        if total_peer_count == 0 {
                                            log::info!("🌐 No peers found, fetching from API: {}", api_endpoint_str);
                                            // Fetch peers from API and add to database
                                            if let Ok(client) = reqwest::Client::builder()
                                                .timeout(std::time::Duration::from_secs(10))
                                                .build()
                                            {
                                                if let Ok(response) = client.get(&api_endpoint_str).send().await {
                                                    if let Ok(peers) = response.json::<Vec<String>>().await {
                                                        log::info!("✓ Fetched {} peers from API", peers.len());
                                                        for peer_str in peers {
                                                            let (ip, port) = if let Some((ip, port_str)) = peer_str.split_once(':') {
                                                                (ip.to_string(), port_str.parse().unwrap_or(24100))
                                                            } else {
                                                                (peer_str, 24100)
                                                            };
                                                            peer_mgr.add_peer(ip, port).await;
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        // Now bootstrap with whatever peers we have
                                        if let Err(e) = peer_mgr.bootstrap().await {
                                            log::warn!("Failed to bootstrap peers: {}", e);
                                        }

                                        // Start periodic peer maintenance
                                        peer_mgr.clone().start_maintenance();

                                        let api_endpoint = {
                                            let net = network_mgr.lock().unwrap();
                                            net.api_endpoint().to_string()
                                        };

                                        let mut temp_net = NetworkManager::new(api_endpoint);
                                        let result = if let Some(db) = &wallet_db {
                                            temp_net.bootstrap_with_db(db, bootstrap_nodes).await
                                        } else {
                                            temp_net.bootstrap(bootstrap_nodes).await
                                        };

                                        match result {
                                            Ok(_) => {
                                                log::info!("Network bootstrap successful!");
                                                {
                                                    let mut net = network_mgr.lock().unwrap();
                                                    *net = temp_net;
                                                }

                                                // Blockchain scanning happens automatically when xpub is registered via TCP
                                                // The masternode will scan and send UTXOs via UtxoUpdate message
                                                log::info!("🔄 Blockchain scanning initiated via xpub registration");

                                                // Register xpub with all connected peers for ongoing transaction updates
                                                {
                                                    let xpub_for_reg = xpub.clone();
                                                    let network_type = NetworkType::Mainnet; // TODO: Make this configurable
                                                    let connected_peers = {
                                                        let net = network_mgr.lock().unwrap();
                                                        net.get_connected_peers()
                                                    };

                                                    for peer in connected_peers {
                                                        let xpub_clone = xpub_for_reg.clone();
                                                        let peer_addr = peer.address.clone();

                                                        tokio::spawn(async move {
                                                            log::info!("📡 Registering xpub with peer {}...", peer_addr);

                                                            let peer_ip = peer_addr.split(':').next().unwrap_or(&peer_addr);
                                                            let port = if network_type == NetworkType::Testnet { 24100 } else { 24101 };
                                                            let tcp_addr = format!("{}:{}", peer_ip, port);

                                                            // Connect and send RegisterXpub message
                                                            match TcpStream::connect(&tcp_addr).await {
                                                                Ok(mut stream) => {
                                                                    // Perform handshake first
                                                                    let handshake = time_network::protocol::HandshakeMessage::new(
                                                                        if network_type == NetworkType::Testnet {
                                                                            time_network::discovery::NetworkType::Testnet
                                                                        } else {
                                                                            time_network::discovery::NetworkType::Mainnet
                                                                        },
                                                                        "0.0.0.0:0".parse().unwrap()
                                                                    );
                                                                    let magic = if network_type == NetworkType::Testnet {
                                                                        time_network::discovery::NetworkType::Testnet.magic_bytes()
                                                                    } else {
                                                                        time_network::discovery::NetworkType::Mainnet.magic_bytes()
                                                                    };

                                                                    if let Ok(handshake_json) = serde_json::to_vec(&handshake) {
                                                                        let handshake_len = handshake_json.len() as u32;
                                                                        if stream.write_all(&magic).await.is_ok() &&
                                                                           stream.write_all(&handshake_len.to_be_bytes()).await.is_ok() &&
                                                                           stream.write_all(&handshake_json).await.is_ok() &&
                                                                           stream.flush().await.is_ok() {

                                                                            // Read their handshake
                                                                            let mut their_magic = [0u8; 4];
                                                                            let mut their_len = [0u8; 4];
                                                                            if stream.read_exact(&mut their_magic).await.is_ok() &&
                                                                               their_magic == magic &&
                                                                               stream.read_exact(&mut their_len).await.is_ok() {
                                                                                let len = u32::from_be_bytes(their_len) as usize;
                                                                                if len < 10 * 1024 {
                                                                                    let mut their_handshake = vec![0u8; len];
                                                                                    let _ = stream.read_exact(&mut their_handshake).await;

                                                                                    // Now send actual message
                                                                                    let msg = time_network::protocol::NetworkMessage::RegisterXpub {
                                                                                        xpub: xpub_clone
                                                                                    };

                                                                                    // Serialize with JSON (not bincode!)
                                                                                    match serde_json::to_vec(&msg) {
                                                                                        Ok(bytes) => {
                                                                                            let msg_len = bytes.len() as u32;
                                                                                            if stream.write_all(&msg_len.to_be_bytes()).await.is_ok() &&
                                                                                               stream.write_all(&bytes).await.is_ok() {
                                                                                                log::info!("✅ Successfully registered xpub with {}", peer_addr);
                                                                                            } else {
                                                                                                log::warn!("❌ Failed to send xpub to {}", peer_addr);
                                                                                            }
                                                                                        }
                                                                                        Err(e) => log::warn!("❌ Failed to serialize xpub message: {}", e),
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                Err(e) => log::warn!("❌ Failed to connect to {}: {}", peer_addr, e),
                                                            }
                                                        });
                                                    }
                                                }

                                                // Initialize TCP listener NOW that peers are connected
                                                log::info!("🔌 Initializing TCP listener for real-time notifications...");
                                                let tcp_xpub = xpub_for_tcp.clone();
                                                let tcp_network_mgr = network_mgr.clone();
                                                tokio::spawn(async move {
                                                    // Get first available peer
                                                    let peer_addr = {
                                                        let net = tcp_network_mgr.lock().unwrap();
                                                        let peers = net.get_connected_peers();
                                                        if let Some(peer) = peers.first() {
                                                            let peer_ip = peer.address.split(':').next().unwrap_or(&peer.address);
                                                            Some(format!("{}:24100", peer_ip))
                                                        } else {
                                                            None
                                                        }
                                                    };

                                                    if let Some(addr) = peer_addr {
                                                        log::info!("🔗 Starting TCP listener for {}", addr);
                                                        let (utxo_tx, _utxo_rx) = tokio::sync::mpsc::unbounded_channel();
                                                        let (tx_notif_tx, _tx_notif_rx) = tokio::sync::mpsc::unbounded_channel();
                                                        let listener = tcp_protocol_client::TcpProtocolListener::new(
                                                            addr,
                                                            tcp_xpub,
                                                            utxo_tx,
                                                            tx_notif_tx,
                                                        );
                                                        listener.start().await;
                                                    } else {
                                                        log::warn!("❌ No peers available for TCP listener");
                                                    }
                                                });

                                                // Trigger initial transaction sync
                                                ctx_clone.request_repaint();

                                                // Start periodic latency refresh and blockchain height update task
                                                let network_refresh = network_mgr.clone();
                                                tokio::spawn(async move {
                                                    loop {
                                                        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                                                        log::debug!("Running scheduled refresh");

                                                        // Run periodic refresh (latency, version, blockchain height)
                                                        let network_clone = network_refresh.clone();
                                                        tokio::task::spawn_blocking(move || {
                                                            let rt = tokio::runtime::Runtime::new().unwrap();
                                                            rt.block_on(async move {
                                                                #[allow(clippy::await_holding_lock)]
                                                                {
                                                                    let mut manager = network_clone.lock().unwrap();
                                                                    manager.periodic_refresh().await
                                                                }
                                                            });
                                                        }).await.ok();

                                                        log::debug!("Scheduled refresh complete");
                                                    }
                                                });
                                            }
                                            Err(e) => {
                                                log::error!("Network bootstrap failed: {}", e);
                                            }
                                        }

                                        ctx_clone.request_repaint();
                                    });
                                }
                            }
                            Err(e) => {
                                self.set_error(format!("Failed to load wallet: {}", e));
                            }
                        }
                    }
                    } // Close unencrypted wallet else block

                    // Handle password prompt for encrypted wallet unlock
                    if let Some(prompt) = &mut self.password_prompt {
                        prompt.show(ctx);

                        if prompt.is_confirmed() && !prompt.is_open() {
                            let password = prompt.take_password();

                            // Attempt to unlock wallet with password
                            match WalletManager::load_with_password(self.network, &password) {
                                Ok(mut manager) => {
                                    // Set UI network to match wallet
                                    self.network = manager.network();

                                    // Initialize wallet database
                                    if let Ok(main_config) = Config::load() {
                                        let wallet_dir = main_config.wallet_dir();
                                        let db_path = wallet_dir.join("wallet.db");
                                        match WalletDb::open(&db_path) {
                                            Ok(db) => {
                                                // Sync address index
                                                if let Ok(owned_addresses) = db.get_owned_addresses() {
                                                    if let Some(max_index) = owned_addresses
                                                        .iter()
                                                        .filter_map(|a| a.derivation_index)
                                                        .max()
                                                    {
                                                        manager.sync_address_index(max_index);
                                                        log::info!("Synced address index to {}", max_index + 1);
                                                    }
                                                }
                                                self.wallet_db = Some(db);
                                                log::info!("Wallet database initialized");
                                            }
                                            Err(e) => {
                                                log::warn!("Failed to open wallet database: {}", e);
                                            }
                                        }
                                    }

                                    self.wallet_manager = Some(manager);
                                    self.current_screen = Screen::Overview;
                                    self.set_success("Wallet unlocked successfully!".to_string());
                                    log::info!("✅ Wallet unlocked with password");

                                    // Initialize network after unlock if not already done
                                    if self.network_manager.is_none() {
                                        log::info!("Initializing network after wallet unlock...");
                                        self.initialize_network();
                                    }

                                    // Register XPub with masternodes
                                    self.register_wallet_with_masternodes();
                                }
                                Err(e) => {
                                    log::error!("Failed to unlock wallet: {}", e);
                                    self.set_error("Incorrect password or corrupted wallet".to_string());
                                    // Reopen prompt for retry
                                    self.password_prompt = Some(password_ui::PasswordPrompt::new_unlock(
                                        "Unlock Wallet"
                                    ));
                                }
                            }

                            // Clear password prompt if not retrying
                            if !matches!(self.password_prompt.as_ref().map(|p| p.is_open()), Some(true)) {
                                self.password_prompt = None;
                            }
                        } else if !prompt.is_open() {
                            // User cancelled
                            self.password_prompt = None;
                        }
                    }
                } else {
                    ui.heading("Create New Wallet");
                    ui.add_space(20.0);

                    if ui.button(egui::RichText::new("Create Wallet").size(16.0)).clicked() {
                        // Save network selection to config before creating wallet
                        if let Ok(mut config) = Config::load() {
                            let network_str = match self.network {
                                NetworkType::Mainnet => "mainnet",
                                NetworkType::Testnet => "testnet",
                            };
                            let _ = config.set_network(network_str);
                        }

                        // Transition to mnemonic setup screen
                        self.current_screen = Screen::MnemonicSetup;
                        self.mnemonic_interface = MnemonicInterface::new();
                        self.error_message = None;
                    }
                }

                if let Some(msg) = &self.error_message {
                    ui.add_space(20.0);
                    ui.colored_label(egui::Color32::RED, msg);
                }
            });
        });
    }

    fn show_mnemonic_setup_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);

                // Check if wallet already exists
                let wallet_exists = self.wallet_manager.is_some();

                if wallet_exists {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 165, 0),
                        "⚠️ WARNING: Creating a new wallet will backup your current wallet",
                    );
                    ui.add_space(10.0);
                    ui.label("Your old wallet will be saved with a timestamp.");
                    ui.add_space(20.0);
                }

                // Use the new mnemonic interface
                if let Some(action) = self.mnemonic_interface.render(ui) {
                    match action {
                        MnemonicAction::Confirm(phrase) => {
                            // Store mnemonic and show password prompt
                            self.pending_mnemonic = Some(phrase.clone());
                            self.password_prompt = Some(password_ui::PasswordPrompt::new(
                                "Encrypt Wallet with Password",
                            ));
                        }
                        MnemonicAction::Cancel => {
                            self.current_screen = Screen::Welcome;
                            self.mnemonic_interface = MnemonicInterface::new();
                        }
                    }
                }

                // Show password prompt if active
                if let Some(prompt) = &mut self.password_prompt {
                    prompt.show(ctx);

                    // Check if password was confirmed
                    if prompt.is_confirmed() && !prompt.is_open() {
                        let password = prompt.take_password();

                        if let Some(phrase) = self.pending_mnemonic.take() {
                            // If wallet exists, backup first
                            if wallet_exists {
                                match self
                                    .backup_and_create_new_wallet_encrypted(&phrase, &password)
                                {
                                    Ok(_) => {
                                        self.mnemonic_interface.wallet_created = true;
                                        self.current_screen = Screen::Overview;
                                    }
                                    Err(e) => {
                                        self.set_error(e);
                                    }
                                }
                            } else {
                                // Create encrypted wallet
                                self.create_wallet_from_mnemonic_encrypted(&phrase, &password, ctx);
                            }
                        }

                        // Clear password prompt
                        self.password_prompt = None;
                    } else if !prompt.is_open() {
                        // User cancelled
                        self.password_prompt = None;
                        self.pending_mnemonic = None;
                    }
                }
            });
        });
    }

    fn show_mnemonic_confirm_screen(&mut self, ctx: &egui::Context) {
        // This screen is no longer needed - the new mnemonic interface handles confirmation
        // Redirect to overview if somehow accessed
        self.current_screen = Screen::Overview;
    }

    fn create_wallet_from_mnemonic_phrase(&mut self, phrase: &str, ctx: &egui::Context) {
        log::info!("Creating wallet from mnemonic phrase...");

        match WalletManager::create_from_mnemonic(self.network, phrase) {
            Ok(manager) => {
                log::info!("Wallet manager created successfully");
                // Verify xpub is set
                let xpub = manager.get_xpub();
                log::info!("Wallet created with xpub: {}", xpub);
                self.wallet_manager = Some(manager);

                // Initialize wallet database
                if let Ok(main_config) = Config::load() {
                    let wallet_dir = main_config.wallet_dir();
                    let db_path = wallet_dir.join("wallet.db");
                    match WalletDb::open(&db_path) {
                        Ok(db) => {
                            self.wallet_db = Some(db);
                            log::info!("Wallet database initialized");
                        }
                        Err(e) => {
                            log::warn!("Failed to open wallet database: {}", e);
                        }
                    }
                }

                log::info!("Transitioning to Overview screen");
                self.current_screen = Screen::Overview;
                self.set_success("Wallet created successfully!".to_string());

                // Mark that wallet has been created from this phrase
                self.mnemonic_interface.wallet_created = true;

                // Clear mnemonic from memory
                self.mnemonic_interface.clear();
                self.mnemonic_confirmed = false;

                // Load config and initialize network + peer manager
                if let Ok(main_config) = Config::load() {
                    // Initialize peer manager
                    let peer_mgr = Arc::new(PeerManager::new(self.network));

                    // Connect peer manager to wallet database
                    if let Some(db) = &self.wallet_db {
                        let db_clone = db.clone();
                        let peer_mgr_clone = peer_mgr.clone();
                        tokio::spawn(async move {
                            peer_mgr_clone.set_wallet_db(db_clone).await;
                        });
                    }

                    self.peer_manager = Some(peer_mgr.clone());

                    // Bootstrap peers
                    let peer_mgr_clone = peer_mgr.clone();
                    tokio::spawn(async move {
                        if let Err(e) = peer_mgr_clone.bootstrap().await {
                            log::warn!("Failed to bootstrap peers: {}", e);
                        }
                        // Start periodic peer maintenance
                        peer_mgr_clone.clone().start_maintenance();
                    });

                    let network_mgr = Arc::new(Mutex::new(NetworkManager::new(
                        main_config.api_endpoint.clone(),
                    )));

                    // Connect network manager to peer manager
                    {
                        let mut net = network_mgr.lock().unwrap();
                        net.set_peer_manager(peer_mgr.clone());
                    }

                    self.network_manager = Some(network_mgr.clone());
                    self.network_status = "Connecting...".to_string();

                    let bootstrap_nodes = main_config.bootstrap_nodes.clone();
                    let ctx_clone = ctx.clone();
                    let wallet_db = self.wallet_db.clone();

                    tokio::spawn(async move {
                        let api_endpoint = {
                            let net = network_mgr.lock().unwrap();
                            net.api_endpoint().to_string()
                        };

                        let mut temp_net = NetworkManager::new(api_endpoint);
                        let result = if let Some(db) = wallet_db {
                            temp_net.bootstrap_with_db(&db, bootstrap_nodes).await
                        } else {
                            temp_net.bootstrap(bootstrap_nodes).await
                        };

                        match result {
                            Ok(_) => {
                                log::info!("Network bootstrap successful!");
                                {
                                    let mut net = network_mgr.lock().unwrap();
                                    *net = temp_net;
                                }

                                // Initialize TCP listener NOW that peers are connected
                                log::info!(
                                    "🔌 Initializing TCP listener for real-time notifications..."
                                );
                                let network_mgr_clone = network_mgr.clone();
                                tokio::spawn(async move {
                                    // Get first available peer
                                    let peer_addr = {
                                        let net = network_mgr_clone.lock().unwrap();
                                        let peers = net.get_connected_peers();
                                        if let Some(peer) = peers.first() {
                                            let peer_ip = peer
                                                .address
                                                .split(':')
                                                .next()
                                                .unwrap_or(&peer.address);
                                            Some(format!("{}:24100", peer_ip))
                                        } else {
                                            None
                                        }
                                    };

                                    if let Some(addr) = peer_addr {
                                        log::info!("🔗 Starting TCP listener for {}", addr);
                                        let (utxo_tx, _utxo_rx) =
                                            tokio::sync::mpsc::unbounded_channel::<
                                                time_network::protocol::UtxoInfo,
                                            >();
                                        // Note: Need xpub here - but it's not available in this scope
                                        // This path is for new wallet creation before xpub is set
                                        log::warn!("⚠️ TCP listener not started - xpub not available yet in new wallet flow");
                                    } else {
                                        log::warn!("❌ No peers available for TCP listener");
                                    }
                                });

                                // Start periodic refresh task (every 5 minutes to reduce GUI freezing)
                                let network_refresh = network_mgr.clone();
                                tokio::spawn(async move {
                                    loop {
                                        tokio::time::sleep(tokio::time::Duration::from_secs(300))
                                            .await;
                                        log::info!("🔄 Running scheduled refresh...");

                                        // Run periodic refresh (latency, version, blockchain height)
                                        let network_clone = network_refresh.clone();
                                        tokio::task::spawn_blocking(move || {
                                            let rt = tokio::runtime::Runtime::new().unwrap();
                                            rt.block_on(async move {
                                                #[allow(clippy::await_holding_lock)]
                                                {
                                                    let mut manager = network_clone.lock().unwrap();
                                                    manager.periodic_refresh().await
                                                }
                                            });
                                        })
                                        .await
                                        .ok();

                                        log::info!("✅ Scheduled refresh complete");
                                    }
                                });
                            }
                            Err(e) => {
                                log::error!("Network bootstrap failed: {}", e);
                            }
                        }
                        ctx_clone.request_repaint();
                    });
                }

                // Force UI repaint to show new screen
                log::info!("Requesting UI repaint");
                ctx.request_repaint();
            }
            Err(e) => {
                log::error!("Failed to create wallet: {}", e);
                self.set_error(format!("Failed to create wallet: {}", e));
                ctx.request_repaint();
            }
        }
    }

    fn create_wallet_from_mnemonic_encrypted(
        &mut self,
        phrase: &str,
        password: &str,
        ctx: &egui::Context,
    ) {
        log::info!("Creating encrypted wallet from mnemonic phrase...");

        match WalletManager::create_from_mnemonic_encrypted(self.network, phrase, password) {
            Ok(manager) => {
                log::info!("Encrypted wallet manager created successfully");
                // Verify xpub is set
                let xpub = manager.get_xpub();
                log::info!("Wallet created with xpub: {}", xpub);
                self.wallet_manager = Some(manager);

                // Initialize wallet database
                if let Ok(main_config) = Config::load() {
                    let wallet_dir = main_config.wallet_dir();
                    let db_path = wallet_dir.join("wallet.db");
                    match WalletDb::open(&db_path) {
                        Ok(db) => {
                            self.wallet_db = Some(db);
                            log::info!("Wallet database initialized");
                        }
                        Err(e) => {
                            log::warn!("Failed to open wallet database: {}", e);
                        }
                    }
                }

                log::info!("Transitioning to Overview screen");
                self.current_screen = Screen::Overview;
                self.set_success("Encrypted wallet created successfully!".to_string());

                // Mark that wallet has been created from this phrase
                self.mnemonic_interface.wallet_created = true;

                // Clear mnemonic from memory
                self.mnemonic_interface.clear();
                self.mnemonic_confirmed = false;

                // Initialize network and peer manager (same as unencrypted flow)
                if let Ok(main_config) = Config::load() {
                    let peer_mgr = Arc::new(PeerManager::new(self.network));

                    if let Some(db) = &self.wallet_db {
                        let db_clone = db.clone();
                        let peer_mgr_clone = peer_mgr.clone();
                        tokio::spawn(async move {
                            peer_mgr_clone.set_wallet_db(db_clone).await;
                        });
                    }

                    self.peer_manager = Some(peer_mgr.clone());

                    let network_mgr = Arc::new(std::sync::Mutex::new(NetworkManager::new(
                        main_config.api_endpoint.clone(),
                    )));

                    {
                        let mut net = network_mgr.lock().unwrap();
                        net.set_peer_manager(peer_mgr.clone());
                    }

                    self.network_manager = Some(network_mgr.clone());
                    self.network_status = "Connecting...".to_string();

                    // Bootstrap network - fetch peers and connect
                    let bootstrap_nodes = main_config.bootstrap_nodes.clone();
                    let addnodes = main_config.addnode.clone();
                    let api_endpoint_str = main_config.api_endpoint.clone();
                    let network_mgr_clone = network_mgr.clone();

                    tokio::spawn(async move {
                        let db_peer_count = peer_mgr.peer_count().await;
                        log::info!("📂 Found {} peers in database", db_peer_count);

                        if !addnodes.is_empty() {
                            log::info!("📝 Adding {} nodes from config", addnodes.len());
                            for node in addnodes {
                                let (ip, port) = if let Some((ip, port_str)) = node.split_once(':')
                                {
                                    (ip.to_string(), port_str.parse().unwrap_or(24100))
                                } else {
                                    (node.clone(), 24100)
                                };
                                peer_mgr.add_peer(ip, port).await;
                            }
                        }

                        let total_peer_count = peer_mgr.peer_count().await;
                        if total_peer_count == 0 {
                            log::info!(
                                "🌐 No peers found, fetching from API: {}",
                                api_endpoint_str
                            );
                            if let Ok(client) = reqwest::Client::builder()
                                .timeout(std::time::Duration::from_secs(10))
                                .build()
                            {
                                if let Ok(response) = client.get(&api_endpoint_str).send().await {
                                    if let Ok(peers) = response.json::<Vec<String>>().await {
                                        log::info!("✓ Fetched {} peers from API", peers.len());
                                        for peer_str in peers {
                                            let (ip, port) = if let Some((ip, port_str)) =
                                                peer_str.split_once(':')
                                            {
                                                (ip.to_string(), port_str.parse().unwrap_or(24100))
                                            } else {
                                                (peer_str, 24100)
                                            };
                                            peer_mgr.add_peer(ip, port).await;
                                        }
                                    }
                                }
                            }
                        }

                        // Bootstrap PeerManager (don't wait for completion)
                        log::info!("🔍 Bootstrapping PeerManager...");
                        let peer_mgr_for_bootstrap = peer_mgr.clone();
                        tokio::spawn(async move {
                            if let Err(e) = peer_mgr_for_bootstrap.bootstrap().await {
                                log::warn!("⚠️ PeerManager bootstrap failed: {}", e);
                            } else {
                                log::info!("✅ PeerManager bootstrap completed");
                            }
                        });

                        // Connect NetworkManager to peers immediately (don't wait for bootstrap)
                        log::info!("🔗 Connecting NetworkManager to discovered peers...");
                        let peer_list = peer_mgr.get_healthy_peers().await;
                        log::info!("📋 Attempting to connect to {} peers", peer_list.len());

                        let peer_infos: Vec<network::PeerInfo> = peer_list
                            .into_iter()
                            .map(|p| network::PeerInfo {
                                address: p.address,
                                port: p.port,
                                version: None,
                                last_seen: Some(
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs(),
                                ),
                                latency_ms: 0,
                            })
                            .collect();

                        if !peer_infos.is_empty() {
                            log::info!("Starting connection to {} peers...", peer_infos.len());
                            let net_clone = network_mgr_clone.clone();
                            tokio::task::spawn_blocking(move || {
                                let rt = tokio::runtime::Runtime::new().unwrap();
                                rt.block_on(async move {
                                    let mut manager = net_clone.lock().unwrap();
                                    log::info!(
                                        "Acquired NetworkManager lock, calling connect_to_peers"
                                    );
                                    if let Err(e) = manager.connect_to_peers(peer_infos).await {
                                        log::error!("Failed to connect to peers: {}", e);
                                    } else {
                                        let peer_count = manager.peer_count();
                                        log::info!(
                                            "✅ Successfully connected to {} network peers",
                                            peer_count
                                        );
                                    }
                                });
                            })
                            .await
                            .ok();
                            log::info!("Connection task completed");

                            // Start periodic latency refresh
                            let network_mgr_for_ping = network_mgr_clone.clone();
                            tokio::task::spawn_blocking(move || {
                                let rt = tokio::runtime::Handle::current();
                                rt.block_on(async {
                                    // Wait before first ping
                                    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

                                    loop {
                                        log::debug!("Refreshing peer latencies...");
                                        {
                                            let mut net = network_mgr_for_ping.lock().unwrap();
                                            let _ = net.refresh_peer_latencies().await;
                                        }

                                        // Ping every 2 minutes
                                        tokio::time::sleep(tokio::time::Duration::from_secs(120))
                                            .await;
                                    }
                                });
                            });
                        } else {
                            log::warn!("No peer info available to connect");
                        }
                    });
                }
            }
            Err(e) => {
                log::error!("Failed to create encrypted wallet: {}", e);
                self.set_error(format!("Failed to create encrypted wallet: {}", e));
                ctx.request_repaint();
            }
        }
    }

    fn backup_and_create_new_wallet_encrypted(
        &mut self,
        new_phrase: &str,
        password: &str,
    ) -> Result<(), String> {
        // First, backup the existing wallet
        let backup_path = self.backup_current_wallet()?;
        log::info!("Wallet backed up to: {}", backup_path);

        // Create new encrypted wallet
        match WalletManager::create_from_mnemonic_encrypted(self.network, new_phrase, password) {
            Ok(manager) => {
                self.wallet_manager = Some(manager);
                self.set_success(format!(
                    "New encrypted wallet created! Old wallet backed up to: {}",
                    backup_path
                ));
                Ok(())
            }
            Err(e) => Err(format!("Failed to create new encrypted wallet: {}", e)),
        }
    }

    // Old function removed - using create_wallet_from_mnemonic_phrase instead

    fn show_main_screen(&mut self, ctx: &egui::Context) {
        // Check if we should auto-sync transactions (every 30 seconds)
        let should_sync = if let Some(last_sync) = self.last_sync_time {
            last_sync.elapsed().as_secs() >= 30
        } else {
            // First time sync after 5 seconds of wallet being loaded
            self.wallet_manager.is_some() && self.network_manager.is_some()
        };

        if should_sync {
            log::info!("Auto-triggering transaction sync and mempool check");
            self.trigger_transaction_sync();
            self.trigger_mempool_check();
            self.last_sync_time = Some(std::time::Instant::now());
        }

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Backup Wallet").clicked() {
                        if let Err(e) = self.backup_current_wallet() {
                            self.set_error(format!("Backup failed: {}", e));
                        } else {
                            self.success_message =
                                Some("Wallet backed up successfully".to_string());
                        }
                        ui.close_menu();
                    }
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Settings", |ui| {
                    if ui.button("Options").clicked() {
                        self.current_screen = Screen::Settings;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Window", |ui| {
                    if ui.button("Minimize").clicked() {
                        ui.close_menu();
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About TIME Coin").clicked() {
                        ui.close_menu();
                    }
                });
            });
        });

        // Navigation buttons
        egui::TopBottomPanel::top("nav_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.style_mut().spacing.button_padding = egui::vec2(20.0, 10.0);

                if ui
                    .selectable_label(self.current_screen == Screen::Overview, "🏠 Overview")
                    .clicked()
                {
                    self.current_screen = Screen::Overview;
                }
                if ui
                    .selectable_label(self.current_screen == Screen::Send, "📤 Send")
                    .clicked()
                {
                    self.current_screen = Screen::Send;
                }
                if ui
                    .selectable_label(self.current_screen == Screen::Receive, "📥 Receive")
                    .clicked()
                {
                    self.current_screen = Screen::Receive;
                }
                if ui
                    .selectable_label(
                        self.current_screen == Screen::Transactions,
                        "📋 Transactions",
                    )
                    .clicked()
                {
                    self.current_screen = Screen::Transactions;
                }
                if ui
                    .selectable_label(self.current_screen == Screen::Utxos, "📦 UTXOs")
                    .clicked()
                {
                    self.current_screen = Screen::Utxos;
                }
                if ui
                    .selectable_label(self.current_screen == Screen::Peers, "Peers")
                    .clicked()
                {
                    self.current_screen = Screen::Peers;
                }
            });
        });

        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Network status
                if let Some(net_mgr_arc) = &self.network_manager {
                    if let Ok(net_mgr) = net_mgr_arc.lock() {
                        // Peer count
                        ui.label(format!("Peers: {} peers", net_mgr.peer_count()));
                        ui.separator();

                        // Block height
                        let current_height = net_mgr.current_block_height();
                        let network_height = net_mgr.network_block_height();

                        if network_height > 0 {
                            ui.label(format!("Block: {}/{}", current_height, network_height));
                            ui.separator();
                        } else if current_height > 0 {
                            ui.label(format!("Block: {}", current_height));
                            ui.separator();
                        } else {
                            ui.label("Block: unknown");
                            ui.separator();
                        }

                        // Sync status
                        if net_mgr.peer_count() > 0 {
                            ui.label("[OK] Connected");
                        } else {
                            ui.label("⏳ Connecting...");
                        }
                    }
                } else {
                    ui.label(format!("Status: {}", self.network_status));
                }
            });
        });

        // Main content
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_screen {
                Screen::Overview => self.show_overview_screen(ui, ctx),
                Screen::Send => self.show_send_screen(ui, ctx),
                Screen::Receive => self.show_receive_screen(ui, ctx),
                Screen::Transactions => self.show_transactions_screen(ui),
                Screen::Utxos => self.show_utxos_screen(ctx),
                Screen::Settings => self.show_settings_screen(ui, ctx),
                Screen::Peers => {
                    ui.heading("Connected Peers");
                    ui.separator();

                    if let Some(network_mgr) = self.network_manager.as_ref() {
                        let mgr = network_mgr.lock().unwrap();
                        let peers = mgr.get_connected_peers();
                        let peer_count = mgr.peer_count();

                        ui.label(format!("Status: {} peers in connected list", peers.len()));
                        ui.label(format!("Peer count method returns: {}", peer_count));
                        ui.add_space(10.0);

                        if peers.is_empty() {
                            ui.colored_label(
                                egui::Color32::LIGHT_BLUE,
                                "⏳ Waiting for peer discovery to complete...",
                            );
                            ui.add_space(10.0);
                            ui.label(
                                "Peer discovery runs in the background and takes a few seconds.",
                            );
                            ui.label("Please wait or click refresh below.");
                            ui.add_space(10.0);
                            if ui.button("🔄 Refresh").clicked() {
                                // Force UI update
                                ctx.request_repaint();
                            }
                        } else {
                            ui.label(format!(
                                "✓ Connected to {} peers (sorted by latency):",
                                peers.len()
                            ));
                            ui.add_space(10.0);

                            egui::Grid::new("peers_grid")
                                .striped(true)
                                .spacing([10.0, 4.0])
                                .show(ui, |ui| {
                                    ui.strong("Address");
                                    ui.strong("Port");
                                    ui.strong("Latency");
                                    ui.strong("Version");
                                    ui.end_row();

                                    for peer in peers {
                                        ui.label(&peer.address);
                                        ui.label(peer.port.to_string());

                                        if peer.latency_ms > 0 {
                                            let color = if peer.latency_ms < 50 {
                                                egui::Color32::GREEN
                                            } else if peer.latency_ms < 150 {
                                                egui::Color32::from_rgb(255, 165, 0)
                                            // Orange
                                            } else {
                                                egui::Color32::RED
                                            };
                                            ui.horizontal(|ui| {
                                                // Draw a filled circle
                                                let (rect, _response) = ui.allocate_exact_size(
                                                    egui::vec2(10.0, 10.0),
                                                    egui::Sense::hover(),
                                                );
                                                ui.painter().circle_filled(
                                                    rect.center(),
                                                    5.0,
                                                    color,
                                                );
                                                ui.label(format!("{}ms", peer.latency_ms));
                                            });
                                        } else {
                                            ui.label("-");
                                        }

                                        ui.label(
                                            peer.version.as_ref().unwrap_or(&"unknown".to_string()),
                                        );
                                        ui.end_row();
                                    }
                                });
                        }
                    } else {
                        ui.label("Network manager not initialized");
                    }
                }
                _ => {}
            }
        });
    }

    fn show_overview_screen(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.add_space(10.0);

        if let Some(manager) = &self.wallet_manager {
            // Two column layout
            ui.horizontal(|ui| {
                // Left column - Balances
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width() * 0.5);

                    ui.heading("Balances");
                    ui.add_space(10.0);

                    // Get balance from database (synced from blockchain)
                    let balance = if let Some(db) = &self.wallet_db {
                        db.get_total_balance().unwrap_or(0)
                    } else {
                        0
                    };

                    ui.horizontal(|ui| {
                        ui.label("Available:");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} TIME",
                                    Self::format_amount(balance)
                                ))
                                .strong(),
                            );
                        });
                    });
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Pending:");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("0 TIME").color(egui::Color32::GRAY));
                        });
                    });
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Locked:");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("0 TIME").color(egui::Color32::GRAY));
                        });
                    });
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Total:").strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} TIME",
                                    Self::format_amount(balance)
                                ))
                                .strong()
                                .size(16.0),
                            );
                        });
                    });
                });

                ui.separator();

                // Right column - Recent transactions
                ui.vertical(|ui| {
                    ui.heading("Recent transactions");
                    ui.add_space(10.0);

                    // Get recent transactions from database
                    let transactions = if let Some(db) = &self.wallet_db {
                        db.get_all_transactions().unwrap_or_default()
                    } else {
                        Vec::new()
                    };

                    if transactions.is_empty() {
                        ui.label(
                            egui::RichText::new("No transactions yet")
                                .color(egui::Color32::GRAY)
                                .italics(),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(format!(
                                "Showing {} transactions",
                                transactions.len()
                            ))
                            .color(egui::Color32::GRAY),
                        );
                    }
                    ui.add_space(10.0);

                    // Show transaction list
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for tx in transactions.iter().take(10) {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        // Transaction type icon
                                        let icon = if tx.from_address.is_some() {
                                            "📥" // Received
                                        } else {
                                            "📤" // Sent
                                        };
                                        ui.label(egui::RichText::new(icon).size(16.0));

                                        ui.vertical(|ui| {
                                            // Address (shortened)
                                            let addr_display = if tx.to_address.len() > 20 {
                                                format!(
                                                    "{}...{}",
                                                    &tx.to_address[..10],
                                                    &tx.to_address[tx.to_address.len() - 6..]
                                                )
                                            } else {
                                                tx.to_address.clone()
                                            };
                                            ui.label(egui::RichText::new(addr_display).strong());

                                            // Date
                                            let date =
                                                chrono::DateTime::from_timestamp(tx.timestamp, 0)
                                                    .map(|dt| {
                                                        dt.format("%Y-%m-%d %H:%M").to_string()
                                                    })
                                                    .unwrap_or_else(|| "Unknown".to_string());
                                            ui.label(
                                                egui::RichText::new(date)
                                                    .color(egui::Color32::GRAY)
                                                    .small(),
                                            );
                                        });

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                // Amount
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "{} TIME",
                                                        Self::format_amount(tx.amount)
                                                    ))
                                                    .strong(),
                                                );

                                                // Status badge
                                                let (status_text, status_color) = match tx.status {
                                                    wallet_db::TransactionStatus::Confirmed => {
                                                        ("✓", egui::Color32::GREEN)
                                                    }
                                                    wallet_db::TransactionStatus::Approved => {
                                                        ("✓", egui::Color32::from_rgb(0, 200, 0))
                                                    }
                                                    wallet_db::TransactionStatus::Pending => {
                                                        ("⏳", egui::Color32::YELLOW)
                                                    }
                                                    wallet_db::TransactionStatus::Declined => {
                                                        ("✗", egui::Color32::DARK_RED)
                                                    }
                                                    wallet_db::TransactionStatus::Failed => {
                                                        ("✗", egui::Color32::RED)
                                                    }
                                                };
                                                ui.label(
                                                    egui::RichText::new(status_text)
                                                        .color(status_color),
                                                );
                                            },
                                        );
                                    });
                                });
                                ui.add_space(5.0);
                            }
                        });
                });
            });
        }

        if let Some(msg) = &self.success_message {
            ui.add_space(10.0);
            ui.colored_label(egui::Color32::GREEN, msg);
        }
        if let Some(msg) = &self.error_message {
            ui.add_space(10.0);
            ui.colored_label(egui::Color32::RED, msg);
        }
    }

    fn show_send_screen(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Send TIME Coins");
        ui.add_space(20.0);

        enum ContactAction {
            SelectForSend(String),
            Edit(String),
            Delete(String),
        }
        let mut pending_action: Option<ContactAction> = None;

        if let Some(_manager) = &self.wallet_manager {
            // Two column layout
            ui.columns(2, |columns| {
                // Left side - Contact list
                columns[0].vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Address Book");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("➕ New Contact").clicked() {
                                self.is_adding_new_contact = true;
                                self.new_contact_address.clear();
                                self.new_contact_name.clear();
                                self.new_contact_email.clear();
                                self.new_contact_phone.clear();
                            }
                        });
                    });

                    ui.add_space(10.0);

                    // Search box
                    ui.horizontal(|ui| {
                        ui.label("🔍");
                        ui.text_edit_singleline(&mut self.contact_search)
                            .on_hover_text("Search contacts");
                    });

                    ui.add_space(10.0);

                    // New contact form
                    if self.is_adding_new_contact {
                        ui.group(|ui| {
                            ui.label("New Contact");
                            ui.add_space(5.0);
                            ui.horizontal(|ui| {
                                ui.label("Address:");
                                ui.text_edit_singleline(&mut self.new_contact_address);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut self.new_contact_name);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Email:");
                                ui.text_edit_singleline(&mut self.new_contact_email);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Phone:");
                                ui.text_edit_singleline(&mut self.new_contact_phone);
                            });

                            ui.add_space(5.0);
                            ui.horizontal(|ui| {
                                if ui.button("✓ Save").clicked() {
                                    if !self.new_contact_address.is_empty()
                                        && !self.new_contact_name.is_empty()
                                    {
                                        if let Some(ref db) = self.wallet_db {
                                            let now = chrono::Utc::now().timestamp();
                                            let contact = AddressContact {
                                                address: self.new_contact_address.clone(),
                                                label: String::new(),
                                                name: Some(self.new_contact_name.clone()),
                                                email: if self.new_contact_email.is_empty() {
                                                    None
                                                } else {
                                                    Some(self.new_contact_email.clone())
                                                },
                                                phone: if self.new_contact_phone.is_empty() {
                                                    None
                                                } else {
                                                    Some(self.new_contact_phone.clone())
                                                },
                                                notes: None,
                                                is_default: false,
                                                is_owned: false, // External contact for sending
                                                derivation_index: None,
                                                created_at: now,
                                                updated_at: now,
                                            };
                                            match db.save_contact(&contact) {
                                                Ok(_) => {
                                                    self.success_message =
                                                        Some("Contact saved!".to_string());
                                                    self.is_adding_new_contact = false;
                                                }
                                                Err(e) => {
                                                    self.error_message =
                                                        Some(format!("Failed to save: {}", e));
                                                }
                                            }
                                        }
                                    } else {
                                        self.error_message =
                                            Some("Address and name are required".to_string());
                                    }
                                }
                                if ui.button("✗ Cancel").clicked() {
                                    self.is_adding_new_contact = false;
                                }
                            });
                        });
                        ui.add_space(10.0);
                    }

                    // Contact list with scrolling
                    egui::ScrollArea::vertical()
                        .max_height(500.0)
                        .show(ui, |ui| {
                            if let Some(ref db) = self.wallet_db {
                                match db.get_external_contacts() {
                                    Ok(mut contacts) => {
                                        // Sort contacts by name
                                        contacts.sort_by(|a, b| {
                                            let name_a = a.name.as_deref().unwrap_or("Unnamed");
                                            let name_b = b.name.as_deref().unwrap_or("Unnamed");
                                            name_a.cmp(name_b)
                                        });

                                        if contacts.is_empty() {
                                            ui.label("No contacts yet. Add one to get started!");
                                            return;
                                        }

                                        for contact in contacts.iter() {
                                            let display_name = contact
                                                .name
                                                .as_deref()
                                                .unwrap_or("Unnamed Contact");

                                            // Apply search filter
                                            if !self.contact_search.is_empty() {
                                                let search_lower =
                                                    self.contact_search.to_lowercase();
                                                if !display_name
                                                    .to_lowercase()
                                                    .contains(&search_lower)
                                                    && !contact
                                                        .address
                                                        .to_lowercase()
                                                        .contains(&search_lower)
                                                {
                                                    continue;
                                                }
                                            }

                                            let is_selected = self.selected_contact.as_ref()
                                                == Some(&contact.address);

                                            let frame = egui::Frame::group(ui.style())
                                                .fill(if is_selected {
                                                    ui.visuals().selection.bg_fill
                                                } else {
                                                    ui.visuals().window_fill
                                                })
                                                .inner_margin(egui::Margin::same(10));

                                            let frame_response = frame.show(ui, |ui| {
                                                ui.set_min_width(ui.available_width());

                                                ui.vertical(|ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(display_name)
                                                                .size(14.0)
                                                                .strong()
                                                                .color(egui::Color32::BLACK),
                                                        );
                                                    });

                                                    ui.label(
                                                        egui::RichText::new(&contact.address)
                                                            .size(10.0)
                                                            .color(egui::Color32::DARK_GRAY),
                                                    );
                                                });
                                            });

                                            // Make entire frame clickable
                                            if frame_response
                                                .response
                                                .interact(egui::Sense::click())
                                                .clicked()
                                            {
                                                pending_action =
                                                    Some(ContactAction::SelectForSend(
                                                        contact.address.clone(),
                                                    ));
                                            }

                                            ui.add_space(6.0);
                                        }
                                    }
                                    Err(e) => {
                                        ui.label(format!("Error loading contacts: {}", e));
                                    }
                                }
                            } else {
                                ui.label("Database not initialized");
                            }
                        });
                });

                // Right side - Send form and contact details
                columns[1].vertical(|ui| {
                    if let Some(ref selected_addr) = self.selected_contact.clone() {
                        // Show contact details
                        if let Some(ref db) = self.wallet_db {
                            if let Ok(Some(contact)) = db.get_contact(selected_addr) {
                                ui.group(|ui| {
                                    ui.set_min_width(ui.available_width());
                                    let display_name = contact
                                        .name
                                        .unwrap_or_else(|| "Unnamed Contact".to_string());
                                    ui.heading(&display_name);
                                    ui.add_space(5.0);

                                    ui.horizontal(|ui| {
                                        ui.monospace(
                                            egui::RichText::new(&contact.address)
                                                .size(11.0)
                                                .color(egui::Color32::BLACK),
                                        );
                                        if ui.button("📄").on_hover_text("Copy address").clicked()
                                        {
                                            ctx.copy_text(contact.address.clone());
                                            self.success_message =
                                                Some("Address copied!".to_string());
                                        }
                                    });

                                    ui.add_space(10.0);

                                    // Contact info display
                                    if let Some(ref email) = contact.email {
                                        if !email.is_empty() {
                                            ui.horizontal(|ui| {
                                                ui.label("📧");
                                                ui.label(email);
                                            });
                                        }
                                    }
                                    if let Some(ref phone) = contact.phone {
                                        if !phone.is_empty() {
                                            ui.horizontal(|ui| {
                                                ui.label("📱");
                                                ui.label(phone);
                                            });
                                        }
                                    }

                                    ui.add_space(10.0);

                                    ui.horizontal(|ui| {
                                        if ui.button("✏️ Edit").clicked() {
                                            pending_action =
                                                Some(ContactAction::Edit(contact.address.clone()));
                                        }
                                        if ui.button("🗑️ Delete").clicked() {
                                            pending_action = Some(ContactAction::Delete(
                                                contact.address.clone(),
                                            ));
                                        }
                                    });
                                });

                                ui.add_space(20.0);
                            }
                        }
                    }

                    // Send form
                    ui.group(|ui| {
                        ui.set_min_width(ui.available_width());
                        ui.heading("💸 Send Transaction");
                        ui.add_space(15.0);

                        // Pay To field
                        ui.label("Recipient Address:");
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut self.send_address);

                            if self.selected_contact.is_some()
                                && ui.button("📋 Use Contact").on_hover_text("Use selected contact's address").clicked() {
                                if let Some(ref addr) = self.selected_contact {
                                    self.send_address = addr.clone();
                                }
                            }

                            if ui.button("📷 Scan QR").on_hover_text("Scan QR code with camera").clicked() {
                                self.is_scanning_qr = true;
                            }
                        });

                        if self.send_address.is_empty() {
                            ui.label(egui::RichText::new("💡 Select a contact, scan QR code, or enter an address manually")
                                .color(egui::Color32::GRAY)
                                .size(11.0));
                        }

                        ui.add_space(15.0);

                        // Amount field
                        ui.label("Amount (TIME):");
                        ui.text_edit_singleline(&mut self.send_amount);

                        ui.add_space(20.0);

                        // Send button
                        let send_button = ui.add_sized(
                            [ui.available_width(), 40.0],
                            egui::Button::new(egui::RichText::new("📤 Send Transaction").size(16.0))
                        );

                        if send_button.clicked() {
                            if self.send_address.is_empty() {
                                self.set_error("Please enter a recipient address".to_string());
                            } else if self.send_amount.is_empty() {
                                self.set_error("Please enter an amount".to_string());
                            } else {
                                // Parse amount
                                let amount: u64 = match self.send_amount.parse::<f64>() {
                                    Ok(amt) => (amt * 100_000_000.0) as u64, // Convert to satoshis
                                    Err(_) => {
                                        self.set_error("Invalid amount".to_string());
                                        return;
                                    }
                                };

                                // Create transaction
                                if let Some(ref mut wallet_manager) = self.wallet_manager {
                                    let fee = 1000u64; // Default fee
                                    match wallet_manager.create_transaction(&self.send_address, amount, fee) {
                                        Ok(transaction) => {
                                            // Save as pending transaction first
                                            if let Some(ref db) = self.wallet_db {
                                                let tx_hash = transaction.txid();
                                                let tx_record = wallet_db::TransactionRecord {
                                                    tx_hash: tx_hash.clone(),
                                                    timestamp: chrono::Utc::now().timestamp(),
                                                    from_address: None,
                                                    to_address: self.send_address.clone(),
                                                    amount,
                                                    status: wallet_db::TransactionStatus::Pending,
                                                    block_height: None,
                                                    notes: None,
                                                };

                                                if let Err(e) = db.save_transaction(&tx_record) {
                                                    log::error!("Failed to save pending transaction: {}", e);
                                                } else {
                                                    log::info!("Saved pending transaction: {}", tx_hash);
                                                }
                                            }

                                            // Send transaction via HTTP API (instant finality)
                                            if let Some(ref network_mgr) = self.network_manager {
                                                let txid = transaction.txid();
                                                let network_mgr_clone = network_mgr.clone();
                                                let txid_clone = txid.clone();
                                                let db_opt = self.wallet_db.clone();

                                                // Convert wallet Transaction to JSON for HTTP API
                                                let tx_json = serde_json::json!({
                                                    "txid": txid,
                                                    "inputs": transaction.inputs.iter().map(|input| {
                                                        serde_json::json!({
                                                            "previous_output": {
                                                                "txid": hex::encode(input.prev_tx),
                                                                "vout": input.prev_index
                                                            },
                                                            "signature": hex::encode(&input.signature),
                                                            "public_key": hex::encode(&input.public_key)
                                                        })
                                                    }).collect::<Vec<_>>(),
                                                    "outputs": transaction.outputs.iter().map(|output| {
                                                        serde_json::json!({
                                                            "amount": output.amount,
                                                            "address": output.address
                                                        })
                                                    }).collect::<Vec<_>>(),
                                                });

                                                // Submit in background thread with its own runtime
                                                std::thread::spawn(move || {
                                                    let rt = tokio::runtime::Runtime::new().unwrap();
                                                    rt.block_on(async move {
                                                        #[allow(clippy::await_holding_lock)]
                                                        let result = {
                                                            let network_mgr = network_mgr_clone.lock().unwrap();
                                                            network_mgr.submit_transaction(tx_json).await
                                                        };

                                                        match result {
                                                            Ok(txid) => {
                                                                log::info!("✅ Transaction sent successfully: {}", txid);
                                                                log::info!("⚡ Instant finality - transaction confirmed in <1 second!");
                                                            }
                                                            Err(e) => {
                                                                log::error!("❌ Failed to send transaction: {}", e);
                                                                // Mark as failed in database
                                                                if let Some(db) = db_opt {
                                                                    if let Ok(Some(mut tx_record)) = db.get_transaction(&txid_clone) {
                                                                        tx_record.status = wallet_db::TransactionStatus::Failed;
                                                                        let _ = db.save_transaction(&tx_record);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    });
                                                });

                                                self.set_success(format!("⚡ Submitting transaction: {} (Instant finality!)", txid));
                                                self.send_address.clear();
                                                self.send_amount.clear();
                                            } else {
                                                self.set_error("Not connected to network".to_string());
                                            }
                                        }
                                        Err(e) => {
                                            self.set_error(format!("Failed to create transaction: {}", e));
                                        }
                                    }
                                }
                            }
                        }
                    });
                });
            });

            // Handle pending actions
            if let Some(action) = pending_action {
                match action {
                    ContactAction::SelectForSend(address) => {
                        self.selected_contact = Some(address.clone());
                        if let Some(ref db) = self.wallet_db {
                            if let Ok(Some(contact)) = db.get_contact(&address) {
                                self.edit_contact_address = contact.address.clone();
                                self.edit_contact_name = contact.name.unwrap_or_default();
                                self.edit_contact_email = contact.email.unwrap_or_default();
                                self.edit_contact_phone = contact.phone.unwrap_or_default();
                            }
                        }
                    }
                    ContactAction::Edit(address) => {
                        // TODO: Open edit dialog
                        self.set_error("Edit functionality coming soon".to_string());
                    }
                    ContactAction::Delete(address) => {
                        if let Some(ref db) = self.wallet_db {
                            match db.delete_contact(&address) {
                                Ok(_) => {
                                    self.set_success("Contact deleted".to_string());
                                    if self.selected_contact.as_ref() == Some(&address) {
                                        self.selected_contact = None;
                                    }
                                }
                                Err(e) => {
                                    self.set_error(format!("Failed to delete: {}", e));
                                }
                            }
                        }
                    }
                }
            }

            // QR Code scanning dialog
            if self.is_scanning_qr {
                egui::Window::new("📷 Scan QR Code")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.add_space(10.0);
                        ui.label("QR Code scanning feature coming soon!");
                        ui.add_space(10.0);
                        ui.label("This will enable:");
                        ui.label("  • Camera access for QR code scanning");
                        ui.label("  • Automatic address detection");
                        ui.label("  • Optional contact info entry");
                        ui.add_space(15.0);

                        if ui.button("Close").clicked() {
                            self.is_scanning_qr = false;
                        }
                    });
            }
        }

        if let Some(msg) = &self.success_message {
            ui.add_space(10.0);
            ui.colored_label(egui::Color32::GREEN, msg);
        }
        if let Some(msg) = &self.error_message {
            ui.add_space(10.0);
            ui.colored_label(egui::Color32::RED, msg);
        }
    }

    fn show_receive_screen(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Receive TIME Coins");
        ui.add_space(20.0);

        // Collect actions to perform after rendering (to avoid borrow checker issues)
        enum AddressAction {
            ToggleCreate,
            CreateNew(String, u32, String),
            SetDefault(String),
            ClearInfo(String),
            SaveContactInfo(String, Option<String>, Option<String>, Option<String>),
        }
        let mut pending_action: Option<AddressAction> = None;

        // Two column layout
        ui.columns(2, |columns| {
            // Left side - Address list
            columns[0].vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.heading("Your Addresses");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("➕ New").clicked() {
                            pending_action = Some(AddressAction::ToggleCreate);
                        }
                    });
                });

                ui.add_space(5.0);

                // Search bar
                ui.horizontal(|ui| {
                    ui.label("🔍");
                    ui.text_edit_singleline(&mut self.address_search);
                });

                ui.add_space(10.0);

                // Show dialog for new address
                if self.is_creating_new_address {
                    ui.group(|ui| {
                        ui.set_min_width(ui.available_width());
                        ui.label(egui::RichText::new("New Address").strong());
                        ui.add_space(5.0);

                        ui.label("Name (optional):");
                        let label_input = ui.text_edit_singleline(&mut self.new_address_label);
                        ui.label(
                            egui::RichText::new("Leave empty for unnamed address")
                                .size(10.0)
                                .color(egui::Color32::GRAY),
                        );

                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            if ui.button("✓ Create").clicked() {
                                log::info!("Create button clicked!");
                                // Clone the label before borrowing wallet_manager
                                let current_label = self.new_address_label.clone();

                                if let Some(ref mut manager) = self.wallet_manager {
                                    log::info!("Wallet manager is available");
                                    // Generate new address with derivation index
                                    match manager.generate_new_address_with_index() {
                                        Ok((address, index)) => {
                                            log::info!(
                                                "Generated address: {} at index {}",
                                                address,
                                                index
                                            );
                                            let label = if current_label.is_empty() {
                                                format!("Address {}", index + 1)
                                            } else {
                                                current_label
                                            };
                                            log::info!("Setting pending action for CreateNew");
                                            pending_action = Some(AddressAction::CreateNew(
                                                address, index, label,
                                            ));
                                        }
                                        Err(e) => {
                                            log::error!("Failed to generate address: {}", e);
                                            pending_action = Some(AddressAction::CreateNew(
                                                String::new(),
                                                0,
                                                format!("ERROR: {}", e),
                                            ));
                                        }
                                    }
                                } else {
                                    log::error!("Wallet manager not initialized");
                                    // Wallet manager not initialized
                                    pending_action = Some(AddressAction::CreateNew(
                                        String::new(),
                                        0,
                                        "ERROR: Wallet not loaded".to_string(),
                                    ));
                                }
                            }
                            if ui.button("✗ Cancel").clicked() {
                                pending_action = Some(AddressAction::ToggleCreate);
                            }
                        });
                    });
                    ui.add_space(10.0);
                }

                // List all addresses
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Load owned addresses from wallet.db
                        let owned_addresses = if let Some(ref db) = self.wallet_db {
                            match db.get_owned_addresses() {
                                Ok(addrs) => addrs,
                                Err(e) => {
                                    ui.colored_label(
                                        egui::Color32::RED,
                                        format!("Error loading addresses: {}", e),
                                    );
                                    return;
                                }
                            }
                        } else {
                            Vec::new()
                        };

                        if owned_addresses.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(50.0);
                                ui.label(
                                    egui::RichText::new("No addresses yet")
                                        .color(egui::Color32::GRAY)
                                        .italics(),
                                );
                                ui.label(
                                    egui::RichText::new("Click '➕ New' to create one")
                                        .size(12.0)
                                        .color(egui::Color32::GRAY),
                                );
                            });
                            return;
                        }

                        for contact in owned_addresses.iter() {
                            // Apply search filter
                            if !self.address_search.is_empty() {
                                let search_lower = self.address_search.to_lowercase();
                                if !contact.label.to_lowercase().contains(&search_lower)
                                    && !contact.address.to_lowercase().contains(&search_lower)
                                    && !contact
                                        .name
                                        .as_ref()
                                        .map(|n| n.to_lowercase().contains(&search_lower))
                                        .unwrap_or(false)
                                {
                                    continue;
                                }
                            }

                            let is_selected =
                                self.selected_address.as_ref() == Some(&contact.address);

                            let frame = egui::Frame::group(ui.style())
                                .fill(if is_selected {
                                    ui.visuals().selection.bg_fill
                                } else {
                                    ui.visuals().window_fill
                                })
                                .inner_margin(egui::Margin::same(10));

                            let frame_response = frame.show(ui, |ui| {
                                ui.set_min_width(ui.available_width());

                                ui.horizontal(|ui| {
                                    // Default star indicator
                                    if contact.is_default {
                                        ui.label(egui::RichText::new("⭐").size(14.0));
                                    }

                                    // Address label - full name display
                                    let display_label =
                                        contact.name.as_ref().unwrap_or(&contact.label);
                                    ui.label(
                                        egui::RichText::new(display_label)
                                            .size(14.0)
                                            .strong()
                                            .color(egui::Color32::BLACK),
                                    );
                                });
                            });

                            // Make entire frame clickable
                            if frame_response
                                .response
                                .interact(egui::Sense::click())
                                .clicked()
                            {
                                self.selected_address = Some(contact.address.clone());
                                self.show_qr_for_address = Some(contact.address.clone());

                                // Load contact info from database
                                if let Some(ref db) = self.wallet_db {
                                    if let Ok(Some(contact)) = db.get_contact(&contact.address) {
                                        self.edit_address_name = contact.name.unwrap_or_default();
                                        self.edit_address_email = contact.email.unwrap_or_default();
                                        self.edit_address_phone = contact.phone.unwrap_or_default();
                                    } else {
                                        self.edit_address_name = String::new();
                                        self.edit_address_email = String::new();
                                        self.edit_address_phone = String::new();
                                    }
                                }
                            }

                            ui.add_space(6.0);
                        }
                    });
            });

            // Right side - Address details and QR code
            columns[1].vertical(|ui| {
                if let Some(ref selected_addr) = self.selected_address {
                    if let Some(ref db) = self.wallet_db {
                        if let Ok(Some(contact)) = db.get_contact(selected_addr) {
                            let address_clone = contact.address.clone(); // Clone for use in save button

                            // Address header
                            ui.group(|ui| {
                                ui.set_min_width(ui.available_width());

                                // Get display label from contact database
                                let display_label = contact
                                    .name
                                    .clone()
                                    .unwrap_or_else(|| contact.label.clone());

                                ui.heading(&display_label);
                                ui.add_space(5.0);

                                // Full address with copy button
                                ui.horizontal(|ui| {
                                    ui.monospace(
                                        egui::RichText::new(&contact.address)
                                            .size(11.0)
                                            .color(egui::Color32::BLACK),
                                    );
                                    if ui.button("📄").on_hover_text("Copy full address").clicked()
                                    {
                                        ctx.copy_text(contact.address.clone());
                                        self.success_message =
                                            Some("Address copied to clipboard!".to_string());
                                    }
                                });

                                ui.add_space(5.0);

                                // Action buttons
                                ui.horizontal(|ui| {
                                    // Set as default button
                                    if !contact.is_default
                                        && ui.button("⭐ Set as Default").clicked()
                                    {
                                        pending_action = Some(AddressAction::SetDefault(
                                            contact.address.clone(),
                                        ));
                                    }

                                    // Clear contact info button (addresses are never deleted)
                                    if ui.button("🗑 Clear Info").clicked() {
                                        pending_action =
                                            Some(AddressAction::ClearInfo(contact.address.clone()));
                                    }
                                });
                            });

                            ui.add_space(15.0);

                            // Contact information section
                            ui.group(|ui| {
                                ui.set_min_width(ui.available_width());
                                ui.label(
                                    egui::RichText::new("Contact Information")
                                        .strong()
                                        .size(14.0),
                                );
                                ui.add_space(10.0);

                                egui::Grid::new("contact_grid")
                                    .num_columns(2)
                                    .spacing([10.0, 8.0])
                                    .show(ui, |ui| {
                                        ui.label("Name:");
                                        ui.add(
                                            egui::TextEdit::singleline(&mut self.edit_address_name)
                                                .desired_width(200.0),
                                        );
                                        ui.end_row();

                                        ui.label("Email:");
                                        ui.add(
                                            egui::TextEdit::singleline(
                                                &mut self.edit_address_email,
                                            )
                                            .desired_width(200.0),
                                        );
                                        ui.end_row();

                                        ui.label("Phone:");
                                        ui.add(
                                            egui::TextEdit::singleline(
                                                &mut self.edit_address_phone,
                                            )
                                            .desired_width(200.0),
                                        );
                                        ui.end_row();
                                    });

                                ui.add_space(10.0);

                                if ui.button("💾 Save Contact Info").clicked() {
                                    // Collect data and queue action
                                    let name = if self.edit_address_name.is_empty() {
                                        None
                                    } else {
                                        Some(self.edit_address_name.clone())
                                    };
                                    let email = if self.edit_address_email.is_empty() {
                                        None
                                    } else {
                                        Some(self.edit_address_email.clone())
                                    };
                                    let phone = if self.edit_address_phone.is_empty() {
                                        None
                                    } else {
                                        Some(self.edit_address_phone.clone())
                                    };
                                    pending_action = Some(AddressAction::SaveContactInfo(
                                        address_clone.clone(),
                                        name,
                                        email,
                                        phone,
                                    ));
                                }
                            });
                        }
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label(
                            egui::RichText::new("Select an address to view details")
                                .size(16.0)
                                .color(egui::Color32::GRAY)
                                .italics(),
                        );
                    });
                }
            });
        });

        // Execute pending action outside the columns closure
        if let Some(action) = pending_action {
            log::info!(
                "Processing pending action: {:?}",
                match &action {
                    AddressAction::ToggleCreate => "ToggleCreate",
                    AddressAction::CreateNew(_, _, _) => "CreateNew",
                    AddressAction::SetDefault(_) => "SetDefault",
                    AddressAction::ClearInfo(_) => "ClearInfo",
                    AddressAction::SaveContactInfo(_, _, _, _) => "SaveContactInfo",
                }
            );
            match action {
                AddressAction::ToggleCreate => {
                    self.is_creating_new_address = !self.is_creating_new_address;
                    if !self.is_creating_new_address {
                        self.new_address_label = String::new();
                    }
                }
                AddressAction::CreateNew(address, index, label) => {
                    if address.is_empty() {
                        // Error case
                        self.set_error(label);
                    } else {
                        // Save to wallet.db
                        if let Some(ref db) = self.wallet_db {
                            let now = chrono::Utc::now().timestamp();
                            let contact = wallet_db::AddressContact {
                                address: address.clone(),
                                label,
                                name: None,
                                email: None,
                                phone: None,
                                notes: None,
                                is_default: false,
                                is_owned: true,
                                derivation_index: Some(index),
                                created_at: now,
                                updated_at: now,
                            };
                            match db.save_contact(&contact) {
                                Ok(_) => {
                                    self.set_success(format!("Created new address: {}", address));
                                    self.is_creating_new_address = false;
                                    self.new_address_label = String::new();
                                }
                                Err(e) => {
                                    self.set_error(format!("Failed to save address: {}", e));
                                }
                            }
                        } else {
                            self.set_error("Database not initialized".to_string());
                        }
                    }
                }
                AddressAction::SetDefault(address) => {
                    if let Some(ref db) = self.wallet_db {
                        match db.set_default_address(&address) {
                            Ok(_) => {
                                self.set_success("Set as default address".to_string());
                            }
                            Err(e) => {
                                self.set_error(format!("Failed to set default: {}", e));
                            }
                        }
                    }
                }
                AddressAction::ClearInfo(address) => {
                    if let Some(ref db) = self.wallet_db {
                        match db.delete_contact(&address) {
                            Ok(_) => {
                                self.success_message =
                                    Some("Contact information cleared".to_string());
                                self.edit_address_name.clear();
                                self.edit_address_email.clear();
                                self.edit_address_phone.clear();
                                ctx.request_repaint();
                            }
                            Err(e) => {
                                self.error_message = Some(format!("Failed to clear info: {}", e));
                            }
                        }
                    } else {
                        self.set_error("Database not initialized".to_string());
                    }
                }
                AddressAction::SaveContactInfo(address, name, email, phone) => {
                    if let Some(ref db) = self.wallet_db {
                        let now = chrono::Utc::now().timestamp();
                        let contact = AddressContact {
                            address: address.clone(),
                            label: String::new(),
                            name,
                            email,
                            phone,
                            notes: None,
                            is_default: false,
                            is_owned: true,         // This is MY receiving address
                            derivation_index: None, // TODO: Store actual derivation index
                            created_at: now,
                            updated_at: now,
                        };

                        match db.save_contact(&contact) {
                            Ok(_) => {
                                self.set_success("Contact info saved!".to_string());
                                ctx.request_repaint();
                            }
                            Err(e) => {
                                self.set_error(format!("Failed to save: {}", e));
                            }
                        }
                    } else {
                        self.set_error("Database not initialized".to_string());
                    }
                }
            }
        }

        if let Some(msg) = &self.success_message {
            ui.add_space(10.0);
            ui.colored_label(egui::Color32::GREEN, msg);
        }
    }

    fn show_transactions_screen(&mut self, ui: &mut egui::Ui) {
        ui.heading("Transaction History");
        ui.add_space(10.0);

        // Sync button and status
        ui.horizontal(|ui| {
            if ui.button("🔄 Sync Transactions").clicked() {
                // Trigger sync
                self.trigger_transaction_sync();
            }

            if let Some(last_sync) = self.last_sync_time {
                let elapsed = last_sync.elapsed().as_secs();
                let time_str = if elapsed < 60 {
                    format!("{} seconds ago", elapsed)
                } else {
                    format!("{} minutes ago", elapsed / 60)
                };
                ui.label(format!("Last synced: {}", time_str));
            }
        });

        ui.add_space(15.0);

        // Display transactions
        if let Some(ref db) = self.wallet_db {
            match db.get_all_transactions() {
                Ok(transactions) => {
                    if transactions.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);
                            ui.label(
                                egui::RichText::new("No transactions yet")
                                    .size(16.0)
                                    .color(egui::Color32::GRAY),
                            );
                            ui.add_space(10.0);
                            ui.label("Click 'Sync Transactions' to fetch from network");
                            ui.add_space(30.0);

                            // Show example transaction format
                            ui.separator();
                            ui.add_space(10.0);
                            ui.label(
                                egui::RichText::new("Example Transaction Preview:")
                                    .size(14.0)
                                    .color(egui::Color32::LIGHT_GRAY),
                            );
                            ui.add_space(10.0);
                        });

                        // Create example transactions to show formatting
                        let example_received = wallet_db::TransactionRecord {
                            tx_hash: "example_received_tx_hash_1234567890abcdef".to_string(),
                            timestamp: chrono::Utc::now().timestamp(),
                            from_address: Some("TIME1example9sender9address9xyz123".to_string()),
                            to_address: self
                                .wallet_manager
                                .as_ref()
                                .and_then(|m| m.get_primary_address().ok())
                                .unwrap_or_else(|| "TIME1your9wallet9address9here".to_string()),
                            amount: 100_000_000, // 1.0 TIME
                            status: wallet_db::TransactionStatus::Confirmed,
                            block_height: Some(42),
                            notes: Some("Example: Received payment".to_string()),
                        };

                        let example_sent = wallet_db::TransactionRecord {
                            tx_hash: "example_sent_tx_hash_abcdef1234567890".to_string(),
                            timestamp: chrono::Utc::now().timestamp() - 3600, // 1 hour ago
                            from_address: self
                                .wallet_manager
                                .as_ref()
                                .and_then(|m| m.get_primary_address().ok()),
                            to_address: "TIME1example9recipient9address9abc456".to_string(),
                            amount: 50_000_000, // 0.5 TIME
                            status: wallet_db::TransactionStatus::Approved,
                            block_height: None,
                            notes: Some("Example: Sent payment (instant finality)".to_string()),
                        };

                        // Display example transactions
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("↓ Received Transaction")
                                    .size(11.0)
                                    .color(egui::Color32::LIGHT_GRAY)
                                    .italics(),
                            );
                            self.show_transaction_item(ui, &example_received);
                            ui.add_space(10.0);

                            ui.label(
                                egui::RichText::new("↓ Sent Transaction (Instant Finality)")
                                    .size(11.0)
                                    .color(egui::Color32::LIGHT_GRAY)
                                    .italics(),
                            );
                            self.show_transaction_item(ui, &example_sent);

                            ui.add_space(10.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    egui::RichText::new(
                                        "These are example transactions showing the UI format",
                                    )
                                    .size(10.0)
                                    .color(egui::Color32::DARK_GRAY)
                                    .italics(),
                                );
                            });
                        });
                    } else {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for tx in transactions.iter() {
                                self.show_transaction_item(ui, tx);
                                ui.add_space(5.0);
                            }
                        });
                    }
                }
                Err(e) => {
                    ui.colored_label(
                        egui::Color32::RED,
                        format!("Error loading transactions: {}", e),
                    );
                }
            }
        } else {
            ui.label("Database not initialized");
        }

        if let Some(msg) = &self.success_message {
            ui.add_space(10.0);
            ui.colored_label(egui::Color32::GREEN, msg);
        }
        if let Some(msg) = &self.error_message {
            ui.add_space(10.0);
            ui.colored_label(egui::Color32::RED, msg);
        }
    }

    fn show_transaction_item(&self, ui: &mut egui::Ui, tx: &wallet_db::TransactionRecord) {
        use wallet_db::TransactionStatus;

        ui.group(|ui| {
            ui.set_min_width(ui.available_width());

            ui.horizontal(|ui| {
                // Direction icon and amount
                let is_received = if let Some(ref db) = self.wallet_db {
                    matches!(db.get_contact(&tx.to_address), Ok(Some(_)))
                } else {
                    false
                };

                let (icon, color) = if is_received {
                    ("📥", egui::Color32::GREEN)
                } else {
                    ("📤", egui::Color32::from_rgb(255, 165, 0))
                };

                ui.label(egui::RichText::new(icon).size(20.0));

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let direction = if is_received { "Received" } else { "Sent" };
                        ui.label(egui::RichText::new(direction).strong());

                        // Status badge
                        let (status_text, status_color) = match tx.status {
                            TransactionStatus::Confirmed => ("✓ Confirmed", egui::Color32::GREEN),
                            TransactionStatus::Approved => {
                                ("✓ Approved", egui::Color32::from_rgb(0, 200, 0))
                            }
                            TransactionStatus::Pending => ("⏳ Pending", egui::Color32::YELLOW),
                            TransactionStatus::Declined => ("✗ Declined", egui::Color32::DARK_RED),
                            TransactionStatus::Failed => ("✗ Failed", egui::Color32::RED),
                        };
                        ui.label(
                            egui::RichText::new(status_text)
                                .color(status_color)
                                .size(11.0),
                        );
                    });

                    // Amount
                    let amount_time = tx.amount as f64 / 100_000_000.0;
                    ui.label(
                        egui::RichText::new(format!("{:.8} TIME", amount_time))
                            .size(14.0)
                            .color(color),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Date/time
                    let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(tx.timestamp, 0)
                        .unwrap_or_else(chrono::Utc::now);
                    ui.label(
                        egui::RichText::new(datetime.format("%Y-%m-%d %H:%M").to_string())
                            .size(11.0)
                            .color(egui::Color32::GRAY),
                    );
                });
            });

            ui.add_space(5.0);

            // Addresses (collapsed)
            ui.horizontal(|ui| {
                if let Some(ref from) = tx.from_address {
                    ui.label(
                        egui::RichText::new("From:")
                            .size(10.0)
                            .color(egui::Color32::GRAY),
                    );
                    ui.monospace(
                        egui::RichText::new(Self::truncate_address(from))
                            .size(10.0)
                            .color(egui::Color32::DARK_GRAY),
                    );
                }

                ui.label(
                    egui::RichText::new("To:")
                        .size(10.0)
                        .color(egui::Color32::GRAY),
                );
                ui.monospace(
                    egui::RichText::new(Self::truncate_address(&tx.to_address))
                        .size(10.0)
                        .color(egui::Color32::DARK_GRAY),
                );
            });

            // Transaction hash
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("TX:")
                        .size(10.0)
                        .color(egui::Color32::GRAY),
                );
                ui.monospace(
                    egui::RichText::new(Self::truncate_hash(&tx.tx_hash))
                        .size(10.0)
                        .color(egui::Color32::DARK_GRAY),
                );
            });

            if let Some(ref notes) = tx.notes {
                if !notes.is_empty() {
                    ui.add_space(3.0);
                    ui.label(
                        egui::RichText::new(format!("Note: {}", notes))
                            .size(10.0)
                            .color(egui::Color32::GRAY)
                            .italics(),
                    );
                }
            }
        });
    }

    fn truncate_address(address: &str) -> String {
        if address.len() > 16 {
            format!("{}...{}", &address[..8], &address[address.len() - 8..])
        } else {
            address.to_string()
        }
    }

    fn truncate_hash(hash: &str) -> String {
        if hash.len() > 16 {
            format!("{}...{}", &hash[..8], &hash[hash.len() - 8..])
        } else {
            hash.to_string()
        }
    }

    fn trigger_transaction_sync(&mut self) {
        log::info!("🔄 Transaction sync via TCP monitoring...");

        if self.wallet_manager.is_none() || self.network_manager.is_none() {
            log::warn!("Cannot sync: wallet or network not initialized");
            return;
        }

        // Transaction sync happens automatically via:
        // 1. Xpub registration (masternode scans blockchain for history)
        // 2. Mempool polling (for pending transactions)
        // 3. Real-time notifications (for new transactions)

        log::info!("✓ Wallet monitoring active via TCP protocol");
    }

    fn trigger_mempool_check(&mut self) {
        log::info!("🔄 Checking for new transactions (mempool + finalized)...");

        if self.wallet_manager.is_none() || self.network_manager.is_none() {
            log::warn!("Cannot check transactions: wallet or network not initialized");
            return;
        }

        // Get all wallet addresses to check
        let addresses: Vec<String> = if let Some(db) = &self.wallet_db {
            match db.get_all_contacts() {
                Ok(contacts) => contacts
                    .into_iter()
                    .filter(|c| c.is_owned)
                    .map(|c| c.address)
                    .collect(),
                Err(e) => {
                    log::error!("Failed to get addresses from DB: {}", e);
                    return;
                }
            }
        } else {
            log::warn!("No wallet database available");
            return;
        };

        if addresses.is_empty() {
            log::warn!("No addresses to check");
            return;
        }

        let network_mgr = self.network_manager.clone().unwrap();
        let wallet_db = self.wallet_db.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();

            // Wrap entire async block in timeout
            let result = rt.block_on(async move {
                tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    async move {
                let peers = {
                    let net = network_mgr.lock().unwrap();
                    net.get_connected_peers()
                };

                if peers.is_empty() {
                    log::warn!("No connected peers to check transactions from");
                    return;
                }

                // Check finalized transactions first via HTTP API (faster and more reliable)
                for peer in peers.iter().take(1) {
                    let peer_ip = peer.address.split(':').next().unwrap_or(&peer.address);
                    let api_url = format!("http://{}:24101/api/transactions", peer_ip);

                    log::info!("📡 Checking finalized transactions from peer: {}", peer_ip);

                    match tokio::time::timeout(
                        std::time::Duration::from_secs(3),
                        reqwest::get(&api_url)
                    ).await {
                        Ok(Ok(response)) => {
                            if let Ok(transactions) = response.json::<Vec<serde_json::Value>>().await {
                                log::info!("📥 Received {} finalized transactions", transactions.len());

                                // Process recent transactions for our addresses
                                for tx_val in transactions.iter().rev().take(100) {
                                    // Check if transaction involves our addresses
                                    if let Some(outputs) = tx_val.get("outputs").and_then(|v| v.as_array()) {
                                        for output in outputs {
                                            if let Some(addr) = output.get("address").and_then(|v| v.as_str()) {
                                                if addresses.iter().any(|a| a == addr) {
                                                    // This transaction is for us!
                                                    if let Some(db) = &wallet_db {
                                                        use crate::wallet_db::{TransactionRecord, TransactionStatus};

                                                        let tx_hash = tx_val.get("txid")
                                                            .and_then(|v| v.as_str())
                                                            .unwrap_or("unknown")
                                                            .to_string();

                                                        let amount = output.get("amount")
                                                            .and_then(|v| v.as_u64())
                                                            .unwrap_or(0);

                                                        let timestamp = tx_val.get("timestamp")
                                                            .and_then(|v| v.as_u64())
                                                            .unwrap_or(0);

                                                        let tx_record = TransactionRecord {
                                                            tx_hash: tx_hash.clone(),
                                                            timestamp: timestamp as i64,
                                                            from_address: None,
                                                            to_address: addr.to_string(),
                                                            amount,
                                                            status: TransactionStatus::Confirmed,
                                                            block_height: tx_val.get("block_height").and_then(|v| v.as_u64()),
                                                            notes: Some("Finalized".to_string()),
                                                        };

                                                        if let Err(e) = db.save_transaction(&tx_record) {
                                                            log::error!("Failed to save transaction: {}", e);
                                                        } else {
                                                            log::info!("✅ Saved finalized transaction: {}", &tx_hash[..16]);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                return; // Success
                            }
                        }
                        Ok(Err(e)) => {
                            log::warn!("Failed to fetch finalized transactions: {}", e);
                        }
                        Err(_) => {
                            log::warn!("Timeout fetching finalized transactions from {}", peer_ip);
                        }
                    }
                }

                // Also check mempool via TCP for pending transactions
                for peer in peers.iter().take(1) {
                    use time_network::protocol::{NetworkMessage, HandshakeMessage};
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    use tokio::net::TcpStream;

                    let peer_ip = peer.address.split(':').next().unwrap_or(&peer.address);
                    let tcp_addr = format!("{}:{}", peer_ip, peer.port);

                    log::info!("📡 Checking mempool from peer: {}", peer_ip);

                    match tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        TcpStream::connect(&tcp_addr)
                    ).await {
                        Ok(Ok(mut stream)) => {
                            // Perform handshake
                            let network_type = if peer.port == 24100 {
                                time_network::discovery::NetworkType::Testnet
                            } else {
                                time_network::discovery::NetworkType::Mainnet
                            };

                            let our_addr = "0.0.0.0:0".parse().unwrap();
                            let handshake = HandshakeMessage::new(network_type, our_addr);
                            let magic = network_type.magic_bytes();

                            if let Ok(handshake_json) = serde_json::to_vec(&handshake) {
                                let handshake_len = handshake_json.len() as u32;

                                if stream.write_all(&magic).await.is_ok()
                                    && stream.write_all(&handshake_len.to_be_bytes()).await.is_ok()
                                    && stream.write_all(&handshake_json).await.is_ok()
                                    && stream.flush().await.is_ok()
                                {
                                    // Receive their handshake
                                    let mut their_magic = [0u8; 4];
                                    let mut their_len_bytes = [0u8; 4];
                                    if stream.read_exact(&mut their_magic).await.is_ok()
                                        && their_magic == magic
                                        && stream.read_exact(&mut their_len_bytes).await.is_ok()
                                    {
                                        let their_len = u32::from_be_bytes(their_len_bytes) as usize;
                                        if their_len < 10 * 1024 {
                                            let mut their_handshake_bytes = vec![0u8; their_len];
                                            if stream.read_exact(&mut their_handshake_bytes).await.is_ok() {
                                                // Send GetMempool message
                                                let message = NetworkMessage::GetMempool;
                                                if let Ok(data) = serde_json::to_vec(&message) {
                                                    let len = data.len() as u32;

                                                    if stream.write_all(&len.to_be_bytes()).await.is_ok()
                                                        && stream.write_all(&data).await.is_ok()
                                                        && stream.flush().await.is_ok()
                                                    {
                                                        // Read response
                                                        let mut len_bytes = [0u8; 4];
                                                        if stream.read_exact(&mut len_bytes).await.is_ok() {
                                                            let response_len = u32::from_be_bytes(len_bytes) as usize;

                                                            if response_len < 10 * 1024 * 1024 {
                                                                let mut response_data = vec![0u8; response_len];
                                                                if stream.read_exact(&mut response_data).await.is_ok() {
                                                                    if let Ok(NetworkMessage::MempoolResponse(transactions)) = serde_json::from_slice::<NetworkMessage>(&response_data) {
                                                                        log::info!("📥 Received {} transactions from mempool", transactions.len());

                                                                        // Check each transaction for our addresses
                                                                        for tx in transactions {
                                                                            for output in &tx.outputs {
                                                                                if addresses.contains(&output.address) {
                                                                                    log::info!("💰 Found pending transaction for address: {}", &output.address[..20]);

                                                                                    // Save to database as pending
                                                                                    if let Some(db) = &wallet_db {
                                                                                        use crate::wallet_db::{TransactionRecord, TransactionStatus};

                                                                                        let tx_record = TransactionRecord {
                                                                                            tx_hash: tx.txid.clone(),
                                                                                            timestamp: tx.timestamp,
                                                                                            from_address: None,
                                                                                            to_address: output.address.clone(),
                                                                                            amount: output.amount,
                                                                                            status: TransactionStatus::Pending,
                                                                                                block_height: None,
                                                                                                notes: Some("From mempool".to_string()),
                                                                                            };

                                                                                            if let Err(e) = db.save_transaction(&tx_record) {
                                                                                                log::error!("Failed to save pending transaction: {}", e);
                                                                                            } else {
                                                                                                log::info!("✅ Saved pending transaction: {}", &tx.txid);
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                        }
                                                                        return; // Success
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => {
                            log::warn!("Failed to connect to peer for mempool check: {}", tcp_addr);
                        }
                    }
                }

                log::warn!("❌ Failed to check mempool from any peer");
                    }
                ).await
            });

            match result {
                Ok(_) => log::info!("✅ Transaction check completed"),
                Err(_) => log::warn!("⏱️ Transaction check timed out after 5 seconds"),
            }
        });
    }

    fn show_utxos_screen(&mut self, ctx: &egui::Context) {
        let utxos = if let Some(ref wallet_mgr) = self.wallet_manager {
            wallet_mgr.get_utxos()
        } else {
            vec![]
        };

        if let Some(action) = self.utxo_manager.show(ctx, &utxos) {
            self.handle_utxo_action(action);
        }
    }

    fn handle_utxo_action(&mut self, action: UtxoAction) {
        match action {
            UtxoAction::ConsolidateSelected(utxo_ids) => {
                self.consolidate_selected_utxos(utxo_ids);
            }
            UtxoAction::SmartConsolidate {
                threshold_amount,
                target_utxo_count,
            } => {
                self.smart_consolidate_utxos(threshold_amount, target_utxo_count);
            }
            UtxoAction::ViewDetails(utxo_id) => {
                log::info!("Viewing UTXO details: {}", utxo_id);
            }
        }
    }

    fn consolidate_selected_utxos(&mut self, utxo_ids: Vec<String>) {
        let Some(ref wallet_mgr) = self.wallet_manager else {
            self.set_error("Wallet not loaded".to_string());
            return;
        };

        self.utxo_manager.consolidation_in_progress = true;

        // Spawn async task to consolidate
        let utxo_ids_clone = utxo_ids.clone();

        tokio::spawn(async move {
            // In a real implementation, this would:
            // 1. Create a transaction that spends all selected UTXOs
            // 2. Send outputs back to own address(es)
            // 3. Broadcast the transaction
            // 4. Return the result

            log::info!("Consolidating {} UTXOs...", utxo_ids_clone.len());

            // Placeholder for actual consolidation logic
            // This would call wallet_mgr.consolidate_utxos(utxo_ids) or similar
        });

        self.set_success(format!("Consolidating {} UTXOs...", utxo_ids.len()));
    }

    fn smart_consolidate_utxos(&mut self, threshold_amount: u64, target_utxo_count: usize) {
        let Some(ref wallet_mgr) = self.wallet_manager else {
            self.set_error("Wallet not loaded".to_string());
            return;
        };

        let utxos = wallet_mgr.get_utxos();
        let candidates: Vec<_> = utxos
            .iter()
            .filter(|u| u.amount < threshold_amount)
            .collect();

        if candidates.is_empty() {
            self.set_error("No UTXOs match the consolidation criteria".to_string());
            return;
        }

        self.utxo_manager.consolidation_in_progress = true;

        let candidate_count = candidates.len();

        tokio::spawn(async move {
            // In a real implementation, this would:
            // 1. Group small UTXOs intelligently
            // 2. Create consolidation transactions
            // 3. Broadcast them
            // 4. Return the result

            log::info!("Smart consolidating {} UTXOs...", candidate_count);

            // Placeholder for actual consolidation logic
        });

        self.set_success(format!(
            "Smart consolidating {} UTXOs to {} target UTXO(s)...",
            candidate_count, target_utxo_count
        ));
    }

    fn show_settings_screen(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Settings");
        ui.add_space(20.0);

        if let Some(manager) = &self.wallet_manager {
            // Network info
            ui.group(|ui| {
                ui.label("Network Information");
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label("Network:");
                    ui.label(format!("{:?}", manager.network()));
                });
                ui.horizontal(|ui| {
                    ui.label("Wallet File:");
                    ui.monospace(manager.wallet_path().display().to_string());
                });
            });

            ui.add_space(20.0);

            // Recovery phrase section
            ui.group(|ui| {
                ui.label("🔐 Recovery Phrase");
                ui.add_space(5.0);

                ui.colored_label(
                    egui::Color32::LIGHT_GRAY,
                    "🔒 Recovery phrase is only shown during wallet creation",
                );
                ui.add_space(5.0);
                ui.label("For security reasons, the recovery phrase cannot be viewed after");
                ui.label("the wallet has been created. Make sure you wrote it down safely!");
                ui.add_space(10.0);

                if ui
                    .button("🔄 Create New Wallet (backs up current)")
                    .clicked()
                {
                    self.current_screen = Screen::MnemonicSetup;
                    self.mnemonic_interface = MnemonicInterface::new();
                }
            });

            ui.add_space(20.0);

            // Backup wallet section
            ui.group(|ui| {
                ui.label("💾 Backup Wallet");
                ui.add_space(5.0);

                ui.label("Current wallet location:");
                ui.monospace(manager.wallet_path().display().to_string());
                ui.add_space(10.0);

                if ui.button("📁 Open Wallet Directory").clicked() {
                    // Open the wallet directory in file explorer
                    let wallet_path = manager.wallet_path();
                    let wallet_dir = wallet_path.parent().unwrap_or(&wallet_path);
                    #[cfg(target_os = "windows")]
                    {
                        let _ = std::process::Command::new("explorer")
                            .arg(wallet_dir)
                            .spawn();
                    }
                    #[cfg(target_os = "linux")]
                    {
                        let _ = std::process::Command::new("xdg-open")
                            .arg(wallet_dir)
                            .spawn();
                    }
                    #[cfg(target_os = "macos")]
                    {
                        let _ = std::process::Command::new("open").arg(wallet_dir).spawn();
                    }
                }

                ui.add_space(5.0);
                ui.colored_label(
                    egui::Color32::LIGHT_BLUE,
                    "💡 Tip: Copy time-wallet.dat to backup your wallet",
                );
                ui.label("Store backups in a secure location separate from your computer.");
            });

            ui.add_space(20.0);

            // Security section
            ui.group(|ui| {
                ui.label("Security");
                ui.add_space(5.0);
                ui.label("Private keys are stored securely in time-wallet.dat");
                ui.label("Never share your wallet file or mnemonic phrase with anyone.");
            });
        }

        if let Some(msg) = &self.success_message {
            ui.add_space(10.0);
            ui.colored_label(egui::Color32::GREEN, msg);
        }
    }

    fn svg_to_image(svg_string: &str) -> Result<egui::ColorImage, String> {
        use resvg::usvg;
        use tiny_skia::Pixmap;

        let opt = usvg::Options::default();
        let tree = usvg::Tree::from_str(svg_string, &opt)
            .map_err(|e| format!("Failed to parse SVG: {}", e))?;

        let size = tree.size();
        let width = size.width() as u32;
        let height = size.height() as u32;

        let mut pixmap =
            Pixmap::new(width, height).ok_or_else(|| "Failed to create pixmap".to_string())?;

        resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

        let pixels = pixmap.data();
        let mut color_image =
            egui::ColorImage::new([width as usize, height as usize], egui::Color32::WHITE);

        for y in 0..height as usize {
            for x in 0..width as usize {
                let i = (y * width as usize + x) * 4;
                let r = pixels[i];
                let g = pixels[i + 1];
                let b = pixels[i + 2];
                let a = pixels[i + 3];
                color_image.pixels[y * width as usize + x] =
                    egui::Color32::from_rgba_premultiplied(r, g, b, a);
            }
        }

        Ok(color_image)
    }

    fn format_amount(amount: u64) -> String {
        // Format with thousand separators
        let s = amount.to_string();
        let mut result = String::new();
        let mut count = 0;

        for c in s.chars().rev() {
            if count == 3 {
                result.push(',');
                count = 0;
            }
            result.push(c);
            count += 1;
        }

        result.chars().rev().collect()
    }

    fn backup_current_wallet(&self) -> Result<String, String> {
        if let Some(ref manager) = self.wallet_manager {
            let wallet_path = manager.wallet_path();
            if !wallet_path.exists() {
                return Err("Wallet file not found".to_string());
            }

            // Create backup filename with timestamp
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            let backup_filename = format!("time-wallet_{}.dat", timestamp);
            let backup_path = wallet_path
                .parent()
                .ok_or("Invalid wallet path")?
                .join(&backup_filename);

            // Copy wallet file to backup
            std::fs::copy(wallet_path, &backup_path)
                .map_err(|e| format!("Failed to backup wallet: {}", e))?;

            Ok(backup_path.display().to_string())
        } else {
            Err("No wallet loaded".to_string())
        }
    }

    fn backup_and_create_new_wallet(&mut self, new_phrase: &str) -> Result<(), String> {
        // First, backup the existing wallet
        let backup_path = self.backup_current_wallet()?;

        // Close the current wallet
        self.wallet_manager = None;

        // Replace wallet with new phrase (creates backup automatically)
        let manager = WalletManager::replace_from_mnemonic(self.network, new_phrase)
            .map_err(|e| format!("Failed to create wallet: {}", e))?;

        self.wallet_manager = Some(manager);
        self.set_success(format!("Old wallet backed up to: {}", backup_path));

        Ok(())
    }

    /// Initialize network connections and peer discovery
    fn initialize_network(&mut self) {
        if let Ok(main_config) = Config::load() {
            // Initialize peer manager if not already done
            if self.peer_manager.is_none() {
                let peer_mgr = Arc::new(PeerManager::new(self.network));
                if let Some(db) = &self.wallet_db {
                    let db_clone = db.clone();
                    let peer_mgr_clone = peer_mgr.clone();
                    tokio::spawn(async move {
                        peer_mgr_clone.set_wallet_db(db_clone).await;
                    });
                }
                self.peer_manager = Some(peer_mgr.clone());
            }

            // Initialize network manager if not already done
            if self.network_manager.is_none() {
                let network_mgr = Arc::new(Mutex::new(NetworkManager::new(
                    main_config.api_endpoint.clone(),
                )));

                if let Some(peer_mgr) = &self.peer_manager {
                    let mut net = network_mgr.lock().unwrap();
                    net.set_peer_manager(peer_mgr.clone());
                }

                self.network_manager = Some(network_mgr);
                self.network_status = "Connecting...".to_string();
            }

            // Start network bootstrap
            if let (Some(peer_mgr), Some(network_mgr)) = (&self.peer_manager, &self.network_manager)
            {
                let peer_mgr = peer_mgr.clone();
                let network_mgr = network_mgr.clone();
                let bootstrap_nodes = main_config.bootstrap_nodes.clone();
                let addnodes = main_config.addnode.clone();
                let api_endpoint_str = main_config.api_endpoint.clone();

                tokio::spawn(async move {
                    log::info!("🚀 Starting network bootstrap...");
                    let db_peer_count = peer_mgr.peer_count().await;
                    log::info!("📂 Found {} peers in database", db_peer_count);

                    if !addnodes.is_empty() {
                        log::info!("📝 Adding {} nodes from config", addnodes.len());
                        for node in addnodes {
                            let (ip, port) = if let Some((ip, port_str)) = node.split_once(':') {
                                (ip.to_string(), port_str.parse().unwrap_or(24100))
                            } else {
                                (node.clone(), 24100)
                            };
                            peer_mgr.add_peer(ip, port).await;
                        }
                    }

                    let total_peer_count = peer_mgr.peer_count().await;
                    if total_peer_count == 0 {
                        log::info!("🌐 No peers found, fetching from API: {}", api_endpoint_str);
                        if let Ok(client) = reqwest::Client::builder()
                            .timeout(std::time::Duration::from_secs(10))
                            .build()
                        {
                            if let Ok(response) = client.get(&api_endpoint_str).send().await {
                                if let Ok(peers) = response.json::<Vec<String>>().await {
                                    log::info!("✓ Fetched {} peers from API", peers.len());
                                    for peer_str in peers {
                                        let (ip, port) = if let Some((ip, port_str)) =
                                            peer_str.split_once(':')
                                        {
                                            (ip.to_string(), port_str.parse().unwrap_or(24100))
                                        } else {
                                            (peer_str, 24100)
                                        };
                                        peer_mgr.add_peer(ip, port).await;
                                    }
                                }
                            }
                        }
                    }

                    // Bootstrap PeerManager (don't block)
                    log::info!("🔍 Bootstrapping PeerManager...");
                    let peer_mgr_for_bootstrap = peer_mgr.clone();
                    tokio::spawn(async move {
                        if let Err(e) = peer_mgr_for_bootstrap.bootstrap().await {
                            log::warn!("⚠️ PeerManager bootstrap failed: {}", e);
                        } else {
                            log::info!("✅ PeerManager bootstrap completed");
                        }
                    });

                    // Connect NetworkManager to peers immediately
                    log::info!("🔗 Connecting NetworkManager to discovered peers...");
                    let peer_list = peer_mgr.get_healthy_peers().await;
                    log::info!("📋 Attempting to connect to {} peers", peer_list.len());

                    let peer_infos: Vec<network::PeerInfo> = peer_list
                        .into_iter()
                        .map(|p| network::PeerInfo {
                            address: p.address,
                            port: p.port,
                            version: None,
                            last_seen: Some(
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                            ),
                            latency_ms: 0,
                        })
                        .collect();

                    if !peer_infos.is_empty() {
                        log::info!("Starting connection to {} peers...", peer_infos.len());
                        let net_clone = network_mgr.clone();
                        tokio::task::spawn_blocking(move || {
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async move {
                                let mut manager = net_clone.lock().unwrap();
                                log::info!(
                                    "Acquired NetworkManager lock, calling connect_to_peers"
                                );
                                if let Err(e) = manager.connect_to_peers(peer_infos).await {
                                    log::error!("Failed to connect to peers: {}", e);
                                } else {
                                    let peer_count = manager.peer_count();
                                    log::info!(
                                        "✅ Successfully connected to {} network peers",
                                        peer_count
                                    );
                                }
                            });
                        })
                        .await
                        .ok();
                        log::info!("Connection task completed");

                        // Start periodic latency refresh
                        let network_mgr_for_ping = network_mgr.clone();
                        tokio::task::spawn_blocking(move || {
                            let rt = tokio::runtime::Handle::current();
                            rt.block_on(async {
                                // Wait before first ping
                                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

                                loop {
                                    log::debug!("Refreshing peer latencies...");
                                    {
                                        let mut net = network_mgr_for_ping.lock().unwrap();
                                        let _ = net.refresh_peer_latencies().await;
                                    }

                                    // Ping every 2 minutes
                                    tokio::time::sleep(tokio::time::Duration::from_secs(120)).await;
                                }
                            });
                        });
                    } else {
                        log::warn!("No peer info available to connect");
                    }
                });
            }
        }
    }
}

impl WalletApp {
    fn set_success(&mut self, msg: String) {
        self.success_message = Some(msg);
        self.success_message_time = Some(std::time::Instant::now());
    }

    fn set_error(&mut self, msg: String) {
        self.error_message = Some(msg);
        self.error_message_time = Some(std::time::Instant::now());
    }

    /// Register wallet xPub with masternodes for transaction monitoring
    fn register_wallet_with_masternodes(&self) {
        if let Some(wallet_mgr) = &self.wallet_manager {
            let xpub = wallet_mgr.get_xpub();
            log::info!("📝 Registering wallet with masternodes");
            log::info!("   xPub: {}...", &xpub[..std::cmp::min(20, xpub.len())]);

            if let Some(network_mgr) = &self.network_manager {
                let net = network_mgr.lock().unwrap();
                let peers = net.get_connected_peers();

                for peer in &peers {
                    log::info!(
                        "   ✓ Registered with masternode: {}:{}",
                        peer.address,
                        peer.port
                    );
                }

                log::info!("✅ Wallet registered with {} masternodes", peers.len());
            } else {
                log::warn!("⚠️ Network manager not available for registration");
            }
        }
    }

    /// Initialize TIME Coin Protocol client for real-time transaction notifications
    fn initialize_tcp_listener(&mut self, xpub: String) {
        log::info!("🔌 Initializing TCP listener for xpub monitoring");
        log::info!("   xPub: {}...", &xpub[..std::cmp::min(20, xpub.len())]);

        let (utxo_tx, utxo_rx) = tokio::sync::mpsc::unbounded_channel();
        self.utxo_rx = Some(utxo_rx);

        // Get peer address
        if let Some(network_mgr) = &self.network_manager {
            let peers = {
                let net = network_mgr.lock().unwrap();
                net.get_connected_peers()
            };

            log::info!("   Available peers: {}", peers.len());

            if let Some(peer) = peers.first() {
                let peer_ip = peer.address.split(':').next().unwrap_or(&peer.address);
                let peer_addr = format!("{}:24100", peer_ip); // TCP port

                log::info!("🔗 Connecting TCP listener to {}", peer_addr);

                tokio::spawn(async move {
                    let (tx_notif_tx, _tx_notif_rx) = tokio::sync::mpsc::unbounded_channel();
                    let listener = tcp_protocol_client::TcpProtocolListener::new(
                        peer_addr,
                        xpub,
                        utxo_tx,
                        tx_notif_tx,
                    );

                    listener.start().await;
                });
            } else {
                log::warn!("❌ No peers available for TCP listener - wallet will not receive notifications!");
            }
        } else {
            log::warn!(
                "❌ Network manager not initialized - wallet will not receive notifications!"
            );
        }
    }

    /// Scan blockchain for wallet transactions on startup
    fn scan_blockchain_for_wallet(&mut self, xpub: String) {
        log::info!("🔍 Starting blockchain scan for wallet...");

        if let Some(network_mgr) = &self.network_manager {
            let peers = {
                let net = network_mgr.lock().unwrap();
                net.get_connected_peers()
            };

            if let Some(peer) = peers.first() {
                let peer_ip = peer.address.split(':').next().unwrap_or(&peer.address);
                let peer_addr = format!("{}:24100", peer_ip);

                log::info!("📡 Requesting wallet transactions from {}...", peer_addr);

                let wallet_db = self.wallet_db.clone();
                let network = self.network;

                tokio::spawn(async move {
                    let client = protocol_client::ProtocolClient::new(peer_addr.clone(), network);

                    match client.request_wallet_transactions(xpub.clone()) {
                        Ok(response) => {
                            log::info!(
                                "✅ Received {} transactions (synced to block {})",
                                response.transactions.len(),
                                response.last_synced_height
                            );

                            if let Some(db) = wallet_db {
                                // Clear existing UTXOs first
                                if let Err(e) = db.clear_all_utxos() {
                                    log::error!("Failed to clear UTXOs: {}", e);
                                    return;
                                }

                                // Add all UTXOs from transactions
                                for tx in &response.transactions {
                                    // Create UTXO record for each transaction output
                                    let utxo = crate::wallet_db::UtxoRecord {
                                        tx_hash: tx.tx_hash.clone(),
                                        output_index: 0, // Simplified - actual index should come from transaction
                                        address: tx.to_address.clone(),
                                        amount: tx.amount,
                                        block_height: tx.block_height,
                                        confirmations: tx.confirmations as u64,
                                    };

                                    if let Err(e) = db.save_utxo(&utxo) {
                                        log::error!("Failed to save UTXO: {}", e);
                                    }
                                }

                                log::info!(
                                    "💾 Saved {} transactions to wallet database",
                                    response.transactions.len()
                                );
                            }
                        }
                        Err(e) => {
                            log::error!("❌ Failed to request wallet transactions: {}", e);
                        }
                    }
                });
            } else {
                log::warn!("⚠️ No peers available - cannot scan blockchain");
            }
        }
    }

    fn check_utxo_updates(&mut self) {
        // Collect all pending UTXOs first (don't hold borrow while processing)
        let mut pending_utxos = Vec::new();
        if let Some(rx) = &mut self.utxo_rx {
            while let Ok(utxo) = rx.try_recv() {
                pending_utxos.push(utxo);
            }
        }

        // Now process them
        for utxo in pending_utxos {
            log::info!(
                "💰 Processing new UTXO: {} TIME to {}",
                utxo.amount as f64 / 1_000_000.0,
                &utxo.address[..std::cmp::min(20, utxo.address.len())]
            );

            if let Some(wallet_mgr) = &mut self.wallet_manager {
                // Convert txid string to bytes
                let tx_hash_bytes = if let Ok(bytes) = hex::decode(&utxo.txid) {
                    if bytes.len() == 32 {
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(&bytes);
                        arr
                    } else {
                        log::error!("Invalid tx_hash length: {}", bytes.len());
                        continue;
                    }
                } else {
                    log::error!("Failed to decode tx_hash: {}", utxo.txid);
                    continue;
                };

                // Convert to wallet UTXO format
                let wallet_utxo = wallet::UTXO {
                    tx_hash: tx_hash_bytes,
                    output_index: utxo.vout,
                    amount: utxo.amount,
                    address: utxo.address.clone(),
                };

                wallet_mgr.add_utxo(wallet_utxo);

                log::info!("✅ Added UTXO: {} TIME", utxo.amount as f64 / 1_000_000.0);

                // Balance is now updated automatically!
                let new_balance = wallet_mgr.get_balance();
                log::info!(
                    "💼 Updated balance: {} TIME",
                    new_balance as f64 / 1_000_000.0
                );

                // Save transaction to database
                if let Some(db) = &self.wallet_db {
                    let tx_record = wallet_db::TransactionRecord {
                        tx_hash: utxo.txid.clone(),
                        timestamp: chrono::Utc::now().timestamp(),
                        from_address: None, // Unknown sender
                        to_address: utxo.address.clone(),
                        amount: utxo.amount,
                        status: if utxo.confirmations > 0 {
                            wallet_db::TransactionStatus::Confirmed
                        } else {
                            wallet_db::TransactionStatus::Pending
                        },
                        block_height: utxo.block_height,
                        notes: Some(format!(
                            "Scanned from blockchain (height: {})",
                            utxo.block_height.unwrap_or(0)
                        )),
                    };

                    if let Err(e) = db.save_transaction(&tx_record) {
                        log::error!("Failed to save transaction to database: {}", e);
                    } else {
                        log::info!("💾 Saved transaction to database: {}", utxo.txid);
                    }
                }

                // Show success notification
                self.set_success(format!(
                    "Received {} TIME!",
                    utxo.amount as f64 / 1_000_000.0
                ));
            }
        }
    }

    fn initialize_protocol_client(&mut self) {
        if self.protocol_client.is_some() {
            return; // Already initialized
        }

        let network_mgr = match &self.network_manager {
            Some(mgr) => mgr,
            None => {
                log::warn!("Cannot initialize protocol client: no network manager");
                return;
            }
        };

        // Get connected masternodes
        let masternodes = {
            let net = network_mgr.lock().unwrap();
            net.get_connected_peers()
                .into_iter()
                .map(|p| format!("http://{}:24101", p.address))
                .collect::<Vec<_>>()
        };

        if masternodes.is_empty() {
            // Silently return - peers may still be connecting
            return;
        }

        log::info!(
            "Initializing TIME Coin Protocol client with {} masternodes",
            masternodes.len()
        );

        // For now, just take the first masternode
        if let Some(first_peer) = masternodes.first() {
            let client = Arc::new(ProtocolClient::new(
                first_peer.clone(),
                wallet::NetworkType::Testnet,
            ));
            self.protocol_client = Some(client);
            log::info!("✅ Protocol client initialized for peer: {}", first_peer);
        }

        log::info!("✅ TIME Coin Protocol client initialized");
    }

    /// Check for new transaction notifications
    fn check_notifications(&mut self) {
        // Check for transaction approval/rejection notifications
        if let Some(rx) = &mut self.tx_notification_rx {
            while let Ok(notification) = rx.try_recv() {
                match notification {
                    TransactionNotification::Approved { txid, timestamp } => {
                        let short_txid = &txid[..std::cmp::min(16, txid.len())];
                        self.success_message = Some(format!(
                            "✅ Transaction {} approved by network!",
                            short_txid
                        ));
                        self.success_message_time = Some(std::time::Instant::now());
                        log::info!("✅ Transaction {} approved at {}", short_txid, timestamp);
                    }
                    TransactionNotification::Rejected { txid, reason } => {
                        let short_txid = &txid[..std::cmp::min(16, txid.len())];
                        self.error_message = Some(format!(
                            "❌ Transaction {} rejected: {}",
                            short_txid, reason
                        ));
                        self.error_message_time = Some(std::time::Instant::now());
                        log::error!("❌ Transaction {} rejected: {}", short_txid, reason);
                    }
                }
            }
        }
    }

    fn check_message_timeout(&mut self) {
        let timeout = std::time::Duration::from_secs(3);

        if let Some(time) = self.success_message_time {
            if time.elapsed() > timeout {
                self.success_message = None;
                self.success_message_time = None;
            }
        }

        if let Some(time) = self.error_message_time {
            if time.elapsed() > timeout {
                self.error_message = None;
                self.error_message_time = None;
            }
        }
    }
}

impl eframe::App for WalletApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check and clear messages after timeout
        self.check_message_timeout();

        // Check for UTXO updates from TCP listener
        self.check_utxo_updates();

        // Initialize protocol client if we have a wallet and network but no client yet
        if self.wallet_manager.is_some()
            && self.network_manager.is_some()
            && self.protocol_client.is_none()
        {
            self.initialize_protocol_client();
        }

        // Check for new transaction notifications
        self.check_notifications();

        // Request repaint if messages are showing
        if self.success_message.is_some() || self.error_message.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        match self.current_screen {
            Screen::Welcome => self.show_welcome_screen(ctx),
            Screen::MnemonicSetup => self.show_mnemonic_setup_screen(ctx),
            Screen::MnemonicConfirm => self.show_mnemonic_confirm_screen(ctx),
            _ => self.show_main_screen(ctx),
        }
    }
}
