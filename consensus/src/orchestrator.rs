//! Phased Consensus Orchestrator
//!
//! Coordinates all 7 phases of the daily block production protocol:
//! 1. Midnight Synchronization
//! 2. Leader Election
//! 3. Block Construction
//! 4. Proposal Distribution
//! 5. Voting Window
//! 6. Consensus Collection
//! 7. Finalization/Fallback

use chrono::{NaiveDate, Utc};
use std::sync::Arc;
use time_core::Block;

use crate::fallback::{FallbackConfig, FallbackManager, FallbackReason, FallbackStrategy};
use crate::heartbeat::{Heartbeat, HeartbeatManager, SyncStatus};
use crate::leader_election::{LeaderElector, LeaderSelection, MasternodeInfo};
use crate::phased_protocol::{Phase, PhasedProtocolManager, WeightedVote};

/// Configuration for the orchestrator
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Genesis date for longevity calculation
    pub genesis_date: NaiveDate,
    
    /// Heartbeat sync timeout (seconds)
    pub heartbeat_timeout_secs: u64,
    
    /// Voting window duration (seconds)
    pub voting_window_secs: u64,
    
    /// Fallback configuration
    pub fallback_config: FallbackConfig,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            genesis_date: NaiveDate::from_ymd_opt(2025, 10, 24).unwrap(),
            heartbeat_timeout_secs: 30,
            voting_window_secs: 4,
            fallback_config: FallbackConfig::default(),
        }
    }
}

/// Result of the orchestrated consensus process
#[derive(Debug)]
pub enum ConsensusResult {
    /// Block successfully finalized
    Success {
        block: Block,
        phase_duration_ms: Vec<(Phase, u64)>,
    },
    
    /// Consensus failed after all fallback attempts
    Failed {
        reason: String,
        attempts: usize,
    },
    
    /// Emergency block created to prevent chain halt
    Emergency {
        block: Block,
        reason: String,
    },
}

/// Main orchestrator for phased consensus
pub struct ConsensusOrchestrator {
    config: OrchestratorConfig,
    protocol: Arc<PhasedProtocolManager>,
    heartbeat: Arc<HeartbeatManager>,
    leader_elector: Arc<LeaderElector>,
    fallback: Arc<FallbackManager>,
}

impl ConsensusOrchestrator {
    pub fn new(config: OrchestratorConfig) -> Self {
        let protocol = Arc::new(PhasedProtocolManager::new());
        let heartbeat = Arc::new(HeartbeatManager::new(config.heartbeat_timeout_secs));
        let leader_elector = Arc::new(LeaderElector::new(config.genesis_date));
        let fallback = Arc::new(FallbackManager::new(config.fallback_config.clone()));
        
        Self {
            config,
            protocol,
            heartbeat,
            leader_elector,
            fallback,
        }
    }
    
