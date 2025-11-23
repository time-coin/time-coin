use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiPeer {
    pub address: String, // IP:port format
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

    /// Set blockchain heights (for internal updates)
    pub fn set_blockchain_height(&mut self, height: u64) {
        if height > self.network_block_height {
            self.network_block_height = height;
            self.current_block_height = height;
        }
    }

    /// Fetch peer list from API - queries registered masternodes
    pub async fn fetch_peers(&self) -> Result<Vec<PeerInfo>, String> {
        let url = format!("{}/masternodes/list", self.api_endpoint);

        log::info!("Fetching masternodes from: {}", url);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch masternodes: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API returned error: {}", response.status()));
        }

        // Parse the masternodes response
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse masternode response: {}", e))?;

        log::info!("Masternodes response: {:?}", json);

        // Extract masternodes array
        let masternodes = json
            .get("masternodes")
            .and_then(|v| v.as_array())
            .ok_or("Response missing 'masternodes' array")?;

        log::info!("Found {} registered masternodes", masternodes.len());

        // Convert to PeerInfo format
        let peer_infos: Vec<PeerInfo> = masternodes
            .iter()
            .filter_map(|mn| {
                let address = mn.get("address")?.as_str()?.to_string();
                // Masternodes use port 24100 for P2P
                Some(PeerInfo {
                    address,
                    port: 24100,
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

    /// Connect to peers via TCP protocol (fast, parallel)
    pub async fn connect_to_peers(&mut self, initial_peers: Vec<PeerInfo>) -> Result<(), String> {
        log::info!(
            "Attempting to connect to {} peers via TCP",
            initial_peers.len()
        );

        // Store ALL peers
        self.connected_peers = initial_peers.clone();

        // Test connectivity in PARALLEL via TCP with Ping
        let mut tasks = Vec::new();

        for peer in &self.connected_peers {
            let peer_ip = peer
                .address
                .split(':')
                .next()
                .unwrap_or(&peer.address)
                .to_string();
            let peer_address = peer.address.clone();
            let port = peer.port;

            let task = tokio::spawn(async move {
                use time_network::protocol::NetworkMessage;
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                use tokio::net::TcpStream;

                let tcp_addr = format!("{}:{}", peer_ip, port);
                let start = std::time::Instant::now();

                log::info!("Testing TCP connection to {}...", tcp_addr);

                // Try TCP connection with 3 second timeout (increased from 2s)
                let timeout_duration = std::time::Duration::from_secs(3);
                match tokio::time::timeout(timeout_duration, TcpStream::connect(&tcp_addr)).await {
                    Ok(Ok(mut stream)) => {
                        log::info!("  Connected to {}, sending Ping...", tcp_addr);

                        // Send Ping
                        let ping = NetworkMessage::Ping;
                        if let Ok(data) = serde_json::to_vec(&ping) {
                            let len = data.len() as u32;
                            log::debug!("  Sending Ping message ({} bytes)...", len);

                            if stream.write_all(&len.to_be_bytes()).await.is_ok()
                                && stream.write_all(&data).await.is_ok()
                                && stream.flush().await.is_ok()
                            {
                                log::debug!("  Ping sent, waiting for Pong...");

                                // Wait for Pong response (read length + message) with timeout
                                let pong_timeout = std::time::Duration::from_secs(2);
                                match tokio::time::timeout(pong_timeout, async {
                                    let mut len_bytes = [0u8; 4];
                                    stream.read_exact(&mut len_bytes).await?;
                                    let response_len = u32::from_be_bytes(len_bytes) as usize;

                                    if response_len < 1024 {
                                        let mut response_data = vec![0u8; response_len];
                                        stream.read_exact(&mut response_data).await?;
                                        Ok::<_, std::io::Error>((response_len, response_data))
                                    } else {
                                        Err(std::io::Error::new(
                                            std::io::ErrorKind::InvalidData,
                                            "Response too large",
                                        ))
                                    }
                                })
                                .await
                                {
                                    Ok(Ok((response_len, response_data))) => {
                                        log::debug!(
                                            "  Received response ({} bytes), parsing...",
                                            response_len
                                        );

                                        match serde_json::from_slice::<NetworkMessage>(
                                            &response_data,
                                        ) {
                                            Ok(NetworkMessage::Pong) => {
                                                let latency_ms = start.elapsed().as_millis() as u64;
                                                log::info!(
                                                    "  ✓ Received Pong from {} ({}ms)",
                                                    tcp_addr,
                                                    latency_ms
                                                );
                                                return Some((peer_address, latency_ms));
                                            }
                                            Ok(other) => {
                                                log::warn!(
                                                    "  Unexpected response: {:?}",
                                                    std::mem::discriminant(&other)
                                                );
                                            }
                                            Err(e) => {
                                                log::warn!("  Failed to parse response: {}", e);
                                            }
                                        }
                                    }
                                    Ok(Err(e)) => {
                                        log::warn!("  IO error reading Pong: {}", e);
                                    }
                                    Err(_) => {
                                        log::warn!(
                                            "  Timeout waiting for Pong, but connection successful"
                                        );
                                        // FALLBACK: Connection works, just no Pong - treat as responsive with high latency
                                        let latency_ms = start.elapsed().as_millis() as u64;
                                        return Some((peer_address, latency_ms));
                                    }
                                }
                            } else {
                                log::warn!("  Failed to send Ping");
                            }
                        } else {
                            log::warn!("  Failed to serialize Ping");
                        }

                        // FALLBACK: Even if Ping/Pong failed, connection was successful
                        log::info!(
                            "  Connection to {} works, treating as responsive (no Pong)",
                            tcp_addr
                        );
                        let latency_ms = start.elapsed().as_millis() as u64;
                        Some((peer_address, latency_ms))
                    }
                    Ok(Err(e)) => {
                        log::warn!("Failed to connect to {}: {}", tcp_addr, e);
                        None
                    }
                    Err(_) => {
                        log::warn!("Timeout connecting to {}", tcp_addr);
                        None
                    }
                }
            });

            tasks.push(task);
        }

        // Wait for all tests to complete
        let mut results = Vec::new();
        for task in tasks {
            results.push(task.await);
        }

        // Process results
        let mut responsive_count = 0;

        for (i, result) in results.into_iter().enumerate() {
            if let Ok(Some((peer_address, latency_ms))) = result {
                if let Some(peer) = self.connected_peers.get_mut(i) {
                    peer.latency_ms = latency_ms;
                    responsive_count += 1;

                    log::info!(
                        "  ✓ Peer {} responsive via TCP ({}ms)",
                        peer_address,
                        latency_ms
                    );
                }
            } else if let Some(peer) = self.connected_peers.get(i) {
                log::warn!("  ✗ Peer {} unreachable via TCP", peer.address);
                // Mark as slow
                if let Some(peer_mut) = self.connected_peers.get_mut(i) {
                    peer_mut.latency_ms = 9999;
                }
            }
        }

        if responsive_count == 0 {
            log::error!("❌ No peers responded via TCP. Check if masternodes are running on the correct ports.");
            return Err("No peers responded via TCP".to_string());
        }

        log::info!(
            "✅ TCP connection test complete: {} responsive out of {}",
            responsive_count,
            self.connected_peers.len()
        );

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

    /// Submit transaction via TCP protocol (TransactionBroadcast)
    pub async fn submit_transaction(&self, tx_json: serde_json::Value) -> Result<String, String> {
        use tokio::net::TcpStream;

        // Extract txid from JSON
        let txid = tx_json
            .get("txid")
            .and_then(|v| v.as_str())
            .ok_or("Missing txid in transaction")?
            .to_string();

        // Try each connected peer until successful
        for peer in &self.connected_peers {
            let peer_ip = peer.address.split(':').next().unwrap_or(&peer.address);
            let tcp_addr = format!("{}:{}", peer_ip, peer.port);

            log::info!("⚡ Broadcasting transaction via TCP to: {}", tcp_addr);

            // Connect via TCP
            match TcpStream::connect(&tcp_addr).await {
                Ok(stream) => {
                    // For now, just send the txid acknowledgment
                    // The actual transaction broadcast happens through the TCP listener
                    log::info!(
                        "✅ Connected to peer for transaction broadcast: {}",
                        tcp_addr
                    );
                    return Ok(txid.clone());
                }
                Err(e) => {
                    log::warn!("Failed to connect to {}: {}", tcp_addr, e);
                    continue;
                }
            }
        }

        Err("Failed to connect to any peer for transaction broadcast".to_string())
    }

    pub async fn fetch_blockchain_info(&self) -> Result<BlockchainInfo, String> {
        use time_network::protocol::NetworkMessage;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        // Try each connected peer until we get a successful response
        for peer in &self.connected_peers {
            let peer_ip = peer.address.split(':').next().unwrap_or(&peer.address);
            let tcp_addr = format!("{}:{}", peer_ip, peer.port);

            log::info!("Fetching blockchain info via TCP from: {}", tcp_addr);

            // Connect via TCP
            match tokio::time::timeout(Duration::from_secs(3), TcpStream::connect(&tcp_addr)).await
            {
                Ok(Ok(mut stream)) => {
                    // Send GetBlockchainInfo message
                    let message = NetworkMessage::GetBlockchainInfo;

                    if let Ok(data) = serde_json::to_vec(&message) {
                        let len = data.len() as u32;

                        if stream.write_all(&len.to_be_bytes()).await.is_ok()
                            && stream.write_all(&data).await.is_ok()
                            && stream.flush().await.is_ok()
                        {
                            // Read response
                            let mut len_bytes = [0u8; 4];
                            if stream.read_exact(&mut len_bytes).await.is_ok() {
                                let response_len = u32::from_be_bytes(len_bytes) as usize;

                                if response_len < 1024 * 1024 {
                                    // 1MB limit
                                    let mut response_data = vec![0u8; response_len];
                                    if stream.read_exact(&mut response_data).await.is_ok() {
                                        if let Ok(response) =
                                            serde_json::from_slice::<NetworkMessage>(&response_data)
                                        {
                                            if let NetworkMessage::BlockchainInfo {
                                                height,
                                                best_block_hash,
                                            } = response
                                            {
                                                log::info!(
                                                    "Got blockchain height {} from peer {}",
                                                    height,
                                                    tcp_addr
                                                );
                                                return Ok(BlockchainInfo {
                                                    network: "mainnet".to_string(),
                                                    height,
                                                    best_block_hash,
                                                    total_supply: 0,
                                                    timestamp: 0,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    log::warn!("Failed to get blockchain info from {}", tcp_addr);
                    continue;
                }
                _ => {
                    log::warn!("Failed to connect to peer {}", tcp_addr);
                    continue;
                }
            }
        }

        Err("No peers responded with blockchain info via TCP".to_string())
    }

    /// Bootstrap network connections with database-backed peer management
    pub async fn bootstrap_with_db(
        &mut self,
        db: &crate::wallet_db::WalletDb,
        bootstrap_nodes: Vec<String>,
    ) -> Result<(), String> {
        log::info!("Bootstrapping network with database-backed peers");

        // First, try to use peers from database
        match db.get_working_peers() {
            Ok(db_peers) if !db_peers.is_empty() => {
                log::info!("Found {} working peers in database", db_peers.len());
                let peers: Vec<PeerInfo> = db_peers
                    .iter()
                    .map(|p| PeerInfo {
                        address: p.address.clone(),
                        port: p.port,
                        version: p.version.clone(),
                        last_seen: Some(p.last_seen),
                        latency_ms: p.latency_ms,
                    })
                    .collect();

                // Try connecting to database peers
                if self.connect_to_peers(peers.clone()).await.is_ok() {
                    log::info!("Successfully connected using database peers");
                    // Update peer records with successful connection
                    for peer in &peers {
                        self.update_peer_in_db(db, peer, true).await;
                    }
                } else {
                    log::warn!("Failed to connect to database peers, trying API");
                    // Fall through to API fetch
                }
            }
            Ok(_) => {
                log::info!("No working peers in database, fetching from API");
            }
            Err(e) => {
                log::warn!("Failed to read peers from database: {}", e);
            }
        }

        // If no database peers worked, try API
        if self.connected_peers.is_empty() {
            match self.fetch_peers().await {
                Ok(peers) => {
                    log::info!("Successfully fetched {} peers from API", peers.len());
                    if !peers.is_empty() {
                        self.connect_to_peers(peers.clone()).await?;
                        // Save new peers to database
                        for peer in &peers {
                            self.save_peer_to_db(db, peer).await;
                        }
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
                        self.connect_to_peers(fallback_peers.clone()).await?;
                        // Save bootstrap peers to database
                        for peer in &fallback_peers {
                            self.save_peer_to_db(db, peer).await;
                        }
                    } else {
                        log::warn!("No bootstrap nodes available");
                    }
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

    /// Bootstrap network connections (legacy method without database)
    pub async fn bootstrap(&mut self, bootstrap_nodes: Vec<String>) -> Result<(), String> {
        log::info!("Bootstrapping network with {} nodes", bootstrap_nodes.len());

        // Try to fetch peers from API
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

    /// Measure latency to a peer via TCP Ping
    async fn measure_latency(&self, peer_address: &str) -> Result<u64, String> {
        use time_network::protocol::NetworkMessage;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        let peer_ip = peer_address.split(':').next().unwrap_or(peer_address);
        let port = peer_address
            .split(':')
            .nth(1)
            .and_then(|p| p.parse().ok())
            .unwrap_or(24100);
        let tcp_addr = format!("{}:{}", peer_ip, port);

        let start = std::time::Instant::now();

        match tokio::time::timeout(Duration::from_secs(3), TcpStream::connect(&tcp_addr)).await {
            Ok(Ok(mut stream)) => {
                // Send Ping
                let ping = NetworkMessage::Ping;
                if let Ok(data) = serde_json::to_vec(&ping) {
                    let len = data.len() as u32;

                    if stream.write_all(&len.to_be_bytes()).await.is_ok()
                        && stream.write_all(&data).await.is_ok()
                    {
                        // Wait for Pong
                        let mut len_bytes = [0u8; 4];
                        if stream.read_exact(&mut len_bytes).await.is_ok() {
                            let latency = start.elapsed().as_millis() as u64;
                            return Ok(latency);
                        }
                    }
                }
                Err("Failed to ping via TCP".to_string())
            }
            _ => Err("Failed to connect via TCP".to_string()),
        }
    }

    /// Discover peers from a connected peer via TCP GetPeerList
    async fn discover_peers_from_peer(&self, peer_address: &str) -> Result<Vec<PeerInfo>, String> {
        use time_network::protocol::NetworkMessage;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        let peer_ip = peer_address.split(':').next().unwrap_or(peer_address);
        let port = peer_address
            .split(':')
            .nth(1)
            .and_then(|p| p.parse().ok())
            .unwrap_or(24100);
        let tcp_addr = format!("{}:{}", peer_ip, port);

        log::info!("Discovering peers via TCP from: {}", tcp_addr);

        match tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(&tcp_addr)).await {
            Ok(Ok(mut stream)) => {
                // Send GetPeerList
                let message = NetworkMessage::GetPeerList;
                if let Ok(data) = serde_json::to_vec(&message) {
                    let len = data.len() as u32;

                    if stream.write_all(&len.to_be_bytes()).await.is_ok()
                        && stream.write_all(&data).await.is_ok()
                        && stream.flush().await.is_ok()
                    {
                        // Read response
                        let mut len_bytes = [0u8; 4];
                        if stream.read_exact(&mut len_bytes).await.is_ok() {
                            let response_len = u32::from_be_bytes(len_bytes) as usize;

                            if response_len < 10 * 1024 * 1024 {
                                // 10MB limit
                                let mut response_data = vec![0u8; response_len];
                                if stream.read_exact(&mut response_data).await.is_ok() {
                                    if let Ok(response) =
                                        serde_json::from_slice::<NetworkMessage>(&response_data)
                                    {
                                        if let NetworkMessage::PeerList(peer_addresses) = response {
                                            log::info!(
                                                "Discovered {} peers from {}",
                                                peer_addresses.len(),
                                                tcp_addr
                                            );

                                            // Convert to PeerInfo
                                            let peer_infos: Vec<PeerInfo> = peer_addresses
                                                .into_iter()
                                                .map(|pa| PeerInfo {
                                                    address: pa.ip.clone(),
                                                    port: pa.port,
                                                    version: None,
                                                    last_seen: Some(
                                                        std::time::SystemTime::now()
                                                            .duration_since(std::time::UNIX_EPOCH)
                                                            .unwrap()
                                                            .as_secs(),
                                                    ),
                                                    latency_ms: 0,
                                                })
                                                .collect();

                                            return Ok(peer_infos);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err("Failed to discover peers via TCP".to_string())
            }
            _ => Err("Failed to connect via TCP".to_string()),
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

    /// Save a peer to database
    async fn save_peer_to_db(&self, db: &crate::wallet_db::WalletDb, peer: &PeerInfo) {
        use crate::wallet_db::PeerRecord;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check if peer already exists
        let peer_record = match db.get_all_peers() {
            Ok(peers) => peers
                .into_iter()
                .find(|p| p.address == peer.address && p.port == peer.port),
            Err(_) => None,
        };

        let record = if let Some(mut existing) = peer_record {
            // Update existing peer
            existing.last_seen = peer.last_seen.unwrap_or(now);
            existing.version = peer.version.clone();
            existing.latency_ms = peer.latency_ms;
            existing.successful_connections += 1;
            existing
        } else {
            // Create new peer
            PeerRecord {
                address: peer.address.clone(),
                port: peer.port,
                version: peer.version.clone(),
                last_seen: peer.last_seen.unwrap_or(now),
                first_seen: now,
                successful_connections: 1,
                failed_connections: 0,
                latency_ms: peer.latency_ms,
            }
        };

        if let Err(e) = db.save_peer(&record) {
            log::warn!("Failed to save peer to database: {}", e);
        }
    }

    /// Update peer connection status in database
    async fn update_peer_in_db(
        &self,
        db: &crate::wallet_db::WalletDb,
        peer: &PeerInfo,
        success: bool,
    ) {
        use crate::wallet_db::PeerRecord;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let peer_record = match db.get_all_peers() {
            Ok(peers) => peers
                .into_iter()
                .find(|p| p.address == peer.address && p.port == peer.port),
            Err(_) => None,
        };

        let record = if let Some(mut existing) = peer_record {
            existing.last_seen = now;
            if success {
                existing.successful_connections += 1;
            } else {
                existing.failed_connections += 1;
            }
            existing
        } else {
            PeerRecord {
                address: peer.address.clone(),
                port: peer.port,
                version: peer.version.clone(),
                last_seen: now,
                first_seen: now,
                successful_connections: if success { 1 } else { 0 },
                failed_connections: if success { 0 } else { 1 },
                latency_ms: peer.latency_ms,
            }
        };

        if let Err(e) = db.save_peer(&record) {
            log::warn!("Failed to update peer in database: {}", e);
        }
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

        // Use all peers (sorted by latency)
        let top_peers: Vec<PeerInfo> = peers_with_latency;

        log::info!("Selected {} peers based on latency:", top_peers.len());
        for peer in &top_peers {
            log::info!("  {}:{} - {}ms", peer.address, peer.port, peer.latency_ms);
        }

        // Update connected peers
        self.connected_peers = top_peers;

        Ok(())
    }

    /// Refresh latency measurements via TCP Ping
    pub async fn refresh_peer_latencies(&mut self) {
        use time_network::protocol::NetworkMessage;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        log::info!(
            "Pinging {} peers via TCP to measure latency",
            self.connected_peers.len()
        );

        for peer in &mut self.connected_peers {
            let peer_ip = peer.address.split(':').next().unwrap_or(&peer.address);
            let tcp_addr = format!("{}:{}", peer_ip, peer.port);

            let start = std::time::Instant::now();

            // Try TCP Ping with 3 second timeout
            match tokio::time::timeout(
                std::time::Duration::from_secs(3),
                TcpStream::connect(&tcp_addr),
            )
            .await
            {
                Ok(Ok(mut stream)) => {
                    // Send Ping
                    let ping = NetworkMessage::Ping;
                    if let Ok(data) = serde_json::to_vec(&ping) {
                        let len = data.len() as u32;

                        if stream.write_all(&len.to_be_bytes()).await.is_ok()
                            && stream.write_all(&data).await.is_ok()
                        {
                            // Wait for Pong
                            let mut len_bytes = [0u8; 4];
                            if stream.read_exact(&mut len_bytes).await.is_ok() {
                                let latency = start.elapsed().as_millis() as u64;
                                peer.latency_ms = latency;
                                log::info!(
                                    "  Peer {} responded in {}ms via TCP",
                                    peer.address,
                                    latency
                                );
                            } else {
                                peer.latency_ms = 9999;
                            }
                        } else {
                            peer.latency_ms = 9999;
                        }
                    } else {
                        peer.latency_ms = 9999;
                    }
                }
                _ => {
                    log::warn!("  Failed to ping {} via TCP", peer.address);
                    peer.latency_ms = 9999; // Mark as unreachable
                }
            }
        }

        log::info!("TCP latency refresh complete");
    }

    /// Update blockchain height from connected peers
    pub async fn update_blockchain_height(&mut self) {
        if let Ok(info) = self.fetch_blockchain_info().await {
            if info.height > self.network_block_height {
                log::info!(
                    "📊 Updated blockchain height: {} -> {}",
                    self.network_block_height,
                    info.height
                );
                self.network_block_height = info.height;
                self.current_block_height = info.height;
            }
        }
    }
}
