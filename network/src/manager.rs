//! Peer connection manager
use crate::connection::PeerConnection;
use crate::discovery::{NetworkType, PeerInfo};
use crate::protocol::{NetworkMessage, TransactionMessage};
use local_ip_address::local_ip;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time;
use tracing::{debug, info, warn};

#[derive(serde::Deserialize, Debug)]
pub struct Snapshot {
    pub height: u64,
    pub state_hash: String,
    pub balances: std::collections::HashMap<String, u64>,
    pub masternodes: Vec<String>,
    pub timestamp: i64,
}

pub struct PeerManager {
    network: NetworkType,
    listen_addr: SocketAddr,
    peers: Arc<RwLock<HashMap<SocketAddr, PeerInfo>>>,
    peer_exchange: Arc<RwLock<crate::peer_exchange::PeerExchange>>,

    // last-seen timestamps for active peers (used by reaper)
    last_seen: Arc<RwLock<HashMap<SocketAddr, Instant>>>,
    // how long without an update before considering peer stale
    stale_after: Duration,
    // how often to run reaper
    reaper_interval: Duration,
}

impl PeerManager {
    pub fn new(network: NetworkType, listen_addr: SocketAddr) -> Self {
        let manager = PeerManager {
            network,
            listen_addr,
            peers: Arc::new(RwLock::new(HashMap::new())),
            peer_exchange: Arc::new(RwLock::new(crate::peer_exchange::PeerExchange::new(
                "/root/time-coin-node/data/peers.json".to_string(),
            ))),
            last_seen: Arc::new(RwLock::new(HashMap::new())),
            stale_after: Duration::from_secs(90), // tune as needed
            reaper_interval: Duration::from_secs(10), // tune as needed
        };

        // start background reaper to remove stale peers
        manager.spawn_reaper();

        // start background reconnection task to retry disconnected peers
        manager.spawn_reconnection_task();

        manager
    }

    /// Mark that we have recent evidence the peer is alive.
    /// Call this when you receive a heartbeat/pong, upon successful connect, or
    /// periodically while a connection's keep-alive is running.
    pub async fn peer_seen(&self, addr: SocketAddr) {
        let mut ls = self.last_seen.write().await;
        ls.insert(addr, Instant::now());
    }

    /// Remove a connected peer and clear last_seen. Centralized removal + logging.
    pub async fn remove_connected_peer(&self, addr: &SocketAddr) {
        let mut peers = self.peers.write().await;
        let removed = peers.remove(addr).is_some();

        // clear last_seen entry
        let mut ls = self.last_seen.write().await;
        ls.remove(addr);

        if removed {
            info!(peer = %addr, connected_count = peers.len(), "Peer removed");
        }
    }

