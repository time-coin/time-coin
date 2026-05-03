//! Transactions screen — transaction history list with detail view.

use egui::Ui;
use egui_phosphor::regular as ph;
use tokio::sync::mpsc;

use crate::events::{Screen, UiEvent};
use crate::masternode_client::TransactionStatus;
use crate::state::AppState;
use crate::theme;
use crate::wallet_db::masternode_tier_from_satoshis;

/// Render the transactions screen.
pub fn show(ui: &mut Ui, state: &mut AppState, ui_tx: &mpsc::UnboundedSender<UiEvent>) {
    // If a transaction is selected and still exists in the list, show its detail view.
    if let Some(ref key) = state.selected_transaction.clone() {
        let exists = state
            .transactions
            .iter()
            .any(|t| t.txid == key.0 && t.is_send == key.1 && t.is_fee == key.2 && t.vout == key.3);
        if exists {
            show_detail(ui, state, ui_tx);
            return;
        } else {
            state.selected_transaction = None;
        }
    }

    show_list(ui, state, ui_tx);
}

/// Detail view for a single transaction.
fn show_detail(ui: &mut Ui, state: &mut AppState, ui_tx: &mpsc::UnboundedSender<UiEvent>) {
    // Look up by stable key (txid, is_send, is_fee, vout) so the view doesn't shift
    // when new transactions are inserted at the front of the list.
    let key = state.selected_transaction.clone();
    let tx = key.as_ref().and_then(|(txid, is_send, is_fee, vout)| {
        state
            .transactions
            .iter()
            .find(|t| {
                t.txid == *txid && t.is_send == *is_send && t.is_fee == *is_fee && t.vout == *vout
            })
            .cloned()
    });
    let tx = match tx {
        Some(t) => t,
        None => {
            state.selected_transaction = None;
            return;
        }
    };

    ui.horizontal(|ui| {
        if ui.button("Back").clicked() {
            state.selected_transaction = None;
        }
        ui.heading("Transaction Details");
    });

    ui.separator();
    ui.add_space(10.0);

    // Direction and amount
    let (dir_icon, dir_label, amount_color) = if tx.is_fee {
        (ph::RECEIPT, "Fee", egui::Color32::from_rgb(255, 165, 0))
    } else if tx.is_send {
        (
            ph::ARROW_UP_RIGHT,
            "Sent",
            egui::Color32::from_rgb(255, 80, 80),
        )
    } else {
        (
            ph::ARROW_DOWN_LEFT,
            "Received",
            egui::Color32::from_rgb(80, 200, 80),
        )
    };

    let is_neg = tx.is_send || tx.is_fee;
    ui.label(
        egui::RichText::new(format!(
            "{} {} {}",
            dir_icon,
            dir_label,
            state.format_time_signed(tx.amount, is_neg)
        ))
        .size(24.0)
        .strong()
        .color(amount_color),
    );

    ui.add_space(15.0);

    egui::Grid::new("tx_detail_grid")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            // Amount
            ui.label(egui::RichText::new("Amount:").strong());
            ui.vertical(|ui| {
                let is_neg = tx.is_send || tx.is_fee;
                ui.label(
                    egui::RichText::new(state.format_time_signed(tx.amount, is_neg))
                        .strong()
                        .color(amount_color),
                );
                let sats_fmt = {
                    let s = tx.amount.to_string();
                    let mut out = String::new();
                    for (i, ch) in s.chars().rev().enumerate() {
                        if i > 0 && i % 3 == 0 {
                            out.push(',');
                        }
                        out.push(ch);
                    }
                    out.chars().rev().collect::<String>()
                };
                ui.label(
                    egui::RichText::new(format!("{} satoshis", sats_fmt))
                        .size(11.0)
                        .color(ui.visuals().weak_text_color()),
                );
            });
            ui.end_row();

            // Status
            ui.label(egui::RichText::new("Status:").strong());
            let (status_text, status_color) = match tx.status {
                TransactionStatus::Approved => ("✅ Approved", egui::Color32::GREEN),
                TransactionStatus::Pending => ("⏳ Pending", egui::Color32::from_rgb(255, 165, 0)),
                TransactionStatus::Declined => ("❌ Declined", egui::Color32::RED),
            };
            ui.label(egui::RichText::new(status_text).color(status_color));
            ui.end_row();

            // Memo (shown prominently, near the top)
            if let Some(ref memo) = tx.memo {
                ui.label(egui::RichText::new("Memo:").strong());
                ui.label(egui::RichText::new(memo).italics());
                ui.end_row();
            }

            // Transaction ID
            ui.label(egui::RichText::new("Transaction ID:").strong());
            {
                let copied = state
                    .copy_feedback
                    .as_ref()
                    .filter(|(k, t)| k == "txid" && t.elapsed().as_secs() < 2)
                    .is_some();
                let label_text = if copied { "Copied!" } else { &tx.txid };
                let label_color = if copied {
                    egui::Color32::GREEN
                } else {
                    ui.visuals().text_color()
                };
                if ui
                    .add(
                        egui::Label::new(
                            egui::RichText::new(label_text)
                                .monospace()
                                .color(label_color),
                        )
                        .sense(egui::Sense::click()),
                    )
                    .on_hover_text("Click to copy")
                    .clicked()
                {
                    ui.ctx().copy_text(tx.txid.clone());
                    state.copy_feedback = Some(("txid".to_string(), std::time::Instant::now()));
                }
            }
            ui.end_row();

            // Vout
            ui.label(egui::RichText::new("Vout:").strong());
            ui.label(format!("{}", tx.vout));
            ui.end_row();

            // Address
            let addr_label = if tx.is_send { "To:" } else { "From:" };
            ui.label(egui::RichText::new(addr_label).strong());
            ui.vertical(|ui| {
                // Show contact name if known
                if let Some(name) = state.contact_name(&tx.address) {
                    ui.label(egui::RichText::new(name).strong());
                }
                if ui
                    .add(
                        egui::Label::new(egui::RichText::new(&tx.address).monospace())
                            .sense(egui::Sense::click()),
                    )
                    .on_hover_text("Click to copy")
                    .clicked()
                {
                    ui.ctx().copy_text(tx.address.clone());
                }
            });
            ui.end_row();

            // Fee
            if tx.fee > 0 {
                ui.label(egui::RichText::new("Fee:").strong());
                ui.label(state.format_time(tx.fee));
                ui.end_row();
            }

            // Date
            if tx.timestamp > 0 {
                ui.label(egui::RichText::new("Date:").strong());
                if let Some(dt) = chrono::DateTime::from_timestamp(tx.timestamp, 0) {
                    let local: chrono::DateTime<chrono::Local> = dt.into();
                    ui.label(local.format("%Y-%m-%d %H:%M:%S").to_string());
                }
                ui.end_row();
            }

            // Block Height
            if tx.block_height > 0 {
                ui.label(egui::RichText::new("Block Height:").strong());
                ui.label(format!("{}", tx.block_height));
                ui.end_row();
            }

            // Confirmations
            if tx.confirmations > 0 {
                ui.label(egui::RichText::new("Confirmations:").strong());
                ui.label(format!("{}", tx.confirmations));
                ui.end_row();
            }

            // Block Hash
            if !tx.block_hash.is_empty() {
                ui.label(egui::RichText::new("Block Hash:").strong());
                if ui
                    .add(
                        egui::Label::new(egui::RichText::new(&tx.block_hash).monospace())
                            .sense(egui::Sense::click()),
                    )
                    .on_hover_text("Click to copy")
                    .clicked()
                {
                    ui.ctx().copy_text(tx.block_hash.clone());
                }
                ui.end_row();
            }
        });

    // Block reward tier breakdown
    let is_block_reward = tx.memo.as_deref() == Some("Block Reward") && tx.block_height > 0;
    if is_block_reward {
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("Block Reward Breakdown")
                .strong()
                .size(14.0),
        );
        ui.add_space(4.0);

        let breakdown_matches = state
            .block_reward_breakdown
            .as_ref()
            .map(|b| b.block_height == tx.block_height)
            .unwrap_or(false);

        if !breakdown_matches && !state.block_reward_breakdown_loading {
            state.block_reward_breakdown_loading = true;
            state.block_reward_breakdown = None;
            state.block_reward_breakdown_error = None;
            let _ = ui_tx.send(UiEvent::FetchBlockRewardBreakdown {
                height: tx.block_height,
            });
        }

        if state.block_reward_breakdown_loading && !breakdown_matches {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading reward breakdown…");
            });
        } else if let Some(ref err) = state.block_reward_breakdown_error {
            ui.colored_label(egui::Color32::RED, err);
        } else if let Some(ref bd) = state.block_reward_breakdown {
            // Filter to addresses owned by this wallet.
            let my_addresses: std::collections::HashSet<&str> =
                state.addresses.iter().map(|a| a.address.as_str()).collect();
            let my_rewards: Vec<(&str, u64)> = bd
                .rewards
                .iter()
                .filter(|(addr, _)| my_addresses.contains(addr.as_str()))
                .map(|(addr, amt)| (addr.as_str(), *amt))
                .collect();

            if my_rewards.is_empty() {
                ui.label(
                    egui::RichText::new(
                        "None of your masternodes received a reward in this block.",
                    )
                    .color(egui::Color32::GRAY)
                    .italics()
                    .size(12.0),
                );
            } else {
                let tier_color = |tier: &str| match tier {
                    "Gold" => egui::Color32::from_rgb(255, 200, 0),
                    "Silver" => egui::Color32::from_rgb(180, 180, 180),
                    "Bronze" => egui::Color32::from_rgb(180, 100, 40),
                    _ => egui::Color32::from_rgb(100, 180, 255),
                };

                egui::Grid::new("my_reward_grid")
                    .num_columns(3)
                    .spacing([16.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Masternode").strong().size(12.0));
                        ui.label(egui::RichText::new("Tier").strong().size(12.0));
                        ui.label(egui::RichText::new("Reward").strong().size(12.0));
                        ui.end_row();

                        let mut grand_total: u64 = 0;
                        for (addr, amount) in &my_rewards {
                            // Match entry by payout_address first; fall back to
                            // matching by collateral UTXO address for entries
                            // imported from masternode.conf (no payout_address set).
                            let entry = state
                                .masternode_entries
                                .iter()
                                .find(|e| e.payout_address.as_deref() == Some(addr))
                                .or_else(|| {
                                    state.masternode_entries.iter().find(|e| {
                                        state.utxos.iter().any(|u| {
                                            u.txid == e.collateral_txid
                                                && u.vout == e.collateral_vout
                                                && u.address.as_str() == *addr
                                        })
                                    })
                                });
                            let alias = entry.map(|e| e.alias.as_str()).unwrap_or(*addr);
                            // Resolve tier: prefer live UTXO lookup (same logic as
                            // the masternodes screen) then fall back to cached amount.
                            let tier = entry
                                .and_then(|e| {
                                    let live = state
                                        .utxos
                                        .iter()
                                        .find(|u| {
                                            u.txid == e.collateral_txid
                                                && u.vout == e.collateral_vout
                                        })
                                        .map(|u| u.amount);
                                    live.or(e.collateral_amount)
                                })
                                .and_then(crate::wallet_db::masternode_tier_from_satoshis)
                                .unwrap_or("Free"); // no collateral found → Free node
                            ui.label(egui::RichText::new(alias).size(12.0).strong());
                            ui.label(egui::RichText::new(tier).size(12.0).color(tier_color(tier)));
                            ui.label(
                                egui::RichText::new(format!(
                                    "{:.8} TIME",
                                    *amount as f64 / 100_000_000.0
                                ))
                                .size(12.0),
                            );
                            ui.end_row();
                            grand_total += amount;
                        }

                        if my_rewards.len() > 1 {
                            ui.label(egui::RichText::new("Total").strong().size(12.0));
                            ui.label(egui::RichText::new("").size(12.0));
                            ui.label(
                                egui::RichText::new(format!(
                                    "{:.8} TIME",
                                    grand_total as f64 / 100_000_000.0
                                ))
                                .strong()
                                .size(12.0),
                            );
                            ui.end_row();
                        }
                    });
            }
        }
        ui.add_space(6.0);
    }

    // Show "Use as Masternode Collateral" for confirmed received transactions
    // whose amount matches a valid collateral tier (1k, 10k, or 100k TIME).
    if !tx.is_send
        && !tx.is_fee
        && matches!(tx.status, TransactionStatus::Approved)
        && masternode_tier_from_satoshis(tx.amount).is_some()
    {
        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);
        if ui
            .button("Use as Masternode Collateral")
            .on_hover_text("Pre-fill the Masternodes add form with this TXID and vout")
            .clicked()
        {
            state.mn_add_txid = tx.txid.clone();
            state.mn_add_vout = tx.vout.to_string();
            // Auto-generate next available name: mn1, mn2, ...
            let existing: std::collections::HashSet<&str> = state
                .masternode_entries
                .iter()
                .map(|e| e.alias.as_str())
                .collect();
            let mut n = 1u32;
            loop {
                let candidate = format!("mn{}", n);
                if !existing.contains(candidate.as_str()) {
                    state.mn_add_name = candidate;
                    break;
                }
                n += 1;
            }
            state.mn_show_add_form = true;
            state.selected_transaction = None;
            state.screen = Screen::Masternodes;
        }
    }
}

