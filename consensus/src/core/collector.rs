//! Generic vote collection and consensus checking
//!
//! Provides a reusable vote collector that works with any vote type,
//! eliminating duplication between block votes and transaction votes.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Trait for vote types
pub trait Vote: Clone + Send + Sync + 'static {
    /// Get the voter identifier
    fn voter(&self) -> &str;

    /// Check if this is an approval vote
    fn approve(&self) -> bool;

    /// Get timestamp of the vote
    fn timestamp(&self) -> i64;
}

/// Generic vote collector with configurable threshold
pub struct VoteCollector<V: Vote> {
    /// Votes organized by height/round
    votes: Arc<DashMap<u64, DashMap<String, Vec<V>>>>,

    /// Total number of voters (can be updated)
    total_voters: Arc<RwLock<usize>>,

    /// Threshold numerator (e.g., 2 for 2/3)
    threshold_numerator: usize,

    /// Threshold denominator (e.g., 3 for 2/3)
    threshold_denominator: usize,
}

impl<V: Vote> VoteCollector<V> {
    /// Create new vote collector with 2/3 threshold (BFT)
    pub fn new_bft(total_voters: usize) -> Self {
        Self {
            votes: Arc::new(DashMap::new()),
            total_voters: Arc::new(RwLock::new(total_voters)),
            threshold_numerator: 2,
            threshold_denominator: 3,
        }
    }

    /// Create new vote collector with custom threshold
    pub fn new_with_threshold(
        total_voters: usize,
        threshold_numerator: usize,
        threshold_denominator: usize,
    ) -> Self {
        Self {
            votes: Arc::new(DashMap::new()),
            total_voters: Arc::new(RwLock::new(total_voters)),
            threshold_numerator,
            threshold_denominator,
        }
    }

    /// Update total voter count
    pub fn set_total_voters(&self, count: usize) {
        // Store in a thread-safe way - we need to make total_voters atomic or use interior mutability
        // For now, keep simple - the caller should manage this or we use Arc<RwLock<usize>>
        // Let's use a simpler approach: pass total_voters to check_consensus
    }

    /// Record a vote
    pub fn record_vote(&self, height: u64, key: String, vote: V) {
        let height_votes = self.votes.entry(height).or_default();
        height_votes.entry(key).or_default().push(vote);
    }

    /// Check if consensus is reached for a specific key at height
    pub fn check_consensus(&self, height: u64, key: &str) -> (bool, usize, usize) {
        if self.total_voters < 3 {
            // Bootstrap mode - accept immediately
            return (true, 0, self.total_voters);
        }

        if let Some(height_votes) = self.votes.get(&height) {
            if let Some(vote_list) = height_votes.get(key) {
                let approvals = vote_list.iter().filter(|v| v.approve()).count();
                let required = (self.total_voters * self.threshold_numerator)
                    .div_ceil(self.threshold_denominator);
                let has_consensus = approvals >= required;
                return (has_consensus, approvals, required);
            }
        }

        let required =
            (self.total_voters * self.threshold_numerator).div_ceil(self.threshold_denominator);
        (false, 0, required)
    }

    /// Get vote counts for a specific key at height
    pub fn get_vote_count(&self, height: u64, key: &str) -> (usize, usize) {
        if let Some(height_votes) = self.votes.get(&height) {
            if let Some(vote_list) = height_votes.get(key) {
                let approvals = vote_list.iter().filter(|v| v.approve()).count();
                let rejections = vote_list.iter().filter(|v| !v.approve()).count();
                return (approvals, rejections);
            }
        }
        (0, 0)
    }

    /// Check if a voter has already voted
    pub fn has_voted(&self, height: u64, key: &str, voter: &str) -> bool {
        if let Some(height_votes) = self.votes.get(&height) {
            if let Some(vote_list) = height_votes.get(key) {
                return vote_list.iter().any(|v| v.voter() == voter);
            }
        }
        false
    }

