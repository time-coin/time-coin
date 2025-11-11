//! Foolproof Block Creation System
//!
//! A multi-tiered fallback system that ensures blocks are ALWAYS created,
//! even under adverse network conditions. This module implements progressive
//! fallback strategies to prevent chain halts.
//!
//! ## Strategy Progression
//!
//! 1. **Normal BFT Consensus** (60s timeout)
//!    - Full 2/3+ masternode agreement
//!    - Standard block with all mempool transactions
//!
//! 2. **Leader Rotation** (45s timeout)
//!    - Rotate to next leader in sequence
//!    - Retry with full BFT consensus
//!
//! 3. **Reduced Threshold** (30s timeout)
//!    - Lower threshold to simple majority (1/2+)
//!    - Still includes mempool transactions
//!
//! 4. **Reward-Only Block** (30s timeout)
//!    - Skip all mempool transactions
//!    - Only treasury + masternode rewards
//!    - Reduced data = better chance of consensus
//!
//! 5. **Emergency Block** (no timeout)
//!    - Treasury reward only
//!    - Single validator signature
//!    - Prevents complete chain halt
//!
//! ## Design Principles
//!
//! - **Never give up**: Always create a block, even if minimal
//! - **Progressive degradation**: Try optimal first, degrade gracefully
//! - **Time-bounded**: Each attempt has clear timeout
//! - **Self-healing**: Track failures, auto-recover in next cycle

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Strategy level for block creation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockCreationStrategy {
    /// Level 1: Normal BFT with 2/3+ consensus
    NormalBFT,
    /// Level 2: Leader rotation with BFT
    LeaderRotation,
    /// Level 3: Reduced threshold (1/2+ simple majority)
    ReducedThreshold,
    /// Level 4: Reward-only block (no mempool txs)
    RewardOnly,
    /// Level 5: Emergency block (treasury only)
    Emergency,
}

impl BlockCreationStrategy {
    /// Get timeout duration for this strategy in seconds
    pub fn timeout_secs(&self) -> u64 {
        match self {
            Self::NormalBFT => 60,
            Self::LeaderRotation => 45,
            Self::ReducedThreshold => 30,
            Self::RewardOnly => 30,
            Self::Emergency => 0, // No timeout, must succeed
        }
    }

    /// Check if this strategy includes mempool transactions
    pub fn includes_mempool_txs(&self) -> bool {
        match self {
            Self::NormalBFT | Self::LeaderRotation | Self::ReducedThreshold => true,
            Self::RewardOnly | Self::Emergency => false,
        }
    }

    /// Get the required vote threshold as a fraction (numerator, denominator)
    pub fn vote_threshold(&self) -> (usize, usize) {
        match self {
            Self::NormalBFT | Self::LeaderRotation => (2, 3), // 2/3+
            Self::ReducedThreshold => (1, 2),                 // 1/2+
            Self::RewardOnly => (1, 3),                       // 1/3+ (lenient)
            Self::Emergency => (1, 10),                       // Any vote (10% minimum)
        }
    }

    /// Get next strategy in fallback chain
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::NormalBFT => Some(Self::LeaderRotation),
            Self::LeaderRotation => Some(Self::ReducedThreshold),
            Self::ReducedThreshold => Some(Self::RewardOnly),
            Self::RewardOnly => Some(Self::Emergency),
            Self::Emergency => None, // Last resort
        }
    }
}

/// Result of a block creation attempt
#[derive(Debug, Clone)]
pub struct BlockCreationAttempt {
    /// Attempt number (1, 2, 3, ...)
    pub attempt_number: u32,
    
    /// Strategy used
    pub strategy: BlockCreationStrategy,
    
    /// Leader/producer for this attempt
    pub leader: String,
    
    /// Timestamp of attempt
    pub timestamp: DateTime<Utc>,
    
    /// Number of votes received
    pub votes_received: usize,
    
    /// Total possible votes
    pub total_voters: usize,
    
    /// Whether attempt succeeded
    pub succeeded: bool,
    
    /// Failure reason if failed
    pub failure_reason: Option<String>,
}

/// Configuration for foolproof block creation
#[derive(Debug, Clone)]
pub struct FoolproofConfig {
    /// Enable all fallback strategies
    pub enable_fallbacks: bool,
    
    /// Maximum total time across all attempts (seconds)
    pub max_total_time_secs: u64,
    
    /// Enable emergency blocks as last resort
    pub enable_emergency_blocks: bool,
    
    /// Minimum number of masternodes for BFT
    pub min_masternodes_for_bft: usize,
}

impl Default for FoolproofConfig {
    fn default() -> Self {
        Self {
            enable_fallbacks: true,
            max_total_time_secs: 300, // 5 minutes max
            enable_emergency_blocks: true,
            min_masternodes_for_bft: 3,
        }
    }
}

