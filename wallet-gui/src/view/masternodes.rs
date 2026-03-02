use egui::{Color32, RichText, Ui};

use crate::events::UiEvent;
use crate::state::AppState;
use crate::wallet_db::MasternodeEntry;

/// Render the Masternodes management screen.
pub fn render(
    ui: &mut Ui,
    state: &mut AppState,
    ui_tx: &tokio::sync::mpsc::UnboundedSender<UiEvent>,
) {
    ui.heading("Masternodes");
    ui.add_space(8.0);

    // ---------- Add / Import buttons ----------
    ui.horizontal(|ui| {
        if ui.button("➕ Add Masternode").clicked() {
            state.mn_show_add_form = !state.mn_show_add_form;
        }
        if ui.button("📥 Import masternode.conf").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Select masternode.conf")
                .add_filter("conf", &["conf"])
                .add_filter("All files", &["*"])
                .pick_file()
            {
                let _ = ui_tx.send(UiEvent::ImportMasternodeConf { path });
            }
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
                        ui.label("Alias:");
                        ui.text_edit_singleline(&mut state.mn_add_alias);
                        ui.end_row();

                        ui.label("IP Address:");
                        ui.text_edit_singleline(&mut state.mn_add_ip);
                        ui.end_row();

                        ui.label("Port:");
                        ui.text_edit_singleline(&mut state.mn_add_port);
                        ui.end_row();

                        ui.label("Masternode Key:");
                        ui.text_edit_singleline(&mut state.mn_add_key);
                        ui.end_row();

                        ui.label("Collateral TXID:");
                        ui.text_edit_singleline(&mut state.mn_add_txid);
                        ui.end_row();

                        ui.label("Collateral Vout:");
                        ui.text_edit_singleline(&mut state.mn_add_vout);
                        ui.end_row();

                        ui.label("Payout Address:");
                        ui.text_edit_singleline(&mut state.mn_add_payout);
                        ui.end_row();
                    });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let can_save = !state.mn_add_alias.trim().is_empty()
                        && !state.mn_add_ip.trim().is_empty()
                        && state.mn_add_port.trim().parse::<u16>().is_ok()
                        && !state.mn_add_key.trim().is_empty()
                        && !state.mn_add_txid.trim().is_empty()
                        && state.mn_add_vout.trim().parse::<u32>().is_ok();

                    if ui
                        .add_enabled(can_save, egui::Button::new("💾 Save"))
                        .clicked()
                    {
                        let entry = MasternodeEntry {
                            alias: state.mn_add_alias.trim().to_string(),
                            ip: state.mn_add_ip.trim().to_string(),
                            port: state.mn_add_port.trim().parse().unwrap_or(24100),
                            masternode_key: state.mn_add_key.trim().to_string(),
                            collateral_txid: state.mn_add_txid.trim().to_string(),
                            collateral_vout: state.mn_add_vout.trim().parse().unwrap_or(0),
                            payout_address: if state.mn_add_payout.trim().is_empty() {
                                None
                            } else {
                                Some(state.mn_add_payout.trim().to_string())
                            },
                        };
                        let _ = ui_tx.send(UiEvent::SaveMasternodeEntry(entry));
                        // Reset form
                        state.mn_add_alias.clear();
                        state.mn_add_ip.clear();
                        state.mn_add_port = "24100".to_string();
                        state.mn_add_key.clear();
                        state.mn_add_txid.clear();
                        state.mn_add_vout = "0".to_string();
                        state.mn_add_payout.clear();
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
        // Collect aliases to delete outside the borrow of state
        let mut to_delete: Option<String> = None;
        let mut register_event: Option<UiEvent> = None;
        let mut register_alias: Option<String> = None;
        let mut update_event: Option<UiEvent> = None;

        for entry in &state.masternode_entries {
            egui::Frame::group(ui.style())
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&entry.alias).strong().size(16.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🗑").on_hover_text("Delete").clicked() {
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

                            ui.label("Key:");
                            let short_key = if entry.masternode_key.len() > 16 {
                                format!(
                                    "{}…{}",
                                    &entry.masternode_key[..8],
                                    &entry.masternode_key[entry.masternode_key.len() - 4..]
                                )
                            } else {
                                entry.masternode_key.clone()
                            };
                            ui.label(short_key);
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
                        if ui
                            .button("📋 Register On-Chain")
                            .on_hover_text("Broadcast a masternode registration transaction")
                            .clicked()
                        {
                            let payout = entry
                                .payout_address
                                .clone()
                                .or_else(|| state.addresses.first().map(|a| a.address.clone()))
                                .unwrap_or_default();
                            register_alias = Some(entry.alias.clone());
                            register_event = Some(UiEvent::RegisterMasternode {
                                alias: entry.alias.clone(),
                                ip: entry.ip.clone(),
                                port: entry.port,
                                collateral_txid: entry.collateral_txid.clone(),
                                collateral_vout: entry.collateral_vout,
                                payout_address: payout,
                            });
                        }
                        if ui
                            .button("💸 Update Payout")
                            .on_hover_text("Change the payout address for this masternode")
                            .clicked()
                        {
                            state.mn_update_payout_alias = Some(entry.alias.clone());
                            state.mn_update_payout_input =
                                entry.payout_address.clone().unwrap_or_default();
                        }
                    });

                    // Inline payout update form
                    if state.mn_update_payout_alias.as_deref() == Some(&entry.alias) {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("New payout address:");
                            ui.text_edit_singleline(&mut state.mn_update_payout_input);
                        });
                        ui.horizontal(|ui| {
                            let valid = !state.mn_update_payout_input.trim().is_empty();
                            if ui
                                .add_enabled(valid, egui::Button::new("✅ Confirm"))
                                .clicked()
                            {
                                let mn_id = format!("{}:{}", entry.ip, entry.port);
                                update_event = Some(UiEvent::UpdateMasternodePayout {
                                    masternode_id: mn_id,
                                    new_payout_address: state
                                        .mn_update_payout_input
                                        .trim()
                                        .to_string(),
                                });
                                state.mn_update_payout_alias = None;
                                state.mn_update_payout_input.clear();
                            }
                            if ui.button("Cancel").clicked() {
                                state.mn_update_payout_alias = None;
                                state.mn_update_payout_input.clear();
                            }
                        });
                    }
                });
            ui.add_space(4.0);
        }

        if let Some(alias) = to_delete {
            let _ = ui_tx.send(UiEvent::DeleteMasternodeEntry { alias });
        }
        if let Some(event) = register_event {
            let _ = ui_tx.send(event);
            if let Some(alias) = register_alias {
                state.success = Some(format!("Registering masternode '{}'…", alias));
            }
        }
        if let Some(event) = update_event {
            let _ = ui_tx.send(event);
        }
    }
}