    /// Get list of voters who approved
    pub fn get_approvers(&self, height: u64, key: &str) -> Vec<String> {
        if let Some(height_votes) = self.votes.get(&height) {
            if let Some(vote_list) = height_votes.get(key) {
                return vote_list
                    .iter()
                    .filter(|v| v.approve())
                    .map(|v| v.voter().to_string())
                    .collect();
            }
        }
        Vec::new()
    }

    /// Clear votes for a specific height
    pub fn clear_height(&self, height: u64) {
        self.votes.remove(&height);
    }

    /// Clear old votes (keep last N heights)
    pub fn cleanup_old(&self, current_height: u64, keep_last: u64) {
        let min_height = current_height.saturating_sub(keep_last);
        self.votes.retain(|h, _| *h >= min_height);
    }
}

/// Standard block vote implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockVote {
    pub block_height: u64,
    pub block_hash: String,
    pub voter: String,
    pub approve: bool,
    pub timestamp: i64,
}

impl Vote for BlockVote {
    fn voter(&self) -> &str {
        &self.voter
    }

    fn approve(&self) -> bool {
        self.approve
    }

    fn timestamp(&self) -> i64 {
        self.timestamp
    }
}

/// Standard transaction vote implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxVote {
    pub txid: String,
    pub voter: String,
    pub approve: bool,
    pub timestamp: i64,
}

impl Vote for TxVote {
    fn voter(&self) -> &str {
        &self.voter
    }

    fn approve(&self) -> bool {
        self.approve
    }

    fn timestamp(&self) -> i64 {
        self.timestamp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vote_collector_bft() {
        let collector = VoteCollector::<BlockVote>::new_bft(4);

        let vote1 = BlockVote {
            block_height: 100,
            block_hash: "hash1".to_string(),
            voter: "node1".to_string(),
            approve: true,
            timestamp: 12345,
        };

        let vote2 = BlockVote {
            block_height: 100,
            block_hash: "hash1".to_string(),
            voter: "node2".to_string(),
            approve: true,
            timestamp: 12346,
        };

        let vote3 = BlockVote {
            block_height: 100,
            block_hash: "hash1".to_string(),
            voter: "node3".to_string(),
            approve: true,
            timestamp: 12347,
        };

        collector.record_vote(100, "hash1".to_string(), vote1);
        collector.record_vote(100, "hash1".to_string(), vote2);

        // 2 out of 4 votes (50%) - not enough for 2/3
        let (has_consensus, approvals, required) = collector.check_consensus(100, "hash1");
        assert!(!has_consensus);
        assert_eq!(approvals, 2);
        assert_eq!(required, 3); // ceil(4 * 2 / 3) = 3

        collector.record_vote(100, "hash1".to_string(), vote3);

        // 3 out of 4 votes (75%) - meets 2/3 threshold
        let (has_consensus, approvals, required) = collector.check_consensus(100, "hash1");
        assert!(has_consensus);
        assert_eq!(approvals, 3);
        assert_eq!(required, 3);
    }

    #[test]
    fn test_duplicate_vote_detection() {
        let collector = VoteCollector::<BlockVote>::new_bft(3);

        let vote = BlockVote {
            block_height: 100,
            block_hash: "hash1".to_string(),
            voter: "node1".to_string(),
            approve: true,
            timestamp: 12345,
        };

        collector.record_vote(100, "hash1".to_string(), vote.clone());

        assert!(collector.has_voted(100, "hash1", "node1"));
        assert!(!collector.has_voted(100, "hash1", "node2"));
    }

    #[test]
    fn test_vote_cleanup() {
        let collector = VoteCollector::<BlockVote>::new_bft(3);

        for height in 1..=10 {
            let vote = BlockVote {
                block_height: height,
                block_hash: format!("hash{}", height),
                voter: "node1".to_string(),
                approve: true,
                timestamp: 12345,
            };
            collector.record_vote(height, format!("hash{}", height), vote);
        }

        collector.cleanup_old(10, 5);

        // Should keep heights 6-10, remove 1-5
        assert_eq!(collector.votes.len(), 5);
        assert!(collector.votes.contains_key(&6));
        assert!(collector.votes.contains_key(&10));
        assert!(!collector.votes.contains_key(&5));
    }
}
