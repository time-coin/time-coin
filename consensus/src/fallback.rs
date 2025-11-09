//! Robust Fallback Protocol for Consensus Failures
//!
//! Implements Phase 7 of the phased protocol: fallback handling
//! when consensus fails or leader times out. Includes:
//! - Leader rotation
//! - Transaction set simplification (skip problematic txs)
//! - Emergency block creation to prevent chain halt

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Reason for fallback activation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FallbackReason {
    /// Leader did not produce proposal in time
    LeaderTimeout,
    
    /// Consensus threshold not met (< 67%)
    ConsensusNotReached,
    
    /// Invalid block proposal
    InvalidProposal,
    
    /// Transaction validation failures
    TransactionErrors,
    
    /// Network partition detected
    NetworkPartition,
}

/// Fallback strategy to apply
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FallbackStrategy {
    /// Rotate to next leader, retry with all transactions
    RotateLeader,
    
    /// Retry with only block rewards (skip all mempool txs)
    RewardOnlyBlock,
    
    /// Emergency block with minimal data to prevent halt
    EmergencyBlock,
}

/// Fallback attempt record
#[derive(Debug, Clone)]
pub struct FallbackAttempt {
    /// Attempt number (1, 2, 3, ...)
    pub attempt_number: u32,
    
    /// Reason for fallback
    pub reason: FallbackReason,
    
    /// Strategy applied
    pub strategy: FallbackStrategy,
    
    /// Leader for this attempt
    pub leader: String,
    
    /// Timestamp of attempt
    pub timestamp: DateTime<Utc>,
    
    /// Whether attempt succeeded
    pub succeeded: bool,
}

/// Configuration for fallback behavior
#[derive(Debug, Clone)]
pub struct FallbackConfig {
    /// Maximum number of leader rotations before emergency block
    pub max_leader_rotations: u32,
    
    /// Timeout for leader to produce proposal (seconds)
    pub leader_timeout_secs: u64,
    
    /// Timeout for voting window (seconds)
    pub voting_timeout_secs: u64,
    
    /// Whether to enable emergency blocks
    pub enable_emergency_blocks: bool,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            max_leader_rotations: 2,
            leader_timeout_secs: 30,
            voting_timeout_secs: 4,
            enable_emergency_blocks: true,
        }
    }
}

/// Fallback protocol manager
pub struct FallbackManager {
    /// Configuration
    config: FallbackConfig,
    
    /// Fallback attempts for current round
    attempts: Arc<RwLock<Vec<FallbackAttempt>>>,
    
