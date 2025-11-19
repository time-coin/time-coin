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

    #[serde(default)]
    pub commit_date: Option<String>,

    #[serde(default)]
    pub commit_count: Option<String>,

    #[serde(default)]
    pub wallet_address: Option<String>,
}

impl PeerInfo {
    /// Create a new peer with unknown version
    pub fn new(address: SocketAddr, network: NetworkType) -> Self {
        PeerInfo {
            address, // <-- use 'address' not 'addr'
            last_seen: current_timestamp(),
            version: "unknown".to_string(),
            network, // <-- use 'network' not 'self.network'
            commit_date: None,
            commit_count: None,
            wallet_address: None,
        }
    }

    /// Create a new peer with known version
    pub fn with_version(address: SocketAddr, network: NetworkType, version: String) -> Self {
        PeerInfo {
            address, // <-- use 'address' not 'peer_addr'
            last_seen: current_timestamp(),
            version,           // <-- use 'version' not 'peer.version'
            network,           // <-- use 'network' not 'self.network'
            commit_date: None, // <-- These should be None for this constructor
            commit_count: None,
            wallet_address: None,
        }
    }

    /// Update the peer's version (called after handshake)
    pub fn update_version(&mut self, version: String) {
        self.version = version;
        self.last_seen = current_timestamp();
    }

    /// Update the peer's version with commit info (called after handshake)
    pub fn update_version_with_build_info(
        &mut self,
        version: String,
        commit_date: Option<String>,
        commit_count: Option<String>,
    ) {
        self.version = version;
        self.commit_date = commit_date;
        self.commit_count = commit_count;
        self.last_seen = current_timestamp();
    }

    /// Update last seen timestamp
    pub fn touch(&mut self) {
        self.last_seen = current_timestamp();
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NetworkType {
    Mainnet,
    Testnet,
}

/// Seed nodes for bootstrap
pub struct SeedNodes;

impl SeedNodes {
    /// Get seed nodes from environment or use DNS-based discovery
    /// For security, seed nodes should be configured via:
    /// - TIMECOIN_SEEDS environment variable (comma-separated)
    /// - DNS seeds (seed.time-coin.io, dnsseed.time-coin.io)
    pub fn mainnet() -> Vec<&'static str> {
        // No hardcoded IPs - use DNS or environment
        vec![]
    }

    /// Testnet seed nodes from environment
    pub fn testnet() -> Vec<&'static str> {
        // No hardcoded IPs - use DNS or environment
        vec![]
    }

