with open('src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Add network module
module_declaration = '''mod wallet_dat;
mod wallet_manager;
mod config;'''

new_module_declaration = '''mod wallet_dat;
mod wallet_manager;
mod config;
mod network;'''

content = content.replace(module_declaration, new_module_declaration)

# Add network imports
import_section = '''use wallet_manager::WalletManager;
use config::WalletConfig;'''

new_import_section = '''use wallet_manager::WalletManager;
use config::WalletConfig;
use network::NetworkManager;'''

content = content.replace(import_section, new_import_section)

# Add network_manager field to struct
struct_fields = '''    // Configuration
    config: WalletConfig,
}'''

new_struct_fields = '''    // Configuration
    config: WalletConfig,
    
    // Network manager
    network_manager: Option<NetworkManager>,
    network_status: String,
}'''

content = content.replace(struct_fields, new_struct_fields)

# Update Default implementation
default_impl = '''            config: WalletConfig::default(),
        }
    }
}'''

new_default_impl = '''            config: WalletConfig::default(),
            network_manager: None,
            network_status: "Not connected".to_string(),
        }
    }
}'''

content = content.replace(default_impl, new_default_impl)

# Add network initialization when wallet loads/creates
# In load_default section
load_section = '''                        match WalletManager::load_default(self.network) {
                            Ok(manager) => {
                                self.wallet_manager = Some(manager);
                                self.current_screen = Screen::Overview;
                                self.success_message = Some("Wallet unlocked successfully!".to_string());
                            }'''

new_load_section = '''                        match WalletManager::load_default(self.network) {
                            Ok(manager) => {
                                self.wallet_manager = Some(manager);
                                self.current_screen = Screen::Overview;
                                self.success_message = Some("Wallet unlocked successfully!".to_string());
                                
                                // Load config and start network
                                if let Ok(config) = WalletConfig::load(self.network) {
                                    self.config = config.clone();
                                    let mut network_mgr = NetworkManager::new(config.api_endpoint.clone());
                                    let bootstrap_nodes = config.addnode.clone();
                                    
                                    // Spawn network bootstrap task
                                    let ctx_clone = ctx.clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = network_mgr.bootstrap(bootstrap_nodes).await {
                                            log::error!("Network bootstrap failed: {}", e);
                                        }
                                        ctx_clone.request_repaint();
                                    });
                                    
                                    self.network_manager = Some(network_mgr);
                                    self.network_status = "Connecting...".to_string();
                                }
                            }'''

content = content.replace(load_section, new_load_section)

# In create_new section
create_section = '''                        match WalletManager::create_new(self.network, "Default".to_string()) {
                            Ok(manager) => {
                                self.wallet_manager = Some(manager);
                                self.current_screen = Screen::Overview;
                                self.success_message = Some("Wallet created successfully!".to_string());
                            }'''

new_create_section = '''                        match WalletManager::create_new(self.network, "Default".to_string()) {
                            Ok(manager) => {
                                self.wallet_manager = Some(manager);
                                self.current_screen = Screen::Overview;
                                self.success_message = Some("Wallet created successfully!".to_string());
                                
                                // Load config and start network
                                if let Ok(config) = WalletConfig::load(self.network) {
                                    self.config = config.clone();
                                    let mut network_mgr = NetworkManager::new(config.api_endpoint.clone());
                                    let bootstrap_nodes = config.addnode.clone();
                                    
                                    // Spawn network bootstrap task
                                    let ctx_clone = ctx.clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = network_mgr.bootstrap(bootstrap_nodes).await {
                                            log::error!("Network bootstrap failed: {}", e);
                                        }
                                        ctx_clone.request_repaint();
                                    });
                                    
                                    self.network_manager = Some(network_mgr);
                                    self.network_status = "Connecting...".to_string();
                                }
                            }'''

content = content.replace(create_section, new_create_section)

# Update status bar to use network_manager data
status_bar = '''        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Sync status
                ui.label(format!("Peers: {}", self.peer_count));
                ui.separator();
                
                // Sync progress bar
                if self.is_synced {
                    ui.label("✓ Synchronized");
                } else {
                    ui.label(format!("Synchronizing... {:.1}%", self.sync_progress * 100.0));
                    ui.add(egui::ProgressBar::new(self.sync_progress).show_percentage());
                }
            });
        });'''

new_status_bar = '''        // Bottom status bar
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

content = content.replace(status_bar, new_status_bar)

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Integrated network manager into main app!')
print('The wallet will now:')
print('  1. Load time-coin.conf on startup')
print('  2. Connect to time-coin.io/api/peers')
print('  3. Bootstrap from addnode entries')
print('  4. Display peer count in status bar')
