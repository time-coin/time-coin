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
mod wallet_dat;
mod wallet_manager;

use config::{Config, WalletConfig};
use mnemonic_ui::{MnemonicAction, MnemonicInterface};
use network::NetworkManager;
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
    network: NetworkType,
    password: String,
    error_message: Option<String>,
    success_message: Option<String>,

    // Send screen fields
    send_address: String,
    send_amount: String,

    // UI state
    show_private_key: bool,

    // Configuration
    config: WalletConfig,

    // Network manager (wrapped for thread safety)
    network_manager: Option<Arc<Mutex<NetworkManager>>>,
    network_status: String,

    // Mnemonic setup - NEW enhanced interface
    mnemonic_interface: MnemonicInterface,
    mnemonic_confirmed: bool,

    // Receiving address management
    selected_address_index: Option<usize>,
    new_address_label: String,
    edit_address_name: String,
    edit_address_email: String,
    edit_address_phone: String,
    show_qr_for_address: Option<String>,
    is_creating_new_address: bool,
}

impl Default for WalletApp {
    fn default() -> Self {
        Self {
            current_screen: Screen::Welcome,
            wallet_manager: None,
            network: NetworkType::Testnet,
            password: String::new(),
            error_message: None,
            success_message: None,
            send_address: String::new(),
            send_amount: String::new(),
            show_private_key: false,
            config: WalletConfig::default(),
            network_manager: None,
            network_status: "Not connected".to_string(),
            mnemonic_interface: MnemonicInterface::new(),
            mnemonic_confirmed: false,
            selected_address_index: None,
            new_address_label: String::new(),
            edit_address_name: String::new(),
            edit_address_email: String::new(),
            edit_address_phone: String::new(),
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

                    ui.selectable_value(&mut self.network, NetworkType::Mainnet, "Mainnet");
                    ui.selectable_value(&mut self.network, NetworkType::Testnet, "Testnet");
                });

                ui.add_space(40.0);

