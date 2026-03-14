//! Connections screen — shows all discovered peers with health and ping info.

use egui::Ui;
use tokio::sync::mpsc;

use crate::events::UiEvent;
use crate::state::AppState;

/// Extract the IP from an endpoint like "http://1.2.3.4:24001".
fn peer_ip(endpoint: &str) -> &str {
    let s = endpoint
        .strip_prefix("http://")
        .or_else(|| endpoint.strip_prefix("https://"))
        .unwrap_or(endpoint);
    // Strip port
    s.rsplit_once(':').map(|(host, _)| host).unwrap_or(s)
}

/// Paint a filled circle as a health indicator.
fn health_dot(ui: &mut Ui, color: egui::Color32) {
    let size = egui::vec2(12.0, 12.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 5.0, color);
}

/// Render the connections screen.
pub fn show(ui: &mut Ui, state: &AppState, ui_tx: &mpsc::UnboundedSender<UiEvent>) {
    ui.heading("Connections");
    ui.separator();
    ui.add_space(10.0);

    if state.peers.is_empty() {
        ui.label(
            egui::RichText::new("Discovering peers...")
                .color(egui::Color32::GRAY)
                .italics(),
        );
        ui.spinner();
        return;
    }

    ui.label(format!("{} healthy peers", state.peers.len()));
    ui.add_space(10.0);

    egui::Grid::new("peers_table")
        .num_columns(9)
        .spacing([12.0, 6.0])
        .striped(true)
        .show(ui, |ui| {
            // Header
            ui.label(egui::RichText::new("#").strong());
            ui.label(egui::RichText::new("").strong());
            ui.label(egui::RichText::new("IP Address").strong());
            ui.label(egui::RichText::new("Status").strong());
            ui.label(egui::RichText::new("WS").strong());
            ui.label(egui::RichText::new("Ping").strong());
            ui.label(egui::RichText::new("Block").strong());
            ui.label(egui::RichText::new("").strong());
            ui.with_layout(
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| {
                    ui.label(egui::RichText::new("Consensus").strong());
                },
            );
            ui.end_row();

            // Determine best (highest) block height for consensus check
            let best_height = state
                .peers
                .iter()
                .filter_map(|p| p.block_height)
                .max()
                .unwrap_or(0);

            for (i, peer) in state.peers.iter().enumerate() {
                // Row number
                ui.label(egui::RichText::new(format!("{}", i + 1)).weak().monospace());

                // Health dot
                let dot_color = if !peer.is_healthy {
                    egui::Color32::RED
                } else if let Some(ms) = peer.ping_ms {
                    if ms < 100 {
                        egui::Color32::GREEN
                    } else if ms < 500 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::RED
                    }
                } else {
                    egui::Color32::GRAY
                };
                health_dot(ui, dot_color);

                // IP
                let ip = peer_ip(&peer.endpoint);
                ui.label(egui::RichText::new(ip).monospace());

                // Status
                if peer.is_active {
                    ui.colored_label(egui::Color32::GREEN, "Active");
                } else {
                    ui.colored_label(egui::Color32::GREEN, "Healthy");
                }

                // WS
                if peer.ws_available {
                    ui.colored_label(egui::Color32::GREEN, "Yes");
                } else if peer.is_healthy {
                    ui.colored_label(egui::Color32::GRAY, "No");
                } else {
                    ui.colored_label(egui::Color32::GRAY, "--");
                }

                // Ping
                if let Some(ms) = peer.ping_ms {
                    ui.label(format!("{}ms", ms));
                } else {
                    ui.colored_label(egui::Color32::GRAY, "--");
                }

                // Block height
                if let Some(height) = peer.block_height {
                    ui.label(format!("#{}", height));
                } else {
                    ui.colored_label(egui::Color32::GRAY, "--");
                }

                // Select link
                if peer.is_active {
                    ui.colored_label(egui::Color32::GRAY, "selected");
                } else if peer.is_healthy {
                    let link = ui.link("select");
                    if link.clicked() {
                        let _ = ui_tx.send(UiEvent::SwitchPeer {
                            endpoint: peer.endpoint.clone(),
                        });
                    }
                } else {
                    ui.label("");
                }

                // Consensus
                ui.with_layout(
                    egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        if best_height == 0 {
                            ui.colored_label(egui::Color32::GRAY, "--");
                        } else {
                            let height = peer.block_height.unwrap_or(0);
                            let lag = best_height.saturating_sub(height);
                            if lag <= 3 {
                                ui.colored_label(egui::Color32::GREEN, "✔")
                                    .on_hover_text(format!(
                                        "Within {} block(s) of best height {}",
                                        lag, best_height
                                    ));
                            } else {
                                ui.colored_label(egui::Color32::RED, "X")
                                    .on_hover_text(format!(
                                        "{} blocks behind consensus height {}",
                                        lag, best_height
                                    ));
                            }
                        }
                    },
                );

                ui.end_row();
            }
        });
}
