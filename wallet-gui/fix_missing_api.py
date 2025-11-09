with open('src/network.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Update bootstrap to handle missing blockchain API gracefully
old_bootstrap_end = '''        // Fetch blockchain info to get current height
        match self.fetch_blockchain_info().await {
            Ok(info) => {
                self.network_block_height = info.height;
                self.current_block_height = info.height;
                log::info!("Synchronized to block height: {}", info.height);
                self.is_syncing = false;
                self.sync_progress = 1.0;
            }
            Err(e) => {
                log::error!("Failed to fetch blockchain info: {}", e);
                self.is_syncing = true;
                self.sync_progress = 0.0;
                return Err(format!("Failed to sync blockchain: {}", e));
            }
        }

        Ok(())
    }'''

new_bootstrap_end = '''        // Try to fetch blockchain info to get current height
        match self.fetch_blockchain_info().await {
            Ok(info) => {
                self.network_block_height = info.height;
                self.current_block_height = info.height;
                log::info!("Synchronized to block height: {}", info.height);
                self.is_syncing = false;
                self.sync_progress = 1.0;
            }
            Err(e) => {
                log::warn!("Blockchain info API not available: {}", e);
                log::info!("Wallet connected to {} peers (blockchain height unknown)", self.connected_peers.len());
                // Set to connected but height unknown
                self.is_syncing = false;
                self.sync_progress = 1.0;
                self.current_block_height = 0; // Will show as "unknown"
                self.network_block_height = 0;
            }
        }

        Ok(())
    }'''

content = content.replace(old_bootstrap_end, new_bootstrap_end)

with open('src/network.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Updated to handle missing blockchain API gracefully!')

# Now update main.rs to show better status
with open('src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()

old_status_display = '''                        // Block height
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
                        }'''

new_status_display = '''                        // Block height
                        let current_height = net_mgr.current_block_height();
                        let network_height = net_mgr.network_block_height();
                        
                        if network_height > 0 {
                            ui.label(format!("📦 Block: {}/{}", current_height, network_height));
                            ui.separator();
                        } else if current_height > 0 {
                            ui.label(format!("📦 Block: {}", current_height));
                            ui.separator();
                        } else {
                            ui.label("📦 Block: unknown");
                            ui.separator();
                        }
                        
                        // Sync status
                        if net_mgr.peer_count() > 0 {
                            ui.label("✓ Connected");
                        } else {
                            ui.label("⏳ Connecting...");
                        }'''

content = content.replace(old_status_display, new_status_display)

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Updated status display to show connection status!')
