with open('src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Remove the old pattern completely - it appears twice
old_pattern1 = '''                                        let mut temp_net = NetworkManager::new(api_endpoint);
                                        let result = temp_net.bootstrap(bootstrap_nodes).await;
                                        
                                        if let Err(e) = result {
                                            log::error!("Network bootstrap failed: {}", e);
                                        } else {
                                            log::info!("Network bootstrap successful!");

                                            // Update the shared network manager with results
                                            if let Ok(temp_net) = result {
                                                let mut net = net_mgr.lock().unwrap();
                                                *net = temp_net;
                                            }
                                        }'''

new_pattern1 = '''                                        let mut temp_net = NetworkManager::new(api_endpoint);
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

# Replace all occurrences
content = content.replace(old_pattern1, new_pattern1)

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Cleaned up duplicate code patterns')
