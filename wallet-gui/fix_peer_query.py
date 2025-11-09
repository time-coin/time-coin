with open('src/network.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Update fetch_blockchain_info to query a connected peer instead of api_endpoint
old_fetch = '''    /// Fetch blockchain info from API
    pub async fn fetch_blockchain_info(&self) -> Result<BlockchainInfo, String> {
        let url = format!("{}/blockchain/info", self.api_endpoint);
        
        log::info!("Fetching blockchain info from: {}", url);
        
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
        
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch blockchain info: {}", e))?;
        
        if !response.status().is_success() {
            return Err(format!("API returned error: {}", response.status()));
        }
        
        let info: BlockchainInfo = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse blockchain info: {}", e))?;
        
        log::info!("Current blockchain height: {}", info.height);
        
        Ok(info)
    }'''

new_fetch = '''    /// Fetch blockchain info from a connected peer
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
    }'''

content = content.replace(old_fetch, new_fetch)

with open('src/network.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Updated to query peers directly for blockchain info!')
