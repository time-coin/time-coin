use egui::{Color32, RichText, Ui};

use crate::events::UiEvent;
use crate::state::AppState;
use crate::wallet_db::{masternode_tier_from_satoshis, MasternodeEntry};

/// Colored tier label text (no emoji — relies on color for distinction).
fn tier_label(tier: &str) -> (&'static str, Color32) {
    match tier {
        "Gold" => ("Gold", Color32::from_rgb(255, 200, 50)),
        "Silver" => ("Silver", Color32::from_rgb(192, 192, 192)),
        "Bronze" => ("Bronze", Color32::from_rgb(180, 100, 50)),
        _ => ("Free", Color32::GRAY),
    }
}

/// Render the Masternodes management screen.
pub fn render(
    ui: &mut Ui,
    state: &mut AppState,
    ui_tx: &tokio::sync::mpsc::UnboundedSender<UiEvent>,
) {
    ui.heading("Masternodes");
    ui.add_space(8.0);

    // ---------- Tier requirements info box ----------
    egui::CollapsingHeader::new("Tier Requirements & Setup Guide")
        .default_open(false)
        .show(ui, |ui| {
            ui.add_space(4.0);
            egui::Grid::new("tier_req_grid")
                .num_columns(2)
                .spacing([16.0, 4.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Tier").strong());
                    ui.label(RichText::new("Collateral Required").strong());
                    ui.end_row();
                    ui.label(RichText::new("Gold").color(Color32::from_rgb(255, 200, 50)).strong());
                    ui.label("100,000 TIME");
                    ui.end_row();
                    ui.label(RichText::new("Silver").color(Color32::from_rgb(192, 192, 192)).strong());
                    ui.label("10,000 TIME");
                    ui.end_row();
                    ui.label(RichText::new("Bronze").color(Color32::from_rgb(180, 100, 50)).strong());
                    ui.label("1,000 TIME");
                    ui.end_row();
                });
            ui.add_space(6.0);
            ui.label(RichText::new("How to activate a tiered masternode:").strong());
            ui.label("1. Send the required collateral to a wallet address and note the TXID/vout.");
            ui.label("2. Add the entry below with the collateral TXID and vout.");
            ui.label("3. Click Copy Conf to get the line for your masternode's masternode.conf.");
            ui.add_space(4.0);
        });

    ui.add_space(8.0);

    // ---------- Add button ----------
    ui.horizontal(|ui| {
        if ui.button("➕ Add Masternode").clicked() {
            state.mn_show_add_form = !state.mn_show_add_form;
        }
    });

    // ---------- Add form ----------
    if state.mn_show_add_form {
        ui.add_space(8.0);
        egui::Frame::group(ui.style())
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(RichText::new("New Masternode Entry").strong());
                ui.add_space(4.0);

                egui::Grid::new("mn_add_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("IP Address:")
                            .on_hover_text("Masternode server IP and optional port, e.g. 1.2.3.4:24100");
                        ui.text_edit_singleline(&mut state.mn_add_ip)
                            .on_hover_text("e.g. 1.2.3.4 or 1.2.3.4:24100");
                        ui.end_row();

                        ui.label("Collateral TXID:");
                        let resp = ui.text_edit_singleline(&mut state.mn_add_txid);
                        resp.context_menu(|ui| {
                            if ui.button("📋 Paste").clicked() {
                                if let Ok(mut cb) = arboard::Clipboard::new() {
                                    if let Ok(t) = cb.get_text() {
                                        state.mn_add_txid = t.trim().to_string();
                                    }
                                }
                                ui.close_menu();
                            }
                        });
                        ui.end_row();

                        ui.label("Collateral Vout:");
                        ui.text_edit_singleline(&mut state.mn_add_vout);
                        ui.end_row();
                    });

                // Preview tier from collateral UTXO if available
                if !state.mn_add_txid.trim().is_empty() {
                    let txid = state.mn_add_txid.trim().to_string();
                    let vout: u32 = state.mn_add_vout.trim().parse().unwrap_or(0);
                    if let Some(utxo) = state
                        .utxos
                        .iter()
                        .find(|u| u.txid == txid && u.vout == vout)
                    {
                        let amount_time = utxo.amount as f64 / 100_000_000.0;
                        ui.add_space(4.0);
                        match masternode_tier_from_satoshis(utxo.amount) {
                            Some(tier) => {
                                let (label, color) = tier_label(tier);
                                ui.horizontal(|ui| {
                                    ui.label("Detected tier:");
                                    ui.label(RichText::new(label).color(color).strong());
                                    ui.label(format!("({:.0} TIME)", amount_time));
                                });
                            }
                            None => {
                                ui.colored_label(
                                    Color32::YELLOW,
                                    format!(
                                        "⚠ Collateral {:.0} TIME is below Bronze minimum (1,000 TIME)",
                                        amount_time
                                    ),
                                );
                            }
                        }
                    }
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let ip_trimmed = state.mn_add_ip.trim().to_string();
                    let can_save = !ip_trimmed.is_empty()
                        && !state.mn_add_txid.trim().is_empty()
                        && state.mn_add_vout.trim().parse::<u32>().is_ok();

                    if ui
                        .add_enabled(can_save, egui::Button::new("💾 Save"))
                        .clicked()
                    {
                        let (ip, port) = if let Some(colon) = ip_trimmed.rfind(':') {
                            let port = ip_trimmed[colon + 1..].parse().unwrap_or(24100u16);
                            (ip_trimmed[..colon].to_string(), port)
                        } else {
                            (ip_trimmed.clone(), 24100u16)
                        };
                        let alias = format!("{}:{}", ip, port);
                        let txid = state.mn_add_txid.trim().to_string();
                        let vout: u32 = state.mn_add_vout.trim().parse().unwrap_or(0);
                        let collateral_amount = state
                            .utxos
                            .iter()
                            .find(|u| u.txid == txid && u.vout == vout)
                            .map(|u| u.amount);
                        let entry = MasternodeEntry {
                            alias,
                            ip,
                            port,
                            masternode_key: String::new(),
                            collateral_txid: txid,
                            collateral_vout: vout,
                            payout_address: None,
                            collateral_amount,
                        };
                        let _ = ui_tx.send(UiEvent::SaveMasternodeEntry(entry));
                        state.mn_add_ip.clear();
                        state.mn_add_txid.clear();
                        state.mn_add_vout = "0".to_string();
                        state.mn_show_add_form = false;
                    }
                    if ui.button("Cancel").clicked() {
                        state.mn_show_add_form = false;
                    }
                });
            });
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);

    // ---------- Masternode list ----------
    if state.masternode_entries.is_empty() {
        ui.label("No masternodes configured. Add one manually or import a masternode.conf file.");
    } else {
        let mut to_delete: Option<String> = None;
        let mut update_event: Option<UiEvent> = None;

        for entry in &state.masternode_entries {
            // Resolve collateral amount: live UTXO first, cached amount as fallback.
            let live_amount = state
                .utxos
                .iter()
                .find(|u| u.txid == entry.collateral_txid && u.vout == entry.collateral_vout)
                .map(|u| u.amount);
            let effective_amount = live_amount.or(entry.collateral_amount);
            let tier = effective_amount.and_then(masternode_tier_from_satoshis);

            egui::Frame::group(ui.style())
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&entry.alias).strong().size(16.0));
                        // Tier badge
                        match tier {
                            Some(t) => {
                                let (label, color) = tier_label(t);
                                ui.label(RichText::new(label).color(color).strong());
                            }
                            None => {
                                match effective_amount {
                                    Some(_) => {
                                        ui.label(RichText::new("⚠ Below threshold").color(Color32::YELLOW));
                                    }
                                    None => {
                                        ui.label(RichText::new("? Tier pending").color(Color32::GRAY))
                                            .on_hover_text("Collateral UTXO not yet fetched — tier will appear after the next sync");
                                    }
                                }
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Del").on_hover_text("Delete").clicked() {
                                to_delete = Some(entry.alias.clone());
                            }
                        });
                    });

                    egui::Grid::new(format!("mn_detail_{}", entry.alias))
                        .num_columns(2)
                        .spacing([8.0, 2.0])
                        .show(ui, |ui| {
                            ui.label("Address:");
                            ui.label(format!("{}:{}", entry.ip, entry.port));
                            ui.end_row();

                            ui.label("Collateral:");
                            let short_txid = if entry.collateral_txid.len() > 16 {
                                format!(
                                    "{}…{}",
                                    &entry.collateral_txid[..8],
                                    &entry.collateral_txid[entry.collateral_txid.len() - 8..]
                                )
                            } else {
                                entry.collateral_txid.clone()
                            };
                            ui.label(format!("{}:{}", short_txid, entry.collateral_vout));
                            ui.end_row();

                            if let Some(amount) = effective_amount {
                                ui.label("Amount:");
                                ui.label(format!("{:.0} TIME", amount as f64 / 100_000_000.0));
                                ui.end_row();
                            }

                            ui.label("Payout:");
                            if let Some(addr) = &entry.payout_address {
                                ui.label(
                                    RichText::new(addr).color(Color32::from_rgb(100, 200, 100)),
                                );
                            } else {
                                ui.label(RichText::new("Not set").color(Color32::GRAY));
                            }
                            ui.end_row();
                        });

                    // --- Action buttons ---
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        // Copy daemon conf line
                        let daemon_line = entry.to_daemon_conf_line();
                        if ui
                            .button("Copy Conf")
                            .on_hover_text(format!(
                                "Copies the line for your masternode server's masternode.conf:\n\n{}\n\nPaste into ~/.timed/masternode.conf on your server, then restart timed.",
                                daemon_line
                            ))
                            .clicked()
                        {
                            ui.ctx().copy_text(daemon_line);
                            state.success = Some(format!(
                                "Daemon conf line for '{}' copied to clipboard.",
                                entry.alias
                            ));
                        }

                        if ui
                            .button("Edit")
                            .on_hover_text("Edit this masternode entry")
                            .clicked()
                        {
                            state.mn_edit_alias = Some(entry.alias.clone());
                            state.mn_edit_ip = format!("{}:{}", entry.ip, entry.port);
                            state.mn_edit_txid = entry.collateral_txid.clone();
                            state.mn_edit_vout = entry.collateral_vout.to_string();
                        }
                    });

                    // Inline edit form
                    if state.mn_edit_alias.as_deref() == Some(&entry.alias) {
                        let old_alias = entry.alias.clone();
                        let old_payout = entry.payout_address.clone();
                        ui.add_space(4.0);
                        egui::Grid::new(format!("mn_edit_{}", entry.alias))
                            .num_columns(2)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("IP Address:");
                                ui.text_edit_singleline(&mut state.mn_edit_ip);
                                ui.end_row();
                                ui.label("Collateral TXID:");
                                let r = ui.text_edit_singleline(&mut state.mn_edit_txid);
                                r.context_menu(|ui| {
                                    if ui.button("Paste").clicked() {
                                        if let Ok(mut cb) = arboard::Clipboard::new() {
                                            if let Ok(t) = cb.get_text() {
                                                state.mn_edit_txid = t.trim().to_string();
                                            }
                                        }
                                        ui.close_menu();
                                    }
                                });
                                ui.end_row();
                                ui.label("Collateral Vout:");
                                ui.text_edit_singleline(&mut state.mn_edit_vout);
                                ui.end_row();
                            });
                        ui.horizontal(|ui| {
                            let ip_t = state.mn_edit_ip.trim().to_string();
                            let valid = !ip_t.is_empty()
                                && !state.mn_edit_txid.trim().is_empty()
                                && state.mn_edit_vout.trim().parse::<u32>().is_ok();
                            if ui.add_enabled(valid, egui::Button::new("Save")).clicked() {
                                let (ip, port) = if let Some(c) = ip_t.rfind(':') {
                                    let port = ip_t[c + 1..].parse().unwrap_or(24100u16);
                                    (ip_t[..c].to_string(), port)
                                } else {
                                    (ip_t.clone(), 24100u16)
                                };
                                let txid = state.mn_edit_txid.trim().to_string();
                                let vout: u32 = state.mn_edit_vout.trim().parse().unwrap_or(0);
                                let collateral_amount = state
                                    .utxos
                                    .iter()
                                    .find(|u| u.txid == txid && u.vout == vout)
                                    .map(|u| u.amount);
                                update_event = Some(UiEvent::UpdateMasternodeEntry {
                                    old_alias,
                                    new_entry: MasternodeEntry {
                                        alias: format!("{}:{}", ip, port),
                                        ip,
                                        port,
                                        masternode_key: String::new(),
                                        collateral_txid: txid,
                                        collateral_vout: vout,
                                        payout_address: old_payout,
                                        collateral_amount,
                                    },
                                });
                                state.mn_edit_alias = None;
                            }
                            if ui.button("Cancel").clicked() {
                                state.mn_edit_alias = None;
                            }
                        });
                    }
                });
            ui.add_space(4.0);
        }

        if let Some(alias) = to_delete {
            let _ = ui_tx.send(UiEvent::DeleteMasternodeEntry { alias });
        }
        if let Some(event) = update_event {
            let _ = ui_tx.send(event);
        }
    }
}
