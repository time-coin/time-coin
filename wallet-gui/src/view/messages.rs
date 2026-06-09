//! Secure messaging view — Signal/Telegram-style chat interface.

use egui::{Color32, RichText, Ui};
use tokio::sync::mpsc;

use crate::events::{Screen, UiEvent};
use crate::state::AppState;
use crate::theme;
use crate::wallet_db::{MessageDirection, StoredMessageStatus};

// ── Colour palette (aligned to theme.rs) ─────────────────────────────────────

/// Sent-bubble — brand blue.
const BUBBLE_OUT: Color32 = theme::PRIMARY;
/// Received-bubble — slightly elevated surface.
const BUBBLE_IN: Color32 = Color32::from_rgb(38, 45, 58);
const BUBBLE_TEXT: Color32 = Color32::WHITE;
/// Left-panel background — one step above the app's panel_fill.
const SIDE_BG: Color32 = Color32::from_rgb(18, 21, 27);
/// Hovered conversation row — visibly brighter than SIDE_BG.
const ROW_HOVER: Color32 = Color32::from_rgb(45, 58, 78);
/// Selected conversation row — clearly blue, high contrast for white text.
const ROW_SELECTED: Color32 = Color32::from_rgb(28, 80, 160);
/// Section heading text.
const SECTION_LABEL: Color32 = Color32::from_rgb(100, 120, 145);
/// Unread count badge.
const BADGE: Color32 = theme::PRIMARY_LIGHT;

/// Pick an avatar background from the chart palette using the first byte of the name.
fn avatar_color(name: &str) -> Color32 {
    let idx = name.bytes().next().unwrap_or(0) as usize % theme::CHART_PALETTE.len();
    theme::CHART_PALETTE[idx]
}

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

fn status_indicator(status: StoredMessageStatus) -> (&'static str, Color32) {
    match status {
        StoredMessageStatus::Pending => ("⏳", Color32::from_rgb(150, 150, 150)),
        StoredMessageStatus::Delivered => ("✓", Color32::from_rgb(150, 150, 150)),
        StoredMessageStatus::Read => ("✓✓", Color32::from_rgb(100, 200, 255)),
        StoredMessageStatus::Failed | StoredMessageStatus::Expired => {
            ("✗", Color32::from_rgb(220, 60, 60))
        }
        StoredMessageStatus::Unread | StoredMessageStatus::ReadByUs => {
            ("", Color32::TRANSPARENT)
        }
    }
}

fn trunc_addr(addr: &str, prefix: usize, suffix: usize) -> String {
    if addr.len() <= prefix + suffix + 1 {
        return addr.to_string();
    }
    format!("{}…{}", &addr[..prefix], &addr[addr.len() - suffix..])
}

// ── Main entry point ─────────────────────────────────────────────────────────

pub fn show(ui: &mut Ui, state: &mut AppState, ui_tx: &mpsc::UnboundedSender<UiEvent>) {
    // Unique conversation partners, ordered by most-recent message.
    let conversations: Vec<String> = {
        let mut seen = std::collections::HashSet::new();
        let mut list = Vec::new();
        for msg in &state.messages {
            if seen.insert(msg.peer_address.clone()) {
                list.push(msg.peer_address.clone());
            }
        }
        list
    };

    let unread_counts: std::collections::HashMap<String, usize> = conversations
        .iter()
        .map(|addr| {
            let n = state
                .messages
                .iter()
                .filter(|m| {
                    m.peer_address == *addr
                        && m.direction == MessageDirection::Incoming
                        && m.status == StoredMessageStatus::Unread
                })
                .count();
            (addr.clone(), n)
        })
        .collect();

    let total_unread: usize = unread_counts.values().sum();

    // ── Top bar ──────────────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label(RichText::new("💬  Messages").size(20.0).strong());

        if total_unread > 0 {
            ui.add_space(4.0);
            egui::Frame::new()
                .fill(BADGE)
                .corner_radius(9.0)
                .inner_margin(egui::Margin::symmetric(6, 2))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(total_unread.to_string())
                            .size(11.0)
                            .color(Color32::WHITE)
                            .strong(),
                    );
                });
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if state.msg_fetching {
                ui.spinner();
                ui.add_space(4.0);
                ui.label(RichText::new("Syncing…").size(12.0).weak());
            } else if ui
                .add(egui::Button::new(RichText::new("↻  Refresh").size(13.0)))
                .clicked()
            {
                let _ = ui_tx.send(UiEvent::FetchMessages);
            }
        });
    });

    if let Some(ref err) = state.msg_send_error.clone() {
        ui.add_space(4.0);
        egui::Frame::new()
            .fill(Color32::from_rgb(60, 20, 20))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("⚠  {err}")).color(Color32::from_rgb(255, 100, 100)).size(12.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✕").clicked() {
                            state.msg_send_error = None;
                        }
                    });
                });
            });
    }

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(0.0);

    // ── Two-column layout ─────────────────────────────────────────────────────
    egui::SidePanel::left("msg_conv_list")
        .exact_width(260.0)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(SIDE_BG)
                .inner_margin(egui::Margin::same(0)),
        )
        .show_inside(ui, |ui| {
            show_left_panel(ui, state, ui_tx, &conversations, &unread_counts);
        });

    egui::CentralPanel::default().show_inside(ui, |ui| {
        if let Some(ref addr) = state.selected_msg_contact.clone() {
            show_chat_panel(ui, state, ui_tx, addr);
        } else {
            show_empty_state(ui);
        }
    });
}

