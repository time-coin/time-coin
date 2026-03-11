//! Application struct — the eframe::App implementation.
//!
//! Thin wrapper: drains service events, dispatches to view modules.
//! No async, no network, no wallet logic.

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::config_new::Config;
use crate::events::{Screen, ServiceEvent, UiEvent};
use crate::state::AppState;
use crate::view;
use crate::wallet_manager::WalletManager;

/// The wallet application.
pub struct App {
    pub state: AppState,
    pub ui_tx: mpsc::UnboundedSender<UiEvent>,
    svc_rx: mpsc::UnboundedReceiver<ServiceEvent>,
    shutdown_token: CancellationToken,
}

impl App {
    /// Create a new App, spawning the background service task.
    pub fn new(cc: &eframe::CreationContext<'_>, config: Config) -> Self {
        setup_fonts(&cc.egui_ctx);

        let (ui_tx, ui_rx) = mpsc::unbounded_channel();
        let (svc_tx, svc_rx) = mpsc::unbounded_channel();
        let token = CancellationToken::new();

        // Check wallet existence synchronously before first render
        let network_type = if config.is_testnet() {
            wallet::NetworkType::Testnet
        } else {
            wallet::NetworkType::Mainnet
        };
        let wallet_exists = WalletManager::exists(network_type);
        let is_first_run = config.is_first_run;

        // Spawn the single background service task
        let svc_token = token.clone();
        tokio::spawn(crate::service::run(svc_token, ui_rx, svc_tx, config));

        let initial_screen = if is_first_run {
            crate::events::Screen::NetworkSelect
        } else {
            crate::events::Screen::Welcome
        };

        let state = AppState {
            wallet_exists,
            screen: initial_screen,
            is_testnet: network_type == wallet::NetworkType::Testnet,
            ..Default::default()
        };

        Self {
            state,
            ui_tx,
            svc_rx,
            shutdown_token: token,
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        // Flush any pending send record updates before shutdown
        if !self.state.dirty_send_records.is_empty() {
            let records = std::mem::take(&mut self.state.dirty_send_records);
            let _ = self.ui_tx.send(UiEvent::PersistSendRecords(records));
        }
        self.shutdown_token.cancel();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Ensure we repaint regularly to pick up background service events
        ctx.request_repaint_after(std::time::Duration::from_secs(1));

        // 1. Drain all pending service events (non-blocking)
        while let Ok(event) = self.svc_rx.try_recv() {
            self.state.apply(event);
            ctx.request_repaint();
        }

        // Auto-resync when balance drift is detected
        if self.state.needs_resync {
            self.state.needs_resync = false;
            self.state.resync_in_progress = true;
            let _ = self.ui_tx.send(UiEvent::ResyncWallet);
        }

        // Persist any send records that changed status (e.g., Declined)
        if !self.state.dirty_send_records.is_empty() {
            let records = std::mem::take(&mut self.state.dirty_send_records);
            let _ = self.ui_tx.send(UiEvent::PersistSendRecords(records));
        }

        // 2. Navigation sidebar
        egui::SidePanel::left("nav").show(ctx, |ui| {
            ui.add_space(10.0);

            // Logo
            let logo_bytes = include_bytes!("../assets/logo.png");
            let image =
                egui::Image::from_bytes("bytes://logo.png", logo_bytes.as_slice()).max_width(48.0);
            ui.add(image);

            ui.add_space(5.0);
            ui.label(egui::RichText::new("TIME Coin").strong());
            ui.separator();
            ui.add_space(5.0);

            if self.state.wallet_loaded {
                nav_button(
                    ui,
                    &mut self.state,
                    "🏠 Overview",
                    Screen::Overview,
                    &self.ui_tx,
                );
                nav_button(ui, &mut self.state, "📤 Send", Screen::Send, &self.ui_tx);
                nav_button(
                    ui,
                    &mut self.state,
                    "📥 Receive",
                    Screen::Receive,
                    &self.ui_tx,
                );
                nav_button(
                    ui,
                    &mut self.state,
                    "📋 Transactions",
                    Screen::Transactions,
                    &self.ui_tx,
                );
                nav_button(
                    ui,
                    &mut self.state,
                    "🖥 Masternodes",
                    Screen::Masternodes,
                    &self.ui_tx,
                );
                ui.separator();
                let healthy_count = self.state.peers.iter().filter(|p| p.is_healthy).count();
                let conn_label = if healthy_count > 0 {
                    format!("🔗 Connections ({healthy_count})")
                } else {
                    "🔗 Connections".to_string()
                };
                nav_button(
                    ui,
                    &mut self.state,
                    &conn_label,
                    Screen::Connections,
                    &self.ui_tx,
                );
                nav_button(
                    ui,
                    &mut self.state,
                    "⚙ Settings",
                    Screen::Settings,
                    &self.ui_tx,
                );
                nav_button(ui, &mut self.state, "🔧 Tools", Screen::Tools, &self.ui_tx);

                ui.add_space(10.0);
                ui.separator();
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new("🚪 Exit").size(14.0))
                            .min_size(egui::vec2(140.0, 28.0)),
                    )
                    .clicked()
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }

            // Network badge + version — always visible (even during setup)
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_space(4.0);
                let (network_label, bg_color, text_color) = if self.state.is_testnet {
                    (
                        "Testnet",
                        egui::Color32::from_rgb(255, 250, 200),
                        egui::Color32::from_rgb(120, 100, 0),
                    )
                } else {
                    (
                        "Mainnet",
                        egui::Color32::from_rgb(200, 225, 255),
                        egui::Color32::from_rgb(0, 60, 120),
                    )
                };
                ui.label(
                    egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .small()
                        .weak(),
                );

                // During setup, show a switch-network button below the badge
                if !self.state.wallet_loaded {
                    let switch_label = if self.state.is_testnet {
                        "Switch to Mainnet"
                    } else {
                        "Switch to Testnet"
                    };
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new(switch_label).small())
                                .min_size(egui::vec2(140.0, 24.0)),
                        )
                        .clicked()
                    {
                        let new_network = if self.state.is_testnet {
                            "mainnet"
                        } else {
                            "testnet"
                        };
                        let _ = self.ui_tx.send(UiEvent::SelectNetwork {
                            network: new_network.to_string(),
                        });
                    }
                    ui.add_space(4.0);
                }

