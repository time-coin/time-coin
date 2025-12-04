//! Simplified Deterministic Consensus Model
//!
//! Clean, simple consensus that works as follows:
//! 1. Midnight arrives → All nodes agree on VRF-selected leader (deterministic)
//! 2. Leader creates block → Based on mempool + deterministic rewards
//! 3. Leader broadcasts proposal → Other nodes verify it matches expectations
//! 4. Quick approval → Nodes vote approve if it matches deterministic state
//! 5. Transaction mismatches → Missing transactions broadcast, validated, block recreated

use crate::core::vrf::{DefaultVRFSelector, VRFSelector};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Simplified consensus manager
pub struct SimplifiedConsensus {
    /// Network type
    _network: String,

    /// Active masternodes (IP addresses)
    masternodes: Arc<RwLock<Vec<String>>>,

    /// Pending block proposals
    proposals: Arc<RwLock<HashMap<u64, BlockProposal>>>,

    /// Votes for current block
    votes: Arc<RwLock<HashMap<u64, Vec<BlockVote>>>>,

    /// Known transactions (mempool)
    _known_transactions: Arc<RwLock<HashSet<String>>>,

    /// VRF selector
    vrf_selector: DefaultVRFSelector,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockProposal {
    pub height: u64,
    pub leader: String,
    pub block_hash: String,
    pub previous_hash: String,
    pub merkle_root: String,
    pub transaction_ids: Vec<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockVote {
    pub height: u64,
    pub block_hash: String,
    pub voter: String,
    pub approve: bool,
    pub reason: Option<String>, // For rejections: "missing_tx:txid1,txid2"
}

impl SimplifiedConsensus {
    pub fn new(network: String) -> Self {
        Self {
            _network: network.to_string(),
            masternodes: Arc::new(RwLock::new(Vec::new())),
            proposals: Arc::new(RwLock::new(HashMap::new())),
            votes: Arc::new(RwLock::new(HashMap::new())),
            _known_transactions: Arc::new(RwLock::new(HashSet::new())),
            vrf_selector: DefaultVRFSelector,
        }
    }

    /// Update masternode list
    pub async fn set_masternodes(&self, nodes: Vec<String>) {
        let mut masternodes = self.masternodes.write().await;
        let old_count = masternodes.len();
        *masternodes = nodes;
        masternodes.sort(); // Keep deterministic
        println!(
            "📋 Simplified consensus: masternode list updated {} → {} nodes",
            old_count,
            masternodes.len()
        );
    }

    /// Get current masternode list
    pub async fn get_masternodes(&self) -> Vec<String> {
        self.masternodes.read().await.clone()
    }

    /// Add transaction to known set
    pub async fn add_known_transaction(&self, txid: String) {
        let mut known = self._known_transactions.write().await;
        known.insert(txid);
    }

    /// Check if transaction is known
    pub async fn has_transaction(&self, txid: &str) -> bool {
        let known = self._known_transactions.read().await;
        known.contains(txid)
    }

    /// Select leader for a block using VRF
    pub async fn select_leader(&self, height: u64, previous_hash: &str) -> Option<String> {
        let masternodes = self.masternodes.read().await;

        if masternodes.is_empty() {
            return None;
        }

        println!(
            "🔐 SimplifiedConsensus: Leader election for block {}:",
            height
        );
        println!(
            "   Prev hash: {}... (note: NOT used in VRF seed)",
            &previous_hash[..previous_hash.len().min(16)]
        );
        println!("   Masternode count: {}", masternodes.len());

        // Use VRF trait for deterministic selection (uses ONLY height internally)
        let leader = self
            .vrf_selector
            .select_leader(&masternodes, height, previous_hash);

        if let Some(ref l) = leader {
            println!("👑 SimplifiedConsensus: Selected leader: {}", l);
        }

        leader
    }

    /// Leader proposes a block
    pub async fn propose_block(&self, proposal: BlockProposal) -> Result<(), String> {
        // Verify proposer is the designated leader
        let expected_leader = self
            .select_leader(proposal.height, &proposal.previous_hash)
            .await;

        if expected_leader.as_ref() != Some(&proposal.leader) {
            return Err(format!(
                "Invalid proposer: expected {:?}, got {}",
                expected_leader, proposal.leader
            ));
        }

        println!("📋 Block proposal received from leader {}", proposal.leader);
        println!("   Height: {}", proposal.height);
        println!("   Transactions: {}", proposal.transaction_ids.len());

        let mut proposals = self.proposals.write().await;
        proposals.insert(proposal.height, proposal);

        Ok(())
    }

    /// Validate proposal matches local deterministic state
    pub async fn validate_proposal(&self, proposal: &BlockProposal) -> Result<(), Vec<String>> {
        let known = self._known_transactions.read().await;

        // Check for missing transactions
        let missing: Vec<String> = proposal
            .transaction_ids
            .iter()
            .filter(|txid| !known.contains(*txid))
            .cloned()
            .collect();

        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }

    /// Vote on a block proposal
    pub async fn vote(
        &self,
        height: u64,
        block_hash: String,
        voter: String,
        approve: bool,
        reason: Option<String>,
    ) -> Result<(), String> {
        // Verify voter is a masternode
        let masternodes = self.masternodes.read().await;
        if !masternodes.contains(&voter) {
            return Err("Not a registered masternode".to_string());
        }
        drop(masternodes);

        let mut votes = self.votes.write().await;
        let vote_list = votes.entry(height).or_insert_with(Vec::new);

        // Check for duplicate vote
        if vote_list.iter().any(|v| v.voter == voter) {
            return Err("Already voted".to_string());
        }

        vote_list.push(BlockVote {
            height,
            block_hash,
            voter: voter.clone(),
            approve,
            reason: reason.clone(),
        });

        if approve {
            println!("   ✅ {} approved", voter);
        } else {
            println!("   ❌ {} rejected: {:?}", voter, reason);
        }

        Ok(())
    }

    /// Check if consensus reached (2/3+)
    pub async fn has_consensus(
        &self,
        height: u64,
        block_hash: &str,
    ) -> (bool, usize, usize, Vec<String>) {
        let masternodes = self.masternodes.read().await;
        let total = masternodes.len();
        drop(masternodes);

        if total < 3 {
            // Bootstrap mode: accept with any vote
            return (true, 1, total, Vec::new());
        }

        let votes = self.votes.read().await;
        let vote_list = votes.get(&height);

        if let Some(votes) = vote_list {
            let approvals = votes
                .iter()
                .filter(|v| v.approve && v.block_hash == block_hash)
                .count();

            let rejections: Vec<String> = votes
                .iter()
                .filter(|v| !v.approve)
                .map(|v| format!("{}: {:?}", v.voter, v.reason))
                .collect();

            let required = crate::quorum::required_for_bft(total);
            let has_consensus = approvals >= required;

            (has_consensus, approvals, required, rejections)
        } else {
            (false, 0, crate::quorum::required_for_bft(total), Vec::new())
        }
    }

    /// Get missing transactions from rejections
    pub async fn get_missing_transactions(&self, height: u64) -> HashSet<String> {
        let votes = self.votes.read().await;
        let mut missing = HashSet::new();

        if let Some(vote_list) = votes.get(&height) {
            for vote in vote_list {
                if !vote.approve {
                    if let Some(reason) = &vote.reason {
                        if reason.starts_with("missing_tx:") {
                            let txids = reason.strip_prefix("missing_tx:").unwrap();
                            for txid in txids.split(',') {
                                missing.insert(txid.to_string());
                            }
                        }
                    }
                }
            }
        }

        missing
    }

    /// Clear votes for a height
    pub async fn clear_votes(&self, height: u64) {
        let mut votes = self.votes.write().await;
        votes.remove(&height);
    }

    /// Get proposal
    pub async fn get_proposal(&self, height: u64) -> Option<BlockProposal> {
        let proposals = self.proposals.read().await;
        proposals.get(&height).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_leader_selection_deterministic() {
        let consensus = SimplifiedConsensus::new("testnet".to_string());

        consensus
            .set_masternodes(vec![
                "192.168.1.1".to_string(),
                "192.168.1.2".to_string(),
                "192.168.1.3".to_string(),
            ])
            .await;

        let leader1 = consensus.select_leader(100, "prev_hash").await;
        let leader2 = consensus.select_leader(100, "prev_hash").await;

        assert_eq!(leader1, leader2, "Leader selection must be deterministic");
    }

    #[tokio::test]
    async fn test_consensus_threshold() {
        let consensus = SimplifiedConsensus::new("testnet".to_string());

        consensus
            .set_masternodes(vec![
                "mn1".to_string(),
                "mn2".to_string(),
                "mn3".to_string(),
            ])
            .await;

        let block_hash = "test_hash".to_string();

        // 2 out of 3 approve
        consensus
            .vote(100, block_hash.clone(), "mn1".to_string(), true, None)
            .await
            .unwrap();
        consensus
            .vote(100, block_hash.clone(), "mn2".to_string(), true, None)
            .await
            .unwrap();

        let (has_consensus, approvals, required, _) =
            consensus.has_consensus(100, &block_hash).await;

        assert!(has_consensus, "2/3 should reach consensus");
        assert_eq!(approvals, 2);
        assert_eq!(required, 2); // Ceiling of 2/3 * 3
    }
}
