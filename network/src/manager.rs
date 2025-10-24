//! Peer connection manager
use crate::connection::PeerConnection;
use crate::discovery::{NetworkType, PeerInfo};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

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
}
