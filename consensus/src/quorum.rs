//! Quorum Selection using Verifiable Random Functions

use crate::{MasternodeInfo, Transaction};
use crate::vrf::Vrf;

/// Selects masternodes for consensus quorum
pub struct QuorumSelector {
    quorum_size: usize,
}

impl QuorumSelector {
    pub fn new(quorum_size: usize) -> Self {
        Self { quorum_size }
    }
    
    /// Select quorum using VRF-based weighted random selection
    pub fn select_quorum(
        &self,
        tx: &Transaction,
        all_nodes: &[MasternodeInfo],
    ) -> Vec<MasternodeInfo> {
        if all_nodes.is_empty() {
            return Vec::new();
        }
        
        // Filter eligible nodes
        let eligible: Vec<_> = all_nodes
            .iter()
            .filter(|n| n.is_eligible())
            .cloned()
            .collect();
        
        if eligible.is_empty() {
            return Vec::new();
        }
        
        // If we have fewer eligible nodes than quorum size, use all
        if eligible.len() <= self.quorum_size {
            return eligible;
        }
        
        // Use transaction hash as VRF seed for deterministic selection
        let seed = self.hash_to_seed(&tx.hash());
        let mut vrf = Vrf::new(seed);
        
        // Calculate weights
        let weights: Vec<u64> = eligible.iter().map(|n| n.voting_power()).collect();
        let total_weight: u64 = weights.iter().sum();
        
        if total_weight == 0 {
            return eligible;
        }
        
        // Weighted random selection without replacement
        let mut selected = Vec::new();
        let mut remaining: Vec<_> = eligible.iter().enumerate().collect();
        
        for _ in 0..self.quorum_size.min(eligible.len()) {
            if remaining.is_empty() {
                break;
            }
            
            // Calculate cumulative weights for remaining nodes
            let mut cumulative = Vec::new();
            let mut sum = 0u64;
            for &(idx, _) in &remaining {
                sum += weights[idx];
                cumulative.push(sum);
            }
            
            // Random selection
            let random = vrf.next_u64() % sum;
            let selected_idx = cumulative
                .iter()
                .position(|&w| random < w)
                .unwrap_or(0);
            
            let (_, node) = remaining.remove(selected_idx);
            selected.push(node.clone());
        }
        
        selected
    }
    
    /// Convert hash string to u64 seed
    fn hash_to_seed(&self, hash: &str) -> u64 {
        let bytes = hash.as_bytes();
        let mut seed = 0u64;
        
        for (i, &b) in bytes.iter().take(8).enumerate() {
            seed |= (b as u64) << (i * 8);
        }
        
        seed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MasternodeTier;
    
    fn create_test_nodes(count: usize) -> Vec<MasternodeInfo> {
        (0..count)
            .map(|i| MasternodeInfo {
                address: format!("node{}", i),
                tier: if i % 3 == 0 {
                    MasternodeTier::Professional
                } else if i % 3 == 1 {
                    MasternodeTier::Verified
                } else {
                    MasternodeTier::Community
                },
                active_since: chrono::Utc::now().timestamp(),
                uptime_score: 0.95,
                reputation: 100,
            })
            .collect()
    }
    
    #[test]
    fn test_quorum_selection() {
        let nodes = create_test_nodes(20);
        let selector = QuorumSelector::new(7);
        
        let tx = Transaction {
            txid: "test_tx".to_string(),
            from: "addr1".to_string(),
            to: "addr2".to_string(),
            amount: 100,
            fee: 1,
            timestamp: chrono::Utc::now().timestamp(),
            nonce: 0,
        };
        
        let quorum = selector.select_quorum(&tx, &nodes);
        
        assert_eq!(quorum.len(), 7);
        
        // Verify all selected nodes are unique
        let addresses: std::collections::HashSet<_> = 
            quorum.iter().map(|n| &n.address).collect();
        assert_eq!(addresses.len(), 7);
    }
    
    #[test]
    fn test_deterministic_selection() {
        let nodes = create_test_nodes(20);
        let selector = QuorumSelector::new(7);
        
        let tx = Transaction {
            txid: "test_tx".to_string(),
            from: "addr1".to_string(),
            to: "addr2".to_string(),
            amount: 100,
            fee: 1,
            timestamp: chrono::Utc::now().timestamp(),
            nonce: 0,
        };
        
        let quorum1 = selector.select_quorum(&tx, &nodes);
        let quorum2 = selector.select_quorum(&tx, &nodes);
        
        // Same transaction should select same quorum
        assert_eq!(
            quorum1.iter().map(|n| &n.address).collect::<Vec<_>>(),
            quorum2.iter().map(|n| &n.address).collect::<Vec<_>>()
        );
    }
}