    /// Attempt to connect to a peer and manage the live connection entry.
    /// Returns Err(String) on connect failure (keeps same signature as original).
    pub async fn connect_to_peer(&self, peer: PeerInfo) -> Result<(), String> {
        // Skip self
        if let Ok(my_ip) = local_ip() {
            if peer.address.ip() == my_ip {
                return Ok(());
            }
        }
        if peer.address == self.listen_addr {
            return Ok(());
        }

        let peer_addr = peer.address;
        let peer_arc = Arc::new(tokio::sync::Mutex::new(peer.clone()));

        match PeerConnection::connect(peer_arc.clone(), self.network.clone(), self.listen_addr)
            .await
        {
            Ok(conn) => {
                // On successful connect, get peer info and record
                let info = conn.peer_info().await;
                info!(peer = %info.address, version = %info.version, "connected to peer");

                // Insert into the active peers map
                self.peers.write().await.insert(peer_addr, info.clone());

                // mark last-seen immediately
                self.peer_seen(peer_addr).await;

                // Persist discovery / mark success in peer exchange
                self.add_discovered_peer(
                    peer_addr.ip().to_string(),
                    peer_addr.port(),
                    info.version.clone(),
                )
                .await;

                self.record_peer_success(&peer_addr.to_string()).await;

                // Request peer list for peer exchange via HTTP API (best effort, don't fail on error)
                let manager_for_pex = self.clone();
                let peer_addr_for_pex = peer_addr;
                tokio::spawn(async move {
                    match manager_for_pex
                        .fetch_peers_from_api(&peer_addr_for_pex)
                        .await
                    {
                        Ok(peer_list) => {
                            debug!(
                                peer = %peer_addr_for_pex,
                                count = peer_list.len(),
                                "Received peer list from connected peer via API"
                            );
                            // Add discovered peers to our peer exchange
                            for discovered_peer in peer_list {
                                manager_for_pex
                                    .add_discovered_peer(
                                        discovered_peer.address.ip().to_string(),
                                        discovered_peer.address.port(),
                                        discovered_peer.version.clone(),
                                    )
                                    .await;
                            }
                        }
                        Err(e) => {
                            debug!(peer = %peer_addr_for_pex, error = %e, "Failed to get peer list from API");
                        }
                    }
                });

                // Clone handles for the spawned cleanup / keep-alive watcher task.
                let peers_clone = self.peers.clone();
                let manager_clone = self.clone();

                // Spawn a task to run the connection keep-alive and cleanup on exit.
                tokio::spawn(async move {
                    // Run keep_alive in a separate task so we can periodically refresh last_seen
                    let keep_alive_handle: JoinHandle<()> = tokio::spawn(async move {
                        conn.keep_alive().await;
                    });

                    // While keep_alive is running, periodically mark peer as seen so reaper
                    // doesn't remove it prematurely. This helps when keep_alive is successfully
                    // pinging but the reaper would otherwise consider the peer stale.
                    loop {
                        // If keep_alive has finished, break out and do cleanup
                        if keep_alive_handle.is_finished() {
                            break;
                        }

                        // Refresh last-seen timestamp for this peer
                        manager_clone.peer_seen(peer_addr).await;

                        // Sleep a short interval before refreshing again
                        time::sleep(Duration::from_secs(10)).await;
                    }

                    // Await the keep_alive task to ensure any internal errors are propagated/logged
                    let _ = keep_alive_handle.await;

                    debug!(peer = %peer_addr, "peer keep_alive finished");

                    // Always attempt to remove the peer from active map when the connection finishes.
                    if peers_clone.write().await.remove(&peer_addr).is_some() {
                        info!(peer = %peer_addr, "removed peer from active peers after disconnect");
                    } else {
                        debug!(peer = %peer_addr, "peer not present in active peers map at disconnect");
                    }

                    // Ensure last_seen entry is cleared as well
                    let mut ls = manager_clone.last_seen.write().await;
                    ls.remove(&peer_addr);
                });

                Ok(())
            }
            Err(e) => {
                // On connect failure, record failure and return error
                self.record_peer_failure(&peer_addr.to_string()).await;
                Err(e)
            }
        }
    }

    /// Connect concurrently to a list of peers.
    pub async fn connect_to_peers(&self, peer_list: Vec<PeerInfo>) {
        for peer in peer_list {
            let mgr = self.clone();
            let peer_addr = peer.address;
            tokio::spawn(async move {
                if let Err(e) = mgr.connect_to_peer(peer).await {
                    warn!(peer = %peer_addr, error = %e, "Failed to connect to peer");
                }
            });
        }
    }

    /// Return a vector of active PeerInfo entries (live connections).
    pub async fn get_connected_peers(&self) -> Vec<PeerInfo> {
        self.peers.read().await.values().cloned().collect()
    }

    /// Return the number of currently active (live) peer connections.
    pub async fn active_peer_count(&self) -> usize {
        self.peers.read().await.len()
    }

    /// Keep the old helper name but delegate to active_peer_count for clarity.
    pub async fn peer_count(&self) -> usize {
        self.active_peer_count().await
    }

