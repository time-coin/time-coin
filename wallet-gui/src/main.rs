#![allow(dead_code)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::get_first)]
#![allow(clippy::manual_while_let_some)]
#![allow(clippy::empty_line_after_doc_comments)]
#![allow(unused_variables)]
#![allow(non_snake_case)]
use eframe::egui;
use std::sync::{Arc, Mutex};
use wallet::NetworkType;

mod config;
mod mnemonic_ui;
mod network;
mod protocol_client;
mod wallet_dat;
mod wallet_db;
mod wallet_manager;

use config::Config;
use mnemonic_ui::{MnemonicAction, MnemonicInterface};
use network::NetworkManager;
use protocol_client::{ProtocolClient, WalletNotification};
use tokio::sync::mpsc;
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
    network_manager: Option<Arc<Mutex<NetworkManager>>>,
    network_status: String,

    // TIME Coin Protocol client for real-time notifications
    protocol_client: Option<Arc<ProtocolClient>>,
    notification_rx: Option<mpsc::UnboundedReceiver<WalletNotification>>,
    recent_notifications: Vec<WalletNotification>,

    // Mnemonic setup - NEW enhanced interface
    mnemonic_interface: MnemonicInterface,
    mnemonic_confirmed: bool,

    // Receiving address management
    selected_address: Option<String>,
    new_address_label: String,
    edit_address_name: String,
    edit_address_email: String,
    edit_address_phone: String,
    address_search: String,
    show_qr_for_address: Option<String>,
    is_creating_new_address: bool,
}

impl Default for WalletApp {
    fn default() -> Self {
        // Don't initialize wallet database here - it will be opened when wallet is loaded
        // to avoid lock conflicts with WalletManager's database

        Self {
            current_screen: Screen::Welcome,
            wallet_manager: None,
            wallet_db: None,
            network: NetworkType::Testnet,
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
            protocol_client: None,
            notification_rx: None,
            recent_notifications: Vec::new(),
            mnemonic_interface: MnemonicInterface::new(),
            mnemonic_confirmed: false,
            selected_address: None,
            new_address_label: String::new(),
            edit_address_name: String::new(),
            edit_address_email: String::new(),
            edit_address_phone: String::new(),
            address_search: String::new(),
            show_qr_for_address: None,
            is_creating_new_address: false,
        }
    }
}

impl WalletApp {
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

                    // TODO: Add password protection, fingerprint, or PIN authentication
                    // For now, auto-load wallet without password

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

                                self.wallet_manager = Some(manager);
                                self.current_screen = Screen::Overview;
                                self.set_success("Wallet unlocked successfully!".to_string());