                egui::Frame::new()
                    .fill(bg_color)
                    .corner_radius(4.0)
                    .inner_margin(egui::Margin::symmetric(8, 3))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(network_label)
                                .small()
                                .strong()
                                .color(text_color),
                        );
                    });
            });
        });

        // 3. Central panel — route to the active view
        egui::CentralPanel::default().show(ctx, |ui| match self.state.screen {
            Screen::Welcome
            | Screen::NetworkSelect
            | Screen::MnemonicSetup
            | Screen::MnemonicConfirm => {
                view::welcome::show(ui, &mut self.state, &self.ui_tx);
            }
            Screen::Overview => {
                view::overview::show(ui, &mut self.state, &self.ui_tx);
            }
            Screen::Send => {
                view::send::show(ui, &mut self.state, &self.ui_tx);
            }
            Screen::Receive => {
                view::receive::show(ui, &mut self.state, &self.ui_tx);
            }
            Screen::Transactions => {
                view::transactions::show(ui, &mut self.state, &self.ui_tx);
            }
            Screen::Masternodes => {
                view::masternodes::render(ui, &mut self.state, &self.ui_tx);
            }
            Screen::Settings => {
                view::settings::show(ui, &mut self.state, &self.ui_tx);
            }
            Screen::Connections => {
                view::connections::show(ui, &self.state);
            }
            Screen::Tools => {
                view::tools::show(ui, &mut self.state, &self.ui_tx);
            }
            Screen::Utxos => {
                view::overview::show(ui, &mut self.state, &self.ui_tx);
            }
        });
    }
}

/// Render a navigation button, highlighting the active screen.
fn nav_button(
    ui: &mut egui::Ui,
    state: &mut AppState,
    label: &str,
    screen: Screen,
    ui_tx: &mpsc::UnboundedSender<UiEvent>,
) {
    let is_active = state.screen == screen;
    let button = egui::Button::new(egui::RichText::new(label).size(14.0))
        .selected(is_active)
        .min_size(egui::vec2(140.0, 28.0));

    if ui.add(button).clicked() && !is_active {
        state.screen = screen;
        let _ = ui_tx.send(UiEvent::NavigatedTo(screen));
    }
}

/// Setup fonts and image loaders.
fn setup_fonts(ctx: &egui::Context) {
    egui_extras::install_image_loaders(ctx);
    ctx.set_fonts(egui::FontDefinitions::default());
}