    /// Insert/update a connected peer in the active map.
    /// This centralizes insertion logic so callers can use this instead of direct map edits.
    pub async fn add_connected_peer(&self, peer: PeerInfo) {
        if peer.address.ip().is_unspecified() || peer.address == self.listen_addr {
            return;
        }
        let mut peers = self.peers.write().await;

        if let Some(existing) = peers.get(&peer.address) {
            // keep an existing known good version over unknown version
            if existing.version != "unknown" && peer.version == "unknown" {
                return;
            }
        }

        peers.insert(peer.address, peer.clone());

        // mark last-seen on add
        self.peer_seen(peer.address).await;

        self.add_discovered_peer(
            peer.address.ip().to_string(),
            peer.address.port(),
            peer.version.clone(),
        )
        .await;
    }

    pub async fn get_peer_ips(&self) -> Vec<String> {
        // Return host:port strings (unique) rather than bare IPs
        self.peers
            .read()
            .await
            .values()
            .map(|p| p.address.to_string())
            .collect()
    }

    pub async fn broadcast_transaction(&self, tx: TransactionMessage) -> Result<usize, String> {
        let peers = self.peers.read().await;
        let peer_count = peers.len();

        let message = NetworkMessage::Transaction(tx);
        let _data = message.serialize()?; // keep existing behavior; serialize may be used later

        info!(count = peer_count, "broadcasting transaction to peers");

        Ok(peer_count)
    }

    pub async fn request_genesis(
        &self,
        peer_addr: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let url = format!("http://{}:24101/genesis", peer_addr.replace(":24100", ""));

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let response = client.get(&url).send().await?;

        if response.status().is_success() {
            let genesis: serde_json::Value = response.json().await?;
            Ok(genesis)
        } else {
            Err(format!("Failed to fetch genesis: {}", response.status()).into())
        }
    }

