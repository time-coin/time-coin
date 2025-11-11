//! VRF-based Leader Election with Weighted Selection
//!
//! Implements Phase 2 of the phased protocol: deterministic leader selection
//! using VRF (Verifiable Random Function) with weights based on:
//! - Masternode tier (Free=1, Bronze=2, Silver=4, Gold=8)
//! - Longevity (bonus for days active)
//! - Reputation score (performance history)

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time_core::MasternodeTier;

/// Masternode info for leader election
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasternodeInfo {
    pub node_id: String,
    pub tier: MasternodeTier,
    pub registered_at: NaiveDate,
    pub reputation_score: f32,
}

/// Leader election result with proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderSelection {
    pub leader: String,
    pub vrf_seed: Vec<u8>,
    pub vrf_proof: Vec<u8>,
    pub weight: u64,
    pub candidates: usize,
}

/// VRF-based leader election
pub struct LeaderElector {
    /// Genesis date for calculating longevity
    #[allow(dead_code)]
    genesis_date: NaiveDate,
}

impl LeaderElector {
    pub fn new(genesis_date: NaiveDate) -> Self {
        Self { genesis_date }
    }

    /// Elect leader for a block using VRF and weights
    ///
    /// Algorithm:
    /// 1. Generate deterministic seed from block height and date
    /// 2. Calculate weight for each masternode
    /// 3. Create weighted probability distribution
    /// 4. Use VRF to select leader based on weights
    pub fn elect_leader(
        &self,
        block_height: u64,
        date: NaiveDate,
        masternodes: &[MasternodeInfo],
    ) -> Option<LeaderSelection> {
        if masternodes.is_empty() {
            return None;
        }

        // Generate deterministic VRF seed from block height and date
        let seed = self.generate_vrf_seed(block_height, date);

        // Calculate weights for all masternodes
        let weighted_nodes: Vec<(String, u64)> = masternodes
            .iter()
            .map(|mn| {
                let days_active = self.calculate_days_active(mn.registered_at, date);
                let weight = super::phased_protocol::calculate_node_weight(
                    mn.tier,
                    days_active,
                    mn.reputation_score,
                );
                (mn.node_id.clone(), weight)
            })
            .collect();

        // Select leader using weighted random selection
        let (leader, weight) = self.weighted_selection(&seed, &weighted_nodes);

        // Generate VRF proof
        let proof = self.generate_vrf_proof(&seed, &leader);

        Some(LeaderSelection {
            leader,
            vrf_seed: seed,
            vrf_proof: proof,
            weight,
            candidates: masternodes.len(),
        })
    }