                                // Load config and initialize network
                                if let Ok(main_config) = Config::load() {
                                    let network_mgr = Arc::new(Mutex::new(NetworkManager::new(main_config.api_endpoint.clone())));
                                    self.network_manager = Some(network_mgr.clone());
                                    self.network_status = "Connecting...".to_string();

                                    // Trigger network bootstrap in background
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

                                                // Trigger initial transaction sync
                                                ctx_clone.request_repaint();

                                                // Start periodic latency refresh task
                                                let network_refresh = network_mgr.clone();
                                                tokio::spawn(async move {
                                                    loop {
                                                        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                                                        log::info!("Running scheduled latency refresh...");
                                                        {
                                                            // Clone peer list to avoid holding lock during async operations
                                                            let mut peers = {
                                                                let manager = network_refresh.lock().unwrap();
                                                                manager.get_connected_peers()
                                                            };

                                                            log::info!("Pinging {} peers to measure latency", peers.len());

                                                            // Measure latencies without holding the lock
                                                            for peer in &mut peers {
                                                                let peer_ip = peer.address.split(':').next().unwrap_or(&peer.address);
                                                                let url = format!("http://{}:24101/blockchain/info", peer_ip);
                                                                let start = std::time::Instant::now();

                                                                let client = reqwest::Client::builder()
                                                                    .timeout(std::time::Duration::from_secs(5))
                                                                    .build()
                                                                    .unwrap();

                                                                match client.get(&url).send().await {
                                                                    Ok(_) => {
                                                                        peer.latency_ms = start.elapsed().as_millis() as u64;
                                                                        log::info!("  Peer {} responded in {}ms", peer.address, peer.latency_ms);
                                                                    }
                                                                    Err(_) => {
                                                                        peer.latency_ms = 9999;
                                                                    }
                                                                }
                                                            }

                                                            // Update the network manager with new latencies
                                                            if let Ok(mut manager) = network_refresh.lock() {
                                                                manager.set_connected_peers(peers);
                                                            }
                                                        }
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
                } else {
                    ui.heading("Create New Wallet");
                    ui.add_space(20.0);

                    if ui.button(egui::RichText::new("Create Wallet").size(16.0)).clicked() {
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
                            // If wallet exists, backup first
                            if wallet_exists {
                                match self.backup_and_create_new_wallet(&phrase) {
                                    Ok(_) => {
                                        self.mnemonic_interface.wallet_created = true;
                                        self.current_screen = Screen::Overview;
                                    }
                                    Err(e) => {
                                        self.set_error(e);
                                    }
                                }
                            } else {
                                // Store the phrase and create wallet
                                self.create_wallet_from_mnemonic_phrase(&phrase, ctx);
                            }
                        }
                        MnemonicAction::Cancel => {
                            self.current_screen = Screen::Welcome;
                            self.mnemonic_interface = MnemonicInterface::new();
                        }
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

                // Load config and initialize network
                if let Ok(main_config) = Config::load() {
                    let network_mgr = Arc::new(Mutex::new(NetworkManager::new(
                        main_config.api_endpoint.clone(),
                    )));
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
            log::info!("Auto-triggering transaction sync");
            self.trigger_transaction_sync();
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

                                            // Send transaction via protocol client
                                            if let Some(ref protocol_client) = self.protocol_client {
                                                let client = protocol_client.clone();
                                                let tx = transaction.clone();
                                                let txid = tx.txid();
                                                let txid_clone = txid.clone();
                                                let db_opt = self.wallet_db.clone();

                                                tokio::spawn(async move {
                                                    match client.send_transaction(tx).await {
                                                        Ok(txid) => {
                                                            log::info!("✓ Transaction sent successfully: {}", txid);
                                                        }
                                                        Err(e) => {
                                                            log::error!("✗ Failed to send transaction: {}", e);
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
                                                self.set_success(format!("Transaction submitted: {}", txid));
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
                        ui.text_edit_singleline(&mut self.new_address_label);
                        ui.label(
                            egui::RichText::new("Leave empty for unnamed address")
                                .size(10.0)
                                .color(egui::Color32::GRAY),
                        );

                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            if ui.button("✓ Create").clicked() {
                                if let Some(ref mut manager) = self.wallet_manager {
                                    // Generate new address with derivation index
                                    match manager.generate_new_address_with_index() {
                                        Ok((address, index)) => {
                                            let label = if self.new_address_label.is_empty() {
                                                format!("Address {}", index + 1)
                                            } else {
                                                self.new_address_label.clone()
                                            };
                                            pending_action = Some(AddressAction::CreateNew(
                                                address, index, label,
                                            ));
                                        }
                                        Err(e) => {
                                            pending_action = Some(AddressAction::CreateNew(
                                                String::new(),
                                                0,
                                                format!("ERROR: {}", e),
                                            ));
                                        }
                                    }
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
                            ui.add_space(50.0);
                            ui.label(
                                egui::RichText::new("No transactions yet")
                                    .size(16.0)
                                    .color(egui::Color32::GRAY),
                            );
                            ui.add_space(10.0);
                            ui.label("Click 'Sync Transactions' to fetch from network");
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
        // Transaction sync now happens automatically via WebSocket protocol_client
        // The xpub subscription is already active from the connection established on startup
        log::info!("✅ Wallet sync via TCP WebSocket (automatic - already connected)");

        // The protocol_client is already subscribed to our xpub and receiving real-time updates
        // No manual HTTP sync needed
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

    /// Initialize TIME Coin Protocol client for real-time transaction notifications
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
            let net = match network_mgr.lock() {
                Ok(n) => n,
                Err(e) => {
                    log::error!("Failed to lock network manager: {}", e);
                    return;
                }
            };
            net.get_connected_peers()
                .into_iter()
                .map(|p| format!("http://{}:24101", p.address))
                .collect::<Vec<_>>()
        };

        if masternodes.is_empty() {
            log::warn!("No masternodes available for protocol client");
            return;
        }

        log::info!(
            "Initializing TIME Coin Protocol client with {} masternodes",
            masternodes.len()
        );

        let (client, rx) = ProtocolClient::new(masternodes);
        let client = Arc::new(client);

        // Connect in background
        let client_clone = client.clone();
        tokio::spawn(async move {
            match client_clone.connect().await {
                Ok(_) => log::info!("✅ TIME Coin Protocol client connected!"),
                Err(e) => log::error!("❌ Protocol client connection failed: {}", e),
            }
        });

        // Subscribe to wallet xpub
        if let Some(manager) = &self.wallet_manager {
            // Get xpub from wallet
            let xpub = manager.get_xpub().to_string();
            log::info!("Subscribing to xpub: {}...", &xpub[..20]);
            let client_clone = client.clone();
            tokio::spawn(async move {
                if let Err(e) = client_clone.subscribe_xpub(xpub).await {
                    log::error!("Failed to subscribe to xpub: {}", e);
                } else {
                    log::info!("✅ Subscribed to xpub for real-time updates!");
                }
            });
        }

        self.protocol_client = Some(client);
        self.notification_rx = Some(rx);

        log::info!("✅ TIME Coin Protocol client initialized");
    }

    /// Check for new transaction notifications
    fn check_notifications(&mut self) {
        // Take receiver temporarily to avoid borrow issues
        let mut rx = match self.notification_rx.take() {
            Some(rx) => rx,
            None => return,
        };

        // Process all pending notifications
        while let Ok(notification) = rx.try_recv() {
            log::info!(
                "📨 Transaction notification: {} - state: {:?}",
                notification.txid,
                notification.state
            );

            // Store or update transaction in database
            if let Some(db) = &self.wallet_db {
                // Check if transaction already exists
                let existing_tx = db.get_transaction(&notification.txid).ok().flatten();

                let status = match notification.state {
                    protocol_client::TransactionState::Pending => {
                        wallet_db::TransactionStatus::Pending
                    }
                    protocol_client::TransactionState::Approved { .. } => {
                        wallet_db::TransactionStatus::Approved
                    }
                    protocol_client::TransactionState::Finalized => {
                        wallet_db::TransactionStatus::Approved
                    }
                    protocol_client::TransactionState::Declined { .. } => {
                        wallet_db::TransactionStatus::Declined
                    }
                    protocol_client::TransactionState::Confirmed { .. } => {
                        wallet_db::TransactionStatus::Confirmed
                    }
                };

                let block_height = match notification.state {
                    protocol_client::TransactionState::Confirmed { block_height } => {
                        Some(block_height)
                    }
                    _ => None,
                };

                // If transaction exists, update its status
                if let Some(mut existing) = existing_tx {
                    existing.status = status;
                    if let Some(height) = block_height {
                        existing.block_height = Some(height);
                    }

                    if let Err(e) = db.save_transaction(&existing) {
                        log::error!("Failed to update transaction: {}", e);
                    } else {
                        log::info!(
                            "✓ Updated transaction {} status to {:?}",
                            notification.txid,
                            existing.status
                        );
                    }
                } else if notification.amount > 0 {
                    // New transaction - only save if we have amount info
                    let tx_record = wallet_db::TransactionRecord {
                        tx_hash: notification.txid.clone(),
                        timestamp: notification.timestamp,
                        from_address: if notification.is_incoming {
                            Some(notification.address.clone())
                        } else {
                            None
                        },
                        to_address: notification.address.clone(),
                        amount: notification.amount,
                        status,
                        block_height,
                        notes: None,
                    };

                    if let Err(e) = db.save_transaction(&tx_record) {
                        log::error!("Failed to save transaction: {}", e);
                    } else {
                        log::info!("💾 Saved new transaction {} to database", notification.txid);
                    }
                }
            }

            // Add to recent notifications
            self.recent_notifications.push(notification.clone());

            // Keep only last 100 notifications
            if self.recent_notifications.len() > 100 {
                self.recent_notifications.remove(0);
            }

            // Show appropriate message based on state
            match notification.state {
                protocol_client::TransactionState::Approved { votes, total_nodes } => {
                    self.set_success(format!(
                        "✓ Transaction approved ({}/{} votes)",
                        votes, total_nodes
                    ));
                }
                protocol_client::TransactionState::Confirmed { block_height } => {
                    self.set_success(format!("✓ Transaction confirmed at block {}", block_height));
                }
                protocol_client::TransactionState::Declined { ref reason } => {
                    self.set_error(format!("✗ Transaction declined: {}", reason));
                }
                _ => {}
            }
        }

        // Put receiver back
        self.notification_rx = Some(rx);
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

        // Initialize protocol client if we have a wallet and network but no client yet
        if self.wallet_manager.is_some()
            && self.network_manager.is_some()
            && self.protocol_client.is_none()
        {
            self.initialize_protocol_client();
        }

        // Check for new transaction notifications
        self.check_notifications();

        // Request repaint if messages are showing or if we're receiving notifications
        if self.success_message.is_some()
            || self.error_message.is_some()
            || self.notification_rx.is_some()
        {
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