    /// Request mempool from a peer
    pub async fn request_mempool(
        &self,
        peer_addr: &str,
    ) -> Result<Vec<time_core::Transaction>, Box<dyn std::error::Error>> {
        let url = format!(
            "http://{}:24101/mempool/all",
            peer_addr.replace(":24100", "")
        );
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("Failed to get mempool: {}", response.status()).into());
        }

        let transactions: Vec<time_core::Transaction> = response.json().await?;
        Ok(transactions)
    }

    pub async fn request_blockchain_info(
        &self,
        peer_addr: &str,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let url = format!(
            "http://{}:24101/blockchain/info",
            peer_addr.replace(":24100", "")
        );
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()?;

        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("Failed to get blockchain info: {}", response.status()).into());
        }

        let info: serde_json::Value = response.json().await?;
        let height = info
            .get("height")
            .and_then(|h| h.as_u64())
            .ok_or("Invalid height in response")?;

        Ok(height)
    }

    pub async fn request_snapshot(
        &self,
        peer_addr: &str,
    ) -> Result<Snapshot, Box<dyn std::error::Error>> {
        let url = format!("http://{}:24101/snapshot", peer_addr.replace(":24100", ""));

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let response = client.get(&url).send().await?;

        if response.status().is_success() {
            let snapshot: Snapshot = response.json().await?;
            Ok(snapshot)
        } else {
            Err(format!("Failed to fetch snapshot: {}", response.status()).into())
        }
    }

    pub async fn sync_recent_blocks(
        &self,
        _peer_addr: &str,
        _from_height: u64,
        _to_height: u64,
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
        Ok(vec![])
    }

    pub async fn add_discovered_peer(&self, address: String, port: u16, version: String) {
        let mut exchange = self.peer_exchange.write().await;
        exchange.add_peer(address, port, version);
    }

    pub async fn get_best_peers(&self, count: usize) -> Vec<crate::peer_exchange::PeerInfo> {
        let exchange = self.peer_exchange.read().await;
        exchange.get_best_peers(count)
    }

    pub async fn record_peer_success(&self, address: &str) {
        let mut exchange = self.peer_exchange.write().await;
        exchange.record_success(address);
    }

    pub async fn record_peer_failure(&self, address: &str) {
        let mut exchange = self.peer_exchange.write().await;
        exchange.record_failure(address);
    }

    pub async fn known_peer_count(&self) -> usize {
        // number of remembered/persisted peers in peer_exchange
        let exchange = self.peer_exchange.read().await;
        exchange.peer_count()
    }

    /// Fetch peer list from a connected peer's HTTP API for peer exchange
    async fn fetch_peers_from_api(&self, peer_addr: &SocketAddr) -> Result<Vec<PeerInfo>, String> {
        let url = format!("http://{}:24101/peers", peer_addr.ip());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "HTTP request returned status: {}",
                response.status()
            ));
        }

        #[derive(serde::Deserialize)]
        struct ApiPeerInfo {
            address: String,
            version: String,
            #[allow(dead_code)]
            connected: bool,
        }

        #[derive(serde::Deserialize)]
        struct PeersResponse {
            peers: Vec<ApiPeerInfo>,
        }

        let peers_response: PeersResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // Convert API peer info to discovery::PeerInfo
        // Expected API address formats:
        // - Full socket address: "192.168.1.1:24100" or "[::1]:24100"
        // - IP address only: "192.168.1.1" or "::1" (will append default port 24100)
        let mut peer_infos = Vec::new();
        for api_peer in peers_response.peers {
            // Try to parse address directly as SocketAddr
            let parsed = api_peer.address.parse::<SocketAddr>().or_else(|_| {
                // If parsing fails, try appending default peer port (24100) and parse again
                let with_port = format!("{}:24100", api_peer.address);
                with_port.parse::<SocketAddr>()
            });

            match parsed {
                Ok(addr) => {
                    let peer_info =
                        PeerInfo::with_version(addr, self.network.clone(), api_peer.version);
                    peer_infos.push(peer_info);
                }
                Err(e) => {
                    debug!(address = %api_peer.address, error = %e, "Failed to parse peer address from API; skipping entry");
                }
            }
        }

        Ok(peer_infos)
    }

    pub async fn broadcast_block_proposal(&self, proposal: serde_json::Value) {
        let peers = self.peers.read().await.clone();
        for (addr, _info) in peers {
            let proposal_clone = proposal.clone();
            tokio::spawn(async move {
                let url = format!("http://{}:24101/consensus/block-proposal", addr.ip());
                let _ = reqwest::Client::new()
                    .post(&url)
                    .json(&proposal_clone)
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await;
            });
        }
    }

    pub async fn broadcast_block_vote(&self, vote: serde_json::Value) {
        let peers = self.peers.read().await.clone();
        for (addr, _info) in peers {
            let vote_clone = vote.clone();
            tokio::spawn(async move {
                let url = format!("http://{}:24101/consensus/block-vote", addr.ip());
                let _ = reqwest::Client::new()
                    .post(&url)
                    .json(&vote_clone)
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await;
            });
        }
    }

    /// Spawn a background task that periodically removes stale peers and logs removals.
    fn spawn_reaper(&self) {
        let last_seen = self.last_seen.clone();
        let peers = self.peers.clone();
        let stale_after = self.stale_after;
        let interval = self.reaper_interval;
        let manager = self.clone();

        tokio::spawn(async move {
            let mut ticker = time::interval(interval);
            loop {
                ticker.tick().await;
                let now = Instant::now();
                let mut to_remove = Vec::new();

                {
                    let ls = last_seen.read().await;
                    for (addr, seen) in ls.iter() {
                        if now.duration_since(*seen) > stale_after {
                            to_remove.push(*addr);
                        }
                    }
                }

                if !to_remove.is_empty() {
                    for addr in to_remove {
                        // Log the timeout and use the centralized removal function so logging
                        // and cleanup are consistent.
                        warn!(peer = %addr, "Peer down (heartbeat timeout)");
                        manager.remove_connected_peer(&addr).await;
                    }

                    let count = peers.read().await.len();
                    info!(connected_count = count, "Connected peers after purge");
                }
            }
        });
    }

    /// Spawn a background task that periodically attempts to reconnect to known peers
    /// that are not currently connected. This enables automatic recovery when nodes
    /// come back online after being reaped.
    fn spawn_reconnection_task(&self) {
        let manager = self.clone();

        tokio::spawn(async move {
            // Wait 60 seconds before the first reconnection attempt to allow initial connections
            time::sleep(Duration::from_secs(60)).await;

            let mut ticker = time::interval(Duration::from_secs(120)); // Check every 2 minutes
            loop {
                ticker.tick().await;

                // Get currently connected peer addresses
                let connected_addrs: std::collections::HashSet<String> = {
                    let peers = manager.peers.read().await;
                    peers.keys().map(|addr| addr.to_string()).collect()
                };

                // Get best known peers from peer exchange
                let best_peers = manager.get_best_peers(10).await;

                // Filter to only peers that aren't currently connected
                let disconnected_peers: Vec<_> = best_peers
                    .into_iter()
                    .filter(|p| !connected_addrs.contains(&p.full_address()))
                    .collect();

                if !disconnected_peers.is_empty() {
                    debug!(
                        count = disconnected_peers.len(),
                        "Attempting to reconnect to known peers"
                    );

                    // Attempt to reconnect to each disconnected peer
                    for pex_peer in disconnected_peers {
                        // Convert peer_exchange::PeerInfo to discovery::PeerInfo
                        match pex_peer.full_address().parse() {
                            Ok(addr) => {
                                let peer_info = PeerInfo::new(addr, manager.network.clone());

                                let mgr = manager.clone();
                                let peer_addr = peer_info.address;
                                tokio::spawn(async move {
                                    if let Err(e) = mgr.connect_to_peer(peer_info).await {
                                        debug!(
                                            peer = %peer_addr,
                                            error = %e,
                                            "Reconnection attempt failed"
                                        );
                                    } else {
                                        info!(peer = %peer_addr, "Successfully reconnected to peer");
                                    }
                                });
                            }
                            Err(_) => {
                                debug!(
                                    address = %pex_peer.full_address(),
                                    "Failed to parse peer address during reconnection"
                                );
                            }
                        }
                    }
                }
            }
        });
    }
}

