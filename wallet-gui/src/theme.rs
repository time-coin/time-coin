//! Visual theme constants matching the TIME mobile app design system.

use egui::Color32;

// ── Brand ──────────────────────────────────────────────────────────────────
pub const PRIMARY: Color32 = Color32::from_rgb(25, 118, 210); // #1976D2
pub const PRIMARY_LIGHT: Color32 = Color32::from_rgb(33, 150, 243); // #2196F3

// ── Status ─────────────────────────────────────────────────────────────────
pub const GREEN: Color32 = Color32::from_rgb(0, 200, 80); // #00C850
pub const ORANGE: Color32 = Color32::from_rgb(255, 165, 0); // #FFA500
#[allow(dead_code)]
pub const RED: Color32 = Color32::from_rgb(255, 59, 48); // #FF3B30

// ── Text ───────────────────────────────────────────────────────────────────
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 160, 170);

// ── Surfaces ───────────────────────────────────────────────────────────────
/// Card/panel background — slightly elevated above the base dark background.
#[allow(dead_code)]
pub const SURFACE_CARD: Color32 = Color32::from_rgb(26, 32, 44);
/// Warning banner background.
#[allow(dead_code)]
pub const SURFACE_WARNING: Color32 = Color32::from_rgb(60, 45, 0);

// ── Egui visuals ───────────────────────────────────────────────────────────
/// Apply the TIME dark theme to an egui context.
pub fn apply(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.selection.bg_fill = PRIMARY;
    v.hyperlink_color = PRIMARY_LIGHT;
    // Slightly warmer/deeper background than egui default
    v.panel_fill = egui::Color32::from_rgb(15, 17, 22);
    v.window_fill = egui::Color32::from_rgb(20, 22, 28);
    v.extreme_bg_color = egui::Color32::from_rgb(10, 11, 14);
    ctx.set_visuals(v);
}
