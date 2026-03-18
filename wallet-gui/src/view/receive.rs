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
                    state.screen = Screen::PaymentRequests;
                    state.show_payment_request_form = true;
                    let _ = ui_tx.send(UiEvent::NavigatedTo(Screen::PaymentRequests));
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
                let mut delete_addr: Option<String> = None;

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
                            if label_resp.changed() {
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

                        // Right: balance + copy + delete buttons
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Delete button — only for non-primary unused addresses
                            let has_txns = state.transactions.iter().any(|tx| tx.address == addr);
                            let can_delete = i > 0 && bal == 0 && !has_txns;
                            if can_delete {
                                if ui
                                    .small_button("🗑")
                                    .on_hover_text("Delete unused address")
                                    .clicked()
                                {
                                    delete_addr = Some(addr.clone());
                                }
                                ui.add_space(4.0);
                            }
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

                if let Some(addr) = delete_addr {
                    let _ = ui_tx.send(UiEvent::DeleteAddress { address: addr });
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