/// Manager for foolproof block creation
pub struct FoolproofBlockManager {
    /// Configuration
    config: FoolproofConfig,
    
    /// Current attempts for this round
    attempts: Arc<RwLock<Vec<BlockCreationAttempt>>>,
    
    /// Round start time
    round_start: Arc<RwLock<Option<DateTime<Utc>>>>,
    
    /// Current strategy
    current_strategy: Arc<RwLock<BlockCreationStrategy>>,
}

impl FoolproofBlockManager {
    pub fn new(config: FoolproofConfig) -> Self {
        Self {
            config,
            attempts: Arc::new(RwLock::new(Vec::new())),
            round_start: Arc::new(RwLock::new(None)),
            current_strategy: Arc::new(RwLock::new(BlockCreationStrategy::NormalBFT)),
        }
    }

    /// Start a new block creation round
    pub async fn start_round(&self) {
        let mut attempts = self.attempts.write().await;
        attempts.clear();
        
        let mut start = self.round_start.write().await;
        *start = Some(Utc::now());
        
        let mut strategy = self.current_strategy.write().await;
        *strategy = BlockCreationStrategy::NormalBFT;
        
        println!();
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║         FOOLPROOF BLOCK CREATION SYSTEM ACTIVATED            ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();
    }

    /// Get current strategy
    pub async fn current_strategy(&self) -> BlockCreationStrategy {
        *self.current_strategy.read().await
    }

    /// Record an attempt
    pub async fn record_attempt(
        &self,
        strategy: BlockCreationStrategy,
        leader: String,
        votes_received: usize,
        total_voters: usize,
        succeeded: bool,
        failure_reason: Option<String>,
    ) {
        let mut attempts = self.attempts.write().await;
        
        let attempt = BlockCreationAttempt {
            attempt_number: (attempts.len() + 1) as u32,
            strategy,
            leader,
            timestamp: Utc::now(),
            votes_received,
            total_voters,
            succeeded,
            failure_reason,
        };
        
        println!();
        println!("─────────────────────────────────────────────────────────────");
        println!("📋 ATTEMPT #{} - {:?}", attempt.attempt_number, attempt.strategy);
        println!("─────────────────────────────────────────────────────────────");
        println!("   Leader: {}", attempt.leader);
        println!("   Votes: {}/{}", attempt.votes_received, attempt.total_voters);
        
        if succeeded {
            println!("   ✅ SUCCESS");
        } else {
            println!("   ❌ FAILED");
            if let Some(reason) = &attempt.failure_reason {
                println!("   Reason: {}", reason);
            }
        }
        
        attempts.push(attempt);
    }

    /// Advance to next strategy
    pub async fn advance_strategy(&self) -> Option<BlockCreationStrategy> {
        let mut strategy = self.current_strategy.write().await;
        
        if let Some(next) = strategy.next() {
            *strategy = next;
            println!();
            println!("🔄 ADVANCING TO NEXT STRATEGY: {:?}", next);
            println!("   Timeout: {}s", next.timeout_secs());
            println!("   Threshold: {:?}", next.vote_threshold());
            println!("   Includes mempool: {}", next.includes_mempool_txs());
            Some(next)
        } else {
            None
        }
    }

    /// Check if we should abort due to total time exceeded
    pub async fn should_abort_on_time(&self) -> bool {
        if let Some(start) = *self.round_start.read().await {
            let elapsed = (Utc::now() - start).num_seconds() as u64;
            elapsed >= self.config.max_total_time_secs
        } else {
            false
        }
    }

    /// Check if consensus is reached with current strategy threshold
    pub async fn check_consensus_with_strategy(
        &self,
        votes_received: usize,
        total_voters: usize,
    ) -> bool {
        let strategy = self.current_strategy().await;
        let (num, denom) = strategy.vote_threshold();
        
        // Calculate required votes: (total * num / denom) rounded up
        let required = (total_voters * num + denom - 1) / denom;
        
        votes_received >= required
    }

    /// Get total number of attempts
    pub async fn attempt_count(&self) -> usize {
        self.attempts.read().await.len()
    }

    /// Check if any attempt succeeded
    pub async fn has_success(&self) -> bool {
        self.attempts.read().await.iter().any(|a| a.succeeded)
    }

    /// Get all attempts
    pub async fn get_attempts(&self) -> Vec<BlockCreationAttempt> {
        self.attempts.read().await.clone()
    }

