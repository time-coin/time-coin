with open('src/network.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Find the end of the bootstrap method and add blockchain info fetch before Ok(())
old_bootstrap_end = '''                log::info!("Connected to {} peers from bootstrap nodes", fallback_peers.len());
                self.connect_to_peers(fallback_peers).await?;
            }
        }

        Ok(())
    }'''

new_bootstrap_end = '''                log::info!("Connected to {} peers from bootstrap nodes", fallback_peers.len());
                self.connect_to_peers(fallback_peers).await?;
            }
        }

        // Try to fetch blockchain info to get current height
        log::info!("Attempting to fetch blockchain info from {} peers...", self.connected_peers.len());
        match self.fetch_blockchain_info().await {
            Ok(info) => {
                self.network_block_height = info.height;
                self.current_block_height = info.height;
                log::info!("✓ Synchronized to block height: {}", info.height);
                self.is_syncing = false;
                self.sync_progress = 1.0;
            }
            Err(e) => {
                log::error!("✗ Failed to fetch blockchain info: {}", e);
                log::info!("Wallet connected to {} peers (blockchain height unknown)", self.connected_peers.len());
                // Set to connected but height unknown
                self.is_syncing = false;
                self.sync_progress = 1.0;
                self.current_block_height = 0; // Will show as "unknown"
                self.network_block_height = 0;
            }
        }

        Ok(())
    }'''

content = content.replace(old_bootstrap_end, new_bootstrap_end)

with open('src/network.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Added blockchain info fetch to bootstrap!')
