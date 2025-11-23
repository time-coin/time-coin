//! Peer Manager for GUI Wallet
//!
//! Manages masternode peers, discovers new peers, and maintains connections.

use crate::wallet_db::{PeerRecord, WalletDb};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

pub struct PeerManager {
    wallet_db: Arc<RwLock<Option<WalletDb>>>,
    network: wallet::NetworkType,
}

impl PeerManager {
    pub fn new(network: wallet::NetworkType) -> Self {
        Self {
            wallet_db: Arc::new(RwLock::new(None)),
            network,
        }
    }

    /// Set the wallet database (called after wallet is initialized)
    pub async fn set_wallet_db(&self, db: WalletDb) {
        let mut wallet_db = self.wallet_db.write().await;
        *wallet_db = Some(db);
        log::info!("📂 Wallet database connected to PeerManager");
    }

    /// Helper to convert PeerRecord to display format
    fn peer_to_info(peer: &PeerRecord) -> (String, u16, i64) {
        (peer.address.clone(), peer.port, peer.last_seen as i64)
    }

    /// Helper to calculate peer score
    fn calculate_score(peer: &PeerRecord) -> i64 {
        let base_score = peer.successful_connections as i64 * 10;
        let penalty = peer.failed_connections as i64 * 20;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let age_penalty = (now as i64 - peer.last_seen as i64) / 3600;
        base_score - penalty - age_penalty
    }

