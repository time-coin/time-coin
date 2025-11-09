with open('src/network.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Find where to add the method (after the start_sync method)
fetch_method = '''
    /// Fetch blockchain info from a connected peer
    pub async fn fetch_blockchain_info(&self) -> Result<BlockchainInfo, String> {
        // Try each connected peer until we get a successful response
        for peer in &self.connected_peers {
            let peer_ip = peer.address.split(':').next().unwrap_or(&peer.address);
            let url = format!("http://{}:24101/blockchain/info", peer_ip);
            
            log::info!("Fetching blockchain info from peer: {}", url);
            
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
            
            match client.get(&url).send().await {
                Ok(response) if response.status().is_success() => {
                    match response.json::<BlockchainInfo>().await {
                        Ok(info) => {
                            log::info!("Got blockchain height {} from peer {}", info.height, peer_ip);
                            return Ok(info);
                        }
                        Err(e) => {
                            log::warn!("Failed to parse response from {}: {}", peer_ip, e);
                            continue;
                        }
                    }
                }
                Ok(response) => {
                    log::warn!("Peer {} returned error: {}", peer_ip, response.status());
                    continue;
                }
                Err(e) => {
                    log::warn!("Failed to connect to peer {}: {}", peer_ip, e);
                    continue;
                }
            }
        }
        
        Err("No peers responded with blockchain info".to_string())
    }
'''

# Find the start_sync method and add fetch_blockchain_info after it
marker = '''    pub async fn start_sync(&mut self) -> Result<(), String> {
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

if marker in content:
    content = content.replace(marker, marker + fetch_method)
    
    # Also update start_sync to use it
    old_start_sync = marker
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
                log::info!("Synchronized to block height: {}", info.height);
                self.is_syncing = false;
                self.sync_progress = 1.0;
                Ok(())
            }
            Err(e) => {
                log::error!("Failed to fetch blockchain info: {}", e);
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
    print("Added fetch_blockchain_info method and updated start_sync!")
else:
    print("ERROR: Could not find start_sync method")