    /// Execute the full phased consensus protocol for a block
    pub async fn execute_consensus(
        &self,
        block_height: u64,
        masternodes: Vec<MasternodeInfo>,
    ) -> ConsensusResult {
        println!();
        println!("╔════════════════════════════════════════════════════════════╗");
        println!("║  PHASED CONSENSUS PROTOCOL - Block #{}                  ", block_height);
        println!("╚════════════════════════════════════════════════════════════╝");
        println!();
        
        // Start the protocol
        self.protocol.start_protocol().await;
        self.fallback.start_round().await;
        
        let mut phase_durations = Vec::new();
        
        // Phase 1: Midnight Synchronization
        let phase1_start = Utc::now();
        match self.phase1_synchronization(masternodes.len()).await {
            Ok(_) => {
                let duration = (Utc::now() - phase1_start).num_milliseconds() as u64;
                phase_durations.push((Phase::Synchronization, duration));
            }
            Err(e) => {
                return ConsensusResult::Failed {
                    reason: format!("Phase 1 failed: {}", e),
                    attempts: 0,
                };
            }
        }
        
        // Phase 2: Leader Election
        let phase2_start = Utc::now();
        let current_date = Utc::now().date_naive();
        let leader_selection = match self
            .phase2_leader_election(block_height, current_date, &masternodes)
            .await
        {
            Ok(selection) => {
                let duration = (Utc::now() - phase2_start).num_milliseconds() as u64;
                phase_durations.push((Phase::LeaderElection, duration));
                selection
            }
            Err(e) => {
                return ConsensusResult::Failed {
                    reason: format!("Phase 2 failed: {}", e),
                    attempts: 0,
                };
            }
        };
        
        // Phases 3-7 with fallback loop
        loop {
            let attempt_count = self.fallback.attempt_count().await;
            
            // Check if emergency block is needed
            if self.fallback.needs_emergency_block().await {
                println!("🚨 EMERGENCY MODE ACTIVATED");
                // In real implementation, would create minimal emergency block
                return ConsensusResult::Emergency {
                    block: self.create_emergency_block(block_height).await,
                    reason: "Persistent consensus failure".to_string(),
                };
            }
            
            // Determine strategy for this attempt
            let strategy = self.fallback.next_strategy().await;
            
            // Select leader (rotate if needed)
            let current_leader = match strategy {
                FallbackStrategy::RotateLeader => {
                    if attempt_count > 0 {
                        // Rotate to next leader
                        self.leader_elector
                            .rotate_leader(
                                block_height,
                                current_date,
                                &masternodes,
                                &leader_selection.leader,
                                attempt_count as u32,
                            )
                            .unwrap_or(leader_selection.clone())
                    } else {
                        leader_selection.clone()
                    }
                }
                _ => leader_selection.clone(),
            };
            
            println!();
            println!("─────────────────────────────────────────────────────────");
            println!("Attempt #{} - Leader: {}", attempt_count + 1, current_leader.leader);
            println!("─────────────────────────────────────────────────────────");
            
            // Phase 3: Block Construction
            let phase3_start = Utc::now();
            self.protocol.advance_phase(Phase::BlockConstruction).await;
            println!("📦 Phase 3: Block Construction");
            
            let include_transactions = !matches!(strategy, FallbackStrategy::RewardOnlyBlock | FallbackStrategy::EmergencyBlock);
            
            if !include_transactions {
                println!("   ⚠️  Using reward-only strategy (skipping mempool txs)");
            }
            
            let duration = (Utc::now() - phase3_start).num_milliseconds() as u64;
            phase_durations.push((Phase::BlockConstruction, duration));
            
            // Phase 4: Proposal Distribution
            let phase4_start = Utc::now();
            self.protocol.advance_phase(Phase::ProposalDistribution).await;
            println!("📡 Phase 4: Proposal Distribution");
            
            // Simulate proposal distribution
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            let duration = (Utc::now() - phase4_start).num_milliseconds() as u64;
            phase_durations.push((Phase::ProposalDistribution, duration));
            
            // Phase 5: Voting Window
            let phase5_start = Utc::now();
            let (consensus_reached, approval_weight, total_weight) = self
                .phase5_voting_window(&masternodes)
                .await;
            
            let duration = (Utc::now() - phase5_start).num_milliseconds() as u64;
            phase_durations.push((Phase::VotingWindow, duration));
            
            // Phase 6: Consensus Collection
            let phase6_start = Utc::now();
            self.protocol.advance_phase(Phase::ConsensusCollection).await;
            
            println!("🗳️  Phase 6: Consensus Collection");
            println!("   Approval weight: {}/{}", approval_weight, total_weight);
            
            let duration = (Utc::now() - phase6_start).num_milliseconds() as u64;
            phase_durations.push((Phase::ConsensusCollection, duration));
            
            // Phase 7: Finalization or Fallback
            let phase7_start = Utc::now();
            self.protocol.advance_phase(Phase::Finalization).await;
            
            if consensus_reached {
                println!("✅ Phase 7: Finalization - Consensus reached!");
                
                self.fallback
                    .record_attempt(
                        FallbackReason::ConsensusNotReached,
                        strategy,
                        current_leader.leader.clone(),
                        true,
                    )
                    .await;
                
                let duration = (Utc::now() - phase7_start).num_milliseconds() as u64;
                phase_durations.push((Phase::Finalization, duration));
                
                // In real implementation, would finalize the actual block
                let block = self.create_finalized_block(block_height).await;
                
                self.fallback.log_summary().await;
                
                return ConsensusResult::Success {
                    block,
                    phase_duration_ms: phase_durations,
                };
            } else {
                println!("❌ Phase 7: Consensus not reached - Initiating fallback");
                
                self.fallback
                    .record_attempt(
                        FallbackReason::ConsensusNotReached,
                        strategy,
                        current_leader.leader.clone(),
                        false,
                    )
                    .await;
                
                // Continue to next fallback attempt
            }
        }
    }
    
    /// Phase 1: Midnight Synchronization
    async fn phase1_synchronization(&self, expected_nodes: usize) -> Result<(), String> {
        self.protocol.advance_phase(Phase::Synchronization).await;
        self.heartbeat.start_sync().await;
        
        println!("💓 Phase 1: Midnight Synchronization");
        println!("   Expected nodes: {}", expected_nodes);
        println!("   Waiting for heartbeats...");
        
        // Wait for heartbeats with timeout
        let mut check_count = 0;
        while check_count < 30 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            if self.heartbeat.check_sync_complete(expected_nodes).await {
                break;
            }
            
            if self.heartbeat.check_timeout().await {
                return Err("Heartbeat timeout".to_string());
            }
            
            check_count += 1;
        }
        
        // Finalize synchronization
        let status = self.heartbeat.finalize_sync(expected_nodes).await;
        
