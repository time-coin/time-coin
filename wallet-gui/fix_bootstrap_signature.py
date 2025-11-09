with open('src/network.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# The bootstrap method should take &mut self, not consume self
# Find and replace the bootstrap signature
old_sig = 'pub async fn bootstrap(&mut self, bootstrap_nodes: Vec<String>) -> Result<(), String>'

if old_sig not in content:
    # It might be using self, let's fix it
    content = content.replace(
        'pub async fn bootstrap(mut self, bootstrap_nodes: Vec<String>) -> Result<(), String>',
        'pub async fn bootstrap(&mut self, bootstrap_nodes: Vec<String>) -> Result<(), String>'
    )
    content = content.replace(
        'pub async fn bootstrap(self, bootstrap_nodes: Vec<String>) -> Result<(), String>',
        'pub async fn bootstrap(&mut self, bootstrap_nodes: Vec<String>) -> Result<(), String>'
    )

with open('src/network.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Fixed bootstrap signature')

# Now update main.rs to not try to assign temp_net after calling bootstrap
with open('src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# The correct approach: bootstrap modifies temp_net, then we assign it
old_code = '''                                        let mut temp_net = NetworkManager::new(api_endpoint);
                                        let result = temp_net.bootstrap(bootstrap_nodes).await;
                                        
                                        if let Err(e) = result {
                                            log::error!("Network bootstrap failed: {}", e);
                                        } else {
                                            log::info!("Network bootstrap successful!");
                                            // Update shared state with the bootstrapped network manager
                                            let mut net = net_mgr.lock().unwrap();
                                            *net = temp_net;
                                        }'''

new_code = '''                                        let mut temp_net = NetworkManager::new(api_endpoint);
                                        match temp_net.bootstrap(bootstrap_nodes).await {
                                            Ok(_) => {
                                                log::info!("Network bootstrap successful!");
                                                // Update shared state with the bootstrapped network manager
                                                let mut net = net_mgr.lock().unwrap();
                                                *net = temp_net;
                                            }
                                            Err(e) => {
                                                log::error!("Network bootstrap failed: {}", e);
                                            }
                                        }'''

content = content.replace(old_code, new_code)

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Fixed main.rs to properly handle bootstrap result')
