//! Charts — monthly income and transactions-per-second visualizations.

use std::collections::BTreeMap;

use chrono::{Datelike, Local};
use egui::Ui;
use egui_plot::{Bar, BarChart, Line, Plot, PlotPoints};

use crate::masternode_client::TransactionStatus;
use crate::state::{AppState, ChartMode, ChartTab};
use crate::theme;

const SATS_PER_TIME: f64 = 100_000_000.0;

/// Month key: (year, month 1-12).
type MonthKey = (i32, u32);

/// Render the full Charts page.
pub fn show_page(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Charts");
    ui.separator();
    ui.add_space(8.0);

    // Tab bar
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.chart_tab, ChartTab::Income, "Income");
        ui.selectable_value(&mut state.chart_tab, ChartTab::Tps, "Transaction Activity");
    });
    ui.add_space(8.0);

    // Scroll area so content is always reachable regardless of window height
    egui::ScrollArea::vertical()
        .id_salt("charts_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| match state.chart_tab {
            ChartTab::Income => {
                render_income_controls(ui, state);
                ui.add_space(6.0);
                render_income_chart(ui, state);
            }
            ChartTab::Tps => {
                render_tps_chart(ui, state);
            }
        });
}

// ============================================================================
// Income Chart
// ============================================================================

/// Render the income chart control bar.
fn render_income_controls(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label("View:");
        egui::ComboBox::from_id_salt("chart_mode")
            .selected_text(match state.chart_mode {
                ChartMode::Total => "Total",
                ChartMode::ByAddress => "By Address",
                ChartMode::SingleAddress => "Single Address",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.chart_mode, ChartMode::Total, "Total");
                ui.selectable_value(&mut state.chart_mode, ChartMode::ByAddress, "By Address");
                if !state.addresses.is_empty() {
                    ui.selectable_value(
                        &mut state.chart_mode,
                        ChartMode::SingleAddress,
                        "Single Address",
                    );
                }
            });

        if state.chart_mode == ChartMode::SingleAddress && !state.addresses.is_empty() {
            if state.chart_address_idx >= state.addresses.len() {
                state.chart_address_idx = 0;
            }
            let current_label = &state.addresses[state.chart_address_idx].label;
            let current_addr = &state.addresses[state.chart_address_idx].address;
            let display = if current_label.is_empty() {
                truncate_addr(current_addr)
            } else {
                current_label.clone()
            };
            egui::ComboBox::from_id_salt("chart_addr")
                .selected_text(display)
                .show_ui(ui, |ui| {
                    for (i, info) in state.addresses.iter().enumerate() {
                        let label = if info.label.is_empty() {
                            truncate_addr(&info.address)
                        } else {
                            format!("{} ({})", info.label, truncate_addr(&info.address))
                        };
                        ui.selectable_value(&mut state.chart_address_idx, i, label);
                    }
                });
        }

        ui.add_space(10.0);
        ui.label("Range:");
        egui::ComboBox::from_id_salt("chart_months")
            .selected_text(match state.chart_months {
                6 => "6 months",
                12 => "12 months",
                24 => "24 months",
                _ => "All time",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.chart_months, 6, "6 months");
                ui.selectable_value(&mut state.chart_months, 12, "12 months");
                ui.selectable_value(&mut state.chart_months, 24, "24 months");
                ui.selectable_value(&mut state.chart_months, 0, "All time");
            });
    });
}

/// Render the income bar chart.
fn render_income_chart(ui: &mut Ui, state: &AppState) {
    // Any txid that has a send record belongs to an internal movement (consolidation,
    // normal send with change, self-transfer).  Receive outputs on those txids must
    // not be counted as income — they are change or re-consolidation outputs.
    //
    // We seed the set from BOTH state.transactions (RPC-returned records) AND
    // state.send_records (the persisted local DB).  The DB is the authoritative
    // source for consolidation sends: if the masternode's tx_index lookup misses
    // (e.g., after a resync) it may return "receive" instead of "consolidate",
    // so relying only on state.transactions would let those through as income.
    let internal_txids: std::collections::HashSet<&str> = state
        .transactions
        .iter()
        .filter(|tx| tx.is_send || tx.is_consolidation)
        .map(|tx| tx.txid.as_str())
        .chain(state.send_records.keys().map(|s| s.as_str()))
        .collect();

    let approved_txs: Vec<_> = state
        .transactions
        .iter()
        .filter(|tx| {
            !tx.is_send
                && !tx.is_fee
                && !tx.is_consolidation
                && !internal_txids.contains(tx.txid.as_str())
                && matches!(tx.status, TransactionStatus::Approved)
                && tx.timestamp > 0
        })
        .collect();

    if approved_txs.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(
                egui::RichText::new("No income data to display")
                    .color(egui::Color32::GRAY)
                    .italics(),
            );
            ui.add_space(20.0);
        });
        return;
    }

    let now = Local::now();
    let month_keys = build_month_range(state.chart_months, &now);

    match state.chart_mode {
        ChartMode::Total => render_total_chart(ui, state, &approved_txs, &month_keys, None),
        ChartMode::SingleAddress => {
            let addr = state
                .addresses
                .get(state.chart_address_idx)
                .map(|a| a.address.as_str());
            render_total_chart(ui, state, &approved_txs, &month_keys, addr);
        }
        ChartMode::ByAddress => {
            render_by_address_chart(ui, state, &approved_txs, &month_keys);
        }
    }
}

