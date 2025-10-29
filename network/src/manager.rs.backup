//! Peer connection manager
use crate::connection::PeerConnection;
use crate::discovery::{NetworkType, PeerInfo};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use local_ip_address::local_ip;
use crate::protocol::{NetworkMessage, TransactionMessage};
use tokio::sync::RwLock;

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
}

impl PeerManager {
    pub fn new(network: NetworkType, listen_addr: SocketAddr) -> Self {
        PeerManager {
            network,
            listen_addr,
            peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn connect_to_peer(&self, peer: PeerInfo) -> Result<(), String> {
        // Skip if peer IP matches our local IP
        if let Ok(my_ip) = local_ip() {
            if peer.address.ip() == my_ip {
                return Ok(());
            }
        }
        if peer.address == self.listen_addr { return Ok(()); }

        let peer_addr = peer.address;
        let peer_arc = Arc::new(tokio::sync::Mutex::new(peer));

        match PeerConnection::connect(peer_arc.clone(), self.network.clone(), self.listen_addr).await {
            Ok(conn) => {
                let info = conn.peer_info().await;
                println!("✓ Connected to {} (v{})", info.address, info.version);

                self.peers.write().await.insert(peer_addr, info.clone());

                let peers_clone = self.peers.clone();
                tokio::spawn(async move {
                    conn.keep_alive().await;
                    peers_clone.write().await.remove(&peer_addr);
                });
                Ok(())
            }
            Err(e) => Err(e)
        }
    }

    pub async fn connect_to_peers(&self, peer_list: Vec<PeerInfo>) {
        for peer in peer_list {
            let mgr = self.clone();
            tokio::spawn(async move {
                let _ = mgr.connect_to_peer(peer).await;
            });
        }
    }

    pub async fn get_connected_peers(&self) -> Vec<PeerInfo> {
        self.peers.read().await.values().cloned().collect()
    }

    pub async fn peer_count(&self) -> usize {
        self.peers.read().await.len()
    }

    pub async fn add_connected_peer(&self, peer: PeerInfo) {
        // Skip adding self
        if peer.address.ip().is_unspecified() || peer.address == self.listen_addr {
            return;
        }
        let mut peers = self.peers.write().await;

        // Check if peer already exists with a known version
        if let Some(existing) = peers.get(&peer.address) {
            if existing.version != "unknown" && peer.version == "unknown" {
                return;
            }
        }

        peers.insert(peer.address, peer);
    }

    fn clone(&self) -> Self {
        PeerManager {
            network: self.network.clone(),
            listen_addr: self.listen_addr,
            peers: self.peers.clone(),
        }
    }

    pub async fn get_peer_ips(&self) -> Vec<String> {
        self.peers.read().await
            .values()
            .map(|p| p.address.ip().to_string())
            .collect()
    }

    pub async fn broadcast_transaction(&self, tx: TransactionMessage) -> Result<usize, String> {
        let peers = self.peers.read().await;
        let peer_count = peers.len();

        // Serialize the transaction
        let message = NetworkMessage::Transaction(tx);
        let _data = message.serialize()?;

        println!("📡 Broadcasting transaction to {} peer(s)...", peer_count);

        // TODO: Actually send to connected peers
        // For now, just return success
        Ok(peer_count)
    }

    /// Request genesis block from a peer
    pub async fn request_genesis(&self, peer_addr: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
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

    /// Request state snapshot from a peer
    pub async fn request_snapshot(&self, peer_addr: &str) -> Result<Snapshot, Box<dyn std::error::Error>> {
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

    /// Download recent blocks after snapshot
    pub async fn sync_recent_blocks(
        &self,
        _peer_addr: &str,
        _from_height: u64,
        _to_height: u64,
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
        // TODO: Implement block range download
        // For now, return empty as we don't have block storage yet
        Ok(vec![])
    }
}
