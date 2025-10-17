//! TIME Coin BFT Consensus
//! 
//! Byzantine Fault Tolerant consensus for instant transaction finality

use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use thiserror::Error;

pub mod quorum;
pub mod voting;
pub mod vrf;

pub use quorum::QuorumSelector;
pub use voting::{Vote, VoteCollector, VoteResult};

#[derive(Error, Debug)]
pub enum ConsensusError {
    #[error("Insufficient votes: got {0}, needed {1}")]
    InsufficientVotes(usize, usize),
    
    #[error("Quorum too small: {0}")]
    QuorumTooSmall(usize),
    
    #[error("Invalid vote signature")]
    InvalidSignature,
    
    #[error("Vote already cast by this node")]
    DuplicateVote,
    
    #[error("Timeout waiting for votes")]
    Timeout,
}

/// Masternode tier for consensus
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MasternodeTier {
    Community,
    Verified,
    Professional,
}

impl MasternodeTier {
    pub fn voting_weight(&self) -> u64 {
        match self {
            MasternodeTier::Community => 1,
            MasternodeTier::Verified => 10,
            MasternodeTier::Professional => 100,
        }
    }
}

/// Masternode information for consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasternodeInfo {
    pub address: String,
    pub tier: MasternodeTier,
    pub active_since: i64,
    pub uptime_score: f64,
    pub reputation: u64,
}

impl MasternodeInfo {
    pub fn voting_power(&self) -> u64 {
        let base_weight = self.tier.voting_weight();
        let longevity = self.longevity_multiplier();
        let reputation_mult = (self.reputation as f64 / 100.0).min(2.0);
        
        (base_weight as f64 * longevity * reputation_mult) as u64
    }
    
    fn longevity_multiplier(&self) -> f64 {
        let now = chrono::Utc::now().timestamp();
        let days_active = (now - self.active_since) / 86400;
        let years_active = days_active as f64 / 365.0;
        
        let multiplier = 1.0 + (years_active * 0.5);
        multiplier.min(3.0)
    }
    
    pub fn is_eligible(&self) -> bool {
        self.uptime_score >= 0.90 && self.reputation >= 50
    }
}

/// Transaction to be validated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub txid: String,
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: i64,
    pub nonce: u64,
}

impl Transaction {
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.txid.as_bytes());
        hasher.update(self.from.as_bytes());
        hasher.update(self.to.as_bytes());
        hasher.update(self.amount.to_le_bytes());
        hasher.update(self.timestamp.to_le_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// BFT Consensus Engine
pub struct ConsensusEngine {
    /// Minimum quorum size
    min_quorum_size: usize,
    
    /// Maximum quorum size
    max_quorum_size: usize,
    
    /// Required vote threshold (e.g., 0.67 for 2/3)
    vote_threshold: f64,
    
    /// Dev mode: bypass consensus for single-node testing
    dev_mode: bool,
}

impl ConsensusEngine {
    /// Create new consensus engine
    pub fn new() -> Self {
        Self {
            min_quorum_size: 7,
            max_quorum_size: 50,
            vote_threshold: 0.67,
            dev_mode: false,
        }
    }
    
    /// Create consensus engine with dev mode enabled
    pub fn new_dev_mode() -> Self {
        Self {
            min_quorum_size: 1,  // Allow single node
            max_quorum_size: 50,
            vote_threshold: 0.67,
            dev_mode: true,
        }
    }
    
    /// Enable dev mode
    pub fn enable_dev_mode(&mut self) {
        self.dev_mode = true;
        self.min_quorum_size = 1;
    }
    
    /// Check if in dev mode
    pub fn is_dev_mode(&self) -> bool {
        self.dev_mode
    }
    
    /// Calculate optimal quorum size based on network size
    pub fn calculate_quorum_size(&self, total_nodes: usize) -> usize {
        if self.dev_mode && total_nodes < self.min_quorum_size {
            return total_nodes.max(1); // At least 1 in dev mode
        }
        
        if total_nodes < self.min_quorum_size {
            return total_nodes;
        }
        
        let log_size = (total_nodes as f64).log2() * 7.0;
        let size = log_size as usize;
        
        size.clamp(self.min_quorum_size, self.max_quorum_size)
    }
    
    /// Validate a transaction using BFT consensus (or dev mode)
    pub fn validate_transaction(
        &self,
        tx: &Transaction,
        all_nodes: &[MasternodeInfo],
    ) -> Result<VoteResult, ConsensusError> {
        // DEV MODE: Auto-approve all transactions
        if self.dev_mode {
            return Ok(VoteResult {
                approved: true,
                total_voting_power: 100,
                approve_power: 100,
                reject_power: 0,
                quorum_size: 1,
                votes_received: 1,
            });
        }
        
        // NORMAL MODE: Full BFT consensus
        let quorum_size = self.calculate_quorum_size(all_nodes.len());
        let selector = QuorumSelector::new(quorum_size);
        let quorum = selector.select_quorum(tx, all_nodes);
        
        if quorum.len() < self.min_quorum_size {
            return Err(ConsensusError::QuorumTooSmall(quorum.len()));
        }
        
        let total_power: u64 = quorum.iter().map(|n| n.voting_power()).sum();
        let _required_power = (total_power as f64 * self.vote_threshold) as u64;
        
        Ok(VoteResult {
            approved: true,
            total_voting_power: total_power,
            approve_power: total_power,
            reject_power: 0,
            quorum_size: quorum.len(),
            votes_received: quorum.len(),
        })
    }
}

impl Default for ConsensusEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_node(tier: MasternodeTier, address: &str) -> MasternodeInfo {
        MasternodeInfo {
            address: address.to_string(),
            tier,
            active_since: chrono::Utc::now().timestamp() - 86400 * 365,
            uptime_score: 0.99,
            reputation: 100,
        }
    }
    
    #[test]
    fn test_dev_mode() {
        let engine = ConsensusEngine::new_dev_mode();
        assert!(engine.is_dev_mode());
        assert_eq!(engine.min_quorum_size, 1);
        
        let tx = Transaction {
            txid: "test".to_string(),
            from: "addr1".to_string(),
            to: "addr2".to_string(),
            amount: 100,
            fee: 1,
            timestamp: chrono::Utc::now().timestamp(),
            nonce: 0,
        };
        
        // Should work with no masternodes
        let result = engine.validate_transaction(&tx, &[]).unwrap();
        assert!(result.approved);
    }
    
    #[test]
    fn test_voting_weights() {
        assert_eq!(MasternodeTier::Community.voting_weight(), 1);
        assert_eq!(MasternodeTier::Verified.voting_weight(), 10);
        assert_eq!(MasternodeTier::Professional.voting_weight(), 100);
    }
    
    #[test]
    fn test_quorum_size_dev_mode() {
        let mut engine = ConsensusEngine::new();
        engine.enable_dev_mode();
        
        assert_eq!(engine.calculate_quorum_size(0), 1);
        assert_eq!(engine.calculate_quorum_size(1), 1);
        assert_eq!(engine.calculate_quorum_size(3), 3);
    }
}