// ── Left panel: search + recent conversations + contacts ─────────────────────

fn show_left_panel(
    ui: &mut Ui,
    state: &mut AppState,
    ui_tx: &mpsc::UnboundedSender<UiEvent>,
    conversations: &[String],
    unread_counts: &std::collections::HashMap<String, usize>,
) {
    // Search bar
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.add(
            egui::TextEdit::singleline(&mut state.msg_search)
                .hint_text("🔍  Search contacts…")
                .desired_width(ui.available_width() - 16.0),
        );
    });
    ui.add_space(6.0);

    let search = state.msg_search.to_lowercase();

    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── Recent conversations ──────────────────────────────────────────────
        let has_convs = conversations.iter().any(|addr| {
            if search.is_empty() {
                return true;
            }
            let name = contact_name(addr, state);
            name.to_lowercase().contains(&search) || addr.to_lowercase().contains(&search)
        });

        if has_convs {
            section_header(ui, "RECENT");

            for addr in conversations {
                let name = contact_name(addr, state);

                if !search.is_empty()
                    && !name.to_lowercase().contains(&search)
                    && !addr.to_lowercase().contains(&search)
                {
                    continue;
                }

                let last_msg = state.messages.iter().find(|m| m.peer_address == *addr);
                let is_selected = state.selected_msg_contact.as_deref() == Some(addr.as_str());
                let unread = *unread_counts.get(addr).unwrap_or(&0);

                let clicked = conv_row(ui, state, &name, addr, last_msg, is_selected, unread);
                if clicked {
                    state.selected_msg_contact = Some(addr.clone());
                    state.msg_compose_text.clear();
                    let _ = ui_tx.send(UiEvent::MarkMessageRead { msg_id: addr.clone() });
                }
            }

            ui.add_space(8.0);
        }

        // ── Contacts not yet in a conversation ────────────────────────────────
        let conv_set: std::collections::HashSet<&str> =
            conversations.iter().map(|s| s.as_str()).collect();
        let new_contacts: Vec<_> = state
            .contacts
            .iter()
            .filter(|c| {
                !conv_set.contains(c.address.as_str())
                    && (search.is_empty()
                        || c.name.to_lowercase().contains(&search)
                        || c.address.to_lowercase().contains(&search))
            })
            .cloned()
            .collect();

        if !new_contacts.is_empty() {
            section_header(ui, "CONTACTS");

            for contact in new_contacts {
                let is_selected =
                    state.selected_msg_contact.as_deref() == Some(contact.address.as_str());

                let clicked = contact_row(ui, &contact.name, &contact.address, is_selected);
                if clicked {
                    state.selected_msg_contact = Some(contact.address.clone());
                    state.msg_compose_text.clear();
                    if contact.pubkey_hex.is_none() {
                        let _ = ui_tx.send(UiEvent::NavigatedTo(Screen::Messages));
                    }
                }
            }
        }

        if state.contacts.is_empty() && conversations.is_empty() {
            ui.add_space(16.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("No contacts yet.").size(13.0).weak());
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Add contacts in the Send screen\nto start messaging.")
                        .size(12.0)
                        .weak(),
                );
            });
        }
    });
}

fn section_header(ui: &mut Ui, text: &str) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        ui.label(RichText::new(text).size(10.0).color(SECTION_LABEL).strong());
    });
    ui.add_space(2.0);
}

