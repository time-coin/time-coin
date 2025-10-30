//! Network protocol for peer communication
//!
//! Handles handshakes, version exchange, and peer identification

use crate::discovery::NetworkType;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Current TIME Coin version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Protocol version for compatibility checking
pub const PROTOCOL_VERSION: u32 = 1;

/// Handshake message sent when connecting to peers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeMessage {
    /// Software version (e.g., "1.0.0")
    pub version: String,
    
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
}

impl HandshakeMessage {
    /// Create a new handshake message
    pub fn new(network: NetworkType, listen_addr: SocketAddr) -> Self {
        HandshakeMessage {
            version: VERSION.to_string(),
            protocol_version: PROTOCOL_VERSION,
            network,
            listen_addr,
            timestamp: current_timestamp(),
            capabilities: vec![
                "masternode".to_string(),
                "sync".to_string(),
            ],
        }
    }
    
    /// Validate handshake from peer
    pub fn validate(&self, expected_network: &NetworkType) -> Result<(), String> {
        // Check network compatibility
        if &self.network != expected_network {
            return Err(format!(
                "Network mismatch: expected {:?}, got {:?}",
                expected_network, self.network
            ));
        }
        
        // Check protocol version compatibility
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
    pub protocol_version: u32,
}

impl ProtocolVersion {
    pub fn current() -> Self {
        ProtocolVersion {
            software_version: VERSION.to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handshake_creation() {
        let addr = "127.0.0.1:24100".parse().unwrap();
        let handshake = HandshakeMessage::new(NetworkType::Testnet, addr);
        
        assert_eq!(handshake.version, VERSION);
        assert_eq!(handshake.protocol_version, PROTOCOL_VERSION);
        assert_eq!(handshake.network, NetworkType::Testnet);
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
