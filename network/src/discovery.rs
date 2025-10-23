//! Peer discovery system for TIME Coin network
//!
//! Multiple discovery methods:
//! 1. Hardcoded seed nodes (bootstrap)
//! 2. HTTP discovery from time-coin.io
//! 3. DNS seeds
//! 4. Peer exchange (PEX)

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::time::{Duration, SystemTime};

/// Peer information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PeerInfo {
    pub address: SocketAddr,
    pub last_seen: u64,
    pub version: String,
    pub network: NetworkType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NetworkType {
    Mainnet,
    Testnet,
}

/// Hardcoded seed nodes for bootstrap
pub struct SeedNodes;

impl SeedNodes {
    /// Mainnet seed nodes (hardcoded, always available)
    pub fn mainnet() -> Vec<&'static str> {
        vec![
            "seed1.time-coin.io:9876",
            "seed2.time-coin.io:9876",
            "seed3.time-coin.io:9876",
            "seed4.time-coin.io:9876",
            // Backup geographic distribution
            "us-seed.time-coin.io:9876",
            "eu-seed.time-coin.io:9876",
            "asia-seed.time-coin.io:9876",
        ]
    }

    /// Testnet seed nodes
    pub fn testnet() -> Vec<&'static str> {
        vec![
            "testnet-seed1.time-coin.io:9876",
            "testnet-seed2.time-coin.io:9876",
            "testnet-seed3.time-coin.io:9876",
        ]
    }

    /// Get seeds for specific network
    pub fn for_network(network: NetworkType) -> Vec<&'static str> {
        match network {
            NetworkType::Mainnet => Self::mainnet(),
            NetworkType::Testnet => Self::testnet(),
        }
    }
}

/// HTTP-based peer discovery
pub struct HttpDiscovery {
    base_url: String,
    client: reqwest::Client,
    network: NetworkType,
}

impl HttpDiscovery {
    /// Create new HTTP discovery client
    pub fn new(network: NetworkType) -> Self {
        let base_url = match network {
            NetworkType::Mainnet => "https://time-coin.io/api/peers",
            NetworkType::Testnet => "https://time-coin.io/api/peers",
        };

        HttpDiscovery {
            base_url: base_url.to_string(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("time-coin/1.0")  // Added user agent
                .build()
                .unwrap(),
            network,
        }
    }

    /// Fetch peer list from time-coin.io
    pub async fn fetch_peers(&self) -> Result<Vec<PeerInfo>, String> {
        let response = self
            .client
            .get(&self.base_url)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        // FIX: The API returns an array of strings ["IP:PORT", ...], not PeerInfo objects
        let peer_strings: Vec<String> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // Convert strings to PeerInfo objects
        let current_time = current_timestamp();
        let peers: Vec<PeerInfo> = peer_strings
            .into_iter()
            .filter_map(|addr_str| {
                addr_str.parse::<SocketAddr>().ok().map(|addr| PeerInfo {
                    address: addr,
                    last_seen: current_time,
                    version: "unknown".to_string(),
                    network: self.network.clone(),
                })
            })
            .collect();

        Ok(peers)
    }
}

/// DNS-based peer discovery
pub struct DnsDiscovery {
    dns_seeds: Vec<String>,
}

impl DnsDiscovery {
    /// Create new DNS discovery
    pub fn new(network: NetworkType) -> Self {
        let dns_seeds = match network {
            NetworkType::Mainnet => vec![
                "dnsseed.time-coin.io".to_string(),
                "seed.time-coin.io".to_string(),
            ],
            NetworkType::Testnet => vec!["testnet-dnsseed.time-coin.io".to_string()],
        };

        DnsDiscovery { dns_seeds }
    }

    /// Resolve DNS seeds to get peer addresses
    pub async fn resolve_peers(&self) -> Result<Vec<SocketAddr>, String> {
        let mut peers = Vec::new();

        for seed in &self.dns_seeds {
            // Create owned string that lives through the await
            let lookup_addr = format!("{}:9876", seed);

            // Now lookup_addr owns the string and lives long enough
            match tokio::net::lookup_host(lookup_addr).await {
                Ok(addrs) => {
                    for addr in addrs {
                        peers.push(addr);
                    }
                }
                Err(e) => {
                    eprintln!("DNS lookup failed for {}: {}", seed, e);
                }
            }
        }

        Ok(peers)
    }
}

/// Complete peer discovery system
pub struct PeerDiscovery {
    network: NetworkType,
    http_discovery: HttpDiscovery,
    dns_discovery: DnsDiscovery,
    known_peers: HashSet<PeerInfo>,
}

impl PeerDiscovery {
    /// Create new peer discovery system
    pub fn new(network: NetworkType) -> Self {
        PeerDiscovery {
            network: network.clone(),
            http_discovery: HttpDiscovery::new(network.clone()),
            dns_discovery: DnsDiscovery::new(network.clone()),
            known_peers: HashSet::new(),
        }
    }

