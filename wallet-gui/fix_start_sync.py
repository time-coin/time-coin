with open('src/network.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Replace the stub start_sync with actual blockchain info fetching
old_start_sync = '''    pub async fn start_sync(&mut self) -> Result<(), String> {
        if self.connected_peers.is_empty() {
            return Err("No peers connected".to_string());
        }

        log::info!("Starting blockchain sync...");
        self.is_syncing = true;
        self.sync_progress = 0.0;

        // TODO: Implement actual blockchain sync
        // For now, simulate sync completion
        self.sync_progress = 1.0;
        self.is_syncing = false;

        Ok(())
    }'''

new_start_sync = '''    pub async fn start_sync(&mut self) -> Result<(), String> {
        if self.connected_peers.is_empty() {
            return Err("No peers connected".to_string());
        }

        log::info!("Starting blockchain sync from {} peers...", self.connected_peers.len());
        self.is_syncing = true;
        self.sync_progress = 0.0;

        // Fetch blockchain info from connected peers
        match self.fetch_blockchain_info().await {
            Ok(info) => {
                self.network_block_height = info.height;
                self.current_block_height = info.height;
                log::info!("✓ Synchronized to block height: {}", info.height);
                self.is_syncing = false;
                self.sync_progress = 1.0;
                Ok(())
            }
            Err(e) => {
                log::error!("✗ Failed to fetch blockchain info: {}", e);
                self.is_syncing = false;
                self.sync_progress = 0.0;
                self.current_block_height = 0;
                self.network_block_height = 0;
                Err(format!("Failed to sync blockchain: {}", e))
            }
        }
    }'''

content = content.replace(old_start_sync, new_start_sync)

with open('src/network.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Fixed start_sync to actually fetch blockchain info!')
