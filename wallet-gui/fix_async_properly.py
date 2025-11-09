with open('src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Find the problematic async blocks and replace them with the correct pattern
# The issue is "let mut net = net_mgr.lock().unwrap();" inside the result block

old_async_block = '''                                    tokio::spawn(async move {
                                        // Call bootstrap which will modify the network manager
                                        let result = {
                                            let mut net = net_mgr.lock().unwrap();
                                            // Clone data needed for async operations
                                            let api_endpoint = net.api_endpoint().to_string();
                                            drop(net); // Release lock before await
                                            
                                            // Create temporary network manager for bootstrap
                                            let mut temp_net = NetworkManager::new(api_endpoint);
                                            temp_net.bootstrap(bootstrap_nodes).await
                                        };
                                        
                                        
                                        ctx_clone.request_repaint();
                                    });'''

new_async_block = '''                                    tokio::spawn(async move {
                                        let api_endpoint = {
                                            let net = net_mgr.lock().unwrap();
                                            net.api_endpoint().to_string()
                                        };
                                        
                                        let mut temp_net = NetworkManager::new(api_endpoint);
                                        match temp_net.bootstrap(bootstrap_nodes).await {
                                            Ok(_) => {
                                                log::info!("Network bootstrap successful!");
                                                let mut net = net_mgr.lock().unwrap();
                                                *net = temp_net;
                                            }
                                            Err(e) => {
                                                log::error!("Network bootstrap failed: {}", e);
                                            }
                                        }
                                        ctx_clone.request_repaint();
                                    });'''

content = content.replace(old_async_block, new_async_block)

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Fixed async blocks properly')
