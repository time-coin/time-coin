//! VRF (Verifiable Random Function) for leader selection
//!
//! Provides deterministic, unpredictable, and verifiable leader selection
//! using SHA256-based VRF with block height as the primary seed.
//!
//! ## Critical Design Decision
//!
//! This VRF uses ONLY the block height (not previous_hash) as the seed to ensure
//! all nodes agree on the leader selection, even when nodes are at different
//! sync states. Using previous_hash would cause nodes with divergent chain tips
//! to select different leaders, breaking consensus.

use sha2::{Digest, Sha256};

/// Trait for VRF-based leader selection
pub trait VRFSelector {
    /// Generate VRF seed from block height and previous block hash
    ///
    /// # Arguments
    /// * `height` - Current block height
    /// * `previous_hash` - Hash of previous block
    /// * `is_synced` - Whether node is synchronized (uses hash if true)
    fn generate_seed(&self, height: u64, previous_hash: &str, is_synced: bool) -> Vec<u8>;

    /// Select leader index from available masternodes
    fn select_index(&self, seed: &[u8], count: usize) -> usize;

    /// Select leader from masternode list
    fn select_leader(
        &self,
        masternodes: &[String],
        height: u64,
        previous_hash: &str,
        is_synced: bool,
    ) -> Option<String> {
        if masternodes.is_empty() {
            return None;
        }

        let seed = self.generate_seed(height, previous_hash, is_synced);
        let index = self.select_index(&seed, masternodes.len());
        Some(masternodes[index].clone())
    }

    /// Generate VRF proof for selected leader
    fn generate_proof(
        &self,
        height: u64,
        previous_hash: &str,
        is_synced: bool,
        leader: &str,
    ) -> Vec<u8> {
        let seed = self.generate_seed(height, previous_hash, is_synced);

        let mut hasher = Sha256::new();
        hasher.update(&seed);
        hasher.update(b"VRF_PROOF");
        hasher.update(leader.as_bytes());
        hasher.finalize().to_vec()
    }

    /// Verify VRF proof for a leader selection
    fn verify_proof(
        &self,
        height: u64,
        previous_hash: &str,
        is_synced: bool,
        leader: &str,
        proof: &[u8],
    ) -> bool {
        let expected_proof = self.generate_proof(height, previous_hash, is_synced, leader);
        proof == expected_proof.as_slice()
    }
}

/// Default SHA256-based VRF implementation
pub struct DefaultVRFSelector;

impl VRFSelector for DefaultVRFSelector {
    fn generate_seed(&self, height: u64, previous_hash: &str, is_synced: bool) -> Vec<u8> {
        // Consensus-aware VRF seed generation:
        // - During bootstrap/sync (is_synced=false): Use ONLY height for agreement
        //   across nodes with different chain tips
        // - When synced (is_synced=true): Include previous_hash to make leader
        //   selection dependent on chain state, preventing same leader on both
        //   sides of a fork
        let mut hasher = Sha256::new();
        hasher.update(b"TIME_COIN_VRF_SEED");
        hasher.update(height.to_le_bytes());

        // Only include chain state once nodes are synchronized
        if is_synced && height > 0 && !previous_hash.is_empty() {
            hasher.update(previous_hash.as_bytes());
        }

        hasher.finalize().to_vec()
    }

    fn select_index(&self, seed: &[u8], count: usize) -> usize {
        let mut hasher = Sha256::new();
        hasher.update(seed);
        let hash = hasher.finalize();

        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&hash[0..8]);
        let value = u64::from_le_bytes(bytes);

        (value % count as u64) as usize
    }
}

/// Weighted VRF selector for tier-based selection
pub struct WeightedVRFSelector;

