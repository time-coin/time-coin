//! Simple TIME Wallet GUI - Clean rewrite with async client
//!
//! Architecture:
//! - Pure async TCP client (no blocking, no mutexes)
//! - Simple state management with channels
//! - Minimal dependencies

#![allow(dead_code)]

use eframe::egui;
use tokio::sync::mpsc;
use wallet::NetworkType;

mod client;
mod encryption;
mod wallet_dat;
mod wallet_manager;
mod wallet_manager_impl;

use client::SimpleClient;
use wallet_manager::WalletState;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    // Create Tokio runtime and keep it alive
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = rt.enter();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "TIME Wallet (Simple)",
        options,
        Box::new(|_cc| Ok(Box::new(WalletApp::default()))),
    )
}

#[derive(Default)]
enum Screen {
    #[default]
    Welcome,
    CreateWallet,
    UnlockWallet,
    Main,
}

struct WalletApp {
    screen: Screen,
    wallet: Option<WalletState>,
    client: Option<SimpleClient>,

    // Channels for async results
    tx_refresh: mpsc::UnboundedSender<RefreshResult>,
    rx_refresh: mpsc::UnboundedReceiver<RefreshResult>,

    // UI state
    password: String,
    mnemonic: String,
    error_message: Option<String>,
    transactions: Vec<Transaction>,
    balance: u64,
    is_refreshing: bool,
}

#[derive(Debug)]
enum RefreshResult {
    Transactions(Vec<Transaction>),
    Balance(u64),
    Error(String),
}

#[derive(Debug, Clone)]
struct Transaction {
    hash: String,
    from: String,
    to: String,
    amount: u64,
    timestamp: i64,
}

impl Default for WalletApp {
    fn default() -> Self {
        let (tx_refresh, rx_refresh) = mpsc::unbounded_channel();

        Self {
            screen: Screen::Welcome,
            wallet: None,
            client: None,
            tx_refresh,
            rx_refresh,
            password: String::new(),
            mnemonic: String::new(),
            error_message: None,
            transactions: Vec::new(),
            balance: 0,
            is_refreshing: false,
        }
    }
}

impl eframe::App for WalletApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process async results
        while let Ok(result) = self.rx_refresh.try_recv() {
            match result {
                RefreshResult::Transactions(txs) => {
                    log::info!("✅ Received {} transactions", txs.len());
                    self.transactions = txs;
                    self.is_refreshing = false;
                }
                RefreshResult::Balance(bal) => {
                    self.balance = bal;
                }
                RefreshResult::Error(err) => {
                    log::error!("❌ Refresh error: {}", err);
                    self.error_message = Some(err);
                    self.is_refreshing = false;
                }
            }
        }

        match self.screen {
            Screen::Welcome => self.show_welcome(ctx),
            Screen::CreateWallet => self.show_create_wallet(ctx),
            Screen::UnlockWallet => self.show_unlock_wallet(ctx),
            Screen::Main => self.show_main_screen(ctx),
        }

        // Request repaint for smooth UI
        ctx.request_repaint();
    }
}