    /// Bootstrap: Get initial peer list from all sources
    pub async fn bootstrap(&mut self) -> Result<Vec<PeerInfo>, String> {
        let mut all_peers = Vec::new();

        // 1. Start with hardcoded seed nodes (always works)
        println!("📡 Discovering peers from seed nodes...");
        let seed_addrs = SeedNodes::for_network(self.network.clone());
        for seed in seed_addrs {
            if let Ok(addr) = seed.parse() {
                all_peers.push(PeerInfo {
                    address: addr,
                    last_seen: current_timestamp(),
                    version: "unknown".to_string(),
                    network: self.network.clone(),
                });
            }
        }
        println!("  ✓ Found {} seed nodes", all_peers.len());

        // 2. Try HTTP discovery from time-coin.io
        println!("📡 Fetching peers from time-coin.io...");
        match self.http_discovery.fetch_peers().await {
            Ok(peers) => {
                println!("  ✓ Found {} peers via HTTP", peers.len());
                all_peers.extend(peers);
            }
            Err(e) => {
                println!("  ⚠ HTTP discovery failed: {}", e);
            }
        }

        // 3. Try DNS discovery
        println!("📡 Resolving DNS seeds...");
        match self.dns_discovery.resolve_peers().await {
            Ok(addrs) => {
                println!("  ✓ Found {} peers via DNS", addrs.len());
                for addr in addrs {
                    all_peers.push(PeerInfo {
                        address: addr,
                        last_seen: current_timestamp(),
                        version: "unknown".to_string(),
                        network: self.network.clone(),
                    });
                }
            }
            Err(e) => {
                println!("  ⚠ DNS discovery failed: {}", e);
            }
        }

        // Deduplicate peers
        let unique_peers: Vec<PeerInfo> = all_peers
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        println!("✓ Total unique peers discovered: {}", unique_peers.len());

        // Store in known peers
        self.known_peers.extend(unique_peers.iter().cloned());

        Ok(unique_peers)
    }

    /// Get peer list for initial connection
    pub fn get_bootstrap_peers(&self, max_peers: usize) -> Vec<PeerInfo> {
        self.known_peers.iter().take(max_peers).cloned().collect()
    }

    /// Add peer learned from peer exchange
    pub fn add_peer(&mut self, peer: PeerInfo) {
        self.known_peers.insert(peer);
    }

    /// Get all known peers
    pub fn all_peers(&self) -> Vec<PeerInfo> {
        self.known_peers.iter().cloned().collect()
    }

    /// Save peers to disk for next startup
    pub fn save_to_file(&self, path: &str) -> Result<(), String> {
        use std::fs::File;
        use std::io::Write;

        let json = serde_json::to_string_pretty(&self.known_peers)
            .map_err(|e| format!("Serialization failed: {}", e))?;

        let mut file = File::create(path).map_err(|e| format!("Failed to create file: {}", e))?;

        file.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    }

    /// Load peers from disk
    pub fn load_from_file(&mut self, path: &str) -> Result<(), String> {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let peers: HashSet<PeerInfo> =
            serde_json::from_str(&contents).map_err(|e| format!("Failed to parse peers: {}", e))?;

        self.known_peers.extend(peers);

        Ok(())
    }
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_seed_nodes_mainnet() {
        let seeds = SeedNodes::mainnet();
        assert!(!seeds.is_empty());
        assert!(seeds.len() >= 3);
    }

    #[test]
    fn test_seed_nodes_testnet() {
        let seeds = SeedNodes::testnet();
        assert!(!seeds.is_empty());
    }

    #[test]
    fn test_peer_info_hash() {
        let peer1 = PeerInfo {
            address: "127.0.0.1:9876".parse().unwrap(),
            last_seen: 12345,
            version: "1.0.0".to_string(),
            network: NetworkType::Mainnet,
        };

        let peer2 = peer1.clone();

        let mut set = HashSet::new();
        set.insert(peer1);
        set.insert(peer2);

        assert_eq!(set.len(), 1);
    }

    #[tokio::test]
    async fn test_peer_discovery_bootstrap() {
        let mut discovery = PeerDiscovery::new(NetworkType::Testnet);

        let result = discovery.bootstrap().await;
        assert!(result.is_ok());

        let peers = result.unwrap();
        assert!(!peers.is_empty());
    }

    #[tokio::test]
    async fn test_http_discovery() {
        let discovery = HttpDiscovery::new(NetworkType::Testnet);
        
        let result = discovery.fetch_peers().await;
        
        match result {
            Ok(peers) => {
                println!("Successfully fetched {} peers", peers.len());
                for peer in peers {
                    println!("  - {}", peer.address);
                }
            }
            Err(e) => {
                println!("Failed to fetch peers: {}", e);
            }
        }
    }
}