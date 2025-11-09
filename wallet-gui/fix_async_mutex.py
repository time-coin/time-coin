with open('src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix the async task to properly scope the mutex lock
old_async = '''                                    tokio::spawn(async move {
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
                                    });'''

new_async = '''                                    tokio::spawn(async move {
                                        // Call bootstrap which will modify the network manager
                                        let result = {
                                            let mut net = net_mgr.lock().unwrap();
                                            // Clone data needed for async operations
                                            let api_endpoint = net.api_endpoint.clone();
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
                                        }
                                        ctx_clone.request_repaint();
                                    });'''

content = content.replace(old_async, new_async)

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Fixed async mutex issue!')
