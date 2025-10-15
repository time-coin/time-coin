//! Masternode reputation tracking system

use serde::{Deserialize, Serialize};
use crate::error::{MasternodeError, Result};

/// Reputation score range: -1000 to +1000
pub const MIN_REPUTATION: i32 = -1000;
pub const MAX_REPUTATION: i32 = 1000;
pub const STARTING_REPUTATION: i32 = 0;

/// Minimum reputation to participate
pub const MIN_REPUTATION_TO_PARTICIPATE: i32 = -100;

/// Reputation changes
pub const REPUTATION_BLOCK_VALIDATED: i32 = 1;
pub const REPUTATION_MISSED_BLOCK: i32 = -5;
pub const REPUTATION_INVALID_BLOCK: i32 = -20;
pub const REPUTATION_OFFLINE: i32 = -10;
pub const REPUTATION_VOTED: i32 = 2;
pub const REPUTATION_SLASHED: i32 = -500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reputation {
    pub masternode_id: String,
    pub score: i32,
    pub blocks_validated: u64,
    pub blocks_missed: u64,
    pub votes_cast: u64,
    pub slashes: u32,
    pub created_at: u64,
    pub last_updated: u64,
}

impl Reputation {
    pub fn new(masternode_id: String, timestamp: u64) -> Self {
        Self {
            masternode_id,
            score: STARTING_REPUTATION,
            blocks_validated: 0,
            blocks_missed: 0,
            votes_cast: 0,
            slashes: 0,
            created_at: timestamp,
            last_updated: timestamp,
        }
    }

    /// Update score (clamped to min/max)
    pub fn update_score(&mut self, change: i32, timestamp: u64) {
        self.score = (self.score + change).clamp(MIN_REPUTATION, MAX_REPUTATION);
        self.last_updated = timestamp;
    }

    /// Record block validation
    pub fn record_block_validated(&mut self, timestamp: u64) {
        self.blocks_validated += 1;
        self.update_score(REPUTATION_BLOCK_VALIDATED, timestamp);
    }

    /// Record missed block
    pub fn record_block_missed(&mut self, timestamp: u64) {
        self.blocks_missed += 1;
        self.update_score(REPUTATION_MISSED_BLOCK, timestamp);
    }

    /// Record invalid block attempt
    pub fn record_invalid_block(&mut self, timestamp: u64) {
        self.update_score(REPUTATION_INVALID_BLOCK, timestamp);
    }

    /// Record vote cast
    pub fn record_vote(&mut self, timestamp: u64) {
        self.votes_cast += 1;
        self.update_score(REPUTATION_VOTED, timestamp);
    }

    /// Record slash
    pub fn record_slash(&mut self, timestamp: u64) {
        self.slashes += 1;
        self.update_score(REPUTATION_SLASHED, timestamp);
    }

    /// Check if eligible to participate
    pub fn is_eligible(&self) -> bool {
        self.score >= MIN_REPUTATION_TO_PARTICIPATE
    }

    /// Get reputation level
    pub fn level(&self) -> ReputationLevel {
        match self.score {
            s if s >= 800 => ReputationLevel::Excellent,
            s if s >= 400 => ReputationLevel::VeryGood,
            s if s >= 100 => ReputationLevel::Good,
            s if s >= -100 => ReputationLevel::Fair,
            s if s >= -500 => ReputationLevel::Poor,
            _ => ReputationLevel::VeryPoor,
        }
    }

    /// Get uptime percentage
    pub fn uptime_percentage(&self) -> f64 {
        if self.blocks_validated == 0 && self.blocks_missed == 0 {
            return 100.0;
        }
        let total = self.blocks_validated + self.blocks_missed;
        (self.blocks_validated as f64 / total as f64) * 100.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReputationLevel {
    Excellent,  // 800+
    VeryGood,   // 400-799
    Good,       // 100-399
    Fair,       // -100 to 99
    Poor,       // -500 to -101
    VeryPoor,   // < -500
}

impl std::fmt::Display for ReputationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Excellent => write!(f, "Excellent"),
            Self::VeryGood => write!(f, "Very Good"),
            Self::Good => write!(f, "Good"),
            Self::Fair => write!(f, "Fair"),
            Self::Poor => write!(f, "Poor"),
            Self::VeryPoor => write!(f, "Very Poor"),
        }
    }
}

/// Manages masternode reputations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationManager {
    reputations: std::collections::HashMap<String, Reputation>,
}

impl ReputationManager {
    pub fn new() -> Self {
        Self {
            reputations: std::collections::HashMap::new(),
        }
    }

    /// Create new reputation for masternode
    pub fn create_reputation(&mut self, masternode_id: String, timestamp: u64) -> &Reputation {
        let reputation = Reputation::new(masternode_id.clone(), timestamp);
        self.reputations.insert(masternode_id.clone(), reputation);
        self.reputations.get(&masternode_id).unwrap()
    }

    /// Get reputation
    pub fn get(&self, masternode_id: &str) -> Result<&Reputation> {
        self.reputations
            .get(masternode_id)
            .ok_or_else(|| MasternodeError::NotFound(masternode_id.to_string()))
    }

    /// Get mutable reputation
    pub fn get_mut(&mut self, masternode_id: &str) -> Result<&mut Reputation> {
        self.reputations
            .get_mut(masternode_id)
            .ok_or_else(|| MasternodeError::NotFound(masternode_id.to_string()))
    }

    /// Get all reputations
    pub fn all(&self) -> Vec<&Reputation> {
        self.reputations.values().collect()
    }

    /// Get eligible masternodes (reputation >= -100)
    pub fn eligible(&self) -> Vec<&Reputation> {
        self.reputations
            .values()
            .filter(|r| r.is_eligible())
            .collect()
    }

    /// Get top N masternodes by reputation
    pub fn top(&self, n: usize) -> Vec<&Reputation> {
        let mut reputations: Vec<_> = self.reputations.values().collect();
        reputations.sort_by(|a, b| b.score.cmp(&a.score));
        reputations.into_iter().take(n).collect()
    }
}

impl Default for ReputationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationScore {
    pub score: i32,
    pub level: ReputationLevel,
    pub uptime: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_reputation() {
        let rep = Reputation::new("mn1".to_string(), 1000);
        assert_eq!(rep.score, STARTING_REPUTATION);
        assert_eq!(rep.blocks_validated, 0);
        assert!(rep.is_eligible());
    }

    #[test]
    fn test_reputation_updates() {
        let mut rep = Reputation::new("mn1".to_string(), 1000);

        // Validate blocks
        rep.record_block_validated(1001);
        assert_eq!(rep.score, 1);
        assert_eq!(rep.blocks_validated, 1);

        // Miss block
        rep.record_block_missed(1002);
        assert_eq!(rep.score, 1 - 5);
        assert_eq!(rep.blocks_missed, 1);

        // Vote
        rep.record_vote(1003);
        assert_eq!(rep.votes_cast, 1);
    }

    #[test]
    fn test_reputation_levels() {
        let mut rep = Reputation::new("mn1".to_string(), 1000);
        
        rep.update_score(900, 1001);
        assert_eq!(rep.level(), ReputationLevel::Excellent);

        rep.update_score(-600, 1002);
        assert_eq!(rep.level(), ReputationLevel::Good);
    }

    #[test]
    fn test_uptime_calculation() {
        let mut rep = Reputation::new("mn1".to_string(), 1000);
        
        for _ in 0..90 {
            rep.record_block_validated(1001);
        }
        for _ in 0..10 {
            rep.record_block_missed(1002);
        }

        assert_eq!(rep.uptime_percentage(), 90.0);
    }
}