// ============================================================================
// TPS Chart
// ============================================================================

/// Render the transaction activity chart.
///
/// Shows how many of the wallet's transactions occurred in each time bucket.
/// Bucket size scales with data span: hourly for ≤1 day, 6-hourly for ≤1 week,
/// daily for longer periods.
fn render_tps_chart(ui: &mut Ui, state: &AppState) {
    // All approved wallet transactions (sends + receives, no fee line items,
    // no consolidations — those don't represent real economic activity).
    let mut timestamps: Vec<i64> = state
        .transactions
        .iter()
        .filter(|tx| {
            matches!(tx.status, TransactionStatus::Approved)
                && tx.timestamp > 0
                && !tx.is_fee
                && !tx.is_consolidation
        })
        .map(|tx| tx.timestamp)
        .collect();

    if timestamps.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(
                egui::RichText::new("No transaction data to display")
                    .color(egui::Color32::GRAY)
                    .italics(),
            );
            ui.add_space(20.0);
        });
        return;
    }

    timestamps.sort_unstable();

    let now = chrono::Utc::now().timestamp();
    let oldest = timestamps[0];
    let span = now - oldest;

    // Choose bucket size based on data span
    let (bucket_secs, bucket_label) = if span <= 86_400 {
        (3_600i64, "per hour")
    } else if span <= 7 * 86_400 {
        (6 * 3_600, "per 6 hours")
    } else {
        (86_400, "per day")
    };

    // Build buckets: count transactions per bucket
    let first_bucket = oldest - (oldest % bucket_secs);
    let last_bucket = now - (now % bucket_secs);

    let mut counts: BTreeMap<i64, u32> = BTreeMap::new();
    let mut t = first_bucket;
    while t <= last_bucket {
        counts.entry(t).or_insert(0);
        t += bucket_secs;
    }
    for &ts in &timestamps {
        let bucket = ts - (ts % bucket_secs);
        *counts.entry(bucket).or_insert(0) += 1;
    }

    // Points: x = bucket timestamp, y = transaction count
    let points: Vec<[f64; 2]> = counts
        .iter()
        .map(|(&bucket_start, &count)| [bucket_start as f64, count as f64])
        .collect();

    if points.is_empty() {
        return;
    }

    let total_tx: u32 = counts.values().sum();
    let peak: u32 = counts.values().copied().max().unwrap_or(0);

    let line = Line::new(PlotPoints::new(points))
        .color(theme::PRIMARY_LIGHT)
        .name("Transactions");

    Plot::new("tps_chart")
        .height(320.0)
        .allow_drag(false)
        .allow_zoom(false)
        .allow_scroll(false)
        .show_axes([true, true])
        .x_axis_formatter(move |mark, _range| {
            if let Some(dt) = chrono::DateTime::from_timestamp(mark.value as i64, 0) {
                let local: chrono::DateTime<Local> = dt.into();
                if bucket_secs >= 86_400 {
                    local.format("%b %d").to_string()
                } else {
                    local.format("%b %d %H:%M").to_string()
                }
            } else {
                String::new()
            }
        })
        .y_axis_formatter(|mark, _range| {
            if mark.value <= 0.0 {
                String::new()
            } else {
                insert_spaces(&format!("{:.0}", mark.value))
            }
        })
        .label_formatter(move |_name, value| {
            let dt_str = chrono::DateTime::from_timestamp(value.x as i64, 0)
                .map(|dt| {
                    let local: chrono::DateTime<Local> = dt.into();
                    if bucket_secs >= 86_400 {
                        local.format("%b %d %Y").to_string()
                    } else {
                        local.format("%b %d %H:%M").to_string()
                    }
                })
                .unwrap_or_default();
            let count = value.y.round() as u32;
            format!(
                "{}\n{} transaction{}",
                dt_str,
                count,
                if count == 1 { "" } else { "s" }
            )
        })
        .include_y(0.0)
        .legend(egui_plot::Legend::default())
        .show(ui, |plot_ui| {
            plot_ui.line(line);
        });

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "{} | {} total transactions | Peak: {} {}",
                bucket_label, total_tx, peak, bucket_label,
            ))
            .small()
            .weak(),
        );
    });
}

