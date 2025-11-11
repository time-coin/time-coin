//! Network protocol for peer communication
//!
//! Handles handshakes, version exchange, and peer identification

use crate::discovery::NetworkType;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Current TIME Coin version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Git information (set at build time)
pub const GIT_HASH: &str = env!("GIT_HASH");
pub const GIT_BRANCH: &str = env!("GIT_BRANCH");
pub const GIT_COMMIT_DATE: &str = env!("GIT_COMMIT_DATE");
pub const GIT_COMMIT_COUNT: &str = env!("GIT_COMMIT_COUNT");

/// Build information (set at build time)
pub const BUILD_TIMESTAMP: &str = env!("BUILD_TIMESTAMP");
pub const GIT_MESSAGE: &str = env!("GIT_MESSAGE");

/// Get full version with git hash
pub fn full_version() -> String {
    // Try to get current git hash at runtime for freshness
    let runtime_hash = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        });

    let hash = runtime_hash.unwrap_or_else(|| GIT_HASH.to_string());
    format!("{}-{}", VERSION, hash)
}

/// Get version with complete build information
pub fn version_with_build_info() -> String {
    format!(
        "v{} | Branch: {} | Committed: {} | Commits: {}",
        full_version(),
        GIT_BRANCH,
        GIT_COMMIT_DATE,
        GIT_COMMIT_COUNT
    )
}

/// Get detailed build information
pub fn build_info_detailed() -> String {
    format!(
        "Version:        {}\n\
         Git Branch:    {}\n\
         Git Commit:    {} (#{})\n\
         Commit Date:   {}\n\
         Message:       {}",
        full_version(),
        GIT_BRANCH,
        GIT_HASH,
        GIT_COMMIT_COUNT,
        GIT_COMMIT_DATE,
        GIT_MESSAGE
    )
}

/// Get version for API/handshake (without build time for deterministic responses)
pub fn version_for_handshake() -> String {
    full_version()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildInfo {
    pub version: String,
    pub git_hash: String,
    pub git_branch: String,
    pub commit_date: String,
    pub git_commit_count: u64,
}

impl BuildInfo {
    /// Create build info from compile-time constants
    pub fn current() -> Self {
        BuildInfo {
            version: full_version(),
            git_hash: GIT_HASH.to_string(),
            git_branch: GIT_BRANCH.to_string(),
            commit_date: GIT_COMMIT_DATE.to_string(),
            git_commit_count: GIT_COMMIT_COUNT.parse().unwrap_or(0),
        }
    }
}

/// Protocol version for compatibility checking
pub const PROTOCOL_VERSION: u32 = 1;

/// Handshake message sent when connecting to peers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeMessage {
    /// Software version (e.g., "0.1.0-9569fe2")
    pub version: String,

    /// Git commit date (e.g., "2025-11-07T15:09:21Z")
    #[serde(default)]
    pub commit_date: Option<String>,

    /// Git commit count
    #[serde(default)]
    pub commit_count: Option<String>,

    /// Protocol version for compatibility
    pub protocol_version: u32,

    /// Network type (Mainnet or Testnet)
    pub network: NetworkType,

    /// Peer's listening address
    pub listen_addr: SocketAddr,

    /// Timestamp of connection
    pub timestamp: u64,

    /// Node capabilities (future use)
    pub capabilities: Vec<String>,

    /// Wallet address for masternode rewards
    #[serde(default)]
    pub wallet_address: Option<String>,
}

impl HandshakeMessage {
    /// Create a new handshake message with optional wallet
    pub fn new(network: NetworkType, listen_addr: SocketAddr) -> Self {
        let wallet_address = std::env::var("MASTERNODE_WALLET").ok();

        HandshakeMessage {
            version: version_for_handshake(),
            commit_date: Some(GIT_COMMIT_DATE.to_string()),
            commit_count: Some(GIT_COMMIT_COUNT.to_string()),
            protocol_version: PROTOCOL_VERSION,
            network,
            listen_addr,
            timestamp: current_timestamp(),
            capabilities: vec!["masternode".to_string(), "sync".to_string()],
            wallet_address,
        }
    }

    /// Validate handshake from peer
    pub fn validate(&self, expected_network: &NetworkType) -> Result<(), String> {
        if &self.network != expected_network {
            return Err(format!(
                "Network mismatch: expected {:?}, got {:?}",
                expected_network, self.network
            ));
        }

        if self.protocol_version != PROTOCOL_VERSION {
            return Err(format!(
                "Protocol version mismatch: expected {}, got {}",
                PROTOCOL_VERSION, self.protocol_version
            ));
        }

        Ok(())
    }

