# First, add a getter method to NetworkManager
with open('src/network.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Add getter method after the new() method
old_new = '''    pub fn new(api_endpoint: String) -> Self {
        Self {
            api_endpoint,
            connected_peers: Vec::new(),
            is_syncing: false,
            sync_progress: 0.0,
        }
    }'''

new_new = '''    pub fn new(api_endpoint: String) -> Self {
        Self {
            api_endpoint,
            connected_peers: Vec::new(),
            is_syncing: false,
            sync_progress: 0.0,
        }
    }
    
    pub fn api_endpoint(&self) -> &str {
        &self.api_endpoint
    }'''

content = content.replace(old_new, new_new)

with open('src/network.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Added api_endpoint getter to NetworkManager')

# Now fix the main.rs to use the getter and handle the result properly
with open('src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix the async blocks to use the getter method
old_pattern = '''                                    tokio::spawn(async move {
                                        let api_endpoint = {
                                            let net = net_mgr.lock().unwrap();
                                            net.api_endpoint.clone()
                                        };
                                        
                                        let mut temp_net = NetworkManager::new(api_endpoint);
                                        let result = temp_net.bootstrap(bootstrap_nodes).await;
                                        
                                        if let Err(e) = &result {
                                            log::error!("Network bootstrap failed: {}", e);
                                        } else {
                                            log::info!("Network bootstrap successful!");
                                            // Update shared state
                                            let mut net = net_mgr.lock().unwrap();
                                            *net = temp_net;
                                        }
                                        ctx_clone.request_repaint();
                                    });'''

new_pattern = '''                                    tokio::spawn(async move {
                                        let api_endpoint = {
                                            let net = net_mgr.lock().unwrap();
                                            net.api_endpoint().to_string()
                                        };
                                        
                                        let mut temp_net = NetworkManager::new(api_endpoint);
                                        let result = temp_net.bootstrap(bootstrap_nodes).await;
                                        
                                        if let Err(e) = &result {
                                            log::error!("Network bootstrap failed: {}", e);
                                        } else {
                                            log::info!("Network bootstrap successful!");
                                            // Update shared state with the bootstrapped network manager
                                            let mut net = net_mgr.lock().unwrap();
                                            *net = temp_net;
                                        }
                                        ctx_clone.request_repaint();
                                    });'''

content = content.replace(old_pattern, new_pattern)

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Fixed main.rs to use getter and handle NetworkManager properly')
