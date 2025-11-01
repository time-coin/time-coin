//! TIME Coin Consensus Engine
//!
//! Implements leader-based block production with Byzantine Fault Tolerance
//! Requires minimum 3 masternodes for full BFT consensus

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use time_core::block::Block;
use time_core::state::BlockchainState;
use time_core::transaction::Transaction;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusMode {
    Development,         // Single node, no consensus needed
    BootstrapNoQuorum,  // < 3 nodes, block production without voting
    BFT,                // >= 3 nodes, full BFT consensus with 2/3+ quorum
}

pub struct ConsensusEngine {
    /// Development mode flag
    dev_mode: bool,

    /// Registered masternodes (addresses)
    masternodes: Arc<RwLock<Vec<String>>>,
    
    /// Map node_id -> wallet_address
    wallet_addresses: Arc<RwLock<HashMap<String, String>>>,

    /// Current blockchain state
    state: Arc<RwLock<Option<BlockchainState>>>,

    /// Pending votes for current block
    pending_votes: Arc<RwLock<HashMap<String, Vec<Vote>>>>, // block_hash -> votes
}

impl ConsensusEngine {
    pub fn new(dev_mode: bool) -> Self {
        Self {
            dev_mode,
            masternodes: Arc::new(RwLock::new(Vec::new())),
            wallet_addresses: Arc::new(RwLock::new(HashMap::new())),
            state: Arc::new(RwLock::new(None)),
            pending_votes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set blockchain state
    pub async fn set_state(&self, state: BlockchainState) {
        let mut s = self.state.write().await;
        *s = Some(state);
    }

    /// Register a masternode
    pub async fn add_masternode(&self, address: String) {
        let mut masternodes = self.masternodes.write().await;
        if !masternodes.contains(&address) {
            masternodes.push(address);
            masternodes.sort(); // Keep deterministic ordering
        }
    }

    /// Get registered masternodes
    pub async fn get_masternodes(&self) -> Vec<String> {
        self.masternodes.read().await.clone()
    }

    /// Get masternode count
    pub async fn masternode_count(&self) -> usize {
        self.masternodes.read().await.len()
    }

    /// Register wallet address for a masternode
    pub async fn register_wallet(&self, node_id: String, wallet_address: String) {
        let mut wallets = self.wallet_addresses.write().await;
        wallets.insert(node_id, wallet_address);
    }

    /// Get wallet address for a node
    pub async fn get_wallet_address(&self, node_id: &str) -> Option<String> {
        let wallets = self.wallet_addresses.read().await;
        wallets.get(node_id).cloned()
    }

    /// Get all masternodes with their wallet addresses
    pub async fn get_masternodes_with_wallets(&self) -> Vec<(String, String)> {
        let masternodes = self.masternodes.read().await;
        let wallets = self.wallet_addresses.read().await;
        
        masternodes.iter()
            .filter_map(|node_id| {
                wallets.get(node_id).map(|wallet| (node_id.clone(), wallet.clone()))
            })
            .collect()
    }

    /// Sync masternode list with current peers (replaces existing list)
    pub async fn sync_masternodes(&self, peer_ips: Vec<String>) {
        let mut masternodes = self.masternodes.write().await;
        *masternodes = peer_ips;
        masternodes.sort(); // Keep deterministic ordering
    }

    /// Get block producer for a given block height
    pub async fn get_block_producer(&self, block_height: u64) -> Option<String> {
        let masternodes = self.masternodes.read().await;

        if masternodes.is_empty() {
            return None;
        }

        // Deterministic selection based on block height
        let index = (block_height as usize) % masternodes.len();
        Some(masternodes[index].clone())
    }

    /// Check if it's my turn to produce this block
    pub async fn is_my_turn(&self, block_height: u64, my_ip: &str) -> bool {
        match self.get_block_producer(block_height).await {
            Some(producer) => producer == my_ip,
            None => false,
        }
    }

    /// Check if we have BFT quorum (at least 3 nodes)
    pub async fn has_bft_quorum(&self) -> bool {
        self.masternodes.read().await.len() >= 3
    }

    /// Get current consensus mode
    pub async fn consensus_mode(&self) -> ConsensusMode {
        if self.dev_mode {
            return ConsensusMode::Development;
        }

        let count = self.masternode_count().await;
        match count {
            0..=2 => ConsensusMode::BootstrapNoQuorum,
            _ => ConsensusMode::BFT,
        }
    }

    /// Check if node is the leader for this block
    pub async fn is_leader(&self, block_height: u64, node_address: &str) -> bool {
        let masternodes = self.masternodes.read().await;

        if masternodes.is_empty() {
            return false;
        }

        let leader_index = (block_height as usize) % masternodes.len();
        masternodes.get(leader_index).map(|addr| addr == node_address).unwrap_or(false)
    }

    /// Get the leader for a block
    pub async fn get_leader(&self, block_height: u64) -> Option<String> {
        let masternodes = self.masternodes.read().await;

        if masternodes.is_empty() {
            return None;
        }

        let leader_index = (block_height as usize) % masternodes.len();
        masternodes.get(leader_index).cloned()
    }

    /// Propose a block for voting
    pub async fn propose_block(
        &self,
        block: Block,
        proposer: String,
    ) -> Result<String, ConsensusError> {
        // Verify proposer is the designated block producer
        if !self.is_leader(block.header.block_number, &proposer).await {
            return Err(ConsensusError::UnauthorizedProposer);
        }

        let block_hash = block.calculate_hash();
        
        // In BFT mode, we need votes
        // In bootstrap mode, accept immediately
        let mode = self.consensus_mode().await;
        
        match mode {
            ConsensusMode::Development | ConsensusMode::BootstrapNoQuorum => {
                // Auto-accept in dev/bootstrap mode
                Ok(block_hash)
            }
            ConsensusMode::BFT => {
                // Need to collect votes
                Ok(block_hash)
            }
        }
    }

    /// Validate a proposed block
    pub async fn validate_block(&self, block: &Block) -> Result<bool, ConsensusError> {
        // Basic validation checks
        if block.transactions.is_empty() {
            return Ok(false);
        }

        // Verify block hash
        let computed_hash = block.calculate_hash();
        if computed_hash != block.hash {
            return Ok(false);
        }

        // Check with blockchain state if available
        if let Some(state) = self.state.read().await.as_ref() {
            // Verify previous hash matches
            if block.header.block_number > 0 {
                let expected_prev = state.chain_tip_hash();
                if block.header.previous_hash != expected_prev {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Vote on a proposed block
    pub async fn vote_on_block(
        &self,
        block_hash: String,
        voter: String,
        approve: bool,
    ) -> Result<(), ConsensusError> {
        // Verify voter is a registered masternode
        let masternodes = self.masternodes.read().await;
        if !masternodes.contains(&voter) {
            return Err(ConsensusError::UnauthorizedVoter);
        }
        drop(masternodes);

        let mut votes = self.pending_votes.write().await;
        let vote_list = votes.entry(block_hash.clone()).or_insert_with(Vec::new);

        // Check if already voted
        if vote_list.iter().any(|v| v.voter == voter) {
            return Err(ConsensusError::DuplicateVote);
        }

        vote_list.push(Vote {
            block_hash,
            voter,
            approve,
            timestamp: chrono::Utc::now().timestamp(),
        });

        Ok(())
    }

    /// Check if block has reached consensus (2/3+ approval)
    pub async fn has_consensus(&self, block_hash: &str) -> bool {
        let mode = self.consensus_mode().await;

        match mode {
            ConsensusMode::Development | ConsensusMode::BootstrapNoQuorum => {
                // Always true in dev/bootstrap mode
                true
            }
            ConsensusMode::BFT => {
                let votes = self.pending_votes.read().await;
                let masternodes = self.masternodes.read().await;
                let total_nodes = masternodes.len();

                if total_nodes < 3 {
                    return true; // Shouldn't happen in BFT mode but handle gracefully
                }

                if let Some(vote_list) = votes.get(block_hash) {
                    let approvals = vote_list.iter().filter(|v| v.approve).count();
                    let required = (total_nodes * 2 + 2) / 3; // Ceiling of 2/3
                    approvals >= required
                } else {
                    false
                }
            }
        }
    }

    /// Finalize a block after consensus
    pub async fn finalize_block(&self, block: Block) -> Result<(), ConsensusError> {
        // Add block to state if available
        if let Some(state) = self.state.write().await.as_mut() {
            state.add_block(block)
                .map_err(|_| ConsensusError::InvalidBlock)?;
        }

        Ok(())
    }

    /// Get quorum status for a block
    pub async fn check_quorum(&self, block_hash: &str) -> (bool, usize, usize, usize) {
        let votes = self.pending_votes.read().await;
        let masternodes = self.masternodes.read().await;
        let total_nodes = masternodes.len();

        let (approvals, rejections) = if let Some(vote_list) = votes.get(block_hash) {
            let app = vote_list.iter().filter(|v| v.approve).count();
            let rej = vote_list.iter().filter(|v| !v.approve).count();
            (app, rej)
        } else {
            (0, 0)
        };

        let required = (total_nodes * 2 + 2) / 3;
        let has_quorum = approvals >= required;

        (has_quorum, approvals, rejections, total_nodes)
    }

    /// Get vote counts for a block
    pub async fn get_vote_count(&self, block_hash: &str) -> (usize, usize) {
        let votes = self.pending_votes.read().await;

        if let Some(vote_list) = votes.get(block_hash) {
            let approvals = vote_list.iter().filter(|v| v.approve).count();
            let rejections = vote_list.iter().filter(|v| !v.approve).count();
            (approvals, rejections)
        } else {
            (0, 0)
        }
    }

    /// Validate a transaction (placeholder)
    pub async fn validate_transaction(&self, _tx: &Transaction) -> bool {
        // TODO: Implement transaction validation logic
        // For now, accept all transactions
        true
    }

    /// Announce chain state to peers and check for mismatches
    pub async fn announce_chain_state(&self, height: u64, tip_hash: String, peers: Vec<String>) -> (bool, Vec<String>, Vec<String>) {
        // This would normally involve network communication
        // For now, just return empty lists
        let _ = (height, tip_hash, peers);
        (true, Vec::new(), Vec::new())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub block_hash: String,
    pub voter: String,
    pub approve: bool,
    pub timestamp: i64,
}

#[derive(Debug)]
pub enum ConsensusError {
    UnauthorizedProposer,
    UnauthorizedVoter,
    DuplicateVote,
    InvalidBlock,
    QuorumNotReached,
}

impl std::fmt::Display for ConsensusError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ConsensusError::UnauthorizedProposer => write!(f, "Unauthorized block proposer"),
            ConsensusError::UnauthorizedVoter => write!(f, "Unauthorized voter"),
            ConsensusError::DuplicateVote => write!(f, "Duplicate vote detected"),
            ConsensusError::InvalidBlock => write!(f, "Invalid block"),
            ConsensusError::QuorumNotReached => write!(f, "Quorum not reached"),
        }
    }
}

impl std::error::Error for ConsensusError {}

// Transaction consensus module
pub mod tx_consensus {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TransactionProposal {
        pub block_height: u64,
        pub proposer: String,
        pub tx_ids: Vec<String>,
        pub merkle_root: String,
        pub timestamp: i64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TxSetVote {
        pub block_height: u64,
        pub merkle_root: String,
        pub voter: String,
        pub approve: bool,
        pub timestamp: i64,
    }

    pub struct TxConsensusManager {
        proposals: Arc<RwLock<HashMap<u64, TransactionProposal>>>,
        votes: Arc<RwLock<HashMap<u64, HashMap<String, Vec<TxSetVote>>>>>,
        masternodes: Arc<RwLock<Vec<String>>>,
    }

    impl TxConsensusManager {
        pub fn new() -> Self {
            Self {
                proposals: Arc::new(RwLock::new(HashMap::new())),
                votes: Arc::new(RwLock::new(HashMap::new())),
                masternodes: Arc::new(RwLock::new(Vec::new())),
            }
        }

        pub async fn set_masternodes(&self, nodes: Vec<String>) {
            let mut masternodes = self.masternodes.write().await;
            *masternodes = nodes;
        }

        pub async fn propose_tx_set(&self, proposal: TransactionProposal) {
            let mut proposals = self.proposals.write().await;
            proposals.insert(proposal.block_height, proposal);
        }

        pub async fn vote_on_tx_set(&self, vote: TxSetVote) -> Result<(), String> {
            let masternodes = self.masternodes.read().await;
            if !masternodes.contains(&vote.voter) {
                return Err("Unauthorized voter".to_string());
            }
            drop(masternodes);

            let mut votes = self.votes.write().await;
            let height_votes = votes.entry(vote.block_height).or_insert_with(HashMap::new);
            let merkle_votes = height_votes.entry(vote.merkle_root.clone()).or_insert_with(Vec::new);

            if merkle_votes.iter().any(|v| v.voter == vote.voter) {
                return Err("Duplicate vote".to_string());
            }

            merkle_votes.push(vote);
            Ok(())
        }

        pub async fn has_tx_consensus(&self, block_height: u64, merkle_root: &str) -> (bool, usize, usize) {
            let masternodes = self.masternodes.read().await;
            let total_nodes = masternodes.len();
            drop(masternodes);

            if total_nodes < 3 {
                return (true, 0, total_nodes);
            }

            let votes = self.votes.read().await;
            if let Some(height_votes) = votes.get(&block_height) {
                if let Some(vote_list) = height_votes.get(merkle_root) {
                    let approvals = vote_list.iter().filter(|v| v.approve).count();
                    let required = (total_nodes * 2 + 2) / 3;
                    let has_consensus = approvals >= required;
                    return (has_consensus, approvals, total_nodes);
                }
            }

            (false, 0, total_nodes)
        }

        pub async fn get_agreed_tx_set(&self, block_height: u64) -> Option<TransactionProposal> {
            let proposals = self.proposals.read().await;
            let proposal = proposals.get(&block_height)?;

            let (has_consensus, _, _) = self.has_tx_consensus(block_height, &proposal.merkle_root).await;

            if has_consensus {
                Some(proposal.clone())
            } else {
                None
            }
        }

        pub async fn cleanup_old(&self, current_height: u64) {
            let mut proposals = self.proposals.write().await;
            let mut votes = self.votes.write().await;

            proposals.retain(|&h, _| h >= current_height.saturating_sub(10));
            votes.retain(|&h, _| h >= current_height.saturating_sub(10));
        }

        pub async fn get_proposal(&self, block_height: u64) -> Option<TransactionProposal> {
            let proposals = self.proposals.read().await;
            proposals.get(&block_height).cloned()
        }
        /// Get list of masternodes that voted on this block's transaction set
        pub async fn get_voters(&self, block_height: u64, merkle_root: &str) -> Vec<String> {
            let votes = self.votes.read().await;
            if let Some(height_votes) = votes.get(&block_height) {
                if let Some(vote_list) = height_votes.get(merkle_root) {
                    return vote_list.iter()
                        .filter(|v| v.approve)
                        .map(|v| v.voter.clone())
                        .collect();
                }
            }
            Vec::new()
        }

    }
}

// Block consensus module - for voting on catch-up blocks
pub mod block_consensus {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BlockProposal {
        pub block_height: u64,
        pub proposer: String,
        pub block_hash: String,
        pub merkle_root: String,
        pub previous_hash: String,
        pub timestamp: i64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BlockVote {
        pub block_height: u64,
        pub block_hash: String,
        pub voter: String,
        pub approve: bool,
        pub timestamp: i64,
    }

    pub struct BlockConsensusManager {
        proposals: Arc<RwLock<HashMap<u64, BlockProposal>>>,
        votes: Arc<RwLock<HashMap<u64, HashMap<String, Vec<BlockVote>>>>>,
        masternodes: Arc<RwLock<Vec<String>>>,
    }

    impl BlockConsensusManager {
        pub fn new() -> Self {
            Self {
                proposals: Arc::new(RwLock::new(HashMap::new())),
                votes: Arc::new(RwLock::new(HashMap::new())),
                masternodes: Arc::new(RwLock::new(Vec::new())),
            }
        }

        pub async fn set_masternodes(&self, nodes: Vec<String>) {
            let mut masternodes = self.masternodes.write().await;
            *masternodes = nodes;
        }

        pub async fn propose_block(&self, proposal: BlockProposal) {
            let mut proposals = self.proposals.write().await;
            proposals.insert(proposal.block_height, proposal);
        }

        pub async fn vote_on_block(&self, vote: BlockVote) -> Result<(), String> {
            let masternodes = self.masternodes.read().await;
            if !masternodes.contains(&vote.voter) {
                return Err("Unauthorized voter".to_string());
            }
            drop(masternodes);

            let mut votes = self.votes.write().await;
            let height_votes = votes.entry(vote.block_height).or_insert_with(HashMap::new);
            let block_votes = height_votes.entry(vote.block_hash.clone()).or_insert_with(Vec::new);

            if block_votes.iter().any(|v| v.voter == vote.voter) {
                return Err("Duplicate vote".to_string());
            }

            block_votes.push(vote);
            Ok(())
        }

        pub async fn has_block_consensus(&self, block_height: u64, block_hash: &str) -> (bool, usize, usize) {
            let masternodes = self.masternodes.read().await;
            let total_nodes = masternodes.len();
            drop(masternodes);

            if total_nodes < 3 {
                return (true, 0, total_nodes);
            }

            let votes = self.votes.read().await;
            if let Some(height_votes) = votes.get(&block_height) {
                if let Some(vote_list) = height_votes.get(block_hash) {
                    let approvals = vote_list.iter().filter(|v| v.approve).count();
                    let required = (total_nodes * 2 + 2) / 3;
                    let has_consensus = approvals >= required;
                    return (has_consensus, approvals, total_nodes);
                }
            }

            (false, 0, total_nodes)
        }

        pub async fn get_agreed_block(&self, block_height: u64) -> Option<BlockProposal> {
            let proposals = self.proposals.read().await;
            let proposal = proposals.get(&block_height)?;

            let (has_consensus, _, _) = self.has_block_consensus(block_height, &proposal.block_hash).await;

            if has_consensus {
                Some(proposal.clone())
            } else {
                None
            }
        }

        pub async fn cleanup_old(&self, current_height: u64) {
            let mut proposals = self.proposals.write().await;
            let mut votes = self.votes.write().await;

            proposals.retain(|&h, _| h >= current_height.saturating_sub(10));
            votes.retain(|&h, _| h >= current_height.saturating_sub(10));
        }

        pub async fn get_proposal(&self, block_height: u64) -> Option<BlockProposal> {
            let proposals = self.proposals.read().await;
            proposals.get(&block_height).cloned()
        }

        /// Get list of masternodes that voted on this block
        pub async fn get_voters(&self, block_height: u64, block_hash: &str) -> Vec<String> {
            let votes = self.votes.read().await;
            if let Some(height_votes) = votes.get(&block_height) {
                if let Some(vote_list) = height_votes.get(block_hash) {
                    return vote_list.iter()
                        .filter(|v| v.approve)
                        .map(|v| v.voter.clone())
                        .collect();
                }
            }
            Vec::new()
        }
    }
}
