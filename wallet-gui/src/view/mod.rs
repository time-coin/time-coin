//! UI view modules — pure rendering functions.
//!
//! Each submodule renders one screen. Views read from [`AppState`] and send
//! [`UiEvent`]s on user interaction. No async, no network, no wallet logic.

/// Shorten a string to `prefix` chars + "…" + `suffix` chars.
pub(super) fn truncate_middle(s: &str, prefix: usize, suffix: usize) -> String {
    if s.len() <= prefix + suffix + 1 {
        return s.to_string();
    }
    format!("{}…{}", &s[..prefix], &s[s.len() - suffix..])
}

pub mod connections;
pub mod income_chart;
pub mod masternodes;
pub mod overview;
pub mod payment_requests;
pub mod receive;
pub mod send;
pub mod settings;
pub mod tools;
pub mod transactions;
pub mod welcome;
