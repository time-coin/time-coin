//! Rate limiting for vote reception
//!
//! Prevents vote spam and denial-of-service attacks by limiting
//! the number of votes a peer can submit per height.

use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Rate limiter for votes
pub struct VoteRateLimiter {
    /// Votes per peer: (peer_id, height) -> count
    vote_counts: Arc<DashMap<(String, u64), usize>>,

    /// Maximum votes per peer per height
    max_votes_per_peer_per_height: usize,

    /// Current height being tracked
    current_height: Arc<RwLock<u64>>,
}

impl VoteRateLimiter {
    /// Create a new vote rate limiter
    ///
    /// # Arguments
    /// * `max_votes_per_peer_per_height` - Maximum votes a peer can submit per height
    pub fn new(max_votes_per_peer_per_height: usize) -> Self {
        Self {
            vote_counts: Arc::new(DashMap::new()),
            max_votes_per_peer_per_height,
            current_height: Arc::new(RwLock::new(0)),
        }
    }

    /// Try to accept a vote from a peer
    ///
    /// Returns `Ok(())` if vote is accepted, `Err` if rate limit exceeded
    pub async fn try_accept_vote(&self, peer_id: &str, height: u64) -> Result<(), &'static str> {
        let key = (peer_id.to_string(), height);

        let mut count_ref = self.vote_counts.entry(key).or_insert(0);

        if *count_ref >= self.max_votes_per_peer_per_height {
            return Err("Vote rate limit exceeded for peer");
        }

        *count_ref += 1;
        Ok(())
    }

    /// Reset rate limiting for a new height
    pub async fn advance_height(&self, new_height: u64) {
        let mut current = self.current_height.write().await;
        let old_height = *current;
        *current = new_height;

        // Cleanup old height data
        if new_height > old_height {
            self.cleanup_old_heights(new_height.saturating_sub(10));
        }
    }

    /// Get current vote count for a peer at a height
    pub fn get_vote_count(&self, peer_id: &str, height: u64) -> usize {
        let key = (peer_id.to_string(), height);
        self.vote_counts.get(&key).map(|v| *v).unwrap_or(0)
    }

    /// Cleanup old heights (keep only recent N heights)
    fn cleanup_old_heights(&self, min_height: u64) {
        self.vote_counts
            .retain(|(_, height), _| *height >= min_height);
    }

    /// Reset all rate limits
    pub fn reset(&self) {
        self.vote_counts.clear();
    }
}

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum votes per peer per height
    pub max_votes_per_peer_per_height: usize,

    /// Number of heights to keep in memory
    pub history_depth: usize,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            // Allow up to 3 votes per peer per height
            // (initial vote + 2 retries for network issues)
            max_votes_per_peer_per_height: 3,

            // Keep last 10 heights in memory
            history_depth: 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiting() {
        let limiter = VoteRateLimiter::new(3);

        // First 3 votes should succeed
        assert!(limiter.try_accept_vote("peer1", 100).await.is_ok());
        assert!(limiter.try_accept_vote("peer1", 100).await.is_ok());
        assert!(limiter.try_accept_vote("peer1", 100).await.is_ok());

        // 4th vote should fail
        assert!(limiter.try_accept_vote("peer1", 100).await.is_err());

        // Different peer should not be affected
        assert!(limiter.try_accept_vote("peer2", 100).await.is_ok());
    }

    #[tokio::test]
    async fn test_height_separation() {
        let limiter = VoteRateLimiter::new(2);

        // Use up limit for height 100
        assert!(limiter.try_accept_vote("peer1", 100).await.is_ok());
        assert!(limiter.try_accept_vote("peer1", 100).await.is_ok());
        assert!(limiter.try_accept_vote("peer1", 100).await.is_err());

        // Height 101 should have separate limit
        assert!(limiter.try_accept_vote("peer1", 101).await.is_ok());
        assert!(limiter.try_accept_vote("peer1", 101).await.is_ok());
    }

    #[tokio::test]
    async fn test_vote_count_tracking() {
        let limiter = VoteRateLimiter::new(5);

        limiter.try_accept_vote("peer1", 100).await.ok();
        limiter.try_accept_vote("peer1", 100).await.ok();
        limiter.try_accept_vote("peer1", 100).await.ok();

        assert_eq!(limiter.get_vote_count("peer1", 100), 3);
        assert_eq!(limiter.get_vote_count("peer1", 101), 0);
        assert_eq!(limiter.get_vote_count("peer2", 100), 0);
    }

    #[tokio::test]
    async fn test_height_advancement() {
        let limiter = VoteRateLimiter::new(2);

        limiter.try_accept_vote("peer1", 100).await.ok();
        limiter.try_accept_vote("peer1", 101).await.ok();
        limiter.try_accept_vote("peer1", 102).await.ok();

        // Advance to height 110 (should cleanup heights < 100)
        limiter.advance_height(110).await;

        // Old height data should be cleaned up
        assert_eq!(limiter.vote_counts.len(), 3); // 100, 101, 102

        limiter.advance_height(115).await;
        // Should keep only heights >= 105
        assert!(limiter.vote_counts.is_empty());
    }

    #[tokio::test]
    async fn test_reset() {
        let limiter = VoteRateLimiter::new(2);

        limiter.try_accept_vote("peer1", 100).await.ok();
        limiter.try_accept_vote("peer1", 100).await.ok();

        assert!(limiter.try_accept_vote("peer1", 100).await.is_err());

        limiter.reset();

        // After reset, should accept votes again
        assert!(limiter.try_accept_vote("peer1", 100).await.is_ok());
    }
}
