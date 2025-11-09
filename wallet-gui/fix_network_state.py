with open('src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Add channel imports
import_section = '''use eframe::egui;
use wallet::NetworkType;'''

new_import_section = '''use eframe::egui;
use wallet::NetworkType;
use std::sync::{Arc, Mutex};'''

content = content.replace(import_section, new_import_section)

# Update the struct to use Arc<Mutex<NetworkManager>>
old_struct = '''    // Network manager
    network_manager: Option<NetworkManager>,
    network_status: String,'''

new_struct = '''    // Network manager (wrapped for thread safety)
    network_manager: Option<Arc<Mutex<NetworkManager>>>,
    network_status: String,'''

content = content.replace(old_struct, new_struct)

# Update the load section to use Arc<Mutex>
old_load = '''                                // Load config and initialize network
                                if let Ok(config) = WalletConfig::load(self.network) {
                                    self.config = config.clone();
                                    let network_mgr = NetworkManager::new(config.api_endpoint.clone());
                                    self.network_manager = Some(network_mgr);
                                    self.network_status = "Connecting...".to_string();
                                    
                                    // Trigger network bootstrap
                                    let bootstrap_nodes = config.addnode.clone();
                                    let api_endpoint = config.api_endpoint.clone();
                                    let ctx_clone = ctx.clone();
                                    
                                    tokio::spawn(async move {
                                        let mut net = NetworkManager::new(api_endpoint);
                                        if let Err(e) = net.bootstrap(bootstrap_nodes).await {
                                            log::error!("Network bootstrap failed: {}", e);
                                        } else {
                                            log::info!("Network bootstrap successful!");
                                        }
                                        ctx_clone.request_repaint();
                                    });
                                }'''

new_load = '''                                // Load config and initialize network
                                if let Ok(config) = WalletConfig::load(self.network) {
                                    self.config = config.clone();
                                    let network_mgr = Arc::new(Mutex::new(NetworkManager::new(config.api_endpoint.clone())));
                                    self.network_manager = Some(network_mgr.clone());
                                    self.network_status = "Connecting...".to_string();
                                    
                                    // Trigger network bootstrap
                                    let bootstrap_nodes = config.addnode.clone();
                                    let ctx_clone = ctx.clone();
                                    let net_mgr = network_mgr.clone();
                                    
                                    tokio::spawn(async move {
                                        let result = {
                                            let mut net = net_mgr.lock().unwrap();
                                            net.bootstrap(bootstrap_nodes).await
                                        };
                                        
                                        if let Err(e) = result {
                                            log::error!("Network bootstrap failed: {}", e);
                                        } else {
                                            log::info!("Network bootstrap successful!");
                                        }
                                        ctx_clone.request_repaint();
                                    });
                                }'''

content = content.replace(old_load, new_load)

# Update the create section
old_create = '''                                // Load config and initialize network
                                if let Ok(config) = WalletConfig::load(self.network) {
                                    self.config = config.clone();
                                    let network_mgr = NetworkManager::new(config.api_endpoint.clone());
                                    self.network_manager = Some(network_mgr);
                                    self.network_status = "Connecting...".to_string();
                                    
                                    // Trigger network bootstrap
                                    let bootstrap_nodes = config.addnode.clone();
                                    let api_endpoint = config.api_endpoint.clone();
                                    let ctx_clone = ctx.clone();
                                    
                                    tokio::spawn(async move {
                                        let mut net = NetworkManager::new(api_endpoint);
                                        if let Err(e) = net.bootstrap(bootstrap_nodes).await {
                                            log::error!("Network bootstrap failed: {}", e);
                                        } else {
                                            log::info!("Network bootstrap successful!");
                                        }
                                        ctx_clone.request_repaint();
                                    });
                                }'''

new_create = '''                                // Load config and initialize network
                                if let Ok(config) = WalletConfig::load(self.network) {
                                    self.config = config.clone();
                                    let network_mgr = Arc::new(Mutex::new(NetworkManager::new(config.api_endpoint.clone())));
                                    self.network_manager = Some(network_mgr.clone());
                                    self.network_status = "Connecting...".to_string();
                                    
                                    // Trigger network bootstrap
                                    let bootstrap_nodes = config.addnode.clone();
                                    let ctx_clone = ctx.clone();
                                    let net_mgr = network_mgr.clone();
                                    
                                    tokio::spawn(async move {
                                        let result = {
                                            let mut net = net_mgr.lock().unwrap();
                                            net.bootstrap(bootstrap_nodes).await
                                        };
                                        
                                        if let Err(e) = result {
                                            log::error!("Network bootstrap failed: {}", e);
                                        } else {
                                            log::info!("Network bootstrap successful!");
                                        }
                                        ctx_clone.request_repaint();
                                    });
                                }'''

content = content.replace(old_create, new_create)

# Update the status bar to lock the mutex
old_status = '''        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Network status
                if let Some(net_mgr) = &self.network_manager {
                    ui.label(format!("Peers: {}", net_mgr.peer_count()));
                    ui.separator();
                    
                    // Sync progress bar
                    if net_mgr.is_synced() {
                        ui.label("✓ Synchronized");
                    } else {
                        ui.label(format!("Synchronizing... {:.1}%", net_mgr.sync_progress() * 100.0));
                        ui.add(egui::ProgressBar::new(net_mgr.sync_progress()).show_percentage());
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

content = content.replace(old_status, new_status)

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Fixed network state management with Arc<Mutex>!')