    /// Start time of current consensus round
    round_start: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl FallbackManager {
    pub fn new(config: FallbackConfig) -> Self {
        Self {
            config,
            attempts: Arc::new(RwLock::new(Vec::new())),
            round_start: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Start a new consensus round
    pub async fn start_round(&self) {
        let mut attempts = self.attempts.write().await;
        attempts.clear();
        
        let mut start = self.round_start.write().await;
        *start = Some(Utc::now());
    }
    
    /// Record a fallback attempt
    pub async fn record_attempt(
        &self,
        reason: FallbackReason,
        strategy: FallbackStrategy,
        leader: String,
        succeeded: bool,
    ) {
        let mut attempts = self.attempts.write().await;
        
        let attempt = FallbackAttempt {
            attempt_number: (attempts.len() + 1) as u32,
            reason,
            strategy,
            leader,
            timestamp: Utc::now(),
            succeeded,
        };
        
        println!("🔄 FALLBACK ATTEMPT #{}", attempt.attempt_number);
        println!("   Reason: {:?}", attempt.reason);
        println!("   Strategy: {:?}", attempt.strategy);
        println!("   Leader: {}", attempt.leader);
        
        attempts.push(attempt);
    }
    
    /// Determine next fallback strategy based on attempt history
    pub async fn next_strategy(&self) -> FallbackStrategy {
        let attempts = self.attempts.read().await;
        let attempt_count = attempts.len();
        
        match attempt_count {
            0 => {
                // First failure: rotate leader
                FallbackStrategy::RotateLeader
            }
            1 => {
                // Second failure: rotate leader again
                FallbackStrategy::RotateLeader
            }
            2 => {
                // Third failure: try reward-only block
                println!("⚠️  Multiple leader rotations failed, trying reward-only block");
                FallbackStrategy::RewardOnlyBlock
            }
            _ => {
                // After 3+ attempts: emergency block
                if self.config.enable_emergency_blocks {
                    println!("🚨 EMERGENCY: Persistent consensus failure, creating emergency block");
                    FallbackStrategy::EmergencyBlock
                } else {
                    // Keep trying reward-only if emergency disabled
                    FallbackStrategy::RewardOnlyBlock
                }
            }
        }
    }
    
    /// Check if emergency block is needed
    pub async fn needs_emergency_block(&self) -> bool {
        let attempts = self.attempts.read().await;
        attempts.len() >= (self.config.max_leader_rotations + 1) as usize
            && self.config.enable_emergency_blocks
    }
    
    /// Check if leader timeout has been exceeded
    pub async fn check_leader_timeout(&self, leader_start: DateTime<Utc>) -> bool {
        let elapsed = (Utc::now() - leader_start).num_seconds();
        elapsed >= self.config.leader_timeout_secs as i64
    }
    
    /// Check if voting timeout has been exceeded
    pub async fn check_voting_timeout(&self, voting_start: DateTime<Utc>) -> bool {
        let elapsed = (Utc::now() - voting_start).num_seconds();
        elapsed >= self.config.voting_timeout_secs as i64
    }
    
    /// Get total attempt count
    pub async fn attempt_count(&self) -> usize {
        self.attempts.read().await.len()
    }
    
    /// Get all attempts
    pub async fn get_attempts(&self) -> Vec<FallbackAttempt> {
        self.attempts.read().await.clone()
    }
    
    /// Check if any attempt succeeded
    pub async fn has_success(&self) -> bool {
        self.attempts.read().await.iter().any(|a| a.succeeded)
    }
    
    /// Get elapsed time for current round
    pub async fn elapsed_time(&self) -> Option<chrono::Duration> {
        let start = self.round_start.read().await;
        start.map(|s| Utc::now() - s)
    }
    
    /// Reset for next round
    pub async fn reset(&self) {
        self.attempts.write().await.clear();
        self.round_start.write().await.take();
    }
    
    /// Log fallback summary
    pub async fn log_summary(&self) {
        let attempts = self.attempts.read().await;
        
        if attempts.is_empty() {
            return;
        }
        
        println!();
        println!("📊 ═══════════════════════════════════════════════════════");
        println!("📊 FALLBACK PROTOCOL SUMMARY");
        println!("📊 ═══════════════════════════════════════════════════════");
        println!("   Total attempts: {}", attempts.len());
        
        for attempt in attempts.iter() {
            println!("   #{}: {:?} - {:?} - {}",
                attempt.attempt_number,
                attempt.strategy,
                attempt.reason,
                if attempt.succeeded { "✅ SUCCESS" } else { "❌ FAILED" }
            );
        }
        
        if let Some(elapsed) = self.elapsed_time().await {
            println!("   Total time: {:.1}s", elapsed.num_milliseconds() as f64 / 1000.0);
        }
        
        println!("📊 ═══════════════════════════════════════════════════════");
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_fallback_strategy_progression() {
        let manager = FallbackManager::new(FallbackConfig::default());
        manager.start_round().await;
        
        // First failure: rotate leader
        assert_eq!(manager.next_strategy().await, FallbackStrategy::RotateLeader);
        manager.record_attempt(
            FallbackReason::LeaderTimeout,
            FallbackStrategy::RotateLeader,
            "node1".to_string(),
            false,
        ).await;
        
        // Second failure: rotate leader again
        assert_eq!(manager.next_strategy().await, FallbackStrategy::RotateLeader);
        manager.record_attempt(
            FallbackReason::ConsensusNotReached,
            FallbackStrategy::RotateLeader,
            "node2".to_string(),
            false,
        ).await;
        
        // Third failure: reward-only block
        assert_eq!(manager.next_strategy().await, FallbackStrategy::RewardOnlyBlock);
        manager.record_attempt(
            FallbackReason::ConsensusNotReached,
            FallbackStrategy::RewardOnlyBlock,
            "node3".to_string(),
            false,
        ).await;
        
        // Fourth failure: emergency block
        assert_eq!(manager.next_strategy().await, FallbackStrategy::EmergencyBlock);
    }
    
    #[tokio::test]
    async fn test_emergency_block_detection() {
        let manager = FallbackManager::new(FallbackConfig::default());
        manager.start_round().await;
        
        assert!(!manager.needs_emergency_block().await);
        
        // Record 3 failures
        for i in 0..3 {
            manager.record_attempt(
                FallbackReason::LeaderTimeout,
                FallbackStrategy::RotateLeader,
                format!("node{}", i),
                false,
            ).await;
        }
        
        assert!(manager.needs_emergency_block().await);
    }
    
    #[tokio::test]
    async fn test_timeout_checks() {
        let manager = FallbackManager::new(FallbackConfig {
            leader_timeout_secs: 1,
            voting_timeout_secs: 1,
            ..Default::default()
        });
        
        let start = Utc::now();
        
        // Initially no timeout
        assert!(!manager.check_leader_timeout(start).await);
        assert!(!manager.check_voting_timeout(start).await);
        
        // Wait for timeout
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        assert!(manager.check_leader_timeout(start).await);
        assert!(manager.check_voting_timeout(start).await);
    }
    
    #[tokio::test]
    async fn test_success_tracking() {
        let manager = FallbackManager::new(FallbackConfig::default());
        manager.start_round().await;
        
        assert!(!manager.has_success().await);
        
        manager.record_attempt(
            FallbackReason::LeaderTimeout,
            FallbackStrategy::RotateLeader,
            "node1".to_string(),
            false,
        ).await;
        
        assert!(!manager.has_success().await);
        
        manager.record_attempt(
            FallbackReason::ConsensusNotReached,
            FallbackStrategy::RotateLeader,
            "node2".to_string(),
            true,
        ).await;
        
        assert!(manager.has_success().await);
    }
    
    #[tokio::test]
    async fn test_attempt_count() {
        let manager = FallbackManager::new(FallbackConfig::default());
        manager.start_round().await;
        
        assert_eq!(manager.attempt_count().await, 0);
        
        manager.record_attempt(
            FallbackReason::LeaderTimeout,
            FallbackStrategy::RotateLeader,
            "node1".to_string(),
            false,
        ).await;
        
        assert_eq!(manager.attempt_count().await, 1);
        
        manager.record_attempt(
            FallbackReason::ConsensusNotReached,
            FallbackStrategy::RotateLeader,
            "node2".to_string(),
            false,
        ).await;
        
        assert_eq!(manager.attempt_count().await, 2);
    }
    
    #[tokio::test]
    async fn test_reset() {
        let manager = FallbackManager::new(FallbackConfig::default());
        manager.start_round().await;
        
        manager.record_attempt(
            FallbackReason::LeaderTimeout,
            FallbackStrategy::RotateLeader,
            "node1".to_string(),
            false,
        ).await;
        
        assert_eq!(manager.attempt_count().await, 1);
        
        manager.reset().await;
        
        assert_eq!(manager.attempt_count().await, 0);
        assert!(manager.elapsed_time().await.is_none());
    }
}