fn contact_name(addr: &str, state: &AppState) -> String {
    state
        .contacts
        .iter()
        .find(|c| c.address == addr)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| trunc_addr(addr, 6, 4))
}

/// Render a recent-conversation row. Returns true if clicked.
fn conv_row(
    ui: &mut Ui,
    _state: &AppState,
    name: &str,
    addr: &str,
    last_msg: Option<&crate::wallet_db::StoredMessage>,
    is_selected: bool,
    unread: usize,
) -> bool {
    let avail_w = ui.available_width();
    let row_h = 54.0;

    // Allocate the full row rect first so we know the rect before drawing anything.
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(avail_w, row_h), egui::Sense::click());

    // Paint background before content — correct draw order.
    let bg = if is_selected {
        ROW_SELECTED
    } else if response.hovered() {
        ROW_HOVER
    } else {
        Color32::TRANSPARENT
    };
    if bg != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, 4.0, bg);
    }

    // Draw content in the inner rect (10px H margin, 6px V margin).
    let inner = rect.shrink2(egui::vec2(10.0, 6.0));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );

    draw_avatar(&mut child, name, 36.0);
    child.add_space(8.0);

    child.vertical(|ui| {
        ui.set_width(ui.available_width());

        // Name + timestamp row
        ui.horizontal(|ui| {
            ui.label(RichText::new(name).size(13.0).strong().color(Color32::WHITE));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(msg) = last_msg {
                    ui.label(
                        RichText::new(time_ago(msg.timestamp))
                            .size(10.0)
                            .color(SECTION_LABEL),
                    );
                }
            });
        });

        // Preview + unread badge row
        ui.horizontal(|ui| {
            let preview_text = if let Some(msg) = last_msg {
                let prefix = if msg.direction == MessageDirection::Outgoing { "You: " } else { "" };
                let body = if msg.body.len() > 38 {
                    format!("{}…", &msg.body[..35])
                } else {
                    msg.body.clone()
                };
                format!("{}{}", prefix, body)
            } else {
                trunc_addr(addr, 8, 5)
            };
            // Preview text colour is lighter on selected rows for legibility.
            let preview_col = if is_selected {
                Color32::from_rgb(180, 200, 225)
            } else {
                Color32::from_rgb(120, 135, 155)
            };
            ui.label(RichText::new(preview_text).size(11.0).color(preview_col));
            if unread > 0 {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::Frame::new()
                        .fill(BADGE)
                        .corner_radius(8.0)
                        .inner_margin(egui::Margin::symmetric(5, 1))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(unread.to_string())
                                    .size(10.0)
                                    .color(Color32::WHITE)
                                    .strong(),
                            );
                        });
                });
            }
        });
    });

    response.clicked()
}

/// Render a contact-only row (no message history). Returns true if clicked.
fn contact_row(ui: &mut Ui, name: &str, addr: &str, is_selected: bool) -> bool {
    let avail_w = ui.available_width();
    let row_h = 44.0;

    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(avail_w, row_h), egui::Sense::click());

    let bg = if is_selected {
        ROW_SELECTED
    } else if response.hovered() {
        ROW_HOVER
    } else {
        Color32::TRANSPARENT
    };
    if bg != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, 4.0, bg);
    }

    let inner = rect.shrink2(egui::vec2(10.0, 5.0));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );

    draw_avatar(&mut child, name, 30.0);
    child.add_space(8.0);
    child.vertical(|ui| {
        ui.label(RichText::new(name).size(13.0).strong().color(Color32::WHITE));
        let addr_col = if is_selected {
            Color32::from_rgb(180, 200, 225)
        } else {
            SECTION_LABEL
        };
        ui.label(RichText::new(trunc_addr(addr, 8, 5)).size(10.0).color(addr_col));
    });

    response.clicked()
}

fn draw_avatar(ui: &mut Ui, name: &str, size: f32) {
    let (_, rect) = ui.allocate_space(egui::vec2(size, size));
    let color = avatar_color(name);
    let radius = size / 2.0;
    // Slightly dimmed version of the palette color for the background
    let bg = Color32::from_rgb(
        (color.r() as u16 * 60 / 100) as u8,
        (color.g() as u16 * 60 / 100) as u8,
        (color.b() as u16 * 60 / 100) as u8,
    );
    ui.painter().circle_filled(rect.center(), radius, bg);
    // Thin coloured ring
    ui.painter().circle_stroke(
        rect.center(),
        radius - 1.0,
        egui::Stroke::new(1.5, color),
    );
    let letter = name
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .next()
        .unwrap_or('?');
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        letter.to_string(),
        egui::FontId::proportional(size * 0.45),
        Color32::WHITE,
    );
}

