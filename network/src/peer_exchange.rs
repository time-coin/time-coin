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
        let key = address.clone();

        if let Some(peer) = self.peers.get_mut(&key) {
            peer.last_seen = Utc::now().timestamp();
            peer.version = version;
            // Prefer non-ephemeral ports (< 49152) when updating existing peers
            if port < 49152 && peer.port >= 49152 {
                peer.port = port;
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn get_unique_test_path() -> String {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("/tmp/test_peers_{}_{}.json", timestamp, id)
    }

    #[test]
    fn test_peer_uses_ip_only_as_key() {
        let mut exchange = PeerExchange::new(get_unique_test_path());

        // Add a peer with ephemeral port
        exchange.add_peer("192.168.1.1".to_string(), 55000, "1.0.0".to_string());
        assert_eq!(exchange.peer_count(), 1);

        // Add same IP with different ephemeral port - should update, not duplicate
        exchange.add_peer("192.168.1.1".to_string(), 56000, "1.0.0".to_string());
        assert_eq!(exchange.peer_count(), 1);

        // Add different IP - should create new entry
        exchange.add_peer("192.168.1.2".to_string(), 55000, "1.0.0".to_string());
        assert_eq!(exchange.peer_count(), 2);
    }

    #[test]
    fn test_prefers_non_ephemeral_ports() {
        let mut exchange = PeerExchange::new(get_unique_test_path());

        // Add peer with ephemeral port first
        exchange.add_peer("192.168.1.1".to_string(), 55000, "1.0.0".to_string());
        let peer = exchange.peers.get("192.168.1.1").unwrap();
        assert_eq!(peer.port, 55000);

        // Update with standard port - should replace ephemeral port
        exchange.add_peer("192.168.1.1".to_string(), 24100, "1.0.1".to_string());
        let peer = exchange.peers.get("192.168.1.1").unwrap();
        assert_eq!(peer.port, 24100);
        assert_eq!(peer.version, "1.0.1");

        // Try to update with another ephemeral port - should keep standard port
        exchange.add_peer("192.168.1.1".to_string(), 56000, "1.0.2".to_string());
        let peer = exchange.peers.get("192.168.1.1").unwrap();
        assert_eq!(peer.port, 24100); // Port should remain at standard port
        assert_eq!(peer.version, "1.0.2"); // Version should still update
    }

    #[test]
    fn test_ephemeral_port_detection() {
        let mut exchange = PeerExchange::new(get_unique_test_path());

        // Ports below 49152 are not ephemeral
        exchange.add_peer("192.168.1.1".to_string(), 24100, "1.0.0".to_string());
        let peer = exchange.peers.get("192.168.1.1").unwrap();
        assert_eq!(peer.port, 24100);

        // Update with ephemeral port (>= 49152) should not replace standard port
        exchange.add_peer("192.168.1.1".to_string(), 49152, "1.0.1".to_string());
        let peer = exchange.peers.get("192.168.1.1").unwrap();
        assert_eq!(peer.port, 24100);

        // Update with another standard port should NOT replace (only replaces ephemeral with standard)
        exchange.add_peer("192.168.1.1".to_string(), 8080, "1.0.2".to_string());
        let peer = exchange.peers.get("192.168.1.1").unwrap();
        assert_eq!(peer.port, 24100); // Should keep the first standard port
    }
}