// ============================================================================
// Income chart helpers
// ============================================================================

/// Build the list of month keys to display.
fn build_month_range(months_back: usize, now: &chrono::DateTime<Local>) -> Vec<MonthKey> {
    if months_back == 0 {
        return Vec::new();
    }
    let mut keys = Vec::with_capacity(months_back);
    let mut y = now.year();
    let mut m = now.month();
    for _ in 0..months_back {
        keys.push((y, m));
        if m == 1 {
            m = 12;
            y -= 1;
        } else {
            m -= 1;
        }
    }
    keys.reverse();
    keys
}

/// Aggregate income into month buckets, optionally filtering by address.
fn aggregate_total(
    txs: &[&crate::masternode_client::TransactionRecord],
    address_filter: Option<&str>,
) -> BTreeMap<MonthKey, u64> {
    let mut buckets: BTreeMap<MonthKey, u64> = BTreeMap::new();
    for tx in txs {
        if let Some(filter) = address_filter {
            if tx.address != filter {
                continue;
            }
        }
        if let Some(dt) = chrono::DateTime::from_timestamp(tx.timestamp, 0) {
            let local: chrono::DateTime<Local> = dt.into();
            *buckets.entry((local.year(), local.month())).or_insert(0) += tx.amount;
        }
    }
    buckets
}

/// Aggregate income by (month, address).
fn aggregate_by_address(
    txs: &[&crate::masternode_client::TransactionRecord],
) -> BTreeMap<MonthKey, BTreeMap<String, u64>> {
    let mut buckets: BTreeMap<MonthKey, BTreeMap<String, u64>> = BTreeMap::new();
    for tx in txs {
        if let Some(dt) = chrono::DateTime::from_timestamp(tx.timestamp, 0) {
            let local: chrono::DateTime<Local> = dt.into();
            let key = (local.year(), local.month());
            *buckets
                .entry(key)
                .or_default()
                .entry(tx.address.clone())
                .or_insert(0) += tx.amount;
        }
    }
    buckets
}

/// Render a single-series bar chart (total or single address).
fn render_total_chart(
    ui: &mut Ui,
    state: &AppState,
    txs: &[&crate::masternode_client::TransactionRecord],
    month_keys: &[MonthKey],
    address_filter: Option<&str>,
) {
    let data = aggregate_total(txs, address_filter);
    let keys = if month_keys.is_empty() {
        data.keys().copied().collect::<Vec<_>>()
    } else {
        month_keys.to_vec()
    };

    if keys.is_empty() {
        return;
    }

    let bars: Vec<Bar> = keys
        .iter()
        .enumerate()
        .map(|(i, key)| {
            let amount = data.get(key).copied().unwrap_or(0);
            Bar::new(i as f64, amount as f64 / SATS_PER_TIME).width(0.6)
        })
        .collect();

    let total: u64 = keys.iter().map(|k| data.get(k).copied().unwrap_or(0)).sum();

    let chart = BarChart::new(bars).color(theme::GREEN);
    let labels: Vec<String> = keys.iter().map(|k| month_label(k.0, k.1)).collect();

    show_bar_plot(ui, "income_total", &labels, vec![chart]);

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "Total income ({}): {}",
                range_label(state.chart_months),
                state.format_time(total)
            ))
            .small()
            .weak(),
        );
    });
}

