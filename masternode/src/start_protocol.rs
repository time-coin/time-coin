//! Masternode start protocol (Dash-style)
//!
//! Handles the start-masternode message and collateral verification

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StartProtocolError {
    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Collateral not found: {0}")]
    CollateralNotFound(String),

    #[error("Collateral already spent")]
    CollateralSpent,

    #[error("Insufficient confirmations: have {have}, need {need}")]
    InsufficientConfirmations { have: u64, need: u64 },

    #[error("Invalid collateral amount: {0}")]
    InvalidCollateralAmount(u64),

    #[error("Crypto error: {0}")]
    CryptoError(String),

    #[error("Masternode already active")]
    MasternodeAlreadyActive,
}

/// Collateral output reference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollateralOutput {
    /// Transaction ID (hash)
    pub txid: String,
    
    /// Output index (vout)
    pub vout: u32,
    
    /// Amount in satoshis
    pub amount: u64,
    
    /// Address that owns this output
    pub address: String,
    
    /// Number of confirmations
    pub confirmations: u64,
}

impl CollateralOutput {
    /// Create a new collateral output reference
    pub fn new(txid: String, vout: u32, amount: u64, address: String, confirmations: u64) -> Self {
        Self {
            txid,
            vout,
            amount,
            address,
            confirmations,
        }
    }
    
    /// Get the unique identifier for this output (txid:vout)
    pub fn output_id(&self) -> String {
        format!("{}:{}", self.txid, self.vout)
    }
    
    /// Check if this output meets the minimum confirmation threshold
    pub fn has_sufficient_confirmations(&self, min_confirmations: u64) -> bool {
        self.confirmations >= min_confirmations
    }
    
    /// Determine the masternode tier from the collateral amount
    pub fn tier(&self) -> Result<crate::CollateralTier, StartProtocolError> {
        crate::CollateralTier::from_amount(self.amount)
            .map_err(|e| StartProtocolError::InvalidCollateralAmount(self.amount))
    }
}

/// Start-masternode message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartMasternodeMessage {
    /// Masternode alias/identifier
    pub alias: String,
    
    /// Masternode IP and port
    pub ip_port: String,
    
    /// Public key derived from masternode private key
    pub masternode_pubkey: String,
    
    /// Collateral transaction ID
    pub collateral_txid: String,
    
    /// Collateral output index
    pub collateral_vout: u32,
    
    /// Timestamp when the message was created
    pub timestamp: i64,
    
    /// Signature proving ownership of the collateral
    /// This is signed with the private key of the address that owns the collateral UTXO
    pub signature: Vec<u8>,
}

impl StartMasternodeMessage {
    /// Create a new start-masternode message
    pub fn new(
        alias: String,
        ip_port: String,
        masternode_pubkey: String,
        collateral_txid: String,
        collateral_vout: u32,
    ) -> Self {
        Self {
            alias,
            ip_port,
            masternode_pubkey,
            collateral_txid,
            collateral_vout,
            timestamp: chrono::Utc::now().timestamp(),
            signature: Vec::new(),
        }
    }
    
    /// Get the message hash for signing
    pub fn message_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.alias.as_bytes());
        hasher.update(self.ip_port.as_bytes());
        hasher.update(self.masternode_pubkey.as_bytes());
        hasher.update(self.collateral_txid.as_bytes());
        hasher.update(self.collateral_vout.to_le_bytes());
        hasher.update(self.timestamp.to_le_bytes());
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
    
    /// Sign the message with the collateral owner's private key
    pub fn sign(&mut self, private_key_hex: &str) -> Result<(), StartProtocolError> {
        use time_crypto::KeyPair;
        
        let keypair = KeyPair::from_private_key(private_key_hex)
            .map_err(|e| StartProtocolError::CryptoError(e.to_string()))?;
        
        let message_hash = self.message_hash();
        self.signature = keypair.sign(&message_hash);
        
        Ok(())
    }
    
    /// Verify the message signature
    pub fn verify_signature(&self, public_key_hex: &str) -> Result<(), StartProtocolError> {
        use time_crypto::KeyPair;
        
        let message_hash = self.message_hash();
        
        KeyPair::verify(public_key_hex, &message_hash, &self.signature)
            .map_err(|_| StartProtocolError::InvalidSignature)?;
        
        Ok(())
    }
    
    /// Validate the message (format checks, not signature)
    pub fn validate(&self) -> Result<(), StartProtocolError> {
        // Validate IP:port format
        if !self.ip_port.contains(':') {
            return Err(StartProtocolError::CryptoError(
                "Invalid IP:port format".to_string(),
            ));
        }
        
        // Validate txid format
        if self.collateral_txid.len() != 64
            || !self.collateral_txid.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err(StartProtocolError::CryptoError(
                "Invalid transaction ID format".to_string(),
            ));
        }
        
        // Validate alias
        if self.alias.is_empty() {
            return Err(StartProtocolError::CryptoError(
                "Empty alias".to_string(),
            ));
        }
        
        Ok(())
    }
}

