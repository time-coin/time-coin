//! Phased Daily Block Production Protocol
//!
//! Implements a 7-phase structured consensus protocol for daily block production:
//! 1. Midnight Synchronization - Network-wide heartbeat and sync
//! 2. Leader Election - VRF-based deterministic selection with weights
//! 3. Block Construction - Collect mempool txs and add rewards
//! 4. Proposal Distribution - Broadcast to all masternodes
//! 5. Voting Window - 4-second window with weighted votes
//! 6. Consensus Collection - Aggregate votes, check 67% threshold
//! 7. Finalization/Fallback - Finalize or rotate leader with fallback logic

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use time_core::MasternodeTier;
use tokio::sync::RwLock;

/// Phase status for tracking protocol progress
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    /// Waiting for midnight UTC
    Waiting,
    /// Phase 1: Network synchronization
    Synchronization,
    /// Phase 2: Leader election
    LeaderElection,
    /// Phase 3: Block construction
    BlockConstruction,
    /// Phase 4: Proposal distribution
    ProposalDistribution,
    /// Phase 5: Voting window (4 seconds)
    VotingWindow,
    /// Phase 6: Consensus collection
    ConsensusCollection,
    /// Phase 7: Finalization or fallback
    Finalization,
    /// Emergency fallback mode
    EmergencyFallback,
    /// Protocol complete
    Complete,
}

/// Heartbeat message for Phase 1 synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    pub node_id: String,
    pub timestamp: i64,
    pub block_height: u64,
    pub chain_tip_hash: String,
    pub tier: MasternodeTier,
    pub version: String,
}

/// Leader election result from Phase 2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderElection {
    pub leader: String,
    pub block_height: u64,
    pub vrf_proof: Vec<u8>,
    pub timestamp: i64,
}

/// Weighted vote for Phase 5
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedVote {
    pub voter: String,
    pub block_hash: String,
    pub approve: bool,
    pub weight: u64,
    pub signature: String,
    pub timestamp: i64,
}

/// Fallback attempt tracking
#[derive(Debug, Clone)]
pub struct FallbackAttempt {
    pub attempt: u32,
    pub leader: String,
    pub reason: FallbackReason,
    pub timestamp: DateTime<Utc>,
}

/// Reason for fallback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallbackReason {
    /// Leader did not produce block in time
    LeaderTimeout,
    /// Consensus threshold not met
    ConsensusFailure,
    /// Invalid block proposal
    InvalidProposal,
    /// Emergency block needed
    Emergency,
}

/// Manager for the phased consensus protocol
pub struct PhasedProtocolManager {
    /// Current phase
    current_phase: Arc<RwLock<Phase>>,

    /// Heartbeats received in Phase 1
    heartbeats: Arc<RwLock<HashMap<String, Heartbeat>>>,

    /// Current leader election result
    leader_election: Arc<RwLock<Option<LeaderElection>>>,

    /// Weighted votes for current block
    weighted_votes: Arc<RwLock<HashMap<String, WeightedVote>>>,

    /// Fallback attempts
    fallback_attempts: Arc<RwLock<Vec<FallbackAttempt>>>,

