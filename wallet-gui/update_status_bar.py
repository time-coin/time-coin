with open('src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Find and replace the status bar section
old_status = '''        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Network status
                if let Some(net_mgr_arc) = &self.network_manager {
                    if let Ok(net_mgr) = net_mgr_arc.lock() {
                        ui.label(format!("Peers: {}", net_mgr.peer_count()));
                        ui.separator();
                        
                        // Sync progress bar
                        if net_mgr.is_synced() {
                            ui.label("✓ Synchronized");
                        } else {
                            ui.label(format!("Synchronizing... {:.1}%", net_mgr.sync_progress() * 100.0));
                            ui.add(egui::ProgressBar::new(net_mgr.sync_progress()).show_percentage());
                        }
                    }
                } else {
                    ui.label(format!("Status: {}", self.network_status));
                }
            });
        });'''

new_status = '''        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Network status
                if let Some(net_mgr_arc) = &self.network_manager {
                    if let Ok(net_mgr) = net_mgr_arc.lock() {
                        // Peer count
                        ui.label(format!("🌐 {} peers", net_mgr.peer_count()));
                        ui.separator();
                        
                        // Block height
                        let current_height = net_mgr.current_block_height();
                        let network_height = net_mgr.network_block_height();
                        
                        if network_height > 0 {
                            ui.label(format!("📦 Block: {}/{}", current_height, network_height));
                            ui.separator();
                        } else {
                            ui.label(format!("📦 Block: {}", current_height));
                            ui.separator();
                        }
                        
                        // Sync status with progress bar
                        if net_mgr.is_synced() {
                            ui.label("✓ Synchronized");
                        } else {
                            let progress = net_mgr.sync_progress();
                            ui.label(format!("⏳ Synchronizing... {:.1}%", progress * 100.0));
                            ui.add(
                                egui::ProgressBar::new(progress)
                                    .show_percentage()
                                    .desired_width(150.0)
                            );
                        }
                    }
                } else {
                    ui.label(format!("Status: {}", self.network_status));
                }
            });
        });'''

content = content.replace(old_status, new_status)

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Updated status bar with block height and progress bar!')
