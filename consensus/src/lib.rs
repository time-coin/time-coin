//! BFT Consensus implementation for TIME Coin
//! 
//! Implements leader-based block production with Byzantine Fault Tolerance
//! Requires minimum 3 masternodes for full BFT consensus

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use time_core::{Block, Transaction, BlockchainState, StateError};

pub mod quorum;
pub mod voting;
pub mod vrf;

pub use voting::{Vote, VoteType};

/// Minimum masternodes required for BFT consensus
pub const MIN_BFT_QUORUM: usize = 3;

/// Consensus engine for block production and validation
#[derive(Clone)]
pub struct ConsensusEngine {
    /// Development mode flag
    dev_mode: bool,
    
    /// Registered masternodes (addresses)
    masternodes: Arc<RwLock<Vec<String>>>,
    
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

    /// Sync masternode list with current peers (replaces existing list)
    pub async fn sync_masternodes(&self, peer_ips: Vec<String>) {
        let mut masternodes = self.masternodes.write().await;
        *masternodes = peer_ips;
    }

    /// Get the masternode that should produce the next block (round-robin by IP)
    pub async fn get_block_producer(&self, block_height: u64) -> Option<String> {
        let masternodes = self.masternodes.read().await;
        if masternodes.is_empty() {
            return None;
        }
        
        // Sort masternodes by IP for deterministic order
        let mut sorted_nodes: Vec<String> = masternodes.iter().cloned().collect();
        sorted_nodes.sort();
        
        // Round-robin selection
        let index = (block_height as usize) % sorted_nodes.len();
        Some(sorted_nodes[index].clone())
    }
    
    /// Check if this node is the block producer for given height
    pub async fn is_my_turn(&self, block_height: u64, my_ip: &str) -> bool {
        match self.get_block_producer(block_height).await {
            Some(producer) => producer == my_ip,
            None => false,
        }
    }

    /// Sync masternode list with current peers (replaces existing list)
    /// Check if network has BFT quorum (minimum 3 masternodes)
    pub async fn has_bft_quorum(&self) -> bool {
        self.masternode_count().await >= MIN_BFT_QUORUM
    }

    /// Get consensus mode based on masternode count
    pub async fn consensus_mode(&self) -> ConsensusMode {
        let count = self.masternode_count().await;
        
        if self.dev_mode {
            ConsensusMode::Development
        } else if count < MIN_BFT_QUORUM {
            ConsensusMode::BootstrapNoQuorum
        } else {
            ConsensusMode::BFT
        }
    }

    /// Determine if this node is the leader for given block height
    pub async fn is_leader(&self, block_height: u64, node_address: &str) -> bool {
        let masternodes = self.masternodes.read().await;
        
        if masternodes.is_empty() {
            return false;
        }

        // Deterministic leader selection: block_height % num_masternodes
        let leader_index = (block_height % masternodes.len() as u64) as usize;
        masternodes.get(leader_index).map(|addr| addr == node_address).unwrap_or(false)
    }

    /// Get the leader address for a given block height
    pub async fn get_leader(&self, block_height: u64) -> Option<String> {
        let masternodes = self.masternodes.read().await;
        
        if masternodes.is_empty() {
            return None;
        }

        let leader_index = (block_height % masternodes.len() as u64) as usize;
        masternodes.get(leader_index).cloned()
    }

    /// Propose a new block (called by leader)
    pub async fn propose_block(
        &self,
        transactions: Vec<Transaction>,
        validator_address: String,
    ) -> Result<Block, ConsensusError> {
        let state_lock = self.state.read().await;
        let state = state_lock.as_ref()
            .ok_or(ConsensusError::NoState)?;

        let next_height = state.chain_tip_height() + 1;
        let prev_hash = state.chain_tip_hash().to_string();

        // Verify this node is the leader
        if !self.is_leader(next_height, &validator_address).await {
            return Err(ConsensusError::NotLeader);
        }

        // Create coinbase outputs (treasury + masternode reward)
        let treasury_reward = 5 * 100_000_000; // 5 TIME
        let masternode_reward = time_core::calculate_total_masternode_reward(state.masternode_counts());
        
        let mut coinbase_outputs = vec![
            time_core::TxOutput::new(treasury_reward, "treasury".to_string()),
            time_core::TxOutput::new(masternode_reward, validator_address.clone()),
        ];

        // Add transaction outputs
        for tx in &transactions {
            coinbase_outputs.extend(tx.outputs.clone());
        }

        // Create block
        let block = Block::new(
            next_height,
            prev_hash,
            validator_address,
            coinbase_outputs,
        );

        Ok(block)
    }

