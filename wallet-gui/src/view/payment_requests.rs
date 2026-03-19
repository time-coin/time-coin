//! Payment Requests screen — create, track, and respond to payment requests.

use egui::Ui;
use tokio::sync::mpsc;

use crate::events::{Screen, UiEvent};
use crate::state::AppState;

fn time_ago(ts: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let secs = (now - ts).max(0);
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

fn status_color(status: &str) -> egui::Color32 {
    match status {
        "paid" => egui::Color32::from_rgb(0, 160, 60),
        "declined" | "failed" => egui::Color32::from_rgb(200, 60, 60),
        "cancelled" => egui::Color32::from_rgb(150, 150, 150),
        _ => egui::Color32::from_rgb(200, 140, 0),
    }
}

fn status_label(status: &str) -> &'static str {
    match status {
        "paid" => "Paid",
        "declined" => "Declined",
        "cancelled" => "Cancelled",
        "failed" => "Failed",
        _ => "Pending",
    }
}

/// Render the Payment Requests screen.
pub fn show(ui: &mut Ui, state: &mut AppState, ui_tx: &mpsc::UnboundedSender<UiEvent>) {
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    // Count badges for heading
    let incoming_pending = state
        .payment_requests
        .iter()
        .filter(|r| r.expires > now_ts)
        .count();
    let sent_pending = state
        .sent_payment_requests
        .iter()
        .filter(|r| r.status == "pending" && r.expires > now_ts)
        .count();
    let total_pending = incoming_pending + sent_pending;

    ui.horizontal(|ui| {
        ui.heading("Payment Requests");
        if total_pending > 0 {
            ui.add_space(8.0);
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(200, 80, 0))
                .corner_radius(10.0)
                .inner_margin(egui::Margin::symmetric(8, 2))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(format!("{}", total_pending))
                            .size(12.0)
                            .strong()
                            .color(egui::Color32::WHITE),
                    );
                });
        }
    });
    ui.separator();
    ui.add_space(8.0);

    if !state.wallet_loaded {
        ui.label(
            egui::RichText::new("Load a wallet first.")
                .color(egui::Color32::GRAY)
                .italics(),
        );
        return;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, true])
        .show(ui, |ui| {
            // ── New Request Form ──────────────────────────────────────────
            if !state.show_payment_request_form {
                if ui.button("Request Payment").clicked() {
                    state.show_payment_request_form = true;
                }
            } else {
                ui.label(egui::RichText::new("New Request").size(14.0).strong());
            }

            if state.show_payment_request_form {
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
                        ui.label(egui::RichText::new("Receive To:").strong());
                        // Clamp index in case addresses changed
                        if state.pr_from_address_idx >= state.addresses.len() {
                            state.pr_from_address_idx = 0;
                        }
                        let selected_label = state.addresses.get(state.pr_from_address_idx)
                            .map(|a| {
                                let label = if a.label.is_empty() { "Unlabeled" } else { &a.label };
                                format!("{} ({}…{})", label,
                                    &a.address[..8.min(a.address.len())],
                                    &a.address[a.address.len().saturating_sub(6)..])
                            })
                            .unwrap_or_else(|| "No addresses".to_string());
                        egui::ComboBox::from_id_salt("pr_from_addr")
                            .selected_text(selected_label)
                            .width(350.0)
                            .show_ui(ui, |ui| {
                                for (i, addr_info) in state.addresses.iter().enumerate() {
                                    let label = if addr_info.label.is_empty() { "Unlabeled" } else { &addr_info.label };
                                    let display = format!("{} — {}…{}",
                                        label,
                                        &addr_info.address[..8.min(addr_info.address.len())],
                                        &addr_info.address[addr_info.address.len().saturating_sub(6)..]);
                                    ui.selectable_value(&mut state.pr_from_address_idx, i, display);
                                }
                            });
                        ui.end_row();

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
                        .add_enabled(can_send, egui::Button::new("Send Request"))
                        .clicked()
                    {
                        if let Ok(amount_f64) = state.pr_amount.parse::<f64>() {
                            let amount = (amount_f64 * 100_000.0) as u64;
                            let from_address = state.addresses
                                .get(state.pr_from_address_idx)
                                .map(|a| a.address.clone())
                                .unwrap_or_default();
                            let _ = ui_tx.send(UiEvent::SendPaymentRequest {
                                from_address,
                                from_address_idx: state.pr_from_address_idx,
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
                    if ui.button("Clear").clicked() {
                        state.pr_address.clear();
                        state.pr_amount.clear();
                        state.pr_label.clear();
                        state.pr_memo.clear();
                    }
                });
            }

            // ── Incoming Requests ─────────────────────────────────────────
            let active_incoming: Vec<_> = state
                .payment_requests
                .iter()
                .filter(|r| r.expires > now_ts)
                .collect();

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            ui.label(
                egui::RichText::new(format!(
                    "📨 Incoming  ({})",
                    active_incoming.len()
                ))
                .size(14.0)
                .strong(),
            );
            ui.add_space(6.0);

            if active_incoming.is_empty() {
                ui.label(
                    egui::RichText::new("No incoming payment requests.")
                        .color(egui::Color32::GRAY)
                        .italics()
                        .size(12.0),
                );
            } else {
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

                for req in &active_incoming {
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
                                    ui.label(
                                        egui::RichText::new(&req.label).strong().size(14.0),
                                    );
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

                                    ui.label(
                                        egui::RichText::new("Amount:").weak().size(12.0),
                                    );
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
                                    ui.label(
                                        egui::RichText::new("Memo:").weak().size(12.0),
                                    );
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
                                    .on_disabled_hover_text(
                                        "Not enough funds to pay this request",
                                    )
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
                        state.pending_payment_request_id = Some(id.clone());
                        state.screen = Screen::Send;
                    }
                    state.pr_memo_overrides.remove(&id);
                }
                if let Some(id) = decline_id {
                    state.payment_requests.retain(|r| r.id != id);
                    state.pr_memo_overrides.remove(&id);
                    let _ = ui_tx.send(UiEvent::DeclineRequest { request_id: id });
                }
            }

            // ── Sent Requests ─────────────────────────────────────────────
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            let pending_sent_count = state
                .sent_payment_requests
                .iter()
                .filter(|r| r.status == "pending" && r.expires > now_ts)
                .count();

            ui.label(
                egui::RichText::new(format!(
                    "📤 Sent  ({}{}) ",
                    state.sent_payment_requests.len(),
                    if pending_sent_count > 0 {
                        format!(" · {} pending", pending_sent_count)
                    } else {
                        String::new()
                    }
                ))
                .size(14.0)
                .strong(),
            );
            ui.add_space(6.0);

            if state.sent_payment_requests.is_empty() {
                ui.label(
                    egui::RichText::new("No sent payment requests.")
                        .color(egui::Color32::GRAY)
                        .italics()
                        .size(12.0),
                );
            } else {
                let mut cancel_id: Option<String> = None;

                let mut sorted: Vec<_> = state.sent_payment_requests.iter().collect();
                sorted.sort_by(|a, b| {
                    let a_pending = a.status == "pending";
                    let b_pending = b.status == "pending";
                    b_pending
                        .cmp(&a_pending)
                        .then(b.created_at.cmp(&a.created_at))
                });

                for req in &sorted {
                    let is_pending = req.status == "pending" && req.expires > now_ts;

                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::same(10))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());

                            ui.horizontal(|ui| {
                                if !req.label.is_empty() {
                                    ui.label(
                                        egui::RichText::new(&req.label).strong().size(13.0),
                                    );
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(status_label(&req.status))
                                                .size(11.0)
                                                .strong()
                                                .color(status_color(&req.status)),
                                        );
                                    },
                                );
                            });

                            ui.add_space(4.0);

                            egui::Grid::new(format!("sent_pr_grid_{}", req.id))
                                .num_columns(2)
                                .spacing([8.0, 4.0])
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("To:").weak().size(12.0));
                                    ui.label(
                                        egui::RichText::new(super::truncate_middle(
                                            &req.to_address,
                                            14,
                                            6,
                                        ))
                                        .monospace()
                                        .size(12.0),
                                    )
                                    .on_hover_text(&req.to_address);
                                    ui.end_row();

                                    ui.label(
                                        egui::RichText::new("Amount:").weak().size(12.0),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} TIME",
                                            req.amount as f64 / 100_000.0,
                                        ))
                                        .strong()
                                        .size(13.0),
                                    );
                                    ui.end_row();

                                    if !req.memo.is_empty() {
                                        ui.label(
                                            egui::RichText::new("Memo:").weak().size(12.0),
                                        );
                                        ui.label(
                                            egui::RichText::new(&req.memo)
                                                .size(12.0)
                                                .italics()
                                                .color(egui::Color32::GRAY),
                                        );
                                        ui.end_row();
                                    }

                                    ui.label(egui::RichText::new("Sent:").weak().size(12.0));
                                    ui.label(
                                        egui::RichText::new(time_ago(req.created_at))
                                            .size(11.0)
                                            .color(egui::Color32::GRAY),
                                    );
                                    ui.end_row();
                                });

                            if is_pending {
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    if ui
                                        .button(
                                            egui::RichText::new("✕ Cancel")
                                                .color(egui::Color32::from_rgb(200, 60, 60))
                                                .size(12.0),
                                        )
                                        .on_hover_text("Withdraw this payment request")
                                        .clicked()
                                    {
                                        cancel_id = Some(req.id.clone());
                                    }
                                });
                            }
                        });
                    ui.add_space(4.0);
                }

                if let Some(id) = cancel_id {
                    if let Some(req) =
                        state.sent_payment_requests.iter_mut().find(|r| r.id == id)
                    {
                        req.status = "cancelled".to_string();
                    }
                    let _ =
                        ui_tx.send(UiEvent::CancelPaymentRequest { request_id: id });
                }
            }

            ui.add_space(10.0);
        });
}
