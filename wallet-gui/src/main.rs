#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;

mod app;
#[allow(dead_code)]
mod config_new;
#[allow(dead_code)]
mod encryption;
#[allow(dead_code)]
mod events;
#[allow(dead_code)]
mod masternode_client;
mod memo;
mod peer_discovery;
mod qr_scanner;
mod service;
mod single_instance;
#[allow(dead_code)]
mod state;
mod theme;
mod view;
#[allow(dead_code)]
mod wallet_dat;
#[allow(dead_code)]
mod wallet_db;
#[allow(dead_code)]
mod wallet_manager;
#[allow(dead_code)]
mod ws_client;

fn main() -> Result<(), eframe::Error> {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = rt.enter();

    env_logger::init();

    // Install the ring-based TLS crypto provider (required by rustls for wss://).
    let _ = rustls::crypto::ring::default_provider().install_default();

    let config = config_new::Config::load().unwrap_or_default();

    // Acquire a per-network advisory lock before opening any databases.
    // If another instance is already running with the same network, show a
    // native error dialog and exit cleanly instead of corrupting the database.
    let wallet_dir = config.wallet_dir();
    let _lock = match single_instance::acquire(&wallet_dir) {
        Ok(lock) => lock,
        Err(msg) => {
            rfd::MessageDialog::new()
                .set_title("TIME Coin Wallet — Already Running")
                .set_description(&msg)
                .set_level(rfd::MessageLevel::Error)
                .show();
            return Ok(());
        }
    };

    let icon = load_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([800.0, 600.0])
            .with_icon(icon),
        ..Default::default()
    };

    let result = eframe::run_native(
        &format!("TIME Coin Wallet v{}", env!("CARGO_PKG_VERSION")),
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, config)))),
    );

    drop(_guard);
    rt.shutdown_timeout(std::time::Duration::from_secs(2));

    result
}

/// Load the logo PNG as an eframe window icon.
fn load_icon() -> egui::IconData {
    let png_data = include_bytes!("../assets/logo.png");
    let image = image::load_from_memory(png_data)
        .unwrap_or_else(|_| image::DynamicImage::new_rgba8(32, 32));
    let rgba = image.to_rgba8();
    let (w, h) = rgba.dimensions();
    egui::IconData {
        rgba: rgba.into_raw(),
        width: w,
        height: h,
    }
}