    /// Add a peer
    pub async fn add_peer(&self, address: String, port: u16) {
        let db_guard = self.wallet_db.read().await;
        if let Some(db) = db_guard.as_ref() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let peer = PeerRecord {
                address: address.clone(),
                port,
                version: None,
                last_seen: now,
                first_seen: now,
                successful_connections: 0,
                failed_connections: 0,
                latency_ms: 0,
            };

            if let Err(e) = db.save_peer(&peer) {
                log::error!("❌ Failed to save peer: {}", e);
            } else {
                log::info!("➕ Added new peer: {}:{}", address, port);
            }
        }
    }

    /// Add multiple peers
    pub async fn add_peers(&self, new_peers: Vec<(String, u16)>) {
        let db_guard = self.wallet_db.read().await;
        if let Some(db) = db_guard.as_ref() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let mut added = 0;
            for (address, port) in new_peers {
                let peer = PeerRecord {
                    address: address.clone(),
                    port,
                    version: None,
                    last_seen: now,
                    first_seen: now,
                    successful_connections: 0,
                    failed_connections: 0,
                    latency_ms: 0,
                };

                if db.save_peer(&peer).is_ok() {
                    added += 1;
                }
            }

            if added > 0 {
                if let Ok(total) = db.get_all_peers() {
                    log::info!("➕ Added {} new peers (total: {})", added, total.len());
                }
            }
        }
    }

    /// Record successful connection
    pub async fn record_success(&self, address: &str, port: u16) {
        let db_guard = self.wallet_db.read().await;
        if let Some(db) = db_guard.as_ref() {
            if let Ok(peers) = db.get_all_peers() {
                if let Some(mut peer) = peers
                    .into_iter()
                    .find(|p| p.address == address && p.port == port)
                {
                    peer.successful_connections += 1;
                    peer.failed_connections = 0;
                    peer.last_seen = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    let _ = db.save_peer(&peer);
                }
            }
        }
    }

    /// Record failed connection
    pub async fn record_failure(&self, address: &str, port: u16) {
        let db_guard = self.wallet_db.read().await;
        if let Some(db) = db_guard.as_ref() {
            if let Ok(peers) = db.get_all_peers() {
                if let Some(mut peer) = peers
                    .into_iter()
                    .find(|p| p.address == address && p.port == port)
                {
                    peer.failed_connections += 1;
                    if peer.failed_connections < 5 {
                        let _ = db.save_peer(&peer);
                    } else {
                        log::warn!("🗑️ Removing unhealthy peer: {}:{}", address, port);
                        let _ = db.delete_peer(address, port);
                    }
                }
            }
        }
    }

    /// Get all healthy peers sorted by score
    pub async fn get_healthy_peers(&self) -> Vec<PeerRecord> {
        let db_guard = self.wallet_db.read().await;
        if let Some(db) = db_guard.as_ref() {
            if let Ok(peers) = db.get_all_peers() {
                let mut healthy: Vec<_> = peers
                    .into_iter()
                    .filter(|p| p.failed_connections < 5)
                    .collect();
                healthy.sort_by_key(|p| -Self::calculate_score(p));
                return healthy;
            }
        }
        Vec::new()
    }

    /// Get best peer to connect to
    pub async fn get_best_peer(&self) -> Option<PeerRecord> {
        self.get_healthy_peers().await.into_iter().next()
    }

    /// Get peer count
    pub async fn peer_count(&self) -> usize {
        let db_guard = self.wallet_db.read().await;
        if let Some(db) = db_guard.as_ref() {
            if let Ok(peers) = db.get_all_peers() {
                return peers.len();
            }
        }
        0
    }

    /// Bootstrap from seed peers
    pub async fn bootstrap(&self) -> Result<(), Box<dyn std::error::Error>> {
        let peer_count = self.peer_count().await;

        if peer_count == 0 {
            log::warn!("⚠️  No peers available! Please configure peers in wallet.conf using 'addnode' or via API endpoint");
            return Ok(());
        }

        log::info!("✓ Using {} configured peers", peer_count);

        // Immediately try to get more peers from the network
        log::info!("🔍 Discovering peers from network...");
        if let Some(new_peers) = self.try_get_peer_list().await {
            self.add_peers(new_peers).await;
            log::info!("✓ Peer discovery successful");
        } else {
            log::warn!("⚠️ Could not discover peers from network, will retry periodically");
        }

        Ok(())
    }

    /// Request peer list from a connected peer
    pub async fn request_peer_list(
        &self,
        stream: &mut tokio::net::TcpStream,
    ) -> Result<Vec<(String, u16)>, String> {
        use time_network::protocol::NetworkMessage;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Send GetPeerList request (using JSON like masternode)
        let request = NetworkMessage::GetPeerList;
        let request_bytes = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
        let len = request_bytes.len() as u32;

        log::info!("📤 Sending GetPeerList request ({} bytes)", len);
        log::debug!("Request JSON: {}", String::from_utf8_lossy(&request_bytes));

        stream
            .write_all(&len.to_be_bytes())
            .await
            .map_err(|e| format!("Failed to write length: {}", e))?;
        stream
            .write_all(&request_bytes)
            .await
            .map_err(|e| format!("Failed to write request: {}", e))?;
        stream
            .flush()
            .await
            .map_err(|e| format!("Failed to flush: {}", e))?;
        log::info!("✅ Request sent, waiting for response...");

        // Read response with timeout
        log::info!("📥 Reading response length...");
        let mut len_bytes = [0u8; 4];

        // Add timeout for reading response
        match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            stream.read_exact(&mut len_bytes)
        ).await {
            Ok(Ok(_)) => {
                let response_len = u32::from_be_bytes(len_bytes) as usize;
                log::info!("📥 Response length: {} bytes", response_len);

                if response_len > 10 * 1024 * 1024 {
                    return Err(format!("Response too large: {} bytes", response_len));
                }

                let mut response_bytes = vec![0u8; response_len];
                stream
                    .read_exact(&mut response_bytes)
                    .await
                    .map_err(|e| format!("Failed to read response body: {}", e))?;
                log::info!("📥 Response received, parsing...");
                log::debug!("Response JSON: {}", String::from_utf8_lossy(&response_bytes));

                let response: NetworkMessage = serde_json::from_slice(&response_bytes)
                    .map_err(|e| format!("Failed to parse response: {}", e))?;
                log::info!("✅ Response parsed successfully");

                match response {
                    NetworkMessage::PeerList(peer_addresses) => {
                        let peers: Vec<_> = peer_addresses
                            .into_iter()
                            .map(|pa| (pa.ip, pa.port))
                            .collect();
                        log::info!("Got {} peers from response", peers.len());
                        Ok(peers)
                    }
                    _ => Err("Unexpected response to GetPeerList".into()),
                }
            }
            Ok(Err(e)) => {
                Err(format!("Failed to read response length: {}", e))
            }
            Err(_) => {
                Err("Timeout waiting for response from masternode - masternode may not be responding to GetPeerList".to_string())
            }
        }
    }

    /// Try to get peer list from multiple peers until one succeeds
    pub async fn try_get_peer_list(&self) -> Option<Vec<(String, u16)>> {
        let healthy_peers = self.get_healthy_peers().await;

        if healthy_peers.is_empty() {
            log::warn!("⚠️ No healthy peers available");
            return None;
        }

        // Try up to 3 peers
        for peer in healthy_peers.iter().take(3) {
            let endpoint = format!("{}:{}", peer.address, peer.port);
            log::info!("🔍 Requesting peer list from {}", endpoint);

            match tokio::net::TcpStream::connect(&endpoint).await {
                Ok(mut stream) => {
                    log::info!("✅ TCP connection established to {}", endpoint);

                    match self.request_peer_list(&mut stream).await {
                        Ok(new_peers) => {
                            log::info!("📥 Received {} peers from {}", new_peers.len(), endpoint);
                            self.record_success(&peer.address, peer.port).await;
                            return Some(new_peers);
                        }
                        Err(e) => {
                            log::error!("❌ Failed to get peer list from {}: {}", endpoint, e);
                            log::error!("   This means the masternode at {} is NOT running the latest code with GetPeerList handler", endpoint);
                            log::error!("   The masternode needs to be rebuilt and restarted!");
                            self.record_failure(&peer.address, peer.port).await;
                        }
                    }
                }
                Err(e) => {
                    log::warn!("⚠️ Failed to connect to peer {}: {}", endpoint, e);
                    self.record_failure(&peer.address, peer.port).await;
                }
            }
        }

        None
    }

    /// Start periodic peer discovery and cleanup
    pub fn start_maintenance(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut tick = interval(Duration::from_secs(300)); // Every 5 minutes

            loop {
                tick.tick().await;

                // Try to discover new peers from healthy peers
                if let Some(new_peers) = self.try_get_peer_list().await {
                    self.add_peers(new_peers).await;
                } else {
                    log::warn!("⚠️ Failed to get peer list from any peer, will retry in 5 minutes");
                }

                // Peers are automatically saved to database, no periodic save needed
            }
        });
    }
}
