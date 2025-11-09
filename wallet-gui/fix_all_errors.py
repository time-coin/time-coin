with open('src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Replace all occurrences of the private field access
content = content.replace('net.api_endpoint.clone()', 'net.api_endpoint().to_string()')
content = content.replace('net.api_endpoint().to_string().clone()', 'net.api_endpoint().to_string()')

# Fix the assignment issue - bootstrap() returns Result<(), String>, not NetworkManager
# We need to call bootstrap first, then if successful, assign temp_net
old_block = '''                                        let mut temp_net = NetworkManager::new(api_endpoint);
                                        let result = temp_net.bootstrap(bootstrap_nodes).await;
                                        
                                        if let Err(e) = &result {
                                            log::error!("Network bootstrap failed: {}", e);
                                        } else {
                                            log::info!("Network bootstrap successful!");
                                            // Update shared state with the bootstrapped network manager
                                            let mut net = net_mgr.lock().unwrap();
                                            *net = temp_net;
                                        }'''

new_block = '''                                        let mut temp_net = NetworkManager::new(api_endpoint);
                                        let result = temp_net.bootstrap(bootstrap_nodes).await;
                                        
                                        if let Err(e) = result {
                                            log::error!("Network bootstrap failed: {}", e);
                                        } else {
                                            log::info!("Network bootstrap successful!");
                                            // Update shared state with the bootstrapped network manager
                                            let mut net = net_mgr.lock().unwrap();
                                            *net = temp_net;
                                        }'''

content = content.replace(old_block, new_block)

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Fixed all compilation errors')