    /// Validate a proposed block
    pub async fn validate_block(&self, block: &Block) -> Result<bool, ConsensusError> {
        let mode = self.consensus_mode().await;
        
        if matches!(mode, ConsensusMode::Development) {
            return Ok(true);
        }

        let state_lock = self.state.read().await;
        let state = state_lock.as_ref()
            .ok_or(ConsensusError::NoState)?;

        // Verify block structure
        block.validate_structure()
            .map_err(|e| ConsensusError::InvalidBlock(e.to_string()))?;

        // Verify height
        if block.header.block_number != state.chain_tip_height() + 1 {
            return Err(ConsensusError::InvalidHeight);
        }

        // Verify previous hash
        if block.header.previous_hash != state.chain_tip_hash() {
            return Err(ConsensusError::InvalidPreviousHash);
        }

        // Verify validator is the correct leader
        let expected_leader = self.get_leader(block.header.block_number).await
            .ok_or(ConsensusError::NoLeader)?;
        
        if block.header.validator_address != expected_leader {
            return Err(ConsensusError::WrongLeader);
        }

        Ok(true)
    }

    /// Cast a vote for a block
    pub async fn vote_on_block(
        &self,
        block_hash: &str,
        voter_address: String,
        approve: bool,
    ) -> Result<Vote, ConsensusError> {
        // Verify voter is a registered masternode
        let masternodes = self.masternodes.read().await;
        if !masternodes.contains(&voter_address) {
            return Err(ConsensusError::UnauthorizedVoter);
        }
        drop(masternodes);

        let vote = Vote {
            block_hash: block_hash.to_string(),
            voter: voter_address,
            vote_type: if approve { VoteType::Approve } else { VoteType::Reject },
            timestamp: chrono::Utc::now().timestamp(),
        };

        // Record vote
        let mut votes = self.pending_votes.write().await;
        votes.entry(block_hash.to_string())
            .or_insert_with(Vec::new)
            .push(vote.clone());

        Ok(vote)
    }

    /// Check if block has reached consensus
    pub async fn has_consensus(&self, block_hash: &str) -> bool {
        let mode = self.consensus_mode().await;
        let votes = self.pending_votes.read().await;
        let block_votes = votes.get(block_hash);

        if let Some(votes_list) = block_votes {
            let approve_count = votes_list.iter()
                .filter(|v| matches!(v.vote_type, VoteType::Approve))
                .count();

            let total_masternodes = self.masternodes.read().await.len();
            
            match mode {
                ConsensusMode::Development => {
                    // Dev mode: auto-approve
                    true
                }
                ConsensusMode::BootstrapNoQuorum => {
                    // Less than 3 nodes: Don't require consensus (bootstrap mode)
                    // Just produce blocks without voting
                    true
                }
                ConsensusMode::BFT => {
                    // 3+ nodes: Require 2/3+ approval (BFT)
                    approve_count >= (total_masternodes * 2 / 3) + 1
                }
            }
        } else {
            // No votes yet
            matches!(mode, ConsensusMode::Development | ConsensusMode::BootstrapNoQuorum)
        }
    }

    /// Finalize a block after consensus
    pub async fn finalize_block(&self, block: Block) -> Result<(), ConsensusError> {
        let mut state_lock = self.state.write().await;
        let state = state_lock.as_mut()
            .ok_or(ConsensusError::NoState)?;

        // Add block to state
        state.add_block(block.clone())
            .map_err(|e| ConsensusError::StateError(e))?;

        // Clear votes for this block
        let mut votes = self.pending_votes.write().await;
        votes.remove(&block.hash);

        Ok(())
    }

    /// Get vote count for a block
    /// Check if we have 2/3+ approval (simple version for round-robin)
    pub async fn check_quorum(&self, block_hash: &str) -> (bool, usize, usize, usize) {
        let (approvals, rejections) = self.get_vote_count(block_hash).await;
        let total_nodes = self.masternode_count().await;
        
        if total_nodes == 0 {
            return (false, 0, 0, 0);
        }
        
        let required = (total_nodes * 2 + 2) / 3;
        let has_quorum = approvals >= required;
        
        (has_quorum, approvals, rejections, total_nodes)
    }

    pub async fn get_vote_count(&self, block_hash: &str) -> (usize, usize) {
        let votes = self.pending_votes.read().await;
        
        if let Some(votes_list) = votes.get(block_hash) {
            let approve = votes_list.iter()
                .filter(|v| matches!(v.vote_type, VoteType::Approve))
                .count();
            let reject = votes_list.iter()
                .filter(|v| matches!(v.vote_type, VoteType::Reject))
                .count();
            (approve, reject)
        } else {
            (0, 0)
        }
    }

    pub fn is_dev_mode(&self) -> bool {
        self.dev_mode
    }