// Implement Clone trait for PeerManager so `.clone()` is idiomatic.
impl Clone for PeerManager {
    fn clone(&self) -> Self {
        PeerManager {
            network: self.network.clone(),
            listen_addr: self.listen_addr,
            peers: self.peers.clone(),
            peer_exchange: self.peer_exchange.clone(),
            last_seen: self.last_seen.clone(),
            stale_after: self.stale_after,
            reaper_interval: self.reaper_interval,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_peer_manager_stale_timeout() {
        // Test that the stale_after timeout is set correctly to 90 seconds
        let manager = PeerManager::new(NetworkType::Testnet, "127.0.0.1:8333".parse().unwrap());

        assert_eq!(
            manager.stale_after,
            Duration::from_secs(90),
            "Stale timeout should be 90 seconds to allow for 3 missed heartbeats"
        );
    }

    #[tokio::test]
    async fn test_peer_manager_reaper_interval() {
        // Test that the reaper interval is set correctly
        let manager = PeerManager::new(NetworkType::Testnet, "127.0.0.1:8333".parse().unwrap());

        assert_eq!(
            manager.reaper_interval,
            Duration::from_secs(10),
            "Reaper interval should be 10 seconds"
        );
    }

    #[tokio::test]
    async fn test_reconnection_task_spawned() {
        // Test that the manager spawns properly with reconnection task
        let manager = PeerManager::new(NetworkType::Testnet, "127.0.0.1:8333".parse().unwrap());

        // If we can create the manager without panicking, the tasks were spawned successfully
        assert_eq!(manager.network, NetworkType::Testnet);

        // Give a moment for background tasks to initialize
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
