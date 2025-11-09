//! TIME Coin GUI Wallet Application

use eframe::egui;
use wallet::NetworkType;

mod wallet_dat;
mod wallet_manager;

use wallet_manager::WalletManager;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "TIME Coin Wallet",
        options,
        Box::new(|cc| {
            // Use custom fonts if available
            setup_custom_fonts(&cc.egui_ctx);
            Ok(Box::new(WalletApp::new(cc)))
        }),
    )
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let fonts = egui::FontDefinitions::default();
    
    // Larger default font sizes for better readability
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (egui::TextStyle::Heading, egui::FontId::new(24.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Body, egui::FontId::new(16.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Monospace, egui::FontId::new(14.0, egui::FontFamily::Monospace)),
        (egui::TextStyle::Button, egui::FontId::new(16.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Small, egui::FontId::new(12.0, egui::FontFamily::Proportional)),
    ]
    .into();
    
    ctx.set_style(style);
    ctx.set_fonts(fonts);
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Screen {
    Welcome,
    Main,
    Send,
    Receive,
    Transactions,
    Settings,
}

struct WalletApp {
    wallet_manager: Option<WalletManager>,
    current_screen: Screen,
    network: NetworkType,
    
    // Welcome screen state
    wallet_name: String,
    import_key: String,
    error_message: Option<String>,
    success_message: Option<String>,
    
    // Send screen state
    send_address: String,
    send_amount: String,
    send_fee: String,
    
    // Settings state
    show_private_key: bool,
}

impl WalletApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Start with testnet for now (can be made configurable)
        let network = NetworkType::Testnet;
        
        // Try to load existing wallet
        let wallet_manager = WalletManager::load_default(network).ok();
        let current_screen = if wallet_manager.is_some() {
            Screen::Main
        } else {
            Screen::Welcome
        };
        
        Self {
            wallet_manager,
            current_screen,
            network,
            wallet_name: "Default".to_string(),
            import_key: String::new(),
            error_message: None,
            success_message: None,
            send_address: String::new(),
            send_amount: String::new(),
            send_fee: "10".to_string(), // Default fee
            show_private_key: false,
        }
    }

    fn show_welcome_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.heading("⏰ Welcome to TIME Coin Wallet");
                ui.add_space(20.0);
                ui.label("Create a new wallet or import an existing one");
                ui.add_space(40.0);

                // Error/Success messages
                if let Some(err) = &self.error_message {
                    ui.colored_label(egui::Color32::RED, err);
                    ui.add_space(10.0);
                }
                if let Some(msg) = &self.success_message {
                    ui.colored_label(egui::Color32::GREEN, msg);
                    ui.add_space(10.0);
                }

                ui.horizontal(|ui| {
                    ui.label("Network:");
                    ui.label(match self.network {
                        NetworkType::Mainnet => "Mainnet",
                        NetworkType::Testnet => "Testnet",
                    });
                });
                ui.add_space(20.0);

                // Create new wallet section
                ui.group(|ui| {
                    ui.set_min_width(400.0);
                    ui.heading("Create New Wallet");
                    ui.add_space(10.0);
                    
                    ui.horizontal(|ui| {
                        ui.label("Wallet Name:");
                        ui.text_edit_singleline(&mut self.wallet_name);
                    });
                    ui.add_space(10.0);

                    if ui.button("Create Wallet").clicked() {
                        self.create_new_wallet();
                    }
                });

                ui.add_space(30.0);

                // Import wallet section
                ui.group(|ui| {
                    ui.set_min_width(400.0);
                    ui.heading("Import Existing Key");
                    ui.add_space(10.0);
                    
                    ui.horizontal(|ui| {
                        ui.label("Private Key (hex):");
                    });
                    ui.add_space(5.0);
                    ui.text_edit_singleline(&mut self.import_key);
                    ui.add_space(10.0);

                    if ui.button("Import Key").clicked() {
                        self.import_existing_key();
                    }
                });
            });
        });
    }

    fn create_new_wallet(&mut self) {
        match WalletManager::create_new(self.network, self.wallet_name.clone()) {
            Ok(manager) => {
                self.wallet_manager = Some(manager);
                self.current_screen = Screen::Main;
                self.error_message = None;
                self.success_message = Some("Wallet created successfully!".to_string());
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to create wallet: {}", e));
            }
        }
    }

    fn import_existing_key(&mut self) {
        // First create a wallet manager with a new wallet
        match WalletManager::create_new(self.network, "Imported".to_string()) {
            Ok(mut manager) => {
                // Now import the key
                match manager.import_private_key(&self.import_key, "Imported Key".to_string()) {
                    Ok(addr) => {
                        // Set the imported key as default
                        let address_copy = addr.clone();
                        if manager.set_default_key(&address_copy).is_ok() {
                            self.wallet_manager = Some(manager);
                            self.current_screen = Screen::Main;
                            self.error_message = None;
                            self.success_message = Some("Key imported successfully!".to_string());
                            self.import_key.clear();
                        }
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to import key: {}", e));
                    }
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to create wallet: {}", e));
            }
        }
    }

    fn show_main_screen(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.heading("⏰ TIME Coin Wallet");
                ui.separator();
                
                if ui.button("📊 Overview").clicked() {
                    self.current_screen = Screen::Main;
                }
                if ui.button("📤 Send").clicked() {
                    self.current_screen = Screen::Send;
                }
                if ui.button("📥 Receive").clicked() {
                    self.current_screen = Screen::Receive;
                }
                if ui.button("📜 Transactions").clicked() {
                    self.current_screen = Screen::Transactions;
                }
                if ui.button("⚙ Settings").clicked() {
                    self.current_screen = Screen::Settings;
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_screen {
                Screen::Main => self.show_overview(ui, ctx),
                Screen::Send => self.show_send_screen(ui),
                Screen::Receive => self.show_receive_screen(ui, ctx),
                Screen::Transactions => self.show_transactions_screen(ui),
                Screen::Settings => self.show_settings_screen(ui, ctx),
                Screen::Welcome => {}, // Should not happen
            }
        });
    }

    fn show_overview(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if let Some(manager) = &self.wallet_manager {
            ui.heading("Wallet Overview");
            ui.add_space(20.0);

            // Balance display
            ui.group(|ui| {
                ui.set_min_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.label("Balance");
                    ui.add_space(10.0);
                    let balance = manager.get_balance();
                    ui.heading(format!("{} TIME", Self::format_amount(balance)));
                });
            });

            ui.add_space(20.0);

            // Address display
            ui.group(|ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label("Primary Address:");
                    if let Some(address) = manager.get_primary_address() {
                        ui.monospace(&address);
                        if ui.button("📋 Copy").clicked() {
                            ctx.copy_text(address);
                            self.success_message = Some("Address copied!".to_string());
                        }
                    }
                });
            });

            ui.add_space(20.0);

            // Messages
            if let Some(err) = &self.error_message {
                ui.colored_label(egui::Color32::RED, err);
            }
            if let Some(msg) = &self.success_message {
                ui.colored_label(egui::Color32::GREEN, msg);
            }

            ui.add_space(20.0);

            // UTXOs display
            let utxos = manager.get_utxos();
            ui.heading(format!("Unspent Outputs ({})", utxos.len()));
            ui.add_space(10.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                for utxo in utxos {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Amount:");
                            ui.monospace(format!("{} TIME", Self::format_amount(utxo.amount)));
                            ui.separator();
                            ui.label("TxID:");
                            ui.monospace(format!("{}...", hex::encode(&utxo.tx_hash[..8])));
                        });
                    });
                }
            });
        }
    }

    fn show_send_screen(&mut self, ui: &mut egui::Ui) {
        ui.heading("Send TIME Coins");
        ui.add_space(20.0);

        ui.group(|ui| {
            ui.set_min_width(ui.available_width());
            
            ui.horizontal(|ui| {
                ui.label("To Address:");
                ui.add_space(10.0);
                ui.text_edit_singleline(&mut self.send_address);
            });
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Amount (TIME):");
                ui.add_space(10.0);
                ui.text_edit_singleline(&mut self.send_amount);
            });
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Fee (TIME):");
                ui.add_space(10.0);
                ui.text_edit_singleline(&mut self.send_fee);
            });
            ui.add_space(20.0);

            if let Some(manager) = &self.wallet_manager {
                ui.horizontal(|ui| {
                    ui.label("Available:");
                    ui.monospace(format!("{} TIME", Self::format_amount(manager.get_balance())));
                });
                ui.add_space(10.0);
            }

            if ui.button("Send Transaction").clicked() {
                self.send_transaction();
            }
        });

        ui.add_space(20.0);

        // Messages
        if let Some(err) = &self.error_message {
            ui.colored_label(egui::Color32::RED, err);
        }
        if let Some(msg) = &self.success_message {
            ui.colored_label(egui::Color32::GREEN, msg);
        }
    }

    fn send_transaction(&mut self) {
        // Parse amounts
        let amount = match self.send_amount.parse::<u64>() {
            Ok(a) => a,
            Err(_) => {
                self.error_message = Some("Invalid amount".to_string());
                return;
            }
        };

        let fee = match self.send_fee.parse::<u64>() {
            Ok(f) => f,
            Err(_) => {
                self.error_message = Some("Invalid fee".to_string());
                return;
            }
        };

        if let Some(manager) = &mut self.wallet_manager {
            match manager.create_transaction(&self.send_address, amount, fee) {
                Ok(_tx) => {
                    self.success_message = Some("Transaction created! (Broadcasting not yet implemented)".to_string());
                    self.error_message = None;
                    // Clear fields
                    self.send_address.clear();
                    self.send_amount.clear();
                }
                Err(e) => {
                    self.error_message = Some(format!("Transaction failed: {}", e));
                    self.success_message = None;
                }
            }
        }
    }

    fn show_receive_screen(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Receive TIME Coins");
        ui.add_space(20.0);

        if let Some(manager) = &self.wallet_manager {
            ui.group(|ui| {
                ui.set_min_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.label("Your Receiving Address:");
                    ui.add_space(10.0);
                    
                    if let Some(address) = manager.get_primary_address() {
                        ui.monospace(&address);
                        ui.add_space(10.0);
                        
                        if ui.button("📋 Copy Address").clicked() {
                            ctx.copy_text(address.clone());
                            self.success_message = Some("Address copied to clipboard!".to_string());
                        }
                    }
                });
            });

            ui.add_space(20.0);

            // Show all addresses
            ui.heading("All Addresses");
            ui.add_space(10.0);

            for key in manager.get_keys() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(&key.label);
                        if key.is_default {
                            ui.label("(Default)");
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.monospace(&key.address);
                        if ui.button("📋").clicked() {
                            ctx.copy_text(key.address.clone());
                        }
                    });
                });
            }
        }

        if let Some(msg) = &self.success_message {
            ui.add_space(10.0);
            ui.colored_label(egui::Color32::GREEN, msg);
        }
    }

    fn show_transactions_screen(&mut self, ui: &mut egui::Ui) {
        ui.heading("Transaction History");
        ui.add_space(20.0);

        ui.label("Transaction history will be displayed here once network integration is complete.");
        ui.add_space(10.0);
        ui.label("Features coming soon:");
        ui.label("  • View sent and received transactions");
        ui.label("  • Transaction confirmations");
        ui.label("  • Filter by date/amount");
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

            // Security section
            ui.group(|ui| {
                ui.label("Security");
                ui.add_space(5.0);
                
                ui.checkbox(&mut self.show_private_key, "Show Private Key");
                
                if self.show_private_key {
                    ui.add_space(10.0);
                    ui.colored_label(egui::Color32::RED, "⚠️ WARNING: Never share your private key!");
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

            ui.add_space(20.0);

            // Future encryption info
            ui.group(|ui| {
                ui.label("Wallet Encryption");
                ui.add_space(5.0);
                ui.label("Status: Not encrypted (testnet)");
                ui.label("Encryption will be available in a future update.");
            });
        }
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
}

impl eframe::App for WalletApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Clear messages after a delay (simple approach)
        if self.success_message.is_some() || self.error_message.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_secs(5));
        }

        match self.current_screen {
            Screen::Welcome => self.show_welcome_screen(ctx),
            _ => self.show_main_screen(ctx),
        }
    }
}
