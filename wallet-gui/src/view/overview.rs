//! Overview screen — balance display and recent transactions.

use egui::Ui;
use tokio::sync::mpsc;

use crate::events::{Screen, UiEvent};
use crate::masternode_client::TransactionStatus;
use crate::state::AppState;
use crate::theme;

/// Render the overview screen.
pub fn show(ui: &mut Ui, state: &mut AppState, ui_tx: &mpsc::UnboundedSender<UiEvent>) {
    ui.horizontal(|ui| {
        ui.heading("Overview");
        ui.add_space(10.0);

        if ui
            .add_enabled(
                !state.loading,
                egui::Button::new(if state.loading {
                    "Refreshing..."
                } else {
                    "Refresh"
                }),
            )
            .clicked()
        {
            state.loading = true;
            let _ = ui_tx.send(UiEvent::RefreshBalance);
            let _ = ui_tx.send(UiEvent::RefreshTransactions);
            let _ = ui_tx.send(UiEvent::RefreshUtxos);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let ws_label = if state.ws_connected {
                egui::RichText::new("Connected").color(egui::Color32::GREEN)
            } else {
                egui::RichText::new("Disconnected").color(egui::Color32::RED)
            };
            ui.label(ws_label);
            let healthy = state.peers.iter().filter(|p| p.is_healthy).count();
            if healthy > 0 {
                ui.label(
                    egui::RichText::new(format!("{healthy} peers •"))
                        .small()
                        .weak(),
                );
            }
        });
    });

    ui.separator();
    ui.add_space(10.0);

    // Consolidation suggestion banner (shown when many spendable UTXOs exist)
    if state.suggest_consolidation {
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(120, 90, 0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("⚠ You have many small UTXOs.")
                            .color(egui::Color32::from_rgb(255, 220, 80))
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(
                            "Consolidating them will speed up future transactions.",
                        )
                        .color(egui::Color32::from_rgb(240, 220, 180)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Dismiss").clicked() {
                            state.suggest_consolidation = false;
                            state.consolidation_dismissed = true;
                        }
                        if ui.button("Consolidate now").clicked() {
                            state.suggest_consolidation = false;
                            let _ = ui_tx.send(UiEvent::ConsolidateUtxos);
                            state.consolidation_in_progress = true;
                            state.consolidation_status =
                                "Consolidation started in background…".to_string();
                        }
                    });
                });
            });
        ui.add_space(6.0);
    }

    // Syncing indicator
    if state.syncing {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(
                egui::RichText::new("Synchronizing with network…")
                    .color(egui::Color32::from_rgb(100, 180, 255)),
            );
        });
        ui.add_space(5.0);
    }

    // Balance card
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(173, 216, 230))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            let utxo_total = state.utxo_total();
            let mn_bal = state.masternode_balance;
            let total = if mn_bal > 0 {
                mn_bal
            } else if utxo_total > 0 {
                utxo_total
            } else {
                state.computed_balance()
            };
            let locked = state.locked_balance();
            let available = total.saturating_sub(locked);
            let has_pending = state.transactions.iter().any(|t| {
                matches!(
                    t.status,
                    crate::masternode_client::TransactionStatus::Pending
                )
            });

            // Primary: Available (big)
            ui.label(
                egui::RichText::new("Available Balance")
                    .size(13.0)
                    .color(theme::TEXT_SECONDARY),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(state.format_time(available))
                        .size(36.0)
                        .strong()
                        .color(egui::Color32::BLACK),
                );
                ui.add_space(8.0);
                if has_pending || state.syncing {
                    egui::Frame::new()
                        .fill(theme::ORANGE)
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(8, 2))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("Pending")
                                    .size(12.0)
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            );
                        });
                } else if mn_bal > 0 {
                    egui::Frame::new()
                        .fill(theme::GREEN)
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(8, 2))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("Verified")
                                    .size(12.0)
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            );
                        });
                }
            });

            // Secondary rows: Locked + Total
            if locked > 0 {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Locked:")
                            .size(13.0)
                            .color(theme::TEXT_SECONDARY),
                    );
                    ui.label(
                        egui::RichText::new(state.format_time(locked))
                            .size(13.0)
                            .color(theme::ORANGE),
                    );
                    ui.add_space(20.0);
                    ui.label(
                        egui::RichText::new("Total:")
                            .size(13.0)
                            .color(theme::TEXT_SECONDARY),
                    );
                    ui.label(egui::RichText::new(state.format_time(total)).size(13.0));
                });
            }
        });

    ui.add_space(15.0);

    // Income chart
    crate::view::income_chart::show(ui, state);

    ui.add_space(10.0);

    // Security warning for unencrypted wallets
    if state.wallet_loaded && !state.wallet_encrypted {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("⚠ Your wallet is not password-protected.")
                        .color(egui::Color32::from_rgb(255, 165, 0))
                        .strong(),
                );
                if ui.button("Set Password").clicked() {
                    state.show_encrypt_dialog = true;
                }
            });
        });

        if state.show_encrypt_dialog {
            ui.group(|ui| {
                ui.set_min_width(ui.available_width());
                ui.label("Enter a password to encrypt your wallet:");
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new("⚠ There is no password recovery. If you forget your password, you will need your recovery phrase to restore your wallet.")
                        .color(egui::Color32::from_rgb(255, 165, 0))
                        .size(14.0),
                );
                ui.add_space(8.0);

                let show = state.show_encrypt_password;

                ui.horizontal(|ui| {
                    ui.label("Password:");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.encrypt_password_input)
                            .password(!show)
                            .hint_text("Password")
                            .desired_width(250.0),
                    );
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Confirm:   ");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.encrypt_password_confirm)
                            .password(!show)
                            .hint_text("Confirm password")
                            .desired_width(250.0),
                    );
                });
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.checkbox(&mut state.show_encrypt_password, "Show password");
                });

                // Validation
                let pw = &state.encrypt_password_input;
                let confirm = &state.encrypt_password_confirm;
                let passwords_match = !pw.is_empty() && pw == confirm;

                if !pw.is_empty() && !confirm.is_empty() && !passwords_match {
                    ui.label(
                        egui::RichText::new("Passwords do not match")
                            .color(egui::Color32::RED)
                            .size(14.0),
                    );
                }

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            passwords_match,
                            egui::Button::new("Encrypt Wallet"),
                        )
                        .clicked()
                    {
                        let _ = ui_tx.send(UiEvent::EncryptWallet {
                            password: state.encrypt_password_input.clone(),
                        });
                    }
                    if ui.button("Cancel").clicked() {
                        state.show_encrypt_dialog = false;
                        state.encrypt_password_input.clear();
                        state.encrypt_password_confirm.clear();
                        state.show_encrypt_password = false;
                    }
                });
            });
        }

        ui.add_space(5.0);
    }

    // Real-time notifications
    if !state.recent_notifications.is_empty() {
        ui.heading("Notifications");
        ui.add_space(5.0);
        for notif in state.recent_notifications.iter().rev().take(5) {
            ui.horizontal(|ui| {
                let amount_sats = crate::masternode_client::json_to_satoshis(&notif.amount);
                ui.colored_label(
                    egui::Color32::GREEN,
                    format!("Received {}", state.format_time(amount_sats)),
                );
                let short_addr = if notif.address.len() > 20 {
                    format!("{}..", &notif.address[..20])
                } else {
                    notif.address.clone()
                };
                ui.label(
                    egui::RichText::new(format!("to {}", short_addr))
                        .color(egui::Color32::GRAY)
                        .monospace(),
                );
            });
        }
        ui.add_space(10.0);
    }

    // Recent transactions (last 10, no scroll — compact overview)
    ui.heading("Recent Transactions");
    ui.add_space(5.0);

    if state.transactions.is_empty() {
        ui.label(
            egui::RichText::new("No transactions yet")
                .color(egui::Color32::GRAY)
                .italics(),
        );
    } else {
        let mut clicked_idx: Option<usize> = None;
        egui::Grid::new("recent_tx_grid")
            .num_columns(5)
            .spacing([12.0, 8.0])
            .min_col_width(0.0)
            .striped(true)
            .show(ui, |ui| {
                // Header
                ui.label(egui::RichText::new("Type").size(14.0).strong());
                ui.label(egui::RichText::new("Amount").size(14.0).strong());
                ui.label(egui::RichText::new("Address").size(14.0).strong());
                ui.label(egui::RichText::new("Date").size(14.0).strong());
                ui.label(egui::RichText::new("Status").size(14.0).strong());
                ui.end_row();

                for (i, tx) in state.transactions.iter().enumerate().take(10) {
                    // Type
                    let (dir_icon, amount_color) = if tx.is_fee {
                        ("💸 Fee", egui::Color32::from_rgb(255, 165, 0))
                    } else if tx.is_send {
                        ("📤 Sent", egui::Color32::from_rgb(255, 80, 80))
                    } else {
                        ("📥 Received", egui::Color32::from_rgb(80, 200, 80))
                    };
                    if ui
                        .add(
                            egui::Label::new(
                                egui::RichText::new(dir_icon).size(14.0).color(amount_color),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .clicked()
                    {
                        clicked_idx = Some(i);
                    }

                    // Amount
                    let is_neg = tx.is_send || tx.is_fee;
                    if ui
                        .add(
                            egui::Label::new(
                                egui::RichText::new(state.format_time_signed(tx.amount, is_neg))
                                    .size(14.0)
                                    .strong()
                                    .color(amount_color),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .clicked()
                    {
                        clicked_idx = Some(i);
                    }

                    // Address
                    let addr_display = if tx.is_fee {
                        tx.address.clone()
                    } else if let Some(name) = state.contact_name(&tx.address) {
                        let short = if tx.address.len() > 16 {
                            format!(
                                "{}...{}",
                                &tx.address[..10],
                                &tx.address[tx.address.len() - 6..]
                            )
                        } else {
                            tx.address.clone()
                        };
                        format!("{} ({})", name, short)
                    } else if tx.address.len() > 16 {
                        format!(
                            "{}...{}",
                            &tx.address[..10],
                            &tx.address[tx.address.len() - 6..]
                        )
                    } else {
                        tx.address.clone()
                    };
                    if ui
                        .add(
                            egui::Label::new(
                                egui::RichText::new(addr_display)
                                    .size(14.0)
                                    .color(ui.visuals().text_color()),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .clicked()
                    {
                        clicked_idx = Some(i);
                    }

                    // Date
                    let date_str = if tx.timestamp > 0 {
                        chrono::DateTime::from_timestamp(tx.timestamp, 0)
                            .map(|dt| {
                                let local: chrono::DateTime<chrono::Local> = dt.into();
                                local.format("%Y-%m-%d %H:%M").to_string()
                            })
                            .unwrap_or_default()
                    } else {
                        String::new()
                    };
                    if ui
                        .add(
                            egui::Label::new(
                                egui::RichText::new(date_str)
                                    .size(14.0)
                                    .color(ui.visuals().text_color()),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .clicked()
                    {
                        clicked_idx = Some(i);
                    }

                    // Status
                    let (status_text, status_color) = match tx.status {
                        TransactionStatus::Approved => ("✅ Approved", egui::Color32::GREEN),
                        TransactionStatus::Pending => {
                            ("⏳ Pending", egui::Color32::from_rgb(255, 165, 0))
                        }
                        TransactionStatus::Declined => ("❌ Declined", egui::Color32::RED),
                    };
                    if ui
                        .add(
                            egui::Label::new(
                                egui::RichText::new(status_text)
                                    .size(14.0)
                                    .color(status_color),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .clicked()
                    {
                        clicked_idx = Some(i);
                    }

                    ui.end_row();
                }
            });

        if state.transactions.len() > 10 {
            ui.add_space(4.0);
            if ui
                .link(format!(
                    "View all {} transactions",
                    state.transactions.len()
                ))
                .clicked()
            {
                state.screen = Screen::Transactions;
                let _ = ui_tx.send(UiEvent::NavigatedTo(Screen::Transactions));
            }
        }

        // Navigate to transaction detail
        if let Some(idx) = clicked_idx {
            state.selected_transaction = Some(idx);
            state.screen = Screen::Transactions;
            let _ = ui_tx.send(UiEvent::NavigatedTo(Screen::Transactions));
        }
    }

    // Status messages
    if let Some(ref err) = state.error {
        ui.add_space(10.0);
        ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
    }
    if let Some(ref msg) = state.success {
        ui.add_space(10.0);
        ui.colored_label(egui::Color32::GREEN, msg.as_str());
    }
}
