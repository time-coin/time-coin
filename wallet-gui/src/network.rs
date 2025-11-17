use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiPeer {
    pub address: String, // This includes port like "134.199.175.106:24100"
    pub version: String,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub address: String,
    pub port: u16,
    pub version: Option<String>,
    pub last_seen: Option<u64>,
    #[serde(default)]
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiPeersResponse {
    pub peers: Vec<ApiPeer>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainInfo {
    pub network: String,
    pub height: u64,
    pub best_block_hash: String,
    pub total_supply: u64,
    pub timestamp: i64,
}

/// Request to sync wallet addresses
#[derive(Debug, Serialize)]
pub struct WalletSyncRequest {
    pub addresses: Vec<String>,
}

/// UTXO information from masternode
#[derive(Debug, Deserialize, Clone)]
pub struct UtxoInfo {
    pub tx_hash: String,
    pub output_index: u32,
    pub amount: u64,
    pub address: String,
    pub block_height: u64,
    pub confirmations: u64,
}

/// Transaction notification from masternode
#[derive(Debug, Deserialize, Clone)]
pub struct TransactionNotification {
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: u64,
    pub block_height: u64,
    pub timestamp: u64,
    pub confirmations: u64,
}

/// Response from wallet sync
#[derive(Debug, Deserialize)]
pub struct WalletSyncResponse {
    pub utxos: HashMap<String, Vec<UtxoInfo>>,
    pub total_balance: u64,
    pub recent_transactions: Vec<TransactionNotification>,
    pub current_height: u64,
}

#[derive(Debug, Clone)]
pub struct NetworkManager {
    api_endpoint: String,
    connected_peers: Vec<PeerInfo>,
    is_syncing: bool,
    sync_progress: f32,
    current_block_height: u64,
    network_block_height: u64,
    client: reqwest::Client,
}

impl NetworkManager {
    pub fn new(api_endpoint: String) -> Self {
        Self {
            api_endpoint,
            connected_peers: Vec::new(),
            is_syncing: false,
            sync_progress: 0.0,
            current_block_height: 0,
            network_block_height: 0,
            client: reqwest::Client::new(),
        }
    }

    pub fn api_endpoint(&self) -> &str {
        &self.api_endpoint
    }

    pub fn current_block_height(&self) -> u64 {
        self.current_block_height
    }

    pub fn network_block_height(&self) -> u64 {
        self.network_block_height
    }

    /// Fetch peer list from API
    pub async fn fetch_peers(&self) -> Result<Vec<PeerInfo>, String> {
        let url = format!("{}/peers", self.api_endpoint);

        log::info!("Fetching peers from: {}", url);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch peers: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API returned error: {}", response.status()));
        }

        // API returns a simple array of "ip:port" strings
        let peer_addresses: Vec<String> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse peer response: {}", e))?;

        log::info!("Fetched {} peers from API", peer_addresses.len());

        // Convert address strings to PeerInfo
        let peer_infos: Vec<PeerInfo> = peer_addresses
            .iter()
            .filter_map(|addr| {
                let parts: Vec<&str> = addr.split(':').collect();
                let ip = parts.get(0)?.to_string();
                let port = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(24100);
                Some(PeerInfo {
                    address: ip,
                    port,
                    version: None,
                    last_seen: Some(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    ),
                    latency_ms: 0,
                })
            })
            .collect();
        Ok(peer_infos)
    }

    /// Connect to peers and discover more peers from them
    pub async fn connect_to_peers(&mut self, initial_peers: Vec<PeerInfo>) -> Result<(), String> {
        log::info!(
            "Attempting to connect to {} initial peers",
            initial_peers.len()
        );

        // Store connected peers
        self.connected_peers = initial_peers.clone();

        // For each peer, attempt to discover additional peers
        // This creates a peer discovery chain
        for peer in &initial_peers {
            let peer_addr = format!("{}:{}", peer.address, peer.port);
            log::info!("Requesting peer list from {}", peer_addr);

            // Try to get additional peers from this masternode
            // The actual peer exchange would happen via P2P protocol
            // For now, we'll just log that we would request it
            // TODO: Implement actual P2P peer exchange protocol
        }

        Ok(())
    }

    /// Get peer count
    pub fn peer_count(&self) -> u32 {
        self.connected_peers.len() as u32
    }

    /// Check if synced
    pub fn is_synced(&self) -> bool {
        !self.is_syncing && self.sync_progress >= 1.0 && self.current_block_height > 0
    }

    /// Get sync progress (0.0 to 1.0)
    pub fn sync_progress(&self) -> f32 {
        self.sync_progress
    }

    /// Start syncing blockchain
    pub async fn start_sync(&mut self) -> Result<(), String> {
        if self.connected_peers.is_empty() {
            return Err("No peers connected".to_string());
        }

        log::info!(
            "Starting blockchain sync from {} peers...",
            self.connected_peers.len()
        );
        self.is_syncing = true;
        self.sync_progress = 0.0;

        return match self.fetch_blockchain_info().await {
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
                Err(format!("Failed to sync: {}", e))
            }
        };
    }

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
                            log::info!(
                                "Got blockchain height {} from peer {}",
                                info.height,
                                peer_ip
                            );
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

    /// Bootstrap network connections
    pub async fn bootstrap(&mut self, bootstrap_nodes: Vec<String>) -> Result<(), String> {
        log::info!("Bootstrapping network with {} nodes", bootstrap_nodes.len());

        // First, try to fetch peers from API
        match self.fetch_peers().await {
            Ok(peers) => {
                log::info!("Successfully fetched {} peers from API", peers.len());
                if !peers.is_empty() {
                    self.connect_to_peers(peers).await?;
                } else {
                    log::warn!("API returned 0 peers");
                }
            }
            Err(e) => {
                log::warn!("Failed to fetch peers from API: {}", e);
                log::info!("Falling back to bootstrap nodes");

                // Fall back to bootstrap nodes from config
                let fallback_peers: Vec<PeerInfo> = bootstrap_nodes
                    .into_iter()
                    .filter_map(|addr| {
                        if let Some((host, port_str)) = addr.rsplit_once(':') {
                            if let Ok(port) = port_str.parse() {
                                return Some(PeerInfo {
                                    address: host.to_string(),
                                    port,
                                    version: None,
                                    last_seen: None,
                                    latency_ms: 0,
                                });
                            }
                        }
                        None
                    })
                    .collect();

                if !fallback_peers.is_empty() {
                    self.connect_to_peers(fallback_peers).await?;
                } else {
                    log::warn!("No bootstrap nodes available");
                }
            }
        }

        // Only sync if we have peers
        if !self.connected_peers.is_empty() {
            // Start blockchain sync
            if let Err(e) = self.start_sync().await {
                log::warn!("Blockchain sync failed: {}", e);
            }

            // Discover more peers and optimize connections
            log::info!("Discovering additional peers...");
            if let Err(e) = self.discover_and_connect_peers().await {
                log::warn!("Peer discovery had issues: {}", e);
            }
        } else {
            log::info!("No peers available - wallet running in offline mode");
        }

        Ok(())
    }

    /// Measure latency to a peer
    async fn measure_latency(&self, peer_address: &str) -> Result<u64, String> {
        let peer_ip = peer_address.split(':').next().unwrap_or(peer_address);
        let url = format!("http://{}:24101/blockchain/info", peer_ip);

        let start = std::time::Instant::now();
        match self
            .client
            .get(&url)
            .timeout(Duration::from_secs(3))
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                let latency = start.elapsed().as_millis() as u64;
                Ok(latency)
            }
            Ok(_) => Err("Non-success response".to_string()),
            Err(e) => Err(format!("Failed to measure latency: {}", e)),
        }
    }

    /// Discover peers from a connected peer
    async fn discover_peers_from_peer(&self, peer_address: &str) -> Result<Vec<PeerInfo>, String> {
        let peer_ip = peer_address.split(':').next().unwrap_or(peer_address);
        let url = format!("http://{}:24101/peers", peer_ip);

        log::info!("Discovering peers from: {}", url);

        match self
            .client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                match response.json::<ApiPeersResponse>().await {
                    Ok(data) => {
                        log::info!(
                            "Discovered {} peers from {}",
                            data.peers.len(),
                            peer_address
                        );
                        // Convert ApiPeer to PeerInfo
                        let peer_infos: Vec<PeerInfo> = data
                            .peers
                            .iter()
                            .map(|api_peer| {
                                let parts: Vec<&str> = api_peer.address.split(':').collect();
                                let ip = parts.get(0).unwrap_or(&"").to_string();
                                let port =
                                    parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(24100);
                                PeerInfo {
                                    address: ip,
                                    port,
                                    version: Some(api_peer.version.clone()),
                                    last_seen: Some(
                                        std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap()
                                            .as_secs(),
                                    ),
                                    latency_ms: 0,
                                }
                            })
                            .collect();
                        Ok(peer_infos)
                    }
                    Err(e) => {
                        log::warn!("Failed to parse peers from {}: {}", peer_address, e);
                        Err(format!("Failed to parse peers: {}", e))
                    }
                }
            }
            Ok(response) => {
                log::warn!(
                    "Peer {} returned status: {}",
                    peer_address,
                    response.status()
                );
                Err(format!("Non-success response: {}", response.status()))
            }
            Err(e) => {
                log::warn!("Failed to connect to peer {}: {}", peer_address, e);
                Err(format!("Failed to connect: {}", e))
            }
        }
    }

    /// Discover and connect to peers recursively
    pub fn get_connected_peers(&self) -> Vec<PeerInfo> {
        log::info!(
            "get_connected_peers called, returning {} peers",
            self.connected_peers.len()
        );
        for (i, peer) in self.connected_peers.iter().enumerate() {
            log::info!(
                "  Peer {}: {}:{} - {}ms",
                i + 1,
                peer.address,
                peer.port,
                peer.latency_ms
            );
        }
        self.connected_peers.clone()
    }

    pub fn set_connected_peers(&mut self, peers: Vec<PeerInfo>) {
        self.connected_peers = peers;
    }

    pub async fn discover_and_connect_peers(&mut self) -> Result<(), String> {
        log::info!("Starting peer discovery...");

        let mut discovered_peers: std::collections::HashMap<String, PeerInfo> =
            std::collections::HashMap::new();
        let mut peers_to_check: Vec<String> = self
            .connected_peers
            .iter()
            .map(|p| format!("{}:{}", p.address, p.port))
            .collect();
        let mut checked_peers: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Discover peers from each connected peer
        while !peers_to_check.is_empty() {
            let current_peer = peers_to_check.pop().unwrap();

            if checked_peers.contains(&current_peer) {
                continue;
            }

            checked_peers.insert(current_peer.clone());

            if let Ok(peers) = self.discover_peers_from_peer(&current_peer).await {
                for peer in peers {
                    let peer_key = format!("{}:{}", peer.address, peer.port);
                    if !discovered_peers.contains_key(&peer_key)
                        && !checked_peers.contains(&peer_key)
                    {
                        discovered_peers.insert(peer_key.clone(), peer.clone());
                        peers_to_check.push(peer_key);
                    }
                }
            }

            // Limit discovery to avoid infinite loops
            if checked_peers.len() > 20 {
                log::info!(
                    "Reached discovery limit, stopping at {} peers checked",
                    checked_peers.len()
                );
                break;
            }
        }

        log::info!("Discovered {} total peers", discovered_peers.len());

        // Measure latency for all discovered peers
        let mut peers_with_latency: Vec<PeerInfo> = Vec::new();

        for (address, mut peer) in discovered_peers {
            log::info!("Measuring latency for discovered peer: {}", address);
            if let Ok(latency) = self.measure_latency(&address).await {
                peer.latency_ms = latency;
                peers_with_latency.push(peer);
                log::info!("  ✓ Peer {} latency: {}ms", address, latency);
            } else {
                log::warn!(
                    "  ✗ Failed to measure latency for peer: {} - peer excluded",
                    address
                );
            }
        }

        // Sort by latency (lowest first)
        peers_with_latency.sort_by_key(|p| p.latency_ms);

        // Keep top 5 fastest peers
        let top_peers: Vec<PeerInfo> = peers_with_latency.into_iter().take(5).collect();

        log::info!("Selected top {} peers based on latency:", top_peers.len());
        for peer in &top_peers {
            log::info!("  {}:{} - {}ms", peer.address, peer.port, peer.latency_ms);
        }

        // Update connected peers
        self.connected_peers = top_peers;

        Ok(())
    }

    /// Refresh latency measurements for all connected peers by directly pinging them
    pub async fn refresh_peer_latencies(&mut self) {
        log::info!(
            "Pinging {} peers to measure latency",
            self.connected_peers.len()
        );

        for peer in &mut self.connected_peers {
            let peer_ip = peer.address.split(':').next().unwrap_or(&peer.address);
            let url = format!("http://{}:24101/blockchain/info", peer_ip);

            let start = std::time::Instant::now();

            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap();

            match client.get(&url).send().await {
                Ok(_) => {
                    let latency = start.elapsed().as_millis() as u64;
                    peer.latency_ms = latency;
                    log::info!("  Peer {} responded in {}ms", peer.address, latency);
                }
                Err(e) => {
                    log::warn!("  Failed to ping {}: {}", peer.address, e);
                    peer.latency_ms = 9999; // Mark as unreachable
                }
            }
        }

        log::info!("Latency refresh complete");
    }
}