    /// Check if versions are compatible
    pub fn is_compatible(&self) -> bool {
        self.protocol_version == PROTOCOL_VERSION
    }
}

/// Protocol version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub software_version: String,
    pub commit_date: Option<String>,
    pub protocol_version: u32,
}

impl ProtocolVersion {
    pub fn current() -> Self {
        ProtocolVersion {
            software_version: VERSION.to_string(),
            commit_date: Some(GIT_COMMIT_DATE.to_string()),
            protocol_version: PROTOCOL_VERSION,
        }
    }
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Check if a peer version is outdated
/// Takes peer_version and returns true if versions differ
pub fn is_version_outdated(peer_version: &str) -> bool {
    is_version_outdated_with_build(peer_version, None)
}

/// Check if a peer version is outdated with optional build timestamp
pub fn is_version_outdated_with_build(peer_version: &str, peer_build: Option<&str>) -> bool {
    if peer_version == "unknown" {
        return false;
    }

    let current_hash = GIT_HASH;
    let peer_hash = peer_version.split('-').next_back().unwrap_or("");

    // Different git commits mean different versions
    if current_hash != peer_hash && !peer_hash.is_empty() {
        return true;
    }

    // Same commit - check build time if available
    if let Some(_peer_build_str) = peer_build {
        // Same commit and similar build times = not outdated
        // Different build times from same commit = may indicate different builds
        // For now, same commit = compatible
        return false;
    }

    false
}

/// Get a detailed version mismatch message
pub fn version_mismatch_message_detailed(
    peer_addr: &str,
    peer_version: &str,
    peer_build: Option<&str>,
) -> String {
    match peer_build {
        Some(build_str) => format!(
            "⚠️  Peer {} is running v{} (committed: {}). \
             You are running {} (committed: {}). \
             Please ensure versions match!",
            peer_addr,
            peer_version,
            build_str,
            full_version(),
            GIT_COMMIT_DATE
        ),
        None => format!(
            "⚠️  Peer {} is running version {} (current: {}). Please update!",
            peer_addr,
            peer_version,
            full_version()
        ),
    }
}

/// Get a user-friendly version mismatch message (backward compatible)
pub fn version_mismatch_message(peer_addr: &str, peer_version: &str) -> String {
    version_mismatch_message_detailed(peer_addr, peer_version, None)
}

// ═══════════════════════════════════════════════════════════════
// VERSION COMPARISON AND UPDATE DETECTION
// ═══════════════════════════════════════════════════════════════

/// Compare two timestamps in ISO 8601 format and return true if remote is newer
pub fn is_remote_version_newer(local_timestamp: &str, remote_timestamp: &str) -> bool {
    use chrono::DateTime;

    // Parse ISO 8601 format (e.g., "2025-11-07T15:09:21Z")
    let local_dt = DateTime::parse_from_rfc3339(local_timestamp).ok();
    let remote_dt = DateTime::parse_from_rfc3339(remote_timestamp).ok();

    match (local_dt, remote_dt) {
        (Some(local), Some(remote)) => remote > local,
        _ => false,
    }
}

/// Compare two git commit counts and return true if remote is newer
pub fn is_remote_commit_newer(local_count: &str, remote_count: &str) -> bool {
    let local: u64 = local_count.parse().unwrap_or(0);
    let remote: u64 = remote_count.parse().unwrap_or(0);
    remote > local
}

/// Get a detailed version comparison message
pub fn version_update_warning(
    peer_addr: &str,
    peer_version: &str,
    peer_commit_date: &str,
    peer_commit_count: &str,
) -> String {
    format!(
        "\n\
        ╔══════════════════════════════════════════════════════════════╗\n\
        ║  ⚠️  UPDATE AVAILABLE - NEWER VERSION DETECTED              ║\n\
        ╚══════════════════════════════════════════════════════════════╝\n\
        \n\
        Peer {} is running a NEWER version:\n\
        \n\
        Peer Version:   {} (commit #{})\n\
        Peer Committed: {}\n\
        \n\
        Your Version:   {} (commit #{})\n\
        Your Committed: {}\n\
        \n\
        ⚠️  RECOMMENDED ACTION:\n\
        1. Update your node to the latest version\n\
        2. Run: git pull && cargo build --release\n\
        3. Restart your node service\n\
        \n\
        Running outdated software may cause:\n\
        - Consensus incompatibilities\n\
        - Missing important bug fixes\n\
        - Reduced network participation\n\
        ╚══════════════════════════════════════════════════════════════╝\n",
        peer_addr,
        peer_version,
        peer_commit_count,
        peer_commit_date,
        full_version(),
        GIT_COMMIT_COUNT,
        GIT_COMMIT_DATE
    )
}

/// Check if we should warn about version mismatch (only warn for newer versions)
pub fn should_warn_version_update(
    peer_build: Option<&str>,
    peer_commit_count: Option<&str>,
) -> bool {
    // First check commit counts - this is the primary version indicator
    if let Some(peer_commits) = peer_commit_count {
        if is_remote_commit_newer(GIT_COMMIT_COUNT, peer_commits) {
            return true;
        }
        // If peer has same or older commit count, no update needed
        if peer_commits == GIT_COMMIT_COUNT {
            return false;
        }
    }

    // Only check commit date if commit count is unavailable or older
    if let Some(peer_commit_date) = peer_build {
        if is_remote_version_newer(GIT_COMMIT_DATE, peer_commit_date) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handshake_creation() {
        let addr = "127.0.0.1:24100".parse().unwrap();
        let handshake = HandshakeMessage::new(NetworkType::Testnet, addr);

        assert_eq!(handshake.version, full_version());
        assert_eq!(handshake.protocol_version, PROTOCOL_VERSION);
        assert_eq!(handshake.network, NetworkType::Testnet);
        assert!(handshake.commit_date.is_some());
    }

    #[test]
    fn test_handshake_validation() {
        let addr = "127.0.0.1:24100".parse().unwrap();
        let handshake = HandshakeMessage::new(NetworkType::Testnet, addr);

        assert!(handshake.validate(&NetworkType::Testnet).is_ok());
        assert!(handshake.validate(&NetworkType::Mainnet).is_err());
    }

    #[test]
    fn test_protocol_version() {
        let version = ProtocolVersion::current();
        assert_eq!(version.software_version, VERSION);
        assert_eq!(version.protocol_version, PROTOCOL_VERSION);
        assert!(version.commit_date.is_some());
    }

    #[test]
    fn test_build_info() {
        let info = BuildInfo::current();
        assert!(!info.version.is_empty());
        assert_eq!(info.git_branch, GIT_BRANCH);
    }

    #[test]
    fn test_version_outdated_same_hash() {
        // If hashes are the same, not outdated
        assert!(!is_version_outdated(&format!("0.1.0-{}", GIT_HASH)));
    }

    #[test]
    fn test_version_outdated_different_hash() {
        // If hashes are different, is outdated
        assert!(is_version_outdated("0.1.0-abc1234"));
    }

    #[test]
    fn test_version_outdated_with_build() {
        // Same hash with different build times = not outdated
        let result = is_version_outdated_with_build(
            &format!("0.1.0-{}", GIT_HASH),
            Some("2025-11-07 14:00:00"),
        );
        assert!(!result);
    }

    #[test]
    fn test_version_mismatch_message() {
        let msg = version_mismatch_message("127.0.0.1", "0.1.0-abc1234");
        assert!(msg.contains("Peer 127.0.0.1"));
        assert!(msg.contains("0.1.0-abc1234"));
    }

    #[test]
    fn test_iso_date_comparison() {
        // Test ISO 8601 format dates
        let older_iso = "2025-11-07T15:09:21Z";
        let newer_iso = "2025-11-08T15:09:21Z";
        assert!(is_remote_version_newer(older_iso, newer_iso));
        assert!(!is_remote_version_newer(newer_iso, older_iso));
    }

    #[test]
    fn test_same_date_comparison() {
        // Same commit date should not be considered newer
        let date = "2025-11-07T15:09:21Z";
        assert!(!is_remote_version_newer(date, date));
    }
}

/// Transaction broadcast message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMessage {
    pub txid: String,
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: i64,
    pub signature: String,
    pub nonce: u64,
}

/// Transaction validation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionValidation {
    pub txid: String,
    pub validator: String,
    pub approved: bool,
    pub timestamp: u64,
}

/// Block data for sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockData {
    pub block: Vec<u8>,
    pub height: u64,
}

/// Network message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    Ping,
    Pong,
    Transaction(TransactionMessage),
    ValidationResponse(TransactionValidation),
    BlockProposal(Vec<u8>),
    GetBlockchainHeight,
    BlockchainHeight(u64),
    GetBlocks { start_height: u64, end_height: u64 },
    BlocksData(Vec<BlockData>),
}

impl NetworkMessage {
    pub fn serialize(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|e| e.to_string())
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(data).map_err(|e| e.to_string())
    }
}

/// Request peer list from connected node
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeerListRequest {
    pub requesting_node: String,
}

/// Response with known peers
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeerListResponse {
    pub peers: Vec<PeerAddress>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeerAddress {
    pub ip: String,
    pub port: u16,
    pub version: String,
}

/// Ping message for latency measurement
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Ping {
    pub timestamp: i64,
}

/// Pong response
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Pong {
    pub timestamp: i64,
}
