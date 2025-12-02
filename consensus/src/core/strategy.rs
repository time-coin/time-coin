//! Unified fallback strategy progression
//!
//! Consolidates fallback.rs and foolproof_block.rs into a single
//! trait-based system for progressive consensus degradation.

use serde::{Deserialize, Serialize};

/// Unified fallback strategy with trait-based progression
pub trait FallbackStrategy: Clone + std::fmt::Debug {
    /// Get timeout duration in seconds
    fn timeout_secs(&self) -> u64;

    /// Check if strategy includes mempool transactions
    fn includes_mempool(&self) -> bool;

    /// Get vote threshold as (numerator, denominator)
    fn vote_threshold(&self) -> (usize, usize);

    /// Get next strategy in fallback chain
    fn next(&self) -> Option<Self>
    where
        Self: Sized;

    /// Strategy name for logging
    fn name(&self) -> &str;
}

/// Five-tier block creation strategy (from foolproof_block.rs)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockCreationStrategy {
    /// Level 1: Normal BFT with 2/3+ consensus (60s timeout)
    NormalBFT,
    /// Level 2: Leader rotation with BFT (45s timeout)
    LeaderRotation,
    /// Level 3: Reduced threshold - simple majority 1/2+ (30s timeout)
    ReducedThreshold,
    /// Level 4: Reward-only block, no mempool (30s timeout)
    RewardOnly,
    /// Level 5: Emergency block, treasury only (no timeout)
    Emergency,
}

impl FallbackStrategy for BlockCreationStrategy {
    fn timeout_secs(&self) -> u64 {
        match self {
            Self::NormalBFT => 60,
            Self::LeaderRotation => 45,
            Self::ReducedThreshold => 30,
            Self::RewardOnly => 30,
            Self::Emergency => 0,
        }
    }

    fn includes_mempool(&self) -> bool {
        match self {
            Self::NormalBFT | Self::LeaderRotation | Self::ReducedThreshold => true,
            Self::RewardOnly | Self::Emergency => false,
        }
    }

    fn vote_threshold(&self) -> (usize, usize) {
        match self {
            Self::NormalBFT | Self::LeaderRotation => (2, 3), // 2/3+ BFT
            Self::ReducedThreshold => (1, 2),                 // 1/2+ simple majority
            Self::RewardOnly => (1, 3),                       // 1/3+ lenient
            Self::Emergency => (1, 10),                       // 10% minimum
        }
    }

    fn next(&self) -> Option<Self> {
        match self {
            Self::NormalBFT => Some(Self::LeaderRotation),
            Self::LeaderRotation => Some(Self::ReducedThreshold),
            Self::ReducedThreshold => Some(Self::RewardOnly),
            Self::RewardOnly => Some(Self::Emergency),
            Self::Emergency => None,
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::NormalBFT => "NormalBFT",
            Self::LeaderRotation => "LeaderRotation",
            Self::ReducedThreshold => "ReducedThreshold",
            Self::RewardOnly => "RewardOnly",
            Self::Emergency => "Emergency",
        }
    }
}

impl std::fmt::Display for BlockCreationStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Three-tier fallback strategy (from fallback.rs - simpler version)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimpleFallbackStrategy {
    /// Rotate to next leader, retry with all transactions
    RotateLeader,
    /// Retry with only block rewards (skip mempool)
    RewardOnlyBlock,
    /// Emergency block with minimal data
    EmergencyBlock,
}

impl FallbackStrategy for SimpleFallbackStrategy {
    fn timeout_secs(&self) -> u64 {
        match self {
            Self::RotateLeader => 30,
            Self::RewardOnlyBlock => 30,
            Self::EmergencyBlock => 0,
        }
    }

    fn includes_mempool(&self) -> bool {
        match self {
            Self::RotateLeader => true,
            Self::RewardOnlyBlock | Self::EmergencyBlock => false,
        }
    }