/// Render a multi-series bar chart with one color per address.
fn render_by_address_chart(
    ui: &mut Ui,
    state: &AppState,
    txs: &[&crate::masternode_client::TransactionRecord],
    month_keys: &[MonthKey],
) {
    let data = aggregate_by_address(txs);
    let keys = if month_keys.is_empty() {
        data.keys().copied().collect::<Vec<_>>()
    } else {
        month_keys.to_vec()
    };

    if keys.is_empty() {
        return;
    }

    // Collect all unique addresses that appear in the data
    let mut all_addrs: Vec<String> = Vec::new();
    for month_data in data.values() {
        for addr in month_data.keys() {
            if !all_addrs.contains(addr) {
                all_addrs.push(addr.clone());
            }
        }
    }

    let n = all_addrs.len().max(1) as f64;
    let bar_width = 0.7 / n;

    let mut charts: Vec<BarChart> = Vec::new();
    for (addr_idx, addr) in all_addrs.iter().enumerate() {
        let offset = (addr_idx as f64 - (n - 1.0) / 2.0) * bar_width;
        let color = theme::CHART_PALETTE[addr_idx % theme::CHART_PALETTE.len()];

        let label = state
            .addresses
            .iter()
            .find(|a| a.address == *addr)
            .map(|a| {
                if a.label.is_empty() {
                    truncate_addr(&a.address)
                } else {
                    a.label.clone()
                }
            })
            .unwrap_or_else(|| truncate_addr(addr));

        let bars: Vec<Bar> = keys
            .iter()
            .enumerate()
            .map(|(i, key)| {
                let amount = data
                    .get(key)
                    .and_then(|m| m.get(addr))
                    .copied()
                    .unwrap_or(0);
                Bar::new(i as f64 + offset, amount as f64 / SATS_PER_TIME).width(bar_width * 0.9)
            })
            .collect();

        charts.push(BarChart::new(bars).color(color).name(&label));
    }

    let labels: Vec<String> = keys.iter().map(|k| month_label(k.0, k.1)).collect();
    show_bar_plot(ui, "income_by_addr", &labels, charts);
}

/// Render a bar chart with month labels on the x-axis.
fn show_bar_plot(ui: &mut Ui, id: &str, x_labels: &[String], charts: Vec<BarChart>) {
    let label_vec = x_labels.to_vec();
    let label_vec2 = label_vec.clone();
    let n_bars = x_labels.len();

    Plot::new(id)
        .height(320.0)
        .allow_drag(false)
        .allow_zoom(false)
        .allow_scroll(false)
        .show_axes([true, true])
        .x_axis_formatter(move |mark, _range| {
            let idx = mark.value.round() as usize;
            if idx < label_vec.len() && (mark.value - idx as f64).abs() < 0.01 {
                label_vec[idx].clone()
            } else {
                String::new()
            }
        })
        .y_axis_formatter(|mark, _range| {
            if mark.value <= 0.0 {
                String::new()
            } else {
                insert_spaces(&format!("{:.0}", mark.value))
            }
        })
        .label_formatter(move |series_name, value| {
            let idx = value.x.round() as usize;
            let month = label_vec2.get(idx).map(|s| s.as_str()).unwrap_or("?");
            let amount_str = format_time_spaces(value.y);
            if series_name.is_empty() {
                format!("{}\n{}", month, amount_str)
            } else {
                format!("{} — {}\n{}", series_name, month, amount_str)
            }
        })
        .include_x(-0.5)
        .include_x(n_bars as f64 - 0.5)
        .include_y(0.0)
        .legend(egui_plot::Legend::default())
        .show(ui, |plot_ui| {
            for chart in charts {
                plot_ui.bar_chart(chart);
            }
        });
}

/// Format a TIME amount (f64, already converted from satoshis) with space
/// grouping and 2 decimal places. E.g. 1234567.89 → "1 234 567.89 TIME".
fn format_time_spaces(time: f64) -> String {
    let s = format!("{:.2}", time);
    let (int_part, dec_part) = s.split_once('.').unwrap_or((&s, "00"));
    format!("{}.{} TIME", insert_spaces(int_part), dec_part)
}

/// Insert a space every 3 digits (from the right) into a non-negative integer string.
fn insert_spaces(s: &str) -> String {
    let digits: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, &c) in digits.iter().enumerate() {
        let remaining = digits.len() - i;
        if i > 0 && remaining.is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

fn month_label(year: i32, month: u32) -> String {
    const NAMES: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!("{} '{:02}", NAMES[(month - 1) as usize], year % 100)
}

fn range_label(months: usize) -> &'static str {
    match months {
        6 => "last 6 months",
        12 => "last 12 months",
        24 => "last 24 months",
        _ => "all time",
    }
}

fn truncate_addr(addr: &str) -> String {
    if addr.len() > 14 {
        format!("{}..{}", &addr[..8], &addr[addr.len() - 4..])
    } else {
        addr.to_string()
    }
}
