use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub address: String,
    pub port: u16,
    pub last_seen: i64,
    pub latency_ms: Option<u32>,
    pub version: String,
    pub successful_connections: u32,
    pub failed_connections: u32,
}

impl PeerInfo {
    pub fn new(address: String, port: u16, version: String) -> Self {
        Self {
            address,
            port,
            last_seen: Utc::now().timestamp(),
            latency_ms: None,
            version,
            successful_connections: 0,
            failed_connections: 0,
        }
    }

    pub fn update_latency(&mut self, latency: u32) {
        self.latency_ms = Some(latency);
        self.last_seen = Utc::now().timestamp();
    }

    pub fn record_success(&mut self) {
        self.successful_connections += 1;
        self.last_seen = Utc::now().timestamp();
    }

    pub fn record_failure(&mut self) {
        self.failed_connections += 1;
    }

    pub fn reliability_score(&self) -> f32 {
        let total = self.successful_connections + self.failed_connections;
        if total == 0 {
            return 0.5;
        }
        self.successful_connections as f32 / total as f32
    }

    pub fn full_address(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }
}

pub struct PeerExchange {
    peers: HashMap<String, PeerInfo>,
    storage_path: String,
}

impl PeerExchange {
    pub fn new(storage_path: String) -> Self {
        let mut exchange = Self {
            peers: HashMap::new(),
            storage_path,
        };
        exchange.load_from_disk();
        exchange
    }

    pub fn add_peer(&mut self, address: String, port: u16, version: String) {
        let key = format!("{}:{}", address, port);

        if let Some(peer) = self.peers.get_mut(&key) {
            peer.last_seen = Utc::now().timestamp();
            peer.version = version;
        } else {
            self.peers
                .insert(key, PeerInfo::new(address, port, version));
        }

        self.save_to_disk();
    }

    pub fn update_latency(&mut self, address: &str, latency: u32) {
        if let Some(peer) = self.peers.get_mut(address) {
            peer.update_latency(latency);
            self.save_to_disk();
        }
    }

    pub fn record_success(&mut self, address: &str) {
        if let Some(peer) = self.peers.get_mut(address) {
            peer.record_success();
            self.save_to_disk();
        }
    }

    pub fn record_failure(&mut self, address: &str) {
        if let Some(peer) = self.peers.get_mut(address) {
            peer.record_failure();
            self.save_to_disk();
        }
    }

    pub fn get_best_peers(&self, count: usize) -> Vec<PeerInfo> {
        let mut peers: Vec<PeerInfo> = self.peers.values().cloned().collect();

        let cutoff = Utc::now().timestamp() - 86400;
        peers.retain(|p| p.last_seen > cutoff);
        peers.retain(|p| p.reliability_score() >= 0.3);

        peers.sort_by(|a, b| match (a.latency_ms, b.latency_ms) {
            (Some(a_lat), Some(b_lat)) => {
                let lat_cmp = a_lat.cmp(&b_lat);
                if lat_cmp == std::cmp::Ordering::Equal {
                    b.reliability_score()
                        .partial_cmp(&a.reliability_score())
                        .unwrap()
                } else {
                    lat_cmp
                }
            }
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => b
                .reliability_score()
                .partial_cmp(&a.reliability_score())
                .unwrap(),
        });

        peers.into_iter().take(count).collect()
    }

    pub fn get_all_addresses(&self) -> Vec<String> {
        self.peers.values().map(|p| p.full_address()).collect()
    }

    fn load_from_disk(&mut self) {
        if let Ok(data) = fs::read_to_string(&self.storage_path) {
            if let Ok(peers) = serde_json::from_str(&data) {
                self.peers = peers;
                println!("✓ Loaded {} known peers from disk", self.peers.len());
            }
        }
    }

    fn save_to_disk(&self) {
        if let Some(parent) = Path::new(&self.storage_path).parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(data) = serde_json::to_string_pretty(&self.peers) {
            let _ = fs::write(&self.storage_path, data);
        }
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }
}
