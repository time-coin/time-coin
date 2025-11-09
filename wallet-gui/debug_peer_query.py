with open('src/network.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Make sure we're actually calling fetch_blockchain_info in the bootstrap
# Check if the fetch is happening
if 'match self.fetch_blockchain_info().await' not in content:
    print("ERROR: fetch_blockchain_info not being called in bootstrap!")
else:
    print("Good: fetch_blockchain_info is called in bootstrap")

# Let's add better logging to see what's happening
old_bootstrap_fetch = '''        // Try to fetch blockchain info to get current height
        match self.fetch_blockchain_info().await {
            Ok(info) => {
                self.network_block_height = info.height;
                self.current_block_height = info.height;
                log::info!("Synchronized to block height: {}", info.height);
                self.is_syncing = false;
                self.sync_progress = 1.0;
            }
            Err(e) => {
                log::warn!("Blockchain info API not available: {}", e);
                log::info!("Wallet connected to {} peers (blockchain height unknown)", self.connected_peers.len());
                // Set to connected but height unknown
                self.is_syncing = false;
                self.sync_progress = 1.0;
                self.current_block_height = 0; // Will show as "unknown"
                self.network_block_height = 0;
            }
        }'''

new_bootstrap_fetch = '''        // Try to fetch blockchain info to get current height
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
        }'''

content = content.replace(old_bootstrap_fetch, new_bootstrap_fetch)

with open('src/network.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Added debug logging!')
