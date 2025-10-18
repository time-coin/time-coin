//! Masternode-related transaction types

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MasternodeTransaction {
    Registration(MasternodeRegistration),
    Heartbeat(MasternodeHeartbeat),
    Deregistration(MasternodeDeregistration),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasternodeRegistration {
    /// Public key of the masternode
    pub public_key: String,
    
    /// IP address
    pub ip_address: String,
    
    /// P2P port
    pub port: u16,
    
    /// Transaction ID proving 1000 TIME collateral
    pub collateral_tx: String,
    
    /// Registration timestamp
    pub timestamp: i64,
    
    /// Signature from masternode private key
    pub signature: String,
    
    /// Protocol version
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasternodeHeartbeat {
    pub public_key: String,
    pub timestamp: i64,
    pub current_block: u64,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasternodeDeregistration {
    pub public_key: String,
    pub reason: String,
    pub timestamp: i64,
    pub signature: String,
}

impl MasternodeRegistration {
    pub fn new(
        public_key: String,
        ip_address: String,
        port: u16,
        collateral_tx: String,
    ) -> Self {
        Self {
            public_key,
            ip_address,
            port,
            collateral_tx,
            timestamp: chrono::Utc::now().timestamp(),
            signature: String::new(), // Set after signing
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
    
    pub fn verify_collateral(&self) -> bool {
        // TODO: Verify the collateral transaction exists and has 1000+ TIME
        // For now, just check format
        !self.collateral_tx.is_empty()
    }
}