impl WeightedVRFSelector {
    /// Weighted selection using VRF seed
    pub fn weighted_selection(
        &self,
        seed: &[u8],
        masternodes: &[String],
        weights: &[u64],
    ) -> String {
        let total_weight: u64 = weights.iter().sum();

        if total_weight == 0 || masternodes.is_empty() {
            return masternodes.first().cloned().unwrap_or_default();
        }

        let mut hasher = Sha256::new();
        hasher.update(seed);
        hasher.update(b"WEIGHTED_SELECTION");
        let hash = hasher.finalize();

        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&hash[0..8]);
        let random_value = u64::from_le_bytes(bytes);

        let selection_point = random_value % total_weight;

        let mut cumulative_weight = 0u64;
        for (i, weight) in weights.iter().enumerate() {
            cumulative_weight += weight;
            if selection_point < cumulative_weight {
                return masternodes[i].clone();
            }
        }

        masternodes.last().cloned().unwrap_or_default()
    }
}

impl VRFSelector for WeightedVRFSelector {
    fn generate_seed(&self, height: u64, previous_hash: &str, is_synced: bool) -> Vec<u8> {
        // Same consensus-aware seed as DefaultVRFSelector
        let mut hasher = Sha256::new();
        hasher.update(b"TIME_COIN_VRF_SEED");
        hasher.update(height.to_le_bytes());

        if is_synced && height > 0 && !previous_hash.is_empty() {
            hasher.update(previous_hash.as_bytes());
        }

        hasher.finalize().to_vec()
    }

    fn select_index(&self, seed: &[u8], count: usize) -> usize {
        // Equal weights for basic selection
        let weights = vec![1u64; count];
        let masternodes: Vec<String> = (0..count).map(|i| i.to_string()).collect();
        let selected = self.weighted_selection(seed, &masternodes, &weights);
        selected.parse().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vrf_deterministic() {
        let vrf = DefaultVRFSelector;
        let masternodes = vec![
            "node1".to_string(),
            "node2".to_string(),
            "node3".to_string(),
        ];

        let leader1 = vrf.select_leader(&masternodes, 100, "hash123", true);
        let leader2 = vrf.select_leader(&masternodes, 100, "hash123", true);

        assert_eq!(leader1, leader2, "VRF should be deterministic");
    }

    #[test]
    fn test_vrf_different_heights() {
        let vrf = DefaultVRFSelector;
        let masternodes = vec![
            "node1".to_string(),
            "node2".to_string(),
            "node3".to_string(),
            "node4".to_string(),
        ];

        let leader1 = vrf.select_leader(&masternodes, 100, "hash", true).unwrap();
        let leader2 = vrf.select_leader(&masternodes, 101, "hash", true).unwrap();

        assert!(masternodes.contains(&leader1));
        assert!(masternodes.contains(&leader2));
    }

    #[test]
    fn test_vrf_sync_state_affects_selection() {
        let vrf = DefaultVRFSelector;
        let masternodes = vec![
            "node1".to_string(),
            "node2".to_string(),
            "node3".to_string(),
        ];

        // When not synced, hash is ignored
        let leader_unsynced1 = vrf.select_leader(&masternodes, 100, "hash1", false);
        let leader_unsynced2 = vrf.select_leader(&masternodes, 100, "hash2", false);
        assert_eq!(
            leader_unsynced1, leader_unsynced2,
            "Unsynced nodes should agree on leader regardless of hash"
        );

        // When synced, different hashes may select different leaders
        let seed_synced1 = vrf.generate_seed(100, "hash1", true);
        let seed_synced2 = vrf.generate_seed(100, "hash2", true);
        assert_ne!(
            seed_synced1, seed_synced2,
            "Synced nodes use hash in seed, creating fork-specific leaders"
        );
    }

    #[test]
    fn test_vrf_proof() {
        let vrf = DefaultVRFSelector;
        let height = 100u64;
        let prev_hash = "test_hash";
        let leader = "node1";

        let proof = vrf.generate_proof(height, prev_hash, true, leader);
        assert!(vrf.verify_proof(height, prev_hash, true, leader, &proof));
        assert!(!vrf.verify_proof(height, prev_hash, true, "node2", &proof));
    }
}
