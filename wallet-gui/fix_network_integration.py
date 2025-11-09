with open('src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# First, let's simplify and fix the network initialization
# Replace the load section with proper async handling
old_load = '''                        match WalletManager::load_default(self.network) {
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

new_load = '''                        match WalletManager::load_default(self.network) {
                            Ok(manager) => {
                                self.wallet_manager = Some(manager);
                                self.current_screen = Screen::Overview;
                                self.success_message = Some("Wallet unlocked successfully!".to_string());
                                
                                // Load config and initialize network
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
                                }
                            }'''

content = content.replace(old_load, new_load)

# Replace the create section
old_create = '''                        match WalletManager::create_new(self.network, "Default".to_string()) {
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

new_create = '''                        match WalletManager::create_new(self.network, "Default".to_string()) {
                            Ok(manager) => {
                                self.wallet_manager = Some(manager);
                                self.current_screen = Screen::Overview;
                                self.success_message = Some("Wallet created successfully!".to_string());
                                
                                // Load config and initialize network
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
                                }
                            }'''

content = content.replace(old_create, new_create)

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Fixed network integration!')