/// List view of all transactions.
fn show_list(ui: &mut Ui, state: &mut AppState, ui_tx: &mpsc::UnboundedSender<UiEvent>) {
    ui.horizontal(|ui| {
        ui.heading("Transactions");
        ui.add_space(10.0);

        let search_response = ui.add(
            egui::TextEdit::singleline(&mut state.tx_search)
                .hint_text("Search by address, txid, amount…")
                .desired_width(250.0),
        );
        if search_response.changed() {
            state.tx_page = 0;
        }
        search_response.context_menu(|ui| {
            if ui.button("Paste").clicked() {
                if let Ok(mut cb) = arboard::Clipboard::new() {
                    if let Ok(text) = cb.get_text() {
                        state.tx_search = text;
                        state.tx_page = 0;
                    }
                }
                ui.close_menu();
            }
        });

        if !state.tx_search.is_empty() && ui.button("Clear").clicked() {
            state.tx_search.clear();
            state.tx_page = 0;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Refresh").clicked() {
                let _ = ui_tx.send(UiEvent::RefreshTransactions);
            }
        });
    });

    ui.separator();
    ui.add_space(5.0);

    if state.transactions.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(
                egui::RichText::new("No transactions yet")
                    .size(16.0)
                    .color(egui::Color32::GRAY)
                    .italics(),
            );
            ui.add_space(10.0);
            ui.label("Transactions will appear here once you send or receive TIME.");
        });
        return;
    }

    // Filter transactions by search query
    let search = state.tx_search.to_lowercase();
    let filtered: Vec<usize> = state
        .transactions
        .iter()
        .enumerate()
        .filter(|(_, tx)| {
            if search.is_empty() {
                return true;
            }
            tx.address.to_lowercase().contains(&search)
                || tx.txid.to_lowercase().contains(&search)
                || state
                    .format_time(tx.amount)
                    .to_lowercase()
                    .contains(&search)
                || state
                    .contact_name(&tx.address)
                    .map(|n| n.to_lowercase().contains(&search))
                    .unwrap_or(false)
        })
        .map(|(i, _)| i)
        .collect();

    // Pagination
    const PAGE_SIZE: usize = 100;
    let total_pages = if filtered.is_empty() {
        1
    } else {
        filtered.len().div_ceil(PAGE_SIZE)
    };
    if state.tx_page >= total_pages {
        state.tx_page = total_pages.saturating_sub(1);
    }
    let page_start = state.tx_page * PAGE_SIZE;
    let page_end = (page_start + PAGE_SIZE).min(filtered.len());
    let page_items = &filtered[page_start..page_end];

    ui.horizontal(|ui| {
        ui.label(format!("{} transactions", filtered.len()));
        if total_pages > 1 {
            ui.add_space(15.0);
            if ui
                .add_enabled(state.tx_page > 0, egui::Button::new("◀ Prev"))
                .clicked()
            {
                state.tx_page = state.tx_page.saturating_sub(1);
            }
            ui.label(format!("Page {} of {}", state.tx_page + 1, total_pages));
            if ui
                .add_enabled(state.tx_page < total_pages - 1, egui::Button::new("Next ▶"))
                .clicked()
            {
                state.tx_page += 1;
            }
        }
    });
    ui.add_space(5.0);

    let mut clicked_idx: Option<usize> = None;
    egui::ScrollArea::vertical()
        .id_salt("tx_list_scroll")
        .auto_shrink([false, false])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        .show(ui, |ui| {
            egui::Grid::new("tx_table")
                .num_columns(7)
                .spacing([10.0, 6.0])
                .min_col_width(0.0)
                .striped(true)
                .show(ui, |ui| {
                    // Header: icon | Date | Amount | Address | Memo | TxID | Status
                    ui.label(egui::RichText::new("").size(13.0));
                    ui.label(egui::RichText::new("Date").size(13.0).strong());
                    ui.label(egui::RichText::new("Amount").size(13.0).strong());
                    ui.label(egui::RichText::new("Address").size(13.0).strong());
                    ui.label(egui::RichText::new("Memo").size(13.0).strong());
                    ui.label(egui::RichText::new("TxID").size(13.0).strong());
                    ui.label(egui::RichText::new("Status").size(13.0).strong());
                    ui.end_row();

                    for &i in page_items {
                        let tx = &state.transactions[i];

                        // Col 1 — type icon
                        let (dir_icon, dir_hover, amount_color) = if tx.is_fee {
                            (ph::RECEIPT, "Fee", egui::Color32::from_rgb(255, 165, 0))
                        } else if tx.is_send {
                            (
                                ph::ARROW_UP_RIGHT,
                                "Sent",
                                egui::Color32::from_rgb(255, 80, 80),
                            )
                        } else {
                            (
                                ph::ARROW_DOWN_LEFT,
                                "Received",
                                egui::Color32::from_rgb(80, 200, 80),
                            )
                        };
                        if ui
                            .add(
                                egui::Label::new(
                                    egui::RichText::new(dir_icon).size(13.0).color(amount_color),
                                )
                                .sense(egui::Sense::click()),
                            )
                            .on_hover_text(dir_hover)
                            .clicked()
                        {
                            clicked_idx = Some(i);
                        }

                        // Col 2 — Date (short: "Jan 15 14:30")
                        let date_str = if tx.timestamp > 0 {
                            chrono::DateTime::from_timestamp(tx.timestamp, 0)
                                .map(|dt| {
                                    let local: chrono::DateTime<chrono::Local> = dt.into();
                                    local.format("%b %d %H:%M").to_string()
                                })
                                .unwrap_or_default()
                        } else {
                            String::new()
                        };
                        if ui
                            .add(
                                egui::Label::new(
                                    egui::RichText::new(&date_str)
                                        .size(12.0)
                                        .color(ui.visuals().weak_text_color()),
                                )
                                .sense(egui::Sense::click()),
                            )
                            .clicked()
                        {
                            clicked_idx = Some(i);
                        }

                        // Col 3 — Amount
                        let is_neg = tx.is_send || tx.is_fee;
                        if ui
                            .add(
                                egui::Label::new(
                                    egui::RichText::new(
                                        state.format_time_signed(tx.amount, is_neg),
                                    )
                                    .size(13.0)
                                    .strong()
                                    .color(amount_color),
                                )
                                .sense(egui::Sense::click()),
                            )
                            .clicked()
                        {
                            clicked_idx = Some(i);
                        }

                        // Col 4 — Address (truncated, full on hover)
                        let addr_short = if tx.is_fee {
                            tx.address.clone()
                        } else {
                            let short = if tx.address.len() > 14 {
                                format!(
                                    "{}..{}",
                                    &tx.address[..8],
                                    &tx.address[tx.address.len() - 4..]
                                )
                            } else {
                                tx.address.clone()
                            };
                            if let Some(name) = state.contact_name(&tx.address) {
                                format!("{} ({})", name, short)
                            } else {
                                short
                            }
                        };
                        if ui
                            .add(
                                egui::Label::new(
                                    egui::RichText::new(&addr_short)
                                        .size(12.0)
                                        .monospace()
                                        .color(ui.visuals().text_color()),
                                )
                                .sense(egui::Sense::click()),
                            )
                            .on_hover_text(&tx.address)
                            .clicked()
                        {
                            clicked_idx = Some(i);
                        }

                        // Col 5 — Memo (truncated, full on hover)
                        let memo_short = tx
                            .memo
                            .as_deref()
                            .map(|m| {
                                if m.chars().count() > 22 {
                                    format!("{}…", m.chars().take(22).collect::<String>())
                                } else {
                                    m.to_string()
                                }
                            })
                            .unwrap_or_default();
                        let memo_resp = ui.add(
                            egui::Label::new(
                                egui::RichText::new(&memo_short)
                                    .size(12.0)
                                    .italics()
                                    .color(ui.visuals().text_color()),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if let Some(ref full) = tx.memo {
                            memo_resp.on_hover_text(full.as_str()).clicked().then(|| {
                                clicked_idx = Some(i);
                            });
                        } else if memo_resp.clicked() {
                            clicked_idx = Some(i);
                        }

                        // Col 6 — TxID (truncated, full on hover)
                        let short_txid = if tx.txid.len() > 12 {
                            format!("{}..{}", &tx.txid[..6], &tx.txid[tx.txid.len() - 4..])
                        } else {
                            tx.txid.clone()
                        };
                        if ui
                            .add(
                                egui::Label::new(
                                    egui::RichText::new(&short_txid)
                                        .size(12.0)
                                        .monospace()
                                        .color(theme::TEXT_SECONDARY),
                                )
                                .sense(egui::Sense::click()),
                            )
                            .on_hover_text(&tx.txid)
                            .clicked()
                        {
                            clicked_idx = Some(i);
                        }

                        // Col 7 — Status (icon + full word)
                        let (status_icon, status_label, status_color) = match tx.status {
                            TransactionStatus::Approved => ("✅", "Approved", egui::Color32::GREEN),
                            TransactionStatus::Pending => {
                                ("⏳", "Pending", egui::Color32::from_rgb(255, 165, 0))
                            }
                            TransactionStatus::Declined => ("❌", "Declined", egui::Color32::RED),
                        };
                        if ui
                            .add(
                                egui::Label::new(
                                    egui::RichText::new(format!(
                                        "{} {}",
                                        status_icon, status_label
                                    ))
                                    .size(13.0)
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
        });

    if let Some(idx) = clicked_idx {
        let tx = &state.transactions[idx];
        state.selected_transaction = Some((tx.txid.clone(), tx.is_send, tx.is_fee, tx.vout));
        state.block_reward_breakdown = None;
        state.block_reward_breakdown_loading = false;
        state.block_reward_breakdown_error = None;
    }
}