impl WalletApp {
    fn show_welcome(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.heading(egui::RichText::new("⏳").size(80.0));
                ui.add_space(20.0);
                ui.heading("TIME Wallet (Simple)");
                ui.label("No blocking, no mutexes, just async!");
                ui.add_space(40.0);

                if ui.button("Create New Wallet").clicked() {
                    self.screen = Screen::CreateWallet;
                }

                ui.add_space(10.0);

                if ui.button("Unlock Existing Wallet").clicked() {
                    self.screen = Screen::UnlockWallet;
                }
            });
        });
    }

    fn show_create_wallet(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Create New Wallet");
            ui.add_space(20.0);

            ui.label("Enter a password to encrypt your wallet:");
            let _password_response =
                ui.add(egui::TextEdit::singleline(&mut self.password).password(true));

            ui.add_space(20.0);

            if ui.button("Generate Wallet").clicked()
                || (_password_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                // Generate mnemonic
                let mnemonic = match wallet::generate_mnemonic(12) {
                    Ok(m) => m,
                    Err(e) => {
                        self.error_message = Some(format!("Failed to generate mnemonic: {}", e));
                        return;
                    }
                };

                match WalletState::create(NetworkType::Testnet, &mnemonic, &self.password) {
                    Ok(wallet) => {
                        self.mnemonic = wallet.get_mnemonic();
                        let xpub = wallet.get_xpub();
                        self.wallet = Some(wallet);

                        // Initialize client
                        let client = SimpleClient::new(
                            "134.199.175.106:24100".to_string(),
                            NetworkType::Testnet,
                        );

                        // Register xpub with masternode
                        let xpub_clone = xpub.clone();
                        let client_clone = client.clone();
                        tokio::spawn(async move {
                            match client_clone.register_xpub(&xpub_clone).await {
                                Ok(_) => log::info!("✅ Registered xpub with masternode"),
                                Err(e) => log::warn!("⚠️ Failed to register xpub: {}", e),
                            }
                        });

                        self.client = Some(client);

                        log::info!("✅ Wallet created successfully");
                        self.screen = Screen::Main;
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to create wallet: {}", e));
                    }
                }
            }

            if let Some(err) = &self.error_message {
                ui.add_space(10.0);
                ui.colored_label(egui::Color32::RED, err);
            }
        });
    }

    fn show_unlock_wallet(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Unlock Wallet");
            ui.add_space(20.0);

            ui.label("Enter your password:");
            let _password_response =
                ui.add(egui::TextEdit::singleline(&mut self.password).password(true));

            ui.add_space(20.0);

            if ui.button("Unlock").clicked() {
                match WalletState::load(NetworkType::Testnet, &self.password) {
                    Ok(wallet) => {
                        self.wallet = Some(wallet);

                        // Initialize client
                        self.client = Some(SimpleClient::new(
                            "134.199.175.106:24100".to_string(),
                            NetworkType::Testnet,
                        ));

                        log::info!("✅ Wallet unlocked");
                        self.screen = Screen::Main;

                        // Auto-refresh on unlock
                        self.trigger_refresh();
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to unlock: {}", e));
                    }
                }
            }

            if let Some(err) = &self.error_message {
                ui.add_space(10.0);
                ui.colored_label(egui::Color32::RED, err);
            }
        });
    }

    fn show_main_screen(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("TIME Wallet");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🔄 Refresh").clicked() && !self.is_refreshing {
                        self.trigger_refresh();
                    }

                    if self.is_refreshing {
                        ui.spinner();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("Balance: {} TIME", self.balance as f64 / 1e8));
            ui.add_space(20.0);

            ui.heading("Recent Transactions");
            ui.separator();

            if self.transactions.is_empty() {
                ui.label("No transactions yet. Click Refresh to check.");
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for tx in &self.transactions {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(format!("📄 {}", &tx.hash[..16]));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(format!("{} TIME", tx.amount as f64 / 1e8));
                                    },
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label(format!("From: {}", &tx.from[..20]));
                            });
                            ui.horizontal(|ui| {
                                ui.label(format!("To: {}", &tx.to[..20]));
                            });
                        });
                        ui.add_space(5.0);
                    }
                });
            }
        });
    }

    fn trigger_refresh(&mut self) {
        if let (Some(wallet), Some(client)) = (&self.wallet, &self.client) {
            self.is_refreshing = true;
            let xpub = wallet.get_xpub();
            let client = client.clone();
            let tx = self.tx_refresh.clone();

            log::info!("🔄 Starting refresh for xpub: {}...", &xpub[..30]);

            // Spawn async task - don't block UI
            tokio::spawn(async move {
                log::info!("📡 Connecting to masternode...");
                match client.get_transactions(&xpub).await {
                    Ok(txs) => {
                        log::info!("📦 Received {} transactions from masternode", txs.len());
                        let transactions = txs
                            .into_iter()
                            .map(|t| Transaction {
                                hash: t.tx_hash,
                                from: t.from_address,
                                to: t.to_address,
                                amount: t.amount,
                                timestamp: t.timestamp as i64,
                            })
                            .collect();

                        let _ = tx.send(RefreshResult::Transactions(transactions));
                    }
                    Err(e) => {
                        log::error!("❌ Refresh failed: {}", e);
                        let _ = tx.send(RefreshResult::Error(format!("Refresh failed: {}", e)));
                    }
                }
            });
        }
    }
}