    fn vote_threshold(&self) -> (usize, usize) {
        match self {
            Self::RotateLeader => (2, 3),    // 2/3+ BFT
            Self::RewardOnlyBlock => (1, 2), // 1/2+ simple majority
            Self::EmergencyBlock => (1, 10), // 10% minimum
        }
    }

    fn next(&self) -> Option<Self> {
        match self {
            Self::RotateLeader => Some(Self::RewardOnlyBlock),
            Self::RewardOnlyBlock => Some(Self::EmergencyBlock),
            Self::EmergencyBlock => None,
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::RotateLeader => "RotateLeader",
            Self::RewardOnlyBlock => "RewardOnlyBlock",
            Self::EmergencyBlock => "EmergencyBlock",
        }
    }
}

impl std::fmt::Display for SimpleFallbackStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Strategy manager for progressive fallback
pub struct StrategyManager<S: FallbackStrategy> {
    current_strategy: S,
    initial_strategy: S,
}

impl<S: FallbackStrategy> StrategyManager<S> {
    /// Create new strategy manager starting with given strategy
    pub fn new(initial_strategy: S) -> Self {
        Self {
            current_strategy: initial_strategy.clone(),
            initial_strategy,
        }
    }

    /// Get current strategy
    pub fn current(&self) -> &S {
        &self.current_strategy
    }

    /// Advance to next strategy, returns true if advanced
    pub fn advance(&mut self) -> bool {
        if let Some(next) = self.current_strategy.next() {
            println!(
                "🔄 Advancing fallback strategy: {} → {}",
                self.current_strategy.name(),
                next.name()
            );
            self.current_strategy = next;
            true
        } else {
            println!(
                "⚠️  Already at final fallback strategy: {}",
                self.current_strategy.name()
            );
            false
        }
    }

    /// Reset to initial strategy
    pub fn reset(&mut self) {
        self.current_strategy = self.initial_strategy.clone();
    }

    /// Check if at final strategy
    pub fn is_final(&self) -> bool {
        self.current_strategy.next().is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_strategy_progression() {
        let mut manager = StrategyManager::new(BlockCreationStrategy::NormalBFT);

        assert_eq!(manager.current().name(), "NormalBFT");
        assert!(!manager.is_final());

        assert!(manager.advance());
        assert_eq!(manager.current().name(), "LeaderRotation");

        assert!(manager.advance());
        assert_eq!(manager.current().name(), "ReducedThreshold");

        assert!(manager.advance());
        assert_eq!(manager.current().name(), "RewardOnly");

        assert!(manager.advance());
        assert_eq!(manager.current().name(), "Emergency");
        assert!(manager.is_final());

        assert!(!manager.advance()); // Can't advance past emergency
    }

    #[test]
    fn test_strategy_properties() {
        let strategy = BlockCreationStrategy::NormalBFT;
        assert_eq!(strategy.timeout_secs(), 60);
        assert!(strategy.includes_mempool());
        assert_eq!(strategy.vote_threshold(), (2, 3));

        let strategy = BlockCreationStrategy::Emergency;
        assert_eq!(strategy.timeout_secs(), 0);
        assert!(!strategy.includes_mempool());
        assert_eq!(strategy.vote_threshold(), (1, 10));
    }

    #[test]
    fn test_simple_fallback_progression() {
        let mut manager = StrategyManager::new(SimpleFallbackStrategy::RotateLeader);

        assert!(manager.advance());
        assert_eq!(manager.current().name(), "RewardOnlyBlock");

        assert!(manager.advance());
        assert_eq!(manager.current().name(), "EmergencyBlock");
        assert!(manager.is_final());
    }

    #[test]
    fn test_strategy_reset() {
        let mut manager = StrategyManager::new(BlockCreationStrategy::NormalBFT);

        manager.advance();
        manager.advance();
        assert_eq!(manager.current().name(), "ReducedThreshold");

        manager.reset();
        assert_eq!(manager.current().name(), "NormalBFT");
    }
}