/// Collateral verifier
pub struct CollateralVerifier {
    /// Minimum number of confirmations required
    min_confirmations: u64,
}

impl CollateralVerifier {
    /// Create a new collateral verifier
    pub fn new(min_confirmations: u64) -> Self {
        Self { min_confirmations }
    }
    
    /// Verify a collateral output
    pub fn verify_collateral(
        &self,
        collateral: &CollateralOutput,
    ) -> Result<crate::CollateralTier, StartProtocolError> {
        // Check confirmations
        if !collateral.has_sufficient_confirmations(self.min_confirmations) {
            return Err(StartProtocolError::InsufficientConfirmations {
                have: collateral.confirmations,
                need: self.min_confirmations,
            });
        }
        
        // Determine tier from amount
        let tier = collateral.tier()?;
        
        Ok(tier)
    }
    
    /// Verify a start-masternode message with collateral
    pub fn verify_start_message(
        &self,
        message: &StartMasternodeMessage,
        collateral: &CollateralOutput,
        collateral_owner_pubkey: &str,
    ) -> Result<crate::CollateralTier, StartProtocolError> {
        // Validate message format
        message.validate()?;
        
        // Verify signature
        message.verify_signature(collateral_owner_pubkey)?;
        
        // Verify collateral matches message
        if message.collateral_txid != collateral.txid
            || message.collateral_vout != collateral.vout
        {
            return Err(StartProtocolError::CollateralNotFound(format!(
                "{}:{}",
                message.collateral_txid, message.collateral_vout
            )));
        }
        
        // Verify collateral
        let tier = self.verify_collateral(collateral)?;
        
        Ok(tier)
    }
}

impl Default for CollateralVerifier {
    fn default() -> Self {
        Self::new(15) // Default: 15 confirmations (Dash uses 15)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_collateral_output() {
        let output = CollateralOutput::new(
            "2bcd3c84c84f87eaa86e4e56834c92927a07f9e18718810b92e0d0324456a67c".to_string(),
            0,
            1_000 * 100_000_000, // 1,000 TIME (Community tier)
            "TIME1address".to_string(),
            20,
        );
        
        assert_eq!(
            output.output_id(),
            "2bcd3c84c84f87eaa86e4e56834c92927a07f9e18718810b92e0d0324456a67c:0"
        );
        assert!(output.has_sufficient_confirmations(15));
        assert!(!output.has_sufficient_confirmations(25));
        assert_eq!(output.tier().unwrap(), crate::CollateralTier::Community);
    }
    
    #[test]
    fn test_start_message_creation() {
        let mut message = StartMasternodeMessage::new(
            "mn1".to_string(),
            "192.168.1.100:24000".to_string(),
            "pubkey123".to_string(),
            "2bcd3c84c84f87eaa86e4e56834c92927a07f9e18718810b92e0d0324456a67c".to_string(),
            0,
        );
        
        assert_eq!(message.alias, "mn1");
        assert!(message.validate().is_ok());
    }
    
    #[test]
    fn test_collateral_verifier() {
        let verifier = CollateralVerifier::new(10);
        
        let collateral = CollateralOutput::new(
            "2bcd3c84c84f87eaa86e4e56834c92927a07f9e18718810b92e0d0324456a67c".to_string(),
            0,
            10_000 * 100_000_000, // 10,000 TIME (Verified tier)
            "TIME1address".to_string(),
            15,
        );
        
        let tier = verifier.verify_collateral(&collateral).unwrap();
        assert_eq!(tier, crate::CollateralTier::Verified);
    }
    
    #[test]
    fn test_insufficient_confirmations() {
        let verifier = CollateralVerifier::new(20);
        
        let collateral = CollateralOutput::new(
            "2bcd3c84c84f87eaa86e4e56834c92927a07f9e18718810b92e0d0324456a67c".to_string(),
            0,
            10_000 * 100_000_000,
            "TIME1address".to_string(),
            10, // Only 10 confirmations
        );
        
        let result = verifier.verify_collateral(&collateral);
        assert!(result.is_err());
    }
}
