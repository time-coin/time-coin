//! Tools screen — maintenance and diagnostic utilities.

use crate::config_new::Config;
use crate::events::UiEvent;
use crate::state::AppState;
use tokio::sync::mpsc;

/// Ensure `path` exists (writing a template if new), then open it with the OS default app.
/// Runs entirely on a background thread — never touches the service loop.
fn open_conf_file(path: std::path::PathBuf) {
    std::thread::spawn(move || {
        if !path.exists() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let template = crate::service::config_file_template(&path);
            if let Err(e) = std::fs::write(&path, template) {
                log::error!("Failed to create {}: {}", path.display(), e);
                return;
            }
            log::info!("Created {}", path.display());
        }
        if let Err(e) = open::that(&path) {
            log::error!("Failed to open {}: {}", path.display(), e);
        }
    });
}

pub fn show(ui: &mut egui::Ui, state: &mut AppState, ui_tx: &mpsc::UnboundedSender<UiEvent>) {
    ui.heading("🔧 Tools");
    ui.add_space(10.0);

    // -- Resync Wallet --
    ui.group(|ui| {
        ui.label(egui::RichText::new("Resync Wallet").strong().size(16.0));
        ui.add_space(4.0);
        ui.label("Clears cached transactions and UTXOs, then re-downloads everything from the masternode. Use this if your balance looks wrong or transactions are missing.");
        ui.add_space(6.0);

        if state.resync_in_progress {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Resyncing…");
            });
        } else if ui
            .add(egui::Button::new("🔄 Resync Now").min_size(egui::vec2(120.0, 28.0)))
            .clicked()
        {
            state.resync_in_progress = true;
            state.error = None;
            state.success = None;
            let _ = ui_tx.send(UiEvent::ResyncWallet);
        }
    });

    ui.add_space(16.0);

    // -- Repair Database --
    ui.group(|ui| {
        ui.label(egui::RichText::new("Repair Database").strong().size(16.0));
        ui.add_space(4.0);
        ui.label("If the wallet database is corrupted (e.g. from an improper shutdown), this will back up the damaged database and create a fresh one. Transactions, UTXOs, and balances are re-fetched from the masternodes. Contacts and masternode configurations will need to be re-entered.");
        ui.add_space(6.0);

        if state.repair_in_progress {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Repairing…");
            });
        } else if ui
            .add(egui::Button::new("🛠 Repair Database").min_size(egui::vec2(160.0, 28.0)))
            .clicked()
        {
            state.repair_in_progress = true;
            state.error = None;
            state.success = None;
            let _ = ui_tx.send(UiEvent::RepairDatabase);
        }
    });

    ui.add_space(16.0);

    // -- Consolidate UTXOs --
    ui.group(|ui| {
        ui.label(egui::RichText::new("Consolidate UTXOs").strong().size(16.0));
        ui.add_space(4.0);
        ui.label(
            format!(
                "Combines many small UTXOs into fewer large ones, making future transactions faster and smaller. You currently have {} UTXOs.",
                state.utxos.len()
            ),
        );
        ui.add_space(6.0);

        if state.consolidation_in_progress {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(&state.consolidation_status);
            });
        } else if ui
            .add_enabled(
                state.utxos.len() > 1 && !state.syncing,
                egui::Button::new("🔗 Consolidate Now").min_size(egui::vec2(160.0, 28.0)),
            )
            .clicked()
        {
            state.consolidation_in_progress = true;
            state.consolidation_status = "Starting consolidation...".to_string();
            state.error = None;
            state.success = None;
            let _ = ui_tx.send(UiEvent::ConsolidateUtxos);
        }
    });

    ui.add_space(16.0);

    // -- Open Config Files --
    ui.group(|ui| {
        ui.label(
            egui::RichText::new("Configuration Files")
                .strong()
                .size(16.0),
        );
        ui.add_space(4.0);
        ui.label("Open configuration files in your system text editor.");
        ui.add_space(6.0);

        if let Ok(data_dir) = Config::data_dir() {
            let config_path = data_dir.join("time.conf");

            ui.horizontal(|ui| {
                let btn = ui
                    .add(egui::Button::new("📝 Open time.conf").min_size(egui::vec2(160.0, 28.0)));
                if btn.clicked() {
                    open_conf_file(config_path.clone());
                }
                ui.label(
                    egui::RichText::new(config_path.display().to_string())
                        .weak()
                        .small(),
                );
            });
        } else {
            ui.label(egui::RichText::new("Could not determine data directory.").weak());
        }
    });

    ui.add_space(16.0);

    // -- Status messages --
    if let Some(ref msg) = state.success {
        ui.label(egui::RichText::new(format!("✅ {msg}")).color(egui::Color32::GREEN));
    }
    if let Some(ref msg) = state.error {
        ui.label(egui::RichText::new(format!("❌ {msg}")).color(egui::Color32::RED));
    }
}
