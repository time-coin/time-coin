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
    peer_exchange: Arc<RwLock<crate::peer_exchange::PeerExchange>>,
}

impl PeerManager {
    pub fn new(network: NetworkType, listen_addr: SocketAddr) -> Self {
        PeerManager {
            network,
            listen_addr,
            peers: Arc::new(RwLock::new(HashMap::new())),
            peer_exchange: Arc::new(RwLock::new(
                crate::peer_exchange::PeerExchange::new("/root/time-coin-node/data/peers.json".to_string())
            )),
        }
    }

    pub async fn connect_to_peer(&self, peer: PeerInfo) -> Result<(), String> {
        if let Ok(my_ip) = local_ip() {
            if peer.address.ip() == my_ip {
                return Ok(());
            }
        }
        if peer.address == self.listen_addr { return Ok(()); }

        let peer_addr = peer.address;
        let peer_arc = Arc::new(tokio::sync::Mutex::new(peer.clone()));

        match PeerConnection::connect(peer_arc.clone(), self.network.clone(), self.listen_addr).await {
            Ok(conn) => {
                let info = conn.peer_info().await;
                println!("✓ Connected to {} (v{})", info.address, info.version);

                self.peers.write().await.insert(peer_addr, info.clone());
                
                self.add_discovered_peer(
                    peer_addr.ip().to_string(),
                    peer_addr.port(),
                    info.version.clone()
                ).await;
                
                self.record_peer_success(&peer_addr.to_string()).await;

                let peers_clone = self.peers.clone();
                tokio::spawn(async move {
                    conn.keep_alive().await;
                    peers_clone.write().await.remove(&peer_addr);
                });
                Ok(())
            }
            Err(e) => {
                self.record_peer_failure(&peer_addr.to_string()).await;
                Err(e)
            }
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
        if peer.address.ip().is_unspecified() || peer.address == self.listen_addr {
            return;
        }
        let mut peers = self.peers.write().await;

        if let Some(existing) = peers.get(&peer.address) {
            if existing.version != "unknown" && peer.version == "unknown" {
                return;
            }
        }

        peers.insert(peer.address, peer.clone());
        
        self.add_discovered_peer(
            peer.address.ip().to_string(),
            peer.address.port(),
            peer.version.clone()
        ).await;
    }

    fn clone(&self) -> Self {
        PeerManager {
            network: self.network.clone(),
            listen_addr: self.listen_addr,
            peers: self.peers.clone(),
            peer_exchange: self.peer_exchange.clone(),
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

        let message = NetworkMessage::Transaction(tx);
        let _data = message.serialize()?;

        println!("📡 Broadcasting transaction to {} peer(s)...", peer_count);

        Ok(peer_count)
    }

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

    /// Request blockchain info (height) from a peer
    pub async fn request_blockchain_info(&self, peer_addr: &str) -> Result<u64, Box<dyn std::error::Error>> {
        let url = format!("http://{}:24101/blockchain/info", peer_addr.replace(":24100", ""));
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()?;
        
        let response = client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(format!("Failed to get blockchain info: {}", response.status()).into());
        }
        
        let info: serde_json::Value = response.json().await?;
        let height = info.get("height")
            .and_then(|h| h.as_u64())
            .ok_or("Invalid height in response")?;
        
        Ok(height)
    }


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
        let exchange = self.peer_exchange.read().await;
        exchange.peer_count()
    }
}
