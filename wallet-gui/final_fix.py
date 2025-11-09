with open('src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Find and replace the OLD pattern that has the block with result
old_wrong_pattern = '''                                        let result = {
                                            let mut net = net_mgr.lock().unwrap();
                                            // Clone data needed for async operations
                                            let api_endpoint = net.api_endpoint().to_string();
                                            drop(net); // Release lock before await
                                            
                                            // Create temporary network manager for bootstrap
                                            let mut temp_net = NetworkManager::new(api_endpoint);
                                            temp_net.bootstrap(bootstrap_nodes).await
                                        };

                                        if let Err(e) = &result {
                                            log::error!("Network bootstrap failed: {}", e);
                                        } else {
                                            log::info!("Network bootstrap successful!");

                                            // Update the shared network manager with results
                                            if let Ok(temp_net) = result {
                                                let mut net = net_mgr.lock().unwrap();
                                                *net = temp_net;
                                            }
                                        }'''

correct_pattern = '''                                        let api_endpoint = {
                                            let net = net_mgr.lock().unwrap();
                                            net.api_endpoint().to_string()
                                        };
                                        
                                        let mut temp_net = NetworkManager::new(api_endpoint);
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

content = content.replace(old_wrong_pattern, correct_pattern)

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Applied final fix!')
