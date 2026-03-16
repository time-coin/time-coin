//! Send screen — transaction form with contact address book.

use egui::Ui;
use tokio::sync::mpsc;

use crate::events::UiEvent;
use crate::state::AppState;

/// Render the send screen.
pub fn show(ui: &mut Ui, state: &mut AppState, ui_tx: &mpsc::UnboundedSender<UiEvent>) {
    // -- QR Scanner overlay window --
    render_qr_scanner(ui, state);

    ui.heading("Send TIME");
    ui.separator();
    ui.add_space(10.0);

    let expected_prefix = if state.is_testnet { "TIME0" } else { "TIME1" };
    let wrong_prefix = if state.is_testnet { "TIME1" } else { "TIME0" };
    let network_name = if state.is_testnet {
        "testnet"
    } else {
        "mainnet"
    };

    // -- Send form --
    ui.group(|ui| {
        ui.set_min_width(ui.available_width());

        ui.label("Recipient Address");

        // Auto-fill name from contacts when address matches
        let resolved_name = state
            .contact_name(&state.send_address)
            .map(|s| s.to_string());
        if let Some(ref name) = resolved_name {
            if state.send_recipient_name.is_empty() {
                state.send_recipient_name = name.clone();
            }
        }

        ui.horizontal(|ui| {
            let resp = ui.add(
                egui::TextEdit::singleline(&mut state.send_address)
                    .hint_text(format!("{}...", expected_prefix))
                    .desired_width(ui.available_width() - 80.0),
            );
            resp.context_menu(|ui| {
                if ui.button("📋 Paste").clicked() {
                    if let Ok(mut cb) = arboard::Clipboard::new() {
                        if let Ok(text) = cb.get_text() {
                            state.send_address = text.trim().to_string();
                        }
                    }
                    ui.close_menu();
                }
            });
            if ui
                .add(egui::Button::new("📷 Scan").min_size(egui::vec2(70.0, 24.0)))
                .on_hover_text("Scan QR code with webcam")
                .clicked()
            {
                state.qr_scan_error = None;
                state.qr_scanner = Some(crate::qr_scanner::QrScannerHandle::start());
            }
        });

        // Show QR scan error if any
        if let Some(ref err) = state.qr_scan_error {
            ui.colored_label(egui::Color32::RED, format!("📷 {}", err));
        }

        ui.add_space(4.0);
        ui.label("Recipient Name (optional)");
        ui.add(
            egui::TextEdit::singleline(&mut state.send_recipient_name)
                .hint_text("e.g. Alice")
                .desired_width(300.0),
        );

        // Address validation feedback
        if !state.send_address.is_empty() {
            if state.send_address.starts_with(wrong_prefix) {
                ui.colored_label(
                    egui::Color32::RED,
                    format!(
                        "WARNING: This is a {} address. You are on {}.",
                        if state.is_testnet {
                            "mainnet"
                        } else {
                            "testnet"
                        },
                        network_name,
                    ),
                );
            } else if !state.send_address.starts_with(expected_prefix) {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 165, 0),
                    format!(
                        "WARNING: Expected address starting with {}",
                        expected_prefix
                    ),
                );
            }
        }

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("Amount (TIME)");
                ui.add(
                    egui::TextEdit::singleline(&mut state.send_amount)
                        .hint_text("0.000000")
                        .desired_width(200.0),
                );
            });
            ui.add_space(10.0);
            ui.checkbox(&mut state.send_include_fee, "Include fee in amount");
        });

        ui.add_space(8.0);

        // Auto-calculate tiered fee (matches masternode consensus rule)
        let entered_amount = parse_time_amount(&state.send_amount);
        let available = if state.syncing {
            0
        } else {
            // Use the larger of UTXO-derived available and masternode-reported available.
            // UTXO list may be truncated by the default limit, so masternode is authoritative.
            let utxo_avail = state.available_balance();
            let mn_avail = state.masternode_available;
            let best = utxo_avail.max(mn_avail);
            if best > 0 {
                best
            } else {
                state.computed_balance()
            }
        };

        // When "include fee" is checked, the entered amount is the total to deduct.
        // The actual send amount is entered minus fee, and fee is calculated on the send amount.
        // Solve: send + fee(send) = entered  →  iterate to find send.
        let (send_amount, auto_fee) = if state.send_include_fee && entered_amount > 0 {
            let mut send = entered_amount;
            for _ in 0..10 {
                let fee = wallet::calculate_fee(send);
                if send + fee <= entered_amount {
                    break;
                }
                send = entered_amount.saturating_sub(fee);
            }
            let fee = wallet::calculate_fee(send);
            (send, fee)
        } else if entered_amount > 0 {
            (entered_amount, wallet::calculate_fee(entered_amount))
        } else {
            (0, 0)
        };
        if send_amount > 0 {
            let fee_pct = if send_amount < 100 * 100_000_000 {
                "1%"
            } else if send_amount < 1_000 * 100_000_000 {
                "0.5%"
            } else if send_amount < 10_000 * 100_000_000 {
                "0.25%"
            } else {
                "0.1%"
            };
            ui.label(
                egui::RichText::new(format!(
                    "Network fee: {}.{:06} TIME ({})",
                    auto_fee / 100_000_000,
                    (auto_fee % 100_000_000) / 100,
                    fee_pct,
                ))
                .color(egui::Color32::GRAY),
            );
            if state.send_include_fee {
                ui.label(
                    egui::RichText::new(format!(
                        "Recipient receives: {}.{:06} TIME",
                        send_amount / 100_000_000,
                        (send_amount % 100_000_000) / 100,
                    ))
                    .color(egui::Color32::GRAY),
                );
            }
        }

        ui.add_space(4.0);

        // Available balance and insufficient funds check
        let total_cost = if state.send_include_fee {
            entered_amount
        } else {
            send_amount.saturating_add(auto_fee)
        };
        let insufficient = send_amount > 0 && total_cost > available;

        let bal_color = if insufficient {
            egui::Color32::RED
        } else {
            egui::Color32::GRAY
        };
        let bal_text = format!(
            "Available: {}.{:06} TIME",
            available / 100_000_000,
            (available % 100_000_000) / 100
        );
        if ui
            .link(egui::RichText::new(&bal_text).color(bal_color))
            .clicked()
        {
            let whole = available / 100_000_000;
            let frac = (available % 100_000_000) / 100;
            state.send_amount = format!("{}.{:06}", whole, frac);
        }
        if insufficient {
            ui.colored_label(
                egui::Color32::RED,
                format!(
                    "Insufficient funds. Amount + fee = {}.{:06} TIME exceeds balance.",
                    total_cost / 100_000_000,
                    (total_cost % 100_000_000) / 100
                ),
            );
        }

        ui.add_space(15.0);

        // Memo (optional, max 256 chars)
        ui.label(egui::RichText::new("Memo (optional)").size(14.0));
        ui.add(
            egui::TextEdit::singleline(&mut state.send_memo)
                .hint_text("e.g. Payment for invoice #42")
                .desired_width(350.0)
                .char_limit(256),
        );
        ui.label(
            egui::RichText::new(format!("{}/256", state.send_memo.len()))
                .size(11.0)
                .color(egui::Color32::GRAY),
        );

        ui.add_space(15.0);

        // Full address validation with checksum
        let address_valid = if state.send_address.starts_with(expected_prefix) {
            wallet::address::Address::from_string(&state.send_address).is_ok()
        } else {
            false
        };

        // Show checksum error if prefix is right but checksum fails
        if !state.send_address.is_empty()
            && state.send_address.starts_with(expected_prefix)
            && !address_valid
        {
            ui.colored_label(
                egui::Color32::RED,
                "Invalid address checksum — check for typos",
            );
        }

        let can_send = address_valid
            && !state.send_address.is_empty()
            && !state.send_amount.is_empty()
            && !insufficient
            && !state.loading;

        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    can_send,
                    egui::Button::new(egui::RichText::new("Send Transaction").size(16.0))
                        .min_size(egui::vec2(200.0, 36.0)),
                )
                .clicked()
            {
                if send_amount == 0 {
                    state.error = Some("Invalid amount".to_string());
                } else {
                    // Auto-save to address book if name is provided
                    if !state.send_recipient_name.trim().is_empty() {
                        let _ = ui_tx.send(UiEvent::SaveContact {
                            name: state.send_recipient_name.trim().to_string(),
                            address: state.send_address.clone(),
                        });
                    }
                    let _ = ui_tx.send(UiEvent::SendTransaction {
                        to: state.send_address.clone(),
                        amount: send_amount,
                        fee: auto_fee,
                        memo: state.send_memo.clone(),
                    });
                    state.loading = true;
                    state.error = None;
                    state.send_too_large = false;
                }
            }

            ui.add_space(8.0);

            if ui
                .add(
                    egui::Button::new(egui::RichText::new("Clear").size(16.0))
                        .min_size(egui::vec2(80.0, 36.0)),
                )
                .clicked()
            {
                state.send_address.clear();
                state.send_recipient_name.clear();
                state.send_amount.clear();
                state.send_fee.clear();
                state.send_memo.clear();
                state.send_include_fee = false;
                state.send_too_large = false;
                state.error = None;
                state.success = None;
            }

            if state.loading {
                ui.spinner();
            }
        });
    });

    // Status messages
    if state.send_too_large {
        ui.add_space(10.0);
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(80, 40, 0))
            .show(ui, |ui| {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 180, 60),
                    "⚠ Transaction too large: you have too many small UTXOs to send in one transaction.",
                );
                ui.add_space(6.0);
                ui.label("Consolidating your UTXOs will merge them into fewer, larger ones so this send will work.");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Consolidate UTXOs now").clicked() {
                        state.send_too_large = false;
                        let _ = ui_tx.send(UiEvent::ConsolidateUtxos);
                        state.consolidation_in_progress = true;
                        state.consolidation_status =
                            "Consolidation started — try sending again once complete.".to_string();
                    }
                    if ui.button("Dismiss").clicked() {
                        state.send_too_large = false;
                    }
                });
            });
    } else if let Some(ref err) = state.error {
        ui.add_space(10.0);
        ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
    }
    if let Some(ref msg) = state.success {
        ui.add_space(10.0);
        ui.colored_label(egui::Color32::GREEN, format!("Sent: {}", msg));
    }

    ui.add_space(15.0);
    ui.separator();
    ui.add_space(5.0);

    // -- Address Book --
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Address Book").strong().size(16.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .selectable_label(state.show_add_contact, "+ Add Contact")
                .clicked()
            {
                state.show_add_contact = !state.show_add_contact;
            }
        });
    });

    // Add contact form
    if state.show_add_contact {
        ui.add_space(4.0);
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.add(
                    egui::TextEdit::singleline(&mut state.new_contact_name)
                        .hint_text("e.g. Alice")
                        .desired_width(150.0),
                );
                ui.label("Address:");
                ui.add(
                    egui::TextEdit::singleline(&mut state.new_contact_address)
                        .hint_text(format!("{}...", expected_prefix))
                        .desired_width(250.0),
                );
                let contact_addr_valid = !state.new_contact_name.is_empty()
                    && wallet::address::Address::from_string(&state.new_contact_address).is_ok();
                if ui
                    .add_enabled(contact_addr_valid, egui::Button::new("Save"))
                    .clicked()
                {
                    let _ = ui_tx.send(UiEvent::SaveContact {
                        name: state.new_contact_name.clone(),
                        address: state.new_contact_address.clone(),
                    });
                    state.new_contact_name.clear();
                    state.new_contact_address.clear();
                    state.show_add_contact = false;
                }
            });
        });
    }

    ui.add_space(4.0);

    if state.contacts.is_empty() {
        ui.label(
            egui::RichText::new("No contacts yet. Add one to quickly send TIME.")
                .color(egui::Color32::GRAY)
                .italics(),
        );
    } else {
        // Search box
        ui.add(
            egui::TextEdit::singleline(&mut state.contact_search)
                .hint_text("Search contacts...")
                .desired_width(ui.available_width()),
        );
        ui.add_space(4.0);

        let search = state.contact_search.to_lowercase();
        let filtered: Vec<_> = state
            .contacts
            .iter()
            .filter(|c| {
                search.is_empty()
                    || c.name.to_lowercase().contains(&search)
                    || c.address.to_lowercase().contains(&search)
            })
            .collect();

        if filtered.is_empty() {
            ui.label(
                egui::RichText::new("No contacts match your search.")
                    .color(egui::Color32::GRAY)
                    .italics(),
            );
        } else {
            ui.label(
                egui::RichText::new(format!("{} contacts", filtered.len()))
                    .color(egui::Color32::GRAY)
                    .small(),
            );
            ui.add_space(4.0);

            let accent_color = egui::Color32::from_rgb(0, 120, 200);
            let hover_fill = egui::Color32::from_rgba_unmultiplied(0, 120, 200, 18);
            let selected_fill = egui::Color32::from_rgba_unmultiplied(0, 120, 200, 30);

            let mut delete_addr = None;
            let mut save_edit = None;
            egui::ScrollArea::vertical()
                .id_salt("contacts_scroll")
                .max_height(300.0)
                .show(ui, |ui| {
                    for contact in &filtered {
                        let is_editing =
                            state.editing_contact_address.as_deref() == Some(&contact.address);
                        let is_selected = state.send_address == contact.address;

                        if is_editing {
                            // Inline edit row
                            let row_height = 48.0;
                            let (row_rect, _) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), row_height),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(row_rect, 4.0, selected_fill);
                            ui.painter().rect_filled(
                                egui::Rect::from_min_max(
                                    row_rect.min,
                                    egui::pos2(row_rect.min.x + 3.0, row_rect.max.y),
                                ),
                                0.0,
                                egui::Color32::from_rgb(200, 160, 0),
                            );
                            let mut child_ui = ui.new_child(
                                egui::UiBuilder::new()
                                    .max_rect(row_rect.shrink2(egui::vec2(10.0, 6.0))),
                            );
                            child_ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut state.editing_contact_name)
                                            .font(egui::TextStyle::Body)
                                            .desired_width(200.0)
                                            .hint_text("Contact name"),
                                    );
                                    let short = super::truncate_middle(&contact.address, 14, 6);
                                    ui.label(
                                        egui::RichText::new(short)
                                            .monospace()
                                            .size(11.0)
                                            .color(egui::Color32::GRAY),
                                    );
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.small_button("Cancel").clicked() {
                                            state.editing_contact_address = None;
                                        }
                                        if ui.small_button("Save").clicked() {
                                            save_edit = Some((
                                                state.editing_contact_name.clone(),
                                                contact.address.clone(),
                                            ));
                                        }
                                    },
                                );
                            });
                            // Separator
                            ui.painter().line_segment(
                                [
                                    egui::pos2(row_rect.min.x + 10.0, row_rect.max.y),
                                    egui::pos2(row_rect.max.x - 10.0, row_rect.max.y),
                                ],
                                egui::Stroke::new(0.5, egui::Color32::from_gray(210)),
                            );
                        } else {
                            // Normal display row
                            let row_height = 48.0;
                            let (row_rect, row_response) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), row_height),
                                egui::Sense::click(),
                            );

                            let fill = if is_selected {
                                selected_fill
                            } else if row_response.hovered() {
                                hover_fill
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            ui.painter().rect_filled(row_rect, 4.0, fill);

                            if is_selected {
                                ui.painter().rect_filled(
                                    egui::Rect::from_min_max(
                                        row_rect.min,
                                        egui::pos2(row_rect.min.x + 3.0, row_rect.max.y),
                                    ),
                                    0.0,
                                    accent_color,
                                );
                            }

                            if row_response.clicked() {
                                state.send_address = contact.address.clone();
                                state.send_recipient_name = contact.name.clone();
                            }

                            let mut child_ui = ui.new_child(
                                egui::UiBuilder::new()
                                    .max_rect(row_rect.shrink2(egui::vec2(10.0, 6.0))),
                            );
                            child_ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(&contact.name).size(13.0).strong(),
                                    );
                                    let short = super::truncate_middle(&contact.address, 14, 6);
                                    ui.label(
                                        egui::RichText::new(short)
                                            .monospace()
                                            .size(11.0)
                                            .color(egui::Color32::GRAY),
                                    );
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .small_button("X")
                                            .on_hover_text("Remove contact")
                                            .clicked()
                                        {
                                            delete_addr = Some(contact.address.clone());
                                        }
                                        ui.add_space(4.0);
                                        if ui
                                            .small_button("✏")
                                            .on_hover_text("Edit contact")
                                            .clicked()
                                        {
                                            state.editing_contact_address =
                                                Some(contact.address.clone());
                                            state.editing_contact_name = contact.name.clone();
                                        }
                                        ui.add_space(4.0);
                                        if ui
                                            .small_button("📋")
                                            .on_hover_text("Copy address")
                                            .clicked()
                                        {
                                            ui.ctx().copy_text(contact.address.clone());
                                        }
                                    },
                                );
                            });

                            // Thin separator line between rows
                            ui.painter().line_segment(
                                [
                                    egui::pos2(row_rect.min.x + 10.0, row_rect.max.y),
                                    egui::pos2(row_rect.max.x - 10.0, row_rect.max.y),
                                ],
                                egui::Stroke::new(0.5, egui::Color32::from_gray(210)),
                            );
                        }
                    }
                });
            if let Some(addr) = delete_addr {
                // Clear edit state if we deleted the one being edited
                if state.editing_contact_address.as_deref() == Some(addr.as_str()) {
                    state.editing_contact_address = None;
                }
                // Clear send form if the deleted contact was selected
                if state.send_address == addr {
                    state.send_address.clear();
                }
                let _ = ui_tx.send(UiEvent::DeleteContact { address: addr });
            }
            if let Some((name, address)) = save_edit {
                let _ = ui_tx.send(UiEvent::SaveContact { name, address });
                state.editing_contact_address = None;
            }
        }
    }
}