// ── Empty state (no conversation selected) ────────────────────────────────────

fn show_empty_state(ui: &mut Ui) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.label(RichText::new("💬").size(48.0));
            ui.add_space(12.0);
            ui.label(
                RichText::new("Select a contact to start\na secure conversation")
                    .size(16.0)
                    .color(Color32::from_rgb(100, 115, 135)),
            );
            ui.add_space(12.0);
            ui.label(
                RichText::new("End-to-end encrypted with TIME-MSG v1")
                    .size(12.0)
                    .color(Color32::from_rgb(70, 85, 105)),
            );
        });
    });
}

// ── Chat panel (right side) ───────────────────────────────────────────────────

fn show_chat_panel(
    ui: &mut Ui,
    state: &mut AppState,
    ui_tx: &mpsc::UnboundedSender<UiEvent>,
    peer_addr: &str,
) {
    let contact = state.contacts.iter().find(|c| c.address == peer_addr).cloned();
    let display = contact
        .as_ref()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| trunc_addr(peer_addr, 10, 6));
    let pubkey_known = contact.as_ref().and_then(|c| c.pubkey_hex.as_ref()).is_some();

    // Header
    egui::TopBottomPanel::top("chat_header")
        .frame(
            egui::Frame::new()
                .fill(Color32::from_rgb(18, 22, 30))
                .inner_margin(egui::Margin::symmetric(12, 8))
                .stroke(egui::Stroke::new(1.0, Color32::from_rgb(35, 45, 60))),
        )
        .show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                draw_avatar(ui, &display, 36.0);
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&display)
                            .size(15.0)
                            .strong()
                            .color(Color32::WHITE),
                    );
                    ui.label(
                        RichText::new(trunc_addr(peer_addr, 12, 6))
                            .size(11.0)
                            .color(SECTION_LABEL),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("📋  Copy Address").size(12.0),
                            )
                            .fill(Color32::from_rgb(30, 40, 55)),
                        )
                        .clicked()
                    {
                        ui.ctx().copy_text(peer_addr.to_string());
                    }
                    if !pubkey_known {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("🔑  Request Key").size(12.0).color(Color32::from_rgb(255, 200, 80)),
                                )
                                .fill(Color32::from_rgb(50, 40, 20)),
                            )
                            .on_hover_text("Contact's pubkey unknown — send a key-request envelope so they can register their public key")
                            .clicked()
                        {
                            let _ = ui_tx.send(UiEvent::RequestPubkey {
                                address: peer_addr.to_string(),
                            });
                        }
                    }
                });
            });
        });

    // Compose — declared BEFORE messages so egui allocates bottom space first.
    egui::TopBottomPanel::bottom("chat_compose")
        .frame(
            egui::Frame::new()
                .fill(Color32::from_rgb(18, 22, 30))
                .inner_margin(egui::Margin::symmetric(12, 8))
                .stroke(egui::Stroke::new(1.0, Color32::from_rgb(35, 45, 60))),
        )
        .show_inside(ui, |ui| {
            show_compose(ui, state, ui_tx, peer_addr);
        });

    // Message scroll area
    egui::CentralPanel::default().show_inside(ui, |ui| {
        let msgs: Vec<_> = state
            .messages
            .iter()
            .filter(|m| m.peer_address == peer_addr)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.add_space(8.0);
                let mut prev_ts: i64 = 0;
                for msg in &msgs {
                    if msg.timestamp - prev_ts > 7200 || prev_ts == 0 {
                        let dt = chrono::DateTime::from_timestamp(msg.timestamp, 0)
                            .map(|dt: chrono::DateTime<chrono::Utc>| {
                                dt.format("%B %d, %Y").to_string()
                            })
                            .unwrap_or_default();
                        ui.vertical_centered(|ui| {
                            ui.add_space(8.0);
                            egui::Frame::new()
                                .fill(Color32::from_rgb(28, 35, 48))
                                .corner_radius(10.0)
                                .inner_margin(egui::Margin::symmetric(10, 3))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(dt).size(11.0).color(SECTION_LABEL));
                                });
                            ui.add_space(4.0);
                        });
                    }
                    prev_ts = msg.timestamp;
                    show_message_bubble(ui, msg);
                }
                ui.add_space(8.0);
            });
    });
}

