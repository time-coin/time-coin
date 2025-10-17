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
    /// Voting weight for this tier
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
    /// Calculate total voting power (weight × longevity × reputation)
    pub fn voting_power(&self) -> u64 {
        let base_weight = self.tier.voting_weight();
        let longevity = self.longevity_multiplier();
        let reputation_mult = (self.reputation as f64 / 100.0).min(2.0);
        
        (base_weight as f64 * longevity * reputation_mult) as u64
    }
    
    /// Longevity multiplier (1.0 to 3.0)
    fn longevity_multiplier(&self) -> f64 {
        let now = chrono::Utc::now().timestamp();
        let days_active = (now - self.active_since) / 86400;
        let years_active = days_active as f64 / 365.0;
        
        let multiplier = 1.0 + (years_active * 0.5);
        multiplier.min(3.0)
    }
    
    /// Check if node is eligible to vote
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
    /// Calculate transaction hash (for VRF seed)
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
}

impl ConsensusEngine {
    /// Create new consensus engine
    pub fn new() -> Self {
        Self {
            min_quorum_size: 7,        // Minimum for BFT (2f+1 where f=3)
            max_quorum_size: 50,       // Balance between security and efficiency
            vote_threshold: 0.67,      // 2/3+ consensus
        }
    }
    
    /// Calculate optimal quorum size based on network size
    pub fn calculate_quorum_size(&self, total_nodes: usize) -> usize {
        if total_nodes < self.min_quorum_size {
            return total_nodes;
        }
        
        // Logarithmic scaling: more nodes = larger quorum, but not linear
        let log_size = (total_nodes as f64).log2() * 7.0;
        let size = log_size as usize;
        
        size.clamp(self.min_quorum_size, self.max_quorum_size)
    }
    
    /// Validate a transaction using BFT consensus
    pub fn validate_transaction(
        &self,
        tx: &Transaction,
        all_nodes: &[MasternodeInfo],
    ) -> Result<VoteResult, ConsensusError> {
        // 1. Select quorum
        let quorum_size = self.calculate_quorum_size(all_nodes.len());
        let selector = QuorumSelector::new(quorum_size);
        let quorum = selector.select_quorum(tx, all_nodes);
        
        if quorum.len() < self.min_quorum_size {
            return Err(ConsensusError::QuorumTooSmall(quorum.len()));
        }
        
        // 2. Calculate required votes
        let total_power: u64 = quorum.iter().map(|n| n.voting_power()).sum();
        let _required_power = (total_power as f64 * self.vote_threshold) as u64;
        
        // 3. In a real implementation, this would:
        //    - Broadcast transaction to quorum
        //    - Wait for votes
        //    - Collect and verify signatures
        //    - Return result
        
        // For now, return the quorum info
        Ok(VoteResult {
            approved: true,
            total_voting_power: total_power,
            approve_power: total_power, // Placeholder
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
            active_since: chrono::Utc::now().timestamp() - 86400 * 365, // 1 year
            uptime_score: 0.99,
            reputation: 100,
        }
    }
    
    #[test]
    fn test_voting_weights() {
        assert_eq!(MasternodeTier::Community.voting_weight(), 1);
        assert_eq!(MasternodeTier::Verified.voting_weight(), 10);
        assert_eq!(MasternodeTier::Professional.voting_weight(), 100);
    }
    
    #[test]
    fn test_quorum_size_calculation() {
        let engine = ConsensusEngine::new();
        
        assert_eq!(engine.calculate_quorum_size(5), 5);     // Too small, use all
        assert_eq!(engine.calculate_quorum_size(10), 7);    // Min size
        assert_eq!(engine.calculate_quorum_size(100), 49);  // Scaled
        assert_eq!(engine.calculate_quorum_size(1000), 50); // Max cap
    }
    
    #[test]
    fn test_voting_power() {
        let node = create_test_node(MasternodeTier::Professional, "node1");
        let power = node.voting_power();
        
        // Professional (100) × longevity (~1.5) × reputation (1.0) ≈ 150
        assert!(power >= 100 && power <= 300);
    }
    
    #[test]
    fn test_eligibility() {
        let mut node = create_test_node(MasternodeTier::Community, "node1");
        assert!(node.is_eligible());
        
        node.uptime_score = 0.85;
        assert!(!node.is_eligible());
        
        node.uptime_score = 0.95;
        node.reputation = 40;
        assert!(!node.is_eligible());
    }
}