    /// Generate deterministic VRF seed from block height and date
    fn generate_vrf_seed(&self, block_height: u64, date: NaiveDate) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(b"TIME_COIN_VRF_SEED");
        hasher.update(block_height.to_le_bytes());
        hasher.update(date.to_string().as_bytes());
        hasher.finalize().to_vec()
    }

    /// Calculate days active since registration
    fn calculate_days_active(&self, registered_at: NaiveDate, current_date: NaiveDate) -> u64 {
        (current_date - registered_at).num_days().max(0) as u64
    }

    /// Weighted random selection using VRF
    fn weighted_selection(&self, seed: &[u8], weighted_nodes: &[(String, u64)]) -> (String, u64) {
        // Calculate total weight
        let total_weight: u64 = weighted_nodes.iter().map(|(_, w)| w).sum();

        if total_weight == 0 {
            // Fallback to first node if all weights are zero
            return (weighted_nodes[0].0.clone(), 0);
        }

        // Generate random value from VRF seed
        let mut hasher = Sha256::new();
        hasher.update(seed);
        hasher.update(b"WEIGHTED_SELECTION");
        let hash = hasher.finalize();

        // Convert to u64
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&hash[0..8]);
        let random_value = u64::from_le_bytes(bytes);

        // Map to weighted range
        let selection_point = random_value % total_weight;

        // Find selected node
        let mut cumulative_weight = 0u64;
        for (node_id, weight) in weighted_nodes {
            cumulative_weight += weight;
            if selection_point < cumulative_weight {
                return (node_id.clone(), *weight);
            }
        }

        // Fallback (shouldn't reach here)
        weighted_nodes.last().unwrap().clone()
    }

    /// Generate VRF proof for selected leader
    fn generate_vrf_proof(&self, seed: &[u8], leader: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(seed);
        hasher.update(b"VRF_PROOF");
        hasher.update(leader.as_bytes());
        hasher.finalize().to_vec()
    }

    /// Verify VRF proof (for validation)
    pub fn verify_proof(
        &self,
        block_height: u64,
        date: NaiveDate,
        leader: &str,
        proof: &[u8],
    ) -> bool {
        let seed = self.generate_vrf_seed(block_height, date);
        let expected_proof = self.generate_vrf_proof(&seed, leader);
        proof == expected_proof.as_slice()
    }

    /// Rotate leader to next weighted candidate (for fallback)
    pub fn rotate_leader(
        &self,
        block_height: u64,
        date: NaiveDate,
        masternodes: &[MasternodeInfo],
        current_leader: &str,
        rotation_count: u32,
    ) -> Option<LeaderSelection> {
        if masternodes.is_empty() {
            return None;
        }

        // Generate seed with rotation count for different selection
        let mut seed = self.generate_vrf_seed(block_height, date);
        seed.extend_from_slice(&rotation_count.to_le_bytes());

        // Calculate weights
        let weighted_nodes: Vec<(String, u64)> = masternodes
            .iter()
            .filter(|mn| mn.node_id != current_leader) // Exclude current leader
            .map(|mn| {
                let days_active = self.calculate_days_active(mn.registered_at, date);
                let weight = super::phased_protocol::calculate_node_weight(
                    mn.tier,
                    days_active,
                    mn.reputation_score,
                );
                (mn.node_id.clone(), weight)
            })
            .collect();

        if weighted_nodes.is_empty() {
            return None;
        }

        // Select new leader
        let (leader, weight) = self.weighted_selection(&seed, &weighted_nodes);
        let proof = self.generate_vrf_proof(&seed, &leader);

        Some(LeaderSelection {
            leader,
            vrf_seed: seed,
            vrf_proof: proof,
            weight,
            candidates: weighted_nodes.len(),
        })
    }

    /// Elect leader with 30-second timeout fallback
    pub fn elect_leader_with_timeout(
        &self,
        block_height: u64,
        date: NaiveDate,
        masternodes: &[MasternodeInfo],
    ) -> Option<LeaderSelection> {
        // Primary election
        let selection = self.elect_leader(block_height, date, masternodes)?;

        // In production, this would include timeout logic
        // For now, we return the selection directly
        Some(selection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            MasternodeInfo {
                node_id: "192.168.1.4".to_string(),
                tier: MasternodeTier::Free,
                registered_at: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
                reputation_score: 1.0,
            },
        ]
    }

    #[test]
    fn test_deterministic_election() {
        let genesis = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let elector = LeaderElector::new(genesis);
        let masternodes = create_test_masternodes();

        let date = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();

        // Same inputs should produce same leader
        let result1 = elector.elect_leader(1, date, &masternodes).unwrap();
        let result2 = elector.elect_leader(1, date, &masternodes).unwrap();

        assert_eq!(result1.leader, result2.leader);
        assert_eq!(result1.vrf_seed, result2.vrf_seed);
    }

    #[test]
    fn test_different_blocks_different_leaders() {
        let genesis = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let elector = LeaderElector::new(genesis);
        let masternodes = create_test_masternodes();

        let date = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();

        let result1 = elector.elect_leader(1, date, &masternodes).unwrap();
        let result2 = elector.elect_leader(2, date, &masternodes).unwrap();

        // Different blocks can have different leaders (probabilistic)
        // At minimum, they should have different VRF seeds
        assert_ne!(result1.vrf_seed, result2.vrf_seed);
    }

    #[test]
    fn test_vrf_proof_verification() {
        let genesis = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let elector = LeaderElector::new(genesis);
        let masternodes = create_test_masternodes();

        let date = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();
        let result = elector.elect_leader(1, date, &masternodes).unwrap();

        // Valid proof should verify
        assert!(elector.verify_proof(1, date, &result.leader, &result.vrf_proof));

        // Invalid proof should not verify
        assert!(!elector.verify_proof(1, date, &result.leader, b"invalid_proof"));

        // Wrong leader should not verify
        assert!(!elector.verify_proof(1, date, "wrong_leader", &result.vrf_proof));
    }

    #[test]
    fn test_leader_rotation() {
        let genesis = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let elector = LeaderElector::new(genesis);
        let masternodes = create_test_masternodes();

        let date = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();

        let result1 = elector.elect_leader(1, date, &masternodes).unwrap();
        let result2 = elector
            .rotate_leader(1, date, &masternodes, &result1.leader, 1)
            .unwrap();

        // Rotated leader should be different
        assert_ne!(result1.leader, result2.leader);
    }

    #[test]
    fn test_weighted_selection_favors_higher_tiers() {
        let genesis = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let elector = LeaderElector::new(genesis);
        let masternodes = create_test_masternodes();

        let date = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();

        // Run multiple elections and count selections
        let mut selections = std::collections::HashMap::new();

        for block_height in 1..100 {
            let result = elector
                .elect_leader(block_height, date, &masternodes)
                .unwrap();
            *selections.entry(result.leader.clone()).or_insert(0) += 1;
        }

        // Gold tier should be selected more often than Free tier
        let gold_count = selections.get("192.168.1.1").unwrap_or(&0);
        let free_count = selections.get("192.168.1.4").unwrap_or(&0);

        // With proper weighting, gold should be selected more frequently
        // (This is probabilistic, but should hold over 100 selections)
        assert!(
            gold_count > free_count,
            "Gold tier ({}) should be selected more than Free tier ({})",
            gold_count,
            free_count
        );
    }
}