    /// Protocol start time
    protocol_start: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl Default for PhasedProtocolManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PhasedProtocolManager {
    pub fn new() -> Self {
        Self {
            current_phase: Arc::new(RwLock::new(Phase::Waiting)),
            heartbeats: Arc::new(RwLock::new(HashMap::new())),
            leader_election: Arc::new(RwLock::new(None)),
            weighted_votes: Arc::new(RwLock::new(HashMap::new())),
            fallback_attempts: Arc::new(RwLock::new(Vec::new())),
            protocol_start: Arc::new(RwLock::new(None)),
        }
    }

    /// Start the protocol at midnight
    pub async fn start_protocol(&self) {
        let mut phase = self.current_phase.write().await;
        *phase = Phase::Synchronization;

        let mut start = self.protocol_start.write().await;
        *start = Some(Utc::now());

        // Clear previous state
        self.heartbeats.write().await.clear();
        self.leader_election.write().await.take();
        self.weighted_votes.write().await.clear();
        self.fallback_attempts.write().await.clear();

        println!("🕐 PHASED PROTOCOL STARTED - Phase 1: Synchronization");
    }

    /// Get current phase
    pub async fn current_phase(&self) -> Phase {
        *self.current_phase.read().await
    }

    /// Advance to next phase
    pub async fn advance_phase(&self, next_phase: Phase) {
        let mut phase = self.current_phase.write().await;
        let previous = *phase;
        *phase = next_phase;
        println!("📍 PHASE TRANSITION: {:?} → {:?}", previous, next_phase);
    }

    /// Phase 1: Register heartbeat
    pub async fn register_heartbeat(&self, heartbeat: Heartbeat) {
        let mut heartbeats = self.heartbeats.write().await;
        heartbeats.insert(heartbeat.node_id.clone(), heartbeat);
    }

    /// Phase 1: Get all heartbeats
    pub async fn get_heartbeats(&self) -> Vec<Heartbeat> {
        self.heartbeats.read().await.values().cloned().collect()
    }

    /// Phase 1: Check if synchronization is complete
    pub async fn is_synchronized(&self, expected_nodes: usize) -> bool {
        let heartbeats = self.heartbeats.read().await;
        let received = heartbeats.len();

        // Need at least 2/3 of expected nodes
        let threshold = (expected_nodes * 2).div_ceil(3);
        received >= threshold
    }

    /// Phase 2: Set leader election result
    pub async fn set_leader(&self, election: LeaderElection) {
        let mut leader = self.leader_election.write().await;
        *leader = Some(election.clone());
        println!(
            "👑 Leader elected: {} for block {}",
            election.leader, election.block_height
        );
    }

    /// Phase 2: Get current leader
    pub async fn get_leader(&self) -> Option<LeaderElection> {
        self.leader_election.read().await.clone()
    }

    /// Phase 5: Register weighted vote
    pub async fn register_vote(&self, vote: WeightedVote) -> Result<(), String> {
        let mut votes = self.weighted_votes.write().await;

        // Check for duplicate vote
        if votes.contains_key(&vote.voter) {
            return Err("Duplicate vote".to_string());
        }

        votes.insert(vote.voter.clone(), vote);
        Ok(())
    }

    /// Phase 6: Calculate consensus
    pub async fn check_consensus(&self) -> (bool, u64, u64) {
        let votes = self.weighted_votes.read().await;

        let mut total_weight = 0u64;
        let mut approval_weight = 0u64;

        for vote in votes.values() {
            total_weight += vote.weight;
            if vote.approve {
                approval_weight += vote.weight;
            }
        }

        // Check if 67% threshold met (2/3 + 1)
        let threshold = (total_weight * 2).div_ceil(3);
        let has_consensus = approval_weight >= threshold;

        (has_consensus, approval_weight, total_weight)
    }

    /// Phase 7: Record fallback attempt
    pub async fn record_fallback(&self, leader: String, reason: FallbackReason) {
        let mut attempts = self.fallback_attempts.write().await;
        let attempt = FallbackAttempt {
            attempt: attempts.len() as u32 + 1,
            leader,
            reason,
            timestamp: Utc::now(),
        };

        println!(
            "⚠️  FALLBACK ATTEMPT #{}: {:?}",
            attempt.attempt, attempt.reason
        );
        attempts.push(attempt);
    }

    /// Phase 7: Get fallback attempt count
    pub async fn fallback_count(&self) -> usize {
        self.fallback_attempts.read().await.len()
    }

    /// Phase 7: Check if emergency mode needed
    pub async fn needs_emergency_mode(&self) -> bool {
        let attempts = self.fallback_attempts.read().await;
        attempts.len() >= 3
    }

    /// Reset protocol for next round
    pub async fn reset(&self) {
        let mut phase = self.current_phase.write().await;
        *phase = Phase::Waiting;

        self.heartbeats.write().await.clear();
        self.leader_election.write().await.take();
        self.weighted_votes.write().await.clear();
        self.fallback_attempts.write().await.clear();
        self.protocol_start.write().await.take();
    }

    /// Get protocol elapsed time
    pub async fn elapsed_time(&self) -> Option<chrono::Duration> {
        let start = self.protocol_start.read().await;
        start.map(|s| Utc::now() - s)
    }
}

/// Calculate node weight based on tier, longevity, and reputation
pub fn calculate_node_weight(tier: MasternodeTier, days_active: u64, reputation_score: f32) -> u64 {
    // Base weight from tier
    let tier_weight = match tier {
        MasternodeTier::Free => 1,
        MasternodeTier::Bronze => 2,
        MasternodeTier::Silver => 4,
        MasternodeTier::Gold => 8,
    };

    // Longevity bonus: +1% per 30 days (capped at 100%)
    let longevity_bonus = ((days_active / 30).min(100)) as f32 / 100.0;

    // Reputation multiplier (0.5 to 1.5)
    let reputation_multiplier = reputation_score.clamp(0.5, 1.5);

    // Final weight calculation
    let weight = tier_weight as f32 * (1.0 + longevity_bonus) * reputation_multiplier;

    weight.round() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_weight_calculation() {
        // Free tier, new node, average reputation
        let weight = calculate_node_weight(MasternodeTier::Free, 0, 1.0);
        assert_eq!(weight, 1);

        // Gold tier, 60 days active, good reputation
        let weight = calculate_node_weight(MasternodeTier::Gold, 60, 1.2);
        assert_eq!(weight, 10); // 8 * 1.02 * 1.2 ≈ 9.79 → 10

        // Silver tier, 90 days active, excellent reputation
        let weight = calculate_node_weight(MasternodeTier::Silver, 90, 1.5);
        assert_eq!(weight, 6); // 4 * 1.03 * 1.5 ≈ 6.18 → 6
    }

    #[tokio::test]
    async fn test_phase_progression() {
        let manager = PhasedProtocolManager::new();

        assert_eq!(manager.current_phase().await, Phase::Waiting);

        manager.start_protocol().await;
        assert_eq!(manager.current_phase().await, Phase::Synchronization);

        manager.advance_phase(Phase::LeaderElection).await;
        assert_eq!(manager.current_phase().await, Phase::LeaderElection);
    }

    #[tokio::test]
    async fn test_consensus_calculation() {
        let manager = PhasedProtocolManager::new();

        // Add votes
        let vote1 = WeightedVote {
            voter: "node1".to_string(),
            block_hash: "hash1".to_string(),
            approve: true,
            weight: 8,
            signature: "sig1".to_string(),
            timestamp: Utc::now().timestamp(),
        };

        let vote2 = WeightedVote {
            voter: "node2".to_string(),
            block_hash: "hash1".to_string(),
            approve: true,
            weight: 4,
            signature: "sig2".to_string(),
            timestamp: Utc::now().timestamp(),
        };

        let vote3 = WeightedVote {
            voter: "node3".to_string(),
            block_hash: "hash1".to_string(),
            approve: false,
            weight: 2,
            signature: "sig3".to_string(),
            timestamp: Utc::now().timestamp(),
        };

        manager.register_vote(vote1).await.unwrap();
        manager.register_vote(vote2).await.unwrap();
        manager.register_vote(vote3).await.unwrap();

        let (has_consensus, approval, total) = manager.check_consensus().await;

        assert_eq!(total, 14); // 8 + 4 + 2
        assert_eq!(approval, 12); // 8 + 4
        assert!(has_consensus); // 12 >= (14 * 2 / 3) = 10
    }
}