fn show_message_bubble(ui: &mut Ui, msg: &crate::wallet_db::StoredMessage) {
    let is_out = msg.direction == MessageDirection::Outgoing;

    let max_w = ui.available_width() * 0.72;
    let bubble_color = if is_out { BUBBLE_OUT } else { BUBBLE_IN };

    ui.with_layout(
        if is_out {
            egui::Layout::right_to_left(egui::Align::Min)
        } else {
            egui::Layout::left_to_right(egui::Align::Min)
        },
        |ui| {
            ui.set_max_width(max_w);
            egui::Frame::new()
                .fill(bubble_color)
                .corner_radius(egui::CornerRadius {
                    nw: if is_out { 12 } else { 4 },
                    ne: if is_out { 4 } else { 12 },
                    sw: 12,
                    se: 12,
                })
                .inner_margin(egui::Margin::symmetric(12, 7))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        if !msg.subject.is_empty() {
                            ui.label(
                                RichText::new(&msg.subject)
                                    .size(12.0)
                                    .strong()
                                    .color(BUBBLE_TEXT),
                            );
                            ui.add_space(2.0);
                        }
                        ui.label(RichText::new(&msg.body).size(13.0).color(BUBBLE_TEXT));
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            let time_str = chrono::DateTime::from_timestamp(msg.timestamp, 0)
                                .map(|dt: chrono::DateTime<chrono::Utc>| {
                                    dt.format("%H:%M").to_string()
                                })
                                .unwrap_or_default();
                            ui.label(
                                RichText::new(time_str)
                                    .size(10.0)
                                    .color(Color32::from_rgba_premultiplied(200, 210, 220, 160)),
                            );
                            if is_out {
                                let (icon, color) = status_indicator(msg.status);
                                if !icon.is_empty() {
                                    ui.add_space(2.0);
                                    ui.label(RichText::new(icon).size(10.0).color(color));
                                }
                            }
                        });
                    });
                });
        },
    );
    ui.add_space(3.0);
}

// ── Compose area ──────────────────────────────────────────────────────────────

fn show_compose(
    ui: &mut Ui,
    state: &mut AppState,
    ui_tx: &mpsc::UnboundedSender<UiEvent>,
    peer_addr: &str,
) {
    let btn_w = 68.0;
    let btn_h = 54.0;
    let gap = ui.spacing().item_spacing.x;
    // Subtract button + gap from the full available width to get text box width.
    // Use floor to ensure we never exceed available space.
    let text_w = (ui.available_width() - btn_w - gap).floor().max(40.0);

    let can_send = !state.msg_compose_text.trim().is_empty() && !state.msg_fetching;

    ui.horizontal(|ui| {
        let response = ui.add_sized(
            [text_w, btn_h],
            egui::TextEdit::multiline(&mut state.msg_compose_text)
                .hint_text("Write a secure message…")
                .font(egui::TextStyle::Body),
        );

        if response.has_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.ctrl)
        {
            send_message(state, ui_tx, peer_addr);
        }

        let send_btn = egui::Button::new(
            RichText::new("Send\n▶").size(12.0).color(Color32::WHITE),
        )
        .min_size(egui::vec2(btn_w, btn_h))
        .fill(if can_send {
            theme::PRIMARY
        } else {
            Color32::from_rgb(35, 45, 60)
        });

        if ui.add_enabled(can_send, send_btn).clicked() {
            send_message(state, ui_tx, peer_addr);
        }
    });

    ui.add_space(3.0);
    ui.label(
        RichText::new("Ctrl+Enter to send  ·  🔒 End-to-end encrypted")
            .size(10.0)
            .color(Color32::from_rgb(70, 90, 115)),
    );
}

fn send_message(state: &mut AppState, ui_tx: &mpsc::UnboundedSender<UiEvent>, peer_addr: &str) {
    let body = state.msg_compose_text.trim().to_string();
    if body.is_empty() {
        return;
    }
    state.msg_fetching = true;
    state.msg_send_error = None;
    state.msg_compose_text.clear();
    let _ = ui_tx.send(UiEvent::SendMessage {
        to: peer_addr.to_string(),
        subject: String::new(),
        body,
    });
}
