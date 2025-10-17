//! Vote collection and aggregation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::{ConsensusError, MasternodeInfo};

/// A vote from a masternode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub masternode: String,
    pub txid: String,
    pub approve: bool,
    pub timestamp: i64,
    pub signature: Vec<u8>,
}

/// Result of vote collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteResult {
    pub approved: bool,
    pub total_voting_power: u64,
    pub approve_power: u64,
    pub reject_power: u64,
    pub quorum_size: usize,
    pub votes_received: usize,
}

/// Collects and validates votes
pub struct VoteCollector {
    votes: HashMap<String, Vote>,
    quorum: Vec<MasternodeInfo>,
    required_power: u64,
}

impl VoteCollector {
    pub fn new(quorum: Vec<MasternodeInfo>, threshold: f64) -> Self {
        let total_power: u64 = quorum.iter().map(|n| n.voting_power()).sum();
        let required_power = (total_power as f64 * threshold) as u64;
        
        Self {
            votes: HashMap::new(),
            quorum,
            required_power,
        }
    }
    
    /// Add a vote
    pub fn add_vote(&mut self, vote: Vote) -> Result<(), ConsensusError> {
        // Check if node is in quorum
        if !self.quorum.iter().any(|n| n.address == vote.masternode) {
            return Err(ConsensusError::InvalidSignature);
        }
        
        // Check for duplicate
        if self.votes.contains_key(&vote.masternode) {
            return Err(ConsensusError::DuplicateVote);
        }
        
        // In real implementation, verify signature here
        
        self.votes.insert(vote.masternode.clone(), vote);
        Ok(())
    }
    
    /// Check if consensus reached
    pub fn check_consensus(&self) -> Option<VoteResult> {
        let mut approve_power = 0u64;
        let mut reject_power = 0u64;
        
        for vote in self.votes.values() {
            if let Some(node) = self.quorum.iter().find(|n| n.address == vote.masternode) {
                let power = node.voting_power();
                if vote.approve {
                    approve_power += power;
                } else {
                    reject_power += power;
                }
            }
        }
        
        let total_power: u64 = self.quorum.iter().map(|n| n.voting_power()).sum();
        
        // Check if we have enough votes to reach consensus
        if approve_power >= self.required_power {
            return Some(VoteResult {
                approved: true,
                total_voting_power: total_power,
                approve_power,
                reject_power,
                quorum_size: self.quorum.len(),
                votes_received: self.votes.len(),
            });
        }
        
        // Check if rejection is impossible
        let remaining_power = total_power - approve_power - reject_power;
        if reject_power + remaining_power < self.required_power {
            return Some(VoteResult {
                approved: false,
                total_voting_power: total_power,
                approve_power,
                reject_power,
                quorum_size: self.quorum.len(),
                votes_received: self.votes.len(),
            });
        }
        
        None // Not enough votes yet
    }
    
    /// Get current vote status
    pub fn status(&self) -> (usize, usize, u64, u64) {
        let mut approve_power = 0u64;
        let mut reject_power = 0u64;
        
        for vote in self.votes.values() {
            if let Some(node) = self.quorum.iter().find(|n| n.address == vote.masternode) {
                let power = node.voting_power();
                if vote.approve {
                    approve_power += power;
                } else {
                    reject_power += power;
                }
            }
        }
        
        (self.votes.len(), self.quorum.len(), approve_power, reject_power)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MasternodeTier;
    
    fn create_test_quorum() -> Vec<MasternodeInfo> {
        vec![
            MasternodeInfo {
                address: "node1".to_string(),
                tier: MasternodeTier::Professional,
                active_since: chrono::Utc::now().timestamp(),
                uptime_score: 0.99,
                reputation: 100,
            },
            MasternodeInfo {
                address: "node2".to_string(),
                tier: MasternodeTier::Verified,
                active_since: chrono::Utc::now().timestamp(),
                uptime_score: 0.99,
                reputation: 100,
            },
            MasternodeInfo {
                address: "node3".to_string(),
                tier: MasternodeTier::Community,
                active_since: chrono::Utc::now().timestamp(),
                uptime_score: 0.99,
                reputation: 100,
            },
        ]
    }
    
    #[test]
    fn test_vote_collection() {
        let quorum = create_test_quorum();
        let mut collector = VoteCollector::new(quorum, 0.67);
        
        let vote1 = Vote {
            masternode: "node1".to_string(),
            txid: "tx1".to_string(),
            approve: true,
            timestamp: chrono::Utc::now().timestamp(),
            signature: vec![],
        };
        
        assert!(collector.add_vote(vote1).is_ok());
        assert_eq!(collector.votes.len(), 1);
    }
    
    #[test]
    fn test_consensus_reached() {
        let quorum = create_test_quorum();
        let mut collector = VoteCollector::new(quorum, 0.67);
        
        // Professional node approves (high weight)
        let vote1 = Vote {
            masternode: "node1".to_string(),
            txid: "tx1".to_string(),
            approve: true,
            timestamp: chrono::Utc::now().timestamp(),
            signature: vec![],
        };
        
        collector.add_vote(vote1).unwrap();
        
        // Should reach consensus with Professional node alone
        let result = collector.check_consensus();
        assert!(result.is_some());
        assert!(result.unwrap().approved);
    }
}