    /// Log detailed summary of all attempts
    pub async fn log_summary(&self) {
        let attempts = self.attempts.read().await;
        let start = self.round_start.read().await;
        
        if attempts.is_empty() {
            return;
        }
        
        println!();
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║           FOOLPROOF BLOCK CREATION SUMMARY                   ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();
        println!("Total attempts: {}", attempts.len());
        
        if let Some(start_time) = *start {
            let elapsed = (Utc::now() - start_time).num_seconds();
            println!("Total time: {}s", elapsed);
        }
        
        println!();
        
        for attempt in attempts.iter() {
            let status = if attempt.succeeded { "✅ SUCCESS" } else { "❌ FAILED" };
            println!(
                "Attempt #{}: {:?} - {} ({}/{})",
                attempt.attempt_number,
                attempt.strategy,
                status,
                attempt.votes_received,
                attempt.total_voters
            );
            
            if let Some(reason) = &attempt.failure_reason {
                println!("  └─ Reason: {}", reason);
            }
        }
        
        println!();
        
        let success_count = attempts.iter().filter(|a| a.succeeded).count();
        if success_count > 0 {
            println!("✅ Block creation successful after {} attempt(s)", attempts.len());
        } else {
            println!("⚠️  All attempts failed - this should never happen!");
        }
        
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();
    }

    /// Get elapsed time for current round
    pub async fn elapsed_time_secs(&self) -> u64 {
        if let Some(start) = *self.round_start.read().await {
            (Utc::now() - start).num_seconds() as u64
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_timeouts() {
        assert_eq!(BlockCreationStrategy::NormalBFT.timeout_secs(), 60);
        assert_eq!(BlockCreationStrategy::LeaderRotation.timeout_secs(), 45);
        assert_eq!(BlockCreationStrategy::ReducedThreshold.timeout_secs(), 30);
        assert_eq!(BlockCreationStrategy::RewardOnly.timeout_secs(), 30);
        assert_eq!(BlockCreationStrategy::Emergency.timeout_secs(), 0);
    }

    #[test]
    fn test_strategy_progression() {
        let strat = BlockCreationStrategy::NormalBFT;
        assert_eq!(strat.next(), Some(BlockCreationStrategy::LeaderRotation));
        
        let strat = BlockCreationStrategy::LeaderRotation;
        assert_eq!(strat.next(), Some(BlockCreationStrategy::ReducedThreshold));
        
        let strat = BlockCreationStrategy::ReducedThreshold;
        assert_eq!(strat.next(), Some(BlockCreationStrategy::RewardOnly));
        
        let strat = BlockCreationStrategy::RewardOnly;
        assert_eq!(strat.next(), Some(BlockCreationStrategy::Emergency));
        
        let strat = BlockCreationStrategy::Emergency;
        assert_eq!(strat.next(), None);
    }

    #[test]
    fn test_vote_thresholds() {
        // Normal BFT: 2/3+
        let (num, denom) = BlockCreationStrategy::NormalBFT.vote_threshold();
        assert_eq!((num, denom), (2, 3));
        
        // Reduced: 1/2+
        let (num, denom) = BlockCreationStrategy::ReducedThreshold.vote_threshold();
        assert_eq!((num, denom), (1, 2));
        
        // Emergency: 1/10 (10%)
        let (num, denom) = BlockCreationStrategy::Emergency.vote_threshold();
        assert_eq!((num, denom), (1, 10));
    }

    #[tokio::test]
    async fn test_consensus_calculation() {
        let config = FoolproofConfig::default();
        let manager = FoolproofBlockManager::new(config);
        
        manager.start_round().await;
        
        // Normal BFT with 4 nodes: need 3 votes (2/3+ = 2.67 -> 3)
        assert!(manager.check_consensus_with_strategy(3, 4).await);
        assert!(!manager.check_consensus_with_strategy(2, 4).await);
        
        // Advance to reduced threshold
        manager.advance_strategy().await; // LeaderRotation (still 2/3)
        manager.advance_strategy().await; // ReducedThreshold (1/2)
        
        // Reduced threshold with 4 nodes: need 2 votes (1/2 = 2)
        assert!(manager.check_consensus_with_strategy(2, 4).await);
        assert!(!manager.check_consensus_with_strategy(1, 4).await);
    }

    #[tokio::test]
    async fn test_attempt_tracking() {
        let config = FoolproofConfig::default();
        let manager = FoolproofBlockManager::new(config);
        
        manager.start_round().await;
        
        assert_eq!(manager.attempt_count().await, 0);
        assert!(!manager.has_success().await);
        
        manager.record_attempt(
            BlockCreationStrategy::NormalBFT,
            "node1".to_string(),
            2,
            4,
            false,
            Some("Timeout".to_string()),
        ).await;
        
        assert_eq!(manager.attempt_count().await, 1);
        assert!(!manager.has_success().await);
        
        manager.record_attempt(
            BlockCreationStrategy::LeaderRotation,
            "node2".to_string(),
            3,
            4,
            true,
            None,
        ).await;
        
        assert_eq!(manager.attempt_count().await, 2);
        assert!(manager.has_success().await);
    }
}