fn parse_time_amount(s: &str) -> u64 {
    let s = s.trim();
    if s.is_empty() {
        return 0;
    }
    let (whole, frac) = if let Some(dot) = s.find('.') {
        (&s[..dot], &s[dot + 1..])
    } else {
        (s, "")
    };
    let whole_val: u64 = whole.parse().unwrap_or(0);
    let frac_padded = format!("{:0<8}", frac);
    let frac_val: u64 = frac_padded[..8].parse().unwrap_or(0);
    whole_val
        .saturating_mul(100_000_000)
        .saturating_add(frac_val)
}

/// Render the QR scanner floating window when active.
fn render_qr_scanner(ui: &mut Ui, state: &mut AppState) {
    // Check for scan result or error before rendering
    let mut got_result = false;
    if let Some(ref scanner) = state.qr_scanner {
        if let Some(address) = scanner.take_result() {
            state.send_address = address;
            state.send_recipient_name.clear();
            crate::qr_scanner::play_scan_sound();
            got_result = true;
        }
        if let Some(err) = scanner.get_error() {
            state.qr_scan_error = Some(err);
            got_result = true;
        }
    }
    if got_result {
        state.qr_scanner = None;
    }

    if state.qr_scanner.is_none() {
        return;
    }

    // Request continuous repaints while scanning
    ui.ctx().request_repaint();

    let mut close = false;
    egui::Window::new("📷 QR Scanner")
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .resizable(false)
        .show(ui.ctx(), |ui| {
            ui.label("Point your webcam at a QR code");
            ui.add_space(4.0);

            // Update and display camera preview
            let has_frame = if let Some(ref mut scanner) = state.qr_scanner {
                if let Some(tex) = scanner.update_texture(ui.ctx()) {
                    let size = tex.size_vec2();
                    let scale = (400.0 / size.x).min(300.0 / size.y).min(1.0);
                    ui.image(egui::load::SizedTexture::new(tex.id(), size * scale));
                    true
                } else {
                    false
                }
            } else {
                false
            };

            if !has_frame {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Starting camera...");
                });
                ui.allocate_space(egui::vec2(400.0, 300.0));
            }

            ui.add_space(8.0);
            if ui
                .add(egui::Button::new("Cancel").min_size(egui::vec2(100.0, 28.0)))
                .clicked()
            {
                close = true;
            }
        });

    if close {
        state.qr_scanner = None;
    }
}