        match status {
            SyncStatus::Synchronized => Ok(()),
            _ => Err(format!("Sync failed: {:?}", status)),
        }
    }
    
    /// Phase 2: Leader Election
    async fn phase2_leader_election(
        &self,
        block_height: u64,
        date: NaiveDate,
        masternodes: &[MasternodeInfo],
    ) -> Result<LeaderSelection, String> {
        self.protocol.advance_phase(Phase::LeaderElection).await;
        
        println!("👑 Phase 2: Leader Election");
        println!("   Block height: {}", block_height);
        println!("   Candidates: {}", masternodes.len());
        
        let selection = self
            .leader_elector
            .elect_leader(block_height, date, masternodes)
            .ok_or("Leader election failed")?;
        
        println!("   ✅ Leader elected: {}", selection.leader);
        println!("   Weight: {}", selection.weight);
        
        Ok(selection)
    }
    
    /// Phase 5: Voting Window
    async fn phase5_voting_window(&self, masternodes: &[MasternodeInfo]) -> (bool, u64, u64) {
        self.protocol.advance_phase(Phase::VotingWindow).await;
        
        println!("🗳️  Phase 5: Voting Window ({}s)", self.config.voting_window_secs);
        
        // Simulate voting window
        tokio::time::sleep(tokio::time::Duration::from_secs(
            self.config.voting_window_secs,
        ))
        .await;
        
        // In real implementation, would collect actual votes
        // For now, simulate votes
        self.simulate_votes(masternodes).await;
        
        // Check consensus
        self.protocol.check_consensus().await
    }
    
    /// Simulate votes for testing (in real impl, votes come from network)
    async fn simulate_votes(&self, masternodes: &[MasternodeInfo]) {
        for (i, mn) in masternodes.iter().enumerate() {
            let vote = WeightedVote {
                voter: mn.node_id.clone(),
                block_hash: "simulated_hash".to_string(),
                approve: i < (masternodes.len() * 2 / 3) + 1, // 2/3+ approve
                weight: crate::phased_protocol::calculate_node_weight(
                    mn.tier,
                    30, // days_active
                    mn.reputation_score,
                ),
                signature: format!("sig_{}", i),
                timestamp: Utc::now().timestamp(),
            };
            
            let _ = self.protocol.register_vote(vote).await;
        }
    }
    
    /// Create emergency block (minimal data to prevent chain halt)
    async fn create_emergency_block(&self, block_height: u64) -> Block {
        // In real implementation, would create actual emergency block
        // with only treasury reward and no transactions
        Block {
            header: time_core::BlockHeader {
                block_number: block_height,
                timestamp: Utc::now(),
                previous_hash: "emergency_previous".to_string(),
                merkle_root: "emergency_merkle".to_string(),
                validator_signature: "emergency_sig".to_string(),
                validator_address: "emergency_validator".to_string(),
            },
            transactions: vec![],
            hash: format!("emergency_block_{}", block_height),
        }
    }
    
    /// Create finalized block
    async fn create_finalized_block(&self, block_height: u64) -> Block {
        // In real implementation, would create actual finalized block
        Block {
            header: time_core::BlockHeader {
                block_number: block_height,
                timestamp: Utc::now(),
                previous_hash: "previous_hash".to_string(),
                merkle_root: "merkle_root".to_string(),
                validator_signature: "validator_sig".to_string(),
                validator_address: "validator_addr".to_string(),
            },
            transactions: vec![],
            hash: format!("finalized_block_{}", block_height),
        }
    }
    
    /// Register a heartbeat (called by network layer)
    pub async fn register_heartbeat(&self, heartbeat: Heartbeat) -> Result<(), String> {
        self.heartbeat.register_heartbeat(heartbeat).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time_core::MasternodeTier;
    
    fn create_test_masternodes() -> Vec<MasternodeInfo> {
        vec![
            MasternodeInfo {
                node_id: "192.168.1.1".to_string(),
                tier: MasternodeTier::Gold,
                registered_at: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                reputation_score: 1.2,
            },
            MasternodeInfo {
                node_id: "192.168.1.2".to_string(),
                tier: MasternodeTier::Silver,
                registered_at: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                reputation_score: 1.0,
            },
            MasternodeInfo {
                node_id: "192.168.1.3".to_string(),
                tier: MasternodeTier::Bronze,
                registered_at: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
                reputation_score: 0.8,
            },
        ]
    }
    
    #[tokio::test]
    async fn test_consensus_orchestrator_creation() {
        let config = OrchestratorConfig::default();
        let orchestrator = ConsensusOrchestrator::new(config);
        
        // Verify orchestrator was created successfully
        assert_eq!(orchestrator.config.voting_window_secs, 4);
        assert_eq!(orchestrator.config.heartbeat_timeout_secs, 30);
    }
    
    #[tokio::test]
    async fn test_heartbeat_registration() {
        let config = OrchestratorConfig::default();
        let orchestrator = ConsensusOrchestrator::new(config);
        
        // Start heartbeat collection
        orchestrator.heartbeat.start_sync().await;
        
        let heartbeat = Heartbeat {
            node_id: "192.168.1.1".to_string(),
            timestamp: Utc::now().timestamp(),
            block_height: 99,
            chain_tip_hash: "test_hash".to_string(),
            tier: MasternodeTier::Gold,
            version: "1.0.0".to_string(),
            reputation_score: 1.0,
            days_active: 30,
        };
        
        let result = orchestrator.register_heartbeat(heartbeat).await;
        assert!(result.is_ok());
        
        let heartbeats = orchestrator.heartbeat.get_heartbeats().await;
        assert_eq!(heartbeats.len(), 1);
    }
}