                if WalletManager::exists(self.network) {
                    ui.heading("Welcome Back!");
                    ui.add_space(20.0);

                    ui.label("Password:");
                    ui.add_space(5.0);
                    ui.add(egui::TextEdit::singleline(&mut self.password).password(true).hint_text("Enter password"));

                    ui.add_space(20.0);

                    if ui.button(egui::RichText::new("Unlock Wallet").size(16.0)).clicked() {
                        match WalletManager::load_default(self.network) {
                            Ok(manager) => {
                                self.wallet_manager = Some(manager);
                                self.current_screen = Screen::Overview;
                                self.success_message = Some("Wallet unlocked successfully!".to_string());

                                // Load config and initialize network
                                if let Ok(main_config) = Config::load() {
                                    let wallet_dir = main_config.wallet_dir();
                                    if let Ok(wallet_config) = WalletConfig::load(&wallet_dir) {
                                        self.config = wallet_config.clone();
                                    }
                                    let network_mgr = Arc::new(Mutex::new(NetworkManager::new(main_config.api_endpoint.clone())));
                                    self.network_manager = Some(network_mgr.clone());
                                    self.network_status = "Connecting...".to_string();

                                    // Trigger network bootstrap in background
                                    let bootstrap_nodes = main_config.bootstrap_nodes.clone();
                                    let ctx_clone = ctx.clone();

                                    tokio::spawn(async move {
                                        let api_endpoint = {
                                            let net = network_mgr.lock().unwrap();
                                            net.api_endpoint().to_string()
                                        };

                                        let mut temp_net = NetworkManager::new(api_endpoint);
                                        match temp_net.bootstrap(bootstrap_nodes).await {
                                            Ok(_) => {
                                                log::info!("Network bootstrap successful!");
                                                {
                                                    let mut net = network_mgr.lock().unwrap();
                                                    *net = temp_net;
                                                }

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
                                self.error_message = Some(format!("Failed to load wallet: {}", e));
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
                                        self.error_message = Some(e);
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
        match WalletManager::create_from_mnemonic(
            self.network,
            phrase,
            "", // No passphrase for now
            "Default".to_string(),
        ) {
            Ok(manager) => {
                self.wallet_manager = Some(manager);
                self.current_screen = Screen::Overview;
                self.success_message = Some("Wallet created successfully!".to_string());

                // Mark that wallet has been created from this phrase
                self.mnemonic_interface.wallet_created = true;

                // Clear mnemonic from memory
                self.mnemonic_interface.clear();
                self.mnemonic_confirmed = false;

                // Load config and initialize network
                if let Ok(main_config) = Config::load() {
                    let wallet_dir = main_config.wallet_dir();
                    if let Ok(wallet_config) = WalletConfig::load(&wallet_dir) {
                        self.config = wallet_config.clone();
                    }
                    let network_mgr = Arc::new(Mutex::new(NetworkManager::new(
                        main_config.api_endpoint.clone(),
                    )));
                    self.network_manager = Some(network_mgr.clone());
                    self.network_status = "Connecting...".to_string();

                    let bootstrap_nodes = main_config.bootstrap_nodes.clone();
                    let ctx_clone = ctx.clone();

                    tokio::spawn(async move {
                        let api_endpoint = {
                            let net = network_mgr.lock().unwrap();
                            net.api_endpoint().to_string()
                        };

                        let mut temp_net = NetworkManager::new(api_endpoint);
                        match temp_net.bootstrap(bootstrap_nodes).await {
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
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to create wallet: {}", e));
            }
        }
    }

    // Old function removed - using create_wallet_from_mnemonic_phrase instead

    fn show_main_screen(&mut self, ctx: &egui::Context) {
        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Backup Wallet").clicked() {
                        if let Err(e) = self.backup_current_wallet() {
                            self.error_message = Some(format!("Backup failed: {}", e));
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

                    let balance = manager.get_balance();

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

                    ui.label(
                        egui::RichText::new("No transactions yet")
                            .color(egui::Color32::GRAY)
                            .italics(),
                    );
                    ui.add_space(10.0);

                    // Placeholder for transaction list
                    egui::ScrollArea::vertical().show(ui, |_ui| {
                        // Will show transactions here when network is integrated
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

    fn show_send_screen(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.heading("Send TIME Coins");
        ui.add_space(20.0);

        if let Some(_manager) = &self.wallet_manager {
            ui.label("Pay To:");
            ui.text_edit_singleline(&mut self.send_address);
            ui.add_space(10.0);

            ui.label("Amount:");
            ui.text_edit_singleline(&mut self.send_amount);
            ui.add_space(20.0);

            if ui.button("Send").clicked() {
                self.success_message =
                    Some("Transaction sent! (Network integration pending)".to_string());
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

        if let Some(manager) = &mut self.wallet_manager {
            // Two column layout
            ui.horizontal(|ui| {
                // Left side - Address list
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width() * 0.5);
                    ui.heading("Your Addresses");
                    ui.add_space(10.0);

                    // Button to create new address
                    if ui.button("➕ New Address").clicked() {
                        self.new_address_label =
                            format!("Address {}", manager.get_keys().len() + 1);
                        self.is_creating_new_address = true;
                    }

                    // Show dialog for new address
                    if self.is_creating_new_address {
                        ui.add_space(10.0);
                        ui.group(|ui| {
                            ui.label("New Address Label:");
                            ui.text_edit_singleline(&mut self.new_address_label);
                            ui.add_space(5.0);
                            ui.horizontal(|ui| {
                                if ui.button("✓ Create").clicked() {
                                    match manager
                                        .generate_new_key(self.new_address_label.clone(), false)
                                    {
                                        Ok(address) => {
                                            self.success_message =
                                                Some(format!("Created new address: {}", address));
                                            self.is_creating_new_address = false;
                                            self.new_address_label = String::new();
                                        }
                                        Err(e) => {
                                            self.error_message =
                                                Some(format!("Failed to create address: {}", e));
                                        }
                                    }
                                }
                                if ui.button("✗ Cancel").clicked() {
                                    self.is_creating_new_address = false;
                                    self.new_address_label = String::new();
                                }
                            });
                        });
                    }

                    ui.add_space(10.0);

                    // List all addresses
                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| {
                            let keys = manager.get_keys();
                            for (idx, key) in keys.iter().enumerate() {
                                let is_selected = self.selected_address_index == Some(idx);

                                ui.group(|ui| {
                                    ui.set_min_width(ui.available_width() - 20.0);

                                    let response = ui.selectable_label(
                                        is_selected,
                                        format!(
                                            "{} {}",
                                            if key.is_default { "⭐" } else { "  " },
                                            key.label
                                        ),
                                    );

                                    if response.clicked() {
                                        self.selected_address_index = Some(idx);
                                        self.show_qr_for_address = Some(key.address.clone());
                                        // Don't clear fields - preserve any entered data
                                    }

                                    ui.add_space(5.0);
                                    ui.horizontal(|ui| {
                                        ui.monospace(
                                            egui::RichText::new(format!(
                                                "{}...{}",
                                                &key.address[..8],
                                                &key.address[key.address.len() - 6..]
                                            ))
                                            .size(11.0)
                                            .color(egui::Color32::GRAY),
                                        );

                                        if ui.small_button("📋").clicked() {
                                            ctx.copy_text(key.address.clone());
                                            self.success_message =
                                                Some("Address copied!".to_string());
                                        }

                                        if ui.small_button("🔍").clicked() {
                                            self.selected_address_index = Some(idx);
                                            self.show_qr_for_address = Some(key.address.clone());
                                        }
                                    });
                                });
                                ui.add_space(5.0);
                            }
                        });
                });

                ui.separator();

                // Right side - Address details and QR code
                ui.vertical(|ui| {
                    if let Some(idx) = self.selected_address_index {
                        let keys = manager.get_keys();
                        if let Some(key) = keys.get(idx) {
                            ui.heading(&key.label);
                            ui.add_space(10.0);

                            // Full address with copy button
                            ui.horizontal(|ui| {
                                ui.label("Address:");
                            });
                            ui.horizontal(|ui| {
                                ui.monospace(&key.address);
                                if ui.button("📋 Copy").clicked() {
                                    ctx.copy_text(key.address.clone());
                                    self.success_message =
                                        Some("Address copied to clipboard!".to_string());
                                }
                            });

                            ui.add_space(20.0);

                            // Contact information section
                            ui.heading("Contact Information");
                            ui.add_space(10.0);

                            ui.horizontal(|ui| {
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut self.edit_address_name);
                            });

                            ui.horizontal(|ui| {
                                ui.label("Email:");
                                ui.text_edit_singleline(&mut self.edit_address_email);
                            });

                            ui.horizontal(|ui| {
                                ui.label("Phone:");
                                ui.text_edit_singleline(&mut self.edit_address_phone);
                            });

                            ui.add_space(10.0);

                            if ui.button("💾 Save Contact Info").clicked() {
                                // TODO: Save contact info to wallet metadata
                                self.success_message = Some("Contact info saved!".to_string());
                            }

                            ui.add_space(20.0);

                            // QR Code
                            if let Some(address) = &self.show_qr_for_address {
                                if let Ok(svg_string) = manager.get_address_qr_code_svg(address) {
                                    ui.vertical_centered(|ui| {
                                        ui.label("📱 Scan QR Code:");
                                        ui.add_space(10.0);

                                        // Convert SVG to image and display (smaller size)
                                        if let Ok(image_data) = Self::svg_to_image(&svg_string) {
                                            let texture = ctx.load_texture(
                                                "qr_code",
                                                image_data,
                                                egui::TextureOptions::default(),
                                            );
                                            ui.add(
                                                egui::Image::new(&texture)
                                                    .max_size(egui::vec2(200.0, 200.0)),
                                            );
                                        } else {
                                            ui.label("Failed to render QR code");
                                        }
                                    });
                                }
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
        }

        if let Some(msg) = &self.success_message {
            ui.add_space(10.0);
            ui.colored_label(egui::Color32::GREEN, msg);
        }
    }

    fn show_transactions_screen(&mut self, ui: &mut egui::Ui) {
        ui.heading("Transaction History");
        ui.add_space(20.0);

        ui.label(
            "Transaction history will be displayed here once network integration is complete.",
        );
        ui.add_space(10.0);
        ui.label("Features coming soon:");
        ui.label("  📥 Received transactions");
        ui.label("  📤 Sent transactions");
        ui.label("  ⚡ Masternode rewards");
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
                    let wallet_dir = manager
                        .wallet_path()
                        .parent()
                        .unwrap_or(manager.wallet_path());
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

                ui.checkbox(&mut self.show_private_key, "Show Private Key");

                if self.show_private_key {
                    ui.add_space(10.0);
                    ui.colored_label(
                        egui::Color32::RED,
                        "⚠️ WARNING: Never share your private key!",
                    );
                    ui.add_space(5.0);

                    if let Some(address) = manager.get_primary_address() {
                        if let Some(private_key) = manager.export_private_key(&address) {
                            ui.monospace(&private_key);
                            if ui.button("📋 Copy Private Key").clicked() {
                                ctx.copy_text(private_key);
                            }
                        }
                    }
                }
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

        // Create new wallet from the new phrase using the existing method
        let manager = WalletManager::create_from_mnemonic(
            self.network,
            new_phrase,
            "", // No passphrase
            "Default".to_string(),
        )
        .map_err(|e| format!("Failed to create wallet: {}", e))?;

        self.wallet_manager = Some(manager);
        self.success_message = Some(format!("Old wallet backed up to: {}", backup_path));

        Ok(())
    }
}

impl eframe::App for WalletApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Clear messages after a delay
        if self.success_message.is_some() || self.error_message.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_secs(5));
        }

        match self.current_screen {
            Screen::Welcome => self.show_welcome_screen(ctx),
            Screen::MnemonicSetup => self.show_mnemonic_setup_screen(ctx),
            Screen::MnemonicConfirm => self.show_mnemonic_confirm_screen(ctx),
            _ => self.show_main_screen(ctx),
        }
    }
}
