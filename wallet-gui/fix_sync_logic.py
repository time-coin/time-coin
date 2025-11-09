with open('src/network.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Update the bootstrap method to fetch blockchain info and set proper sync state
old_bootstrap = '''    pub async fn bootstrap(&mut self, bootstrap_nodes: Vec<String>) -> Result<(), String> {
        log::info!("Bootstrapping network with {} nodes", bootstrap_nodes.len());

        // First, try to fetch peers from API
        match self.fetch_peers().await {
            Ok(peers) => {
                log::info!("Fetched {} peers from API", peers.len());
                self.connected_peers = peers;
            }
            Err(e) => {
                log::warn!("Failed to fetch peers from API: {}", e);
                // Fall back to bootstrap nodes
                for node in bootstrap_nodes {
                    self.add_peer(node);
                }
            }
        }

        // Start syncing blockchain
        if !self.connected_peers.is_empty() {
            self.start_sync().await?;
        }

        Ok(())
    }'''

new_bootstrap = '''    pub async fn bootstrap(&mut self, bootstrap_nodes: Vec<String>) -> Result<(), String> {
        log::info!("Bootstrapping network with {} nodes", bootstrap_nodes.len());

        // First, try to fetch peers from API
        match self.fetch_peers().await {
            Ok(peers) => {
                log::info!("Fetched {} peers from API", peers.len());
                self.connected_peers = peers;
            }
            Err(e) => {
                log::warn!("Failed to fetch peers from API: {}", e);
                // Fall back to bootstrap nodes
                for node in bootstrap_nodes {
                    self.add_peer(node);
                }
            }
        }

        // Fetch blockchain info to get current height
        match self.fetch_blockchain_info().await {
            Ok(info) => {
                self.network_block_height = info.height;
                self.current_block_height = info.height;
                log::info!("Synchronized to block height: {}", info.height);
                self.is_syncing = false;
                self.sync_progress = 1.0;
            }
            Err(e) => {
                log::error!("Failed to fetch blockchain info: {}", e);
                self.is_syncing = true;
                self.sync_progress = 0.0;
                return Err(format!("Failed to sync blockchain: {}", e));
            }
        }

        Ok(())
    }'''

content = content.replace(old_bootstrap, new_bootstrap)

# Update start_sync to be simpler since bootstrap handles it now
old_start_sync = '''    /// Start syncing blockchain
    pub async fn start_sync(&mut self) -> Result<(), String> {
        if self.connected_peers.is_empty() {
            return Err("No peers connected".to_string());
        }
        
        log::info!("Starting blockchain sync...");
        self.is_syncing = true;
        self.sync_progress = 0.0;
        
        // Fetch network block height
        match self.fetch_blockchain_info().await {
            Ok(info) => {
                self.network_block_height = info.height;
                log::info!("Network block height: {}", self.network_block_height);
                
                // TODO: Implement actual blockchain sync
                // For now, simulate catching up
                self.current_block_height = self.network_block_height;
                self.sync_progress = 1.0;
                self.is_syncing = false;
            }
            Err(e) => {
                log::error!("Failed to fetch blockchain info: {}", e);
                // Continue without blockchain info
                self.sync_progress = 1.0;
                self.is_syncing = false;
            }
        }
        
        Ok(())
    }'''

new_start_sync = '''    /// Refresh blockchain state (for periodic updates)
    pub async fn refresh_blockchain_state(&mut self) -> Result<(), String> {
        if self.connected_peers.is_empty() {
            return Err("No peers connected".to_string());
        }
        
        log::info!("Refreshing blockchain state...");
        
        // Fetch current network block height
        match self.fetch_blockchain_info().await {
            Ok(info) => {
                let old_height = self.current_block_height;
                self.network_block_height = info.height;
                self.current_block_height = info.height;
                
                if old_height != info.height {
                    log::info!("Block height updated: {} -> {}", old_height, info.height);
                }
                
                self.is_syncing = false;
                self.sync_progress = 1.0;
            }
            Err(e) => {
                log::error!("Failed to refresh blockchain state: {}", e);
                return Err(format!("Failed to refresh blockchain: {}", e));
            }
        }
        
        Ok(())
    }'''

content = content.replace(old_start_sync, new_start_sync)

# Update is_synced to check if we have valid block height
old_is_synced = '''    pub fn is_synced(&self) -> bool {
        !self.is_syncing && self.sync_progress >= 1.0
    }'''

new_is_synced = '''    pub fn is_synced(&self) -> bool {
        !self.is_syncing && self.sync_progress >= 1.0 && self.current_block_height > 0
    }'''

content = content.replace(old_is_synced, new_is_synced)

with open('src/network.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Updated sync logic to fetch real blockchain height!')
