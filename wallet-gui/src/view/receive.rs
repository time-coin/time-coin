//! Receive screen — display wallet addresses with QR codes and editable labels.

use egui::Ui;
use tokio::sync::mpsc;

use crate::events::{Screen, UiEvent};
use crate::state::AppState;

/// Render the receive screen.
pub fn show(ui: &mut Ui, state: &mut AppState, ui_tx: &mpsc::UnboundedSender<UiEvent>) {
    ui.heading("Receive TIME");
    ui.separator();
    ui.add_space(10.0);

    if state.addresses.is_empty() {
        ui.label(
            egui::RichText::new("No addresses available -- load or create a wallet first.")
                .color(egui::Color32::GRAY)
                .italics(),
        );
        return;
    }

    // Clamp selected address
    if state.selected_address >= state.addresses.len() {
        state.selected_address = 0;
    }

    // Clone selected address data to avoid borrow conflicts
    let selected_addr = state.addresses[state.selected_address].address.clone();
    let selected_label = state.addresses[state.selected_address].label.clone();

    // Top section: QR code and selected address details
    ui.horizontal(|ui| {
        let uri = format!("bytes://qr_{}", selected_addr);
        if let Some(png) = qr_png_bytes(&selected_addr) {
            let image =
                egui::Image::from_bytes(uri, png).fit_to_exact_size(egui::vec2(180.0, 180.0));
            ui.add(image);
        }

        ui.add_space(16.0);

        ui.vertical(|ui| {
            ui.add_space(20.0);
            ui.label(egui::RichText::new(&selected_label).size(16.0).strong());
            ui.add_space(8.0);
            ui.label(egui::RichText::new(&selected_addr).monospace().size(13.0));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Copy Address").clicked() {
                    ui.ctx().copy_text(selected_addr.clone());
                }
                ui.add_space(8.0);
                if ui.button("Request Payment").clicked() {
                    state.show_payment_request_form = !state.show_payment_request_form;
                }
            });
        });
    });

    // Wrap everything below the heading in a single scroll area so the
    // payment-request form and incoming requests are never hidden.
    egui::ScrollArea::vertical()
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.add_space(15.0);
            ui.separator();
            ui.add_space(10.0);

            // ── Request Payment Form (toggled by button next to QR) ──
            if state.show_payment_request_form {
                ui.separator();
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("📋 Request Payment")
                        .size(14.0)
                        .strong(),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(
                        "Send a payment request to another wallet via the masternode network.",
                    )
                    .color(egui::Color32::GRAY)
                    .italics()
                    .size(11.0),
                );
                ui.add_space(8.0);

                egui::Grid::new("pr_form_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Payer Address:").strong());
                        ui.add(
                            egui::TextEdit::singleline(&mut state.pr_address)
                                .desired_width(350.0)
                                .hint_text("TIME address of who should pay..."),
                        );
                        ui.end_row();

                        ui.label(egui::RichText::new("Amount (TIME):").strong());
                        ui.add(
                            egui::TextEdit::singleline(&mut state.pr_amount)
                                .desired_width(150.0)
                                .hint_text("0.00000"),
                        );
                        ui.end_row();

                        ui.label(egui::RichText::new("Label:").strong());
                        ui.add(
                            egui::TextEdit::singleline(&mut state.pr_label)
                                .desired_width(300.0)
                                .hint_text("e.g. Invoice #42, Rent, etc."),
                        );
                        ui.end_row();

                        ui.label(egui::RichText::new("Memo:").strong());
                        ui.add(
                            egui::TextEdit::singleline(&mut state.pr_memo)
                                .desired_width(300.0)
                                .hint_text("Optional note for the payer..."),
                        );
                        ui.end_row();
                    });

                ui.add_space(8.0);

                let amount_ok = state.pr_amount.parse::<f64>().is_ok_and(|v| v > 0.0);
                let can_send = !state.pr_address.is_empty() && amount_ok;

                if !state.pr_amount.is_empty() && !amount_ok {
                    ui.label(
                        egui::RichText::new("⚠ Enter a valid amount greater than zero.")
                            .color(egui::Color32::from_rgb(255, 165, 0))
                            .size(12.0),
                    );
                }

                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(can_send, egui::Button::new("OK — Send Request"))
                        .clicked()
                    {
                        if let Ok(amount_f64) = state.pr_amount.parse::<f64>() {
                            let amount = (amount_f64 * 100_000.0) as u64;
                            let _ = ui_tx.send(UiEvent::SendPaymentRequest {
                                to_address: state.pr_address.clone(),
                                amount,
                                label: state.pr_label.clone(),
                                memo: state.pr_memo.clone(),
                            });
                            state.show_payment_request_form = false;
                            state.pr_address.clear();
                            state.pr_amount.clear();
                            state.pr_label.clear();
                            state.pr_memo.clear();
                        }
                    }
                    ui.add_space(8.0);
                    if ui.button("Cancel").clicked() {
                        state.show_payment_request_form = false;
                        state.pr_address.clear();
                        state.pr_amount.clear();
                        state.pr_label.clear();
                        state.pr_memo.clear();
                    }
                });

                ui.add_space(10.0);
            }

            // ── Incoming Payment Requests ──
            let now_ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let active_requests: Vec<_> = state
                .payment_requests
                .iter()
                .filter(|r| r.expires > now_ts)
                .collect();

            if !active_requests.is_empty() {
                ui.separator();
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!(
                        "📨 Incoming Payment Requests ({})",
                        active_requests.len()
                    ))
                    .size(14.0)
                    .strong(),
                );
                ui.add_space(6.0);

                let mut pay_id: Option<String> = None;
                let mut decline_id: Option<String> = None;

                let available = {
                    let total = if state.masternode_balance > 0 {
                        state.masternode_balance
                    } else {
                        state.utxo_total().max(state.computed_balance())
                    };
                    total.saturating_sub(state.locked_balance())
                };

                for req in &active_requests {
                    let remaining_mins = (req.expires - now_ts) / 60;
                    let time_str = if remaining_mins > 60 {
                        format!("{}h {}m left", remaining_mins / 60, remaining_mins % 60)
                    } else {
                        format!("{}m left", remaining_mins)
                    };

                    let fee = wallet::calculate_fee(req.amount);
                    let total_needed = req.amount.saturating_add(fee);
                    let can_pay = available >= total_needed;

                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::same(10))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());

                            ui.horizontal(|ui| {
                                if !req.label.is_empty() {
                                    ui.label(egui::RichText::new(&req.label).strong().size(14.0));
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(format!("⏱ {}", time_str))
                                                .size(11.0)
                                                .color(egui::Color32::from_rgb(200, 150, 0)),
                                        );
                                    },
                                );
                            });

                            ui.add_space(4.0);

                            egui::Grid::new(format!("pr_grid_{}", req.id))
                                .num_columns(2)
                                .spacing([8.0, 4.0])
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("From:").weak().size(12.0));
                                    ui.label(
                                        egui::RichText::new(super::truncate_middle(
                                            &req.from_address,
                                            14,
                                            6,
                                        ))
                                        .monospace()
                                        .size(12.0),
                                    )
                                    .on_hover_text(&req.from_address);
                                    ui.end_row();

                                    ui.label(egui::RichText::new("Amount:").weak().size(12.0));
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} TIME  (+{} TIME fee)",
                                            req.amount as f64 / 100_000.0,
                                            fee as f64 / 100_000.0,
                                        ))
                                        .strong()
                                        .size(13.0),
                                    );
                                    ui.end_row();

                                    let memo_override = state
                                        .pr_memo_overrides
                                        .entry(req.id.clone())
                                        .or_insert_with(|| req.memo.clone());
                                    ui.label(egui::RichText::new("Memo:").weak().size(12.0));
                                    ui.add(
                                        egui::TextEdit::singleline(memo_override)
                                            .desired_width(280.0)
                                            .hint_text("Add a memo...")
                                            .font(egui::TextStyle::Small),
                                    );
                                    ui.end_row();
                                });

                            ui.add_space(6.0);

                            if !can_pay {
                                ui.label(
                                    egui::RichText::new(format!(
                                "⚠ Insufficient funds — need {} TIME (inc. fee), have {} TIME",
                                total_needed as f64 / 100_000.0,
                                available as f64 / 100_000.0,
                            ))
                                    .color(egui::Color32::from_rgb(220, 80, 80))
                                    .size(12.0),
                                );
                                ui.add_space(4.0);
                            }

                            ui.horizontal(|ui| {
                                if ui
                                    .add_enabled(
                                        can_pay,
                                        egui::Button::new(
                                            egui::RichText::new("✔ Approve")
                                                .color(egui::Color32::WHITE),
                                        )
                                        .fill(if can_pay {
                                            egui::Color32::from_rgb(0, 140, 60)
                                        } else {
                                            egui::Color32::from_gray(100)
                                        }),
                                    )
                                    .on_disabled_hover_text("Not enough funds to pay this request")
                                    .clicked()
                                {
                                    pay_id = Some(req.id.clone());
                                }
                                ui.add_space(8.0);
                                if ui
                                    .button(
                                        egui::RichText::new("✕ Decline")
                                            .color(egui::Color32::from_rgb(200, 60, 60)),
                                    )
                                    .clicked()
                                {
                                    decline_id = Some(req.id.clone());
                                }
                            });
                        });
                    ui.add_space(4.0);
                }

                if let Some(id) = pay_id {
                    if let Some(req) = state.payment_requests.iter().find(|r| r.id == id) {
                        let memo = state
                            .pr_memo_overrides
                            .get(&id)
                            .cloned()
                            .unwrap_or_else(|| req.memo.clone());
                        state.send_address = req.from_address.clone();
                        state.send_amount = format!("{:.5}", req.amount as f64 / 100_000.0);
                        state.send_memo = memo;
                        state.screen = Screen::Send;
                    }
                    let _ = ui_tx.send(UiEvent::PayRequest {
                        request_id: id.clone(),
                    });
                    state.pr_memo_overrides.remove(&id);
                }
                if let Some(id) = decline_id {
                    state.payment_requests.retain(|r| r.id != id);
                    state.pr_memo_overrides.remove(&id);
                    let _ = ui_tx.send(UiEvent::DeclineRequest { request_id: id });
                }

                ui.add_space(6.0);
            }

            // ── Address list ──
            ui.separator();
            ui.add_space(10.0);

            // Header with generate button
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Your Addresses").size(14.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("+ New Address").clicked() {
                        let _ = ui_tx.send(UiEvent::GenerateAddress);
                    }
                });
            });
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    "Generate a new address for each sender or transaction to improve privacy.",
                )
                .color(egui::Color32::GRAY)
                .italics()
                .size(11.0),
            );
            ui.add_space(8.0);

            // Search filter
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.add(
                    egui::TextEdit::singleline(&mut state.receive_search)
                        .desired_width(250.0)
                        .hint_text("Filter by label or address..."),
                );
                if !state.receive_search.is_empty() && ui.small_button("✕").clicked() {
                    state.receive_search.clear();
                }
            });
            ui.add_space(8.0);

            let search = state.receive_search.to_lowercase();
            let accent_color = egui::Color32::from_rgb(0, 120, 200);
            let hover_fill = egui::Color32::from_rgba_unmultiplied(0, 120, 200, 18);
            let selected_fill = egui::Color32::from_rgba_unmultiplied(0, 120, 200, 30);

            {
                let mut label_updates: Vec<(usize, String)> = Vec::new();
                let mut clicked_row: Option<usize> = None;

                for i in 0..state.addresses.len() {
                    // Filter by search term
                    if !search.is_empty() {
                        let label_match = state.addresses[i].label.to_lowercase().contains(&search);
                        let addr_match =
                            state.addresses[i].address.to_lowercase().contains(&search);
                        if !label_match && !addr_match {
                            continue;
                        }
                    }

                    let is_selected = i == state.selected_address;
                    let addr = state.addresses[i].address.clone();
                    let bal = state.address_balance(&addr);

                    // Allocate the full row rect for hover/click detection
                    let row_height = 52.0;
                    let (row_rect, row_response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), row_height),
                        egui::Sense::click(),
                    );

                    if row_response.clicked() {
                        clicked_row = Some(i);
                    }

                    let fill = if is_selected {
                        selected_fill
                    } else if row_response.hovered() {
                        hover_fill
                    } else {
                        egui::Color32::TRANSPARENT
                    };

                    // Row background
                    ui.painter().rect_filled(row_rect, 4.0, fill);

                    // Left accent stripe for selected row
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

                    // Render row contents using a child UI inside the allocated rect
                    let mut child_ui = ui.new_child(
                        egui::UiBuilder::new().max_rect(row_rect.shrink2(egui::vec2(10.0, 6.0))),
                    );
                    child_ui.horizontal(|ui| {
                        // Left: label (editable) + truncated address stacked
                        ui.vertical(|ui| {
                            let label_resp = ui.add(
                                egui::TextEdit::singleline(&mut state.addresses[i].label)
                                    .font(egui::TextStyle::Body)
                                    .desired_width(180.0)
                                    .hint_text("Unlabeled"),
                            );
                            if label_resp.lost_focus() {
                                label_updates.push((i, state.addresses[i].label.clone()));
                            }
                            let short = super::truncate_middle(&addr, 14, 6);
                            ui.label(
                                egui::RichText::new(short)
                                    .monospace()
                                    .size(11.0)
                                    .color(egui::Color32::GRAY),
                            );
                        });

                        // Right: balance + copy button
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .small_button("📋")
                                .on_hover_text("Copy address")
                                .clicked()
                            {
                                ui.ctx().copy_text(addr.clone());
                            }
                            ui.add_space(4.0);
                            let bal_text = if bal > 0 {
                                egui::RichText::new(state.format_time(bal))
                                    .monospace()
                                    .size(12.0)
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 160, 60))
                            } else {
                                egui::RichText::new("—")
                                    .size(12.0)
                                    .color(egui::Color32::GRAY)
                            };
                            ui.label(bal_text);
                        });
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

                if let Some(i) = clicked_row {
                    state.selected_address = i;
                }

                // Persist label changes
                for (index, label) in label_updates {
                    let _ = ui_tx.send(UiEvent::UpdateAddressLabel { index, label });
                }
            } // end address list block
        }); // end outer ScrollArea
}

/// Generate QR code as PNG bytes for the given data string.
fn qr_png_bytes(data: &str) -> Option<Vec<u8>> {
    use image::{ImageEncoder, Rgba, RgbaImage};
    use qrcode::QrCode;

    let code = QrCode::new(data.as_bytes()).ok()?;
    let colors = code.to_colors();
    let w = code.width();
    let scale = 8u32;
    let border = 2u32;
    let size = (w as u32 + border * 2) * scale;

    let mut img = RgbaImage::from_pixel(size, size, Rgba([255, 255, 255, 255]));
    for y in 0..w {
        for x in 0..w {
            if colors[y * w + x] == qrcode::Color::Dark {
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = (x as u32 + border) * scale + dx;
                        let py = (y as u32 + border) * scale + dy;
                        img.put_pixel(px, py, Rgba([0, 0, 0, 255]));
                    }
                }
            }
        }
    }

    let mut buf = std::io::Cursor::new(Vec::new());
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    encoder
        .write_image(img.as_raw(), size, size, image::ExtendedColorType::Rgba8)
        .ok()?;
    Some(buf.into_inner())
}