    /// Get seeds for specific network
    pub fn for_network(network: NetworkType) -> Vec<&'static str> {
        match network {
            NetworkType::Mainnet => Self::mainnet(),
            NetworkType::Testnet => Self::testnet(),
        }
    }

    /// Get seeds from environment variable
    /// Format: TIMECOIN_SEEDS="ip1:port1,ip2:port2"
    pub fn from_env() -> Vec<String> {
        std::env::var("TIMECOIN_SEEDS")
            .ok()
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default()
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
        // The API endpoint is the same for both networks
        // The actual network filtering happens based on port numbers
        let base_url = "https://time-coin.io/api/peers";

        HttpDiscovery {
            base_url: base_url.to_string(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("time-coin-node/1.0")
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

        // Filter peers by network based on port
        // Mainnet uses port 24000, Testnet uses port 24100
        let expected_port = match self.network {
            NetworkType::Mainnet => 24000,
            NetworkType::Testnet => 24100,
        };

        let peers: Vec<PeerInfo> = peer_strings
            .into_iter()
            .filter_map(|addr_str| {
                addr_str.parse::<SocketAddr>().ok().and_then(|addr| {
                    // Only include peers matching the expected port for this network
                    if addr.port() == expected_port {
                        Some(PeerInfo::new(addr, self.network.clone()))
                    } else {
                        None
                    }
                })
            })
            .collect();

        Ok(peers)
    }
}

/// DNS-based peer discovery
pub struct DnsDiscovery {
    dns_seeds: Vec<String>,
    network: NetworkType,
}

impl DnsDiscovery {
    /// Create new DNS discovery
    pub fn new(network: NetworkType) -> Self {
        let dns_seeds = match network {
            NetworkType::Mainnet => vec![
                "dnsseed.time-coin.io".to_string(),
                "seed.time-coin.io".to_string(),
            ],
            NetworkType::Testnet => vec![], // DNS seeder not yet deployed
        };

        DnsDiscovery { dns_seeds, network }
    }

    /// Resolve DNS seeds to get peer addresses
    pub async fn resolve_peers(&self) -> Result<Vec<SocketAddr>, String> {
        let mut peers = Vec::new();

        // Use the correct port based on network type
        let port = match self.network {
            NetworkType::Mainnet => 24000,
            NetworkType::Testnet => 24100,
        };

        for seed in &self.dns_seeds {
            // Use network-appropriate port instead of hardcoded 9876
            let lookup_addr = format!("{}:{}", seed, port);

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

        // 1. Start with environment-based seed nodes (if configured)
        let env_seeds = SeedNodes::from_env();
        if !env_seeds.is_empty() {
            println!("📡 Discovering peers from environment seeds...");
            for seed in env_seeds {
                if let Ok(addr) = seed.parse() {
                    all_peers.push(PeerInfo::new(addr, self.network.clone()));
                }
            }
            println!("  ✓ Found {} seed nodes from environment", all_peers.len());
        }

        // 2. Try HTTP discovery from time-coin.io
        println!("📡 Fetching peers from time-coin.io...");
        match self.http_discovery.fetch_peers().await {
            Ok(peers) => {
                let http_count = peers.len();
                println!("  ✓ Found {} peers via HTTP", http_count);
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
                    all_peers.push(PeerInfo::new(addr, self.network.clone()));
                }
            }
            Err(e) => {
                println!("  ⚠ DNS discovery failed: {}", e);
            }
        }

        // Deduplicate peers by address only
        let total_before = all_peers.len();
        let mut seen_addresses = std::collections::HashSet::new();
        let unique_peers: Vec<PeerInfo> = all_peers
            .into_iter()
            .filter(|peer| seen_addresses.insert(peer.address))
            .collect();

        if total_before != unique_peers.len() {
            println!("  ✓ Deduplicated to {} unique peers", unique_peers.len());
        }
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
        // No hardcoded seeds anymore - should be empty (use environment or DNS)
        assert_eq!(seeds.len(), 0);
    }

    #[test]
    fn test_seed_nodes_testnet() {
        let seeds = SeedNodes::testnet();
        // No hardcoded seeds anymore - should be empty (use environment or DNS)
        assert_eq!(seeds.len(), 0);
    }

    #[test]
    fn test_seed_nodes_from_env() {
        // Test that from_env works correctly
        std::env::set_var("TIMECOIN_SEEDS", "192.168.1.1:24100,192.168.1.2:24100");
        let seeds = SeedNodes::from_env();
        assert_eq!(seeds.len(), 2);
        assert_eq!(seeds[0], "192.168.1.1:24100");
        assert_eq!(seeds[1], "192.168.1.2:24100");
        std::env::remove_var("TIMECOIN_SEEDS");
    }

    #[test]
    fn test_peer_info_hash() {
        let peer1 = PeerInfo::new("127.0.0.1:9876".parse().unwrap(), NetworkType::Mainnet);

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

    #[test]
    fn test_peer_deduplication() {
        // Test that deduplication works correctly
        let peer1 = PeerInfo::new("192.168.1.1:8333".parse().unwrap(), NetworkType::Mainnet);
        let peer2 = PeerInfo::new("192.168.1.2:8333".parse().unwrap(), NetworkType::Mainnet);
        let peer3 = PeerInfo::new("192.168.1.1:8333".parse().unwrap(), NetworkType::Mainnet); // Duplicate of peer1

        let peers = vec![peer1.clone(), peer2.clone(), peer3.clone()];

        // Simulate deduplication logic from bootstrap()
        let mut seen_addresses = std::collections::HashSet::new();
        let unique_peers: Vec<PeerInfo> = peers
            .into_iter()
            .filter(|peer| seen_addresses.insert(peer.address))
            .collect();

        assert_eq!(
            unique_peers.len(),
            2,
            "Should have 2 unique peers after deduplication"
        );
        assert!(unique_peers.iter().any(|p| p.address == peer1.address));
        assert!(unique_peers.iter().any(|p| p.address == peer2.address));
    }
}