    /// Validate a transaction (legacy method)
    pub async fn validate_transaction(&self, _tx: &Transaction) -> bool {
        if self.dev_mode {
            true
        } else {
            // TODO: Implement transaction-level consensus
            false
        }
    }
}

/// Consensus mode based on network state
#[derive(Debug, Clone, PartialEq)]
pub enum ConsensusMode {
    /// Development mode - auto-approve everything
    Development,
    
    /// Bootstrap mode - less than 3 masternodes, no BFT consensus
    /// Blocks produced without voting (temporary until quorum reached)
    BootstrapNoQuorum,
    
    /// Full BFT mode - 3+ masternodes, requires 2/3+ approval
    BFT,
}

#[derive(Debug, Clone)]
pub enum ConsensusError {
    NoState,
    NotLeader,
    NoLeader,
    InvalidBlock(String),
    InvalidHeight,
    InvalidPreviousHash,
    WrongLeader,
    UnauthorizedVoter,
    InsufficientVotes,
    NoQuorum,
    StateError(StateError),
}

impl std::fmt::Display for ConsensusError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ConsensusError::NoState => write!(f, "No blockchain state available"),
            ConsensusError::NotLeader => write!(f, "Node is not the leader for this block"),
            ConsensusError::NoLeader => write!(f, "No leader available"),
            ConsensusError::InvalidBlock(e) => write!(f, "Invalid block: {}", e),
            ConsensusError::InvalidHeight => write!(f, "Invalid block height"),
            ConsensusError::InvalidPreviousHash => write!(f, "Invalid previous hash"),
            ConsensusError::WrongLeader => write!(f, "Block from wrong leader"),
            ConsensusError::UnauthorizedVoter => write!(f, "Voter is not a masternode"),
            ConsensusError::InsufficientVotes => write!(f, "Insufficient votes for consensus"),
            ConsensusError::NoQuorum => write!(f, "Network does not have minimum 3 masternodes for BFT"),
            ConsensusError::StateError(e) => write!(f, "State error: {}", e),
        }
    }
}

impl std::error::Error for ConsensusError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_consensus_modes() {
        let consensus = ConsensusEngine::new(false);
        
        // 0 nodes: Bootstrap
        assert_eq!(consensus.consensus_mode().await, ConsensusMode::BootstrapNoQuorum);
        
        // 1 node: Bootstrap
        consensus.add_masternode("node1".to_string()).await;
        assert_eq!(consensus.consensus_mode().await, ConsensusMode::BootstrapNoQuorum);
        
        // 2 nodes: Still bootstrap
        consensus.add_masternode("node2".to_string()).await;
        assert_eq!(consensus.consensus_mode().await, ConsensusMode::BootstrapNoQuorum);
        
        // 3 nodes: BFT mode!
        consensus.add_masternode("node3".to_string()).await;
        assert_eq!(consensus.consensus_mode().await, ConsensusMode::BFT);
        assert!(consensus.has_bft_quorum().await);
    }

    #[tokio::test]
    async fn test_leader_selection() {
        let consensus = ConsensusEngine::new(false);
        
        consensus.add_masternode("node1".to_string()).await;
        consensus.add_masternode("node2".to_string()).await;

        // Block 0: node1
        assert!(consensus.is_leader(0, "node1").await);
        assert!(!consensus.is_leader(0, "node2").await);

        // Block 1: node2
        assert!(!consensus.is_leader(1, "node1").await);
        assert!(consensus.is_leader(1, "node2").await);

        // Block 2: node1 (round-robin)
        assert!(consensus.is_leader(2, "node1").await);
    }

    #[tokio::test]
    async fn test_bootstrap_consensus() {
        let consensus = ConsensusEngine::new(false);
        
        consensus.add_masternode("node1".to_string()).await;
        consensus.add_masternode("node2".to_string()).await;

        // With only 2 nodes, consensus is automatic (bootstrap mode)
        let block_hash = "test_hash";
        assert!(consensus.has_consensus(block_hash).await);
    }

    #[tokio::test]
    async fn test_bft_voting() {
        let consensus = ConsensusEngine::new(false);
        
        consensus.add_masternode("node1".to_string()).await;
        consensus.add_masternode("node2".to_string()).await;
        consensus.add_masternode("node3".to_string()).await;

        let block_hash = "test_hash";

        // Need 2/3 (at least 2 votes)
        consensus.vote_on_block(block_hash, "node1".to_string(), true).await.unwrap();
        assert!(!consensus.has_consensus(block_hash).await); // Only 1 vote

        consensus.vote_on_block(block_hash, "node2".to_string(), true).await.unwrap();
        assert!(consensus.has_consensus(block_hash).await); // 2/3 reached!
    }
}

