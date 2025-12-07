//! TIME Coin Consensus Engine
//!
//! Implements leader-based block production with Byzantine Fault Tolerance
//! Requires minimum 3 masternodes for full BFT consensus
//!
//! ## VRF-Based Leader Selection
//!
//! The consensus engine uses a Verifiable Random Function (VRF) for secure and
//! unpredictable leader selection. Key features:
//!
//! - **Unpredictable**: Uses SHA256-based VRF with block height as seed
//! - **Verifiable**: Cryptographic proof allows anyone to verify selection
//! - **Deterministic**: Same block height always produces same leader across all nodes
//! - **Fair**: Weighted selection based on node characteristics
//! - **Consensus-Safe**: Uses only block height to ensure nodes at different
//!   sync states agree on the leader
//!
//! ## Security Properties
//!
//! - Attackers cannot predict future validator selections beyond brute-force
//! - Leader selection is deterministic for a given block height
//! - All nodes agree on the leader regardless of sync state
//! - Each selection includes verifiable cryptographic proof

// Core shared abstractions (new)
pub mod core;

// Phase 3 optimization - parallel validation
pub mod parallel_validation;

// Public modules
pub mod block_sync;
pub mod fallback;
pub mod foolproof_block;
pub mod height_sync;
pub mod instant_finality;
pub mod leader_election;
pub mod monitoring;
pub mod network_health;
pub mod phased_protocol;
pub mod proposals;
pub mod quorum;
pub mod tx_validation;

// New simplified consensus
pub mod midnight_consensus;
pub mod simplified;
pub mod transaction_approval;

use crate::core::vrf::{DefaultVRFSelector, VRFSelector};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use time_core::block::Block;
use time_core::state::BlockchainState;
use time_core::transaction::Transaction;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusMode {
    Development,       // Single node, no consensus needed
    BootstrapNoQuorum, // < 3 nodes, block production without voting
    BFT,               // >= 3 nodes, full BFT consensus with 2/3+ quorum
}

pub struct ConsensusEngine {
    /// Development mode flag
    dev_mode: bool,

    /// Network type ("mainnet" or "testnet")
    network: String,

    /// Registered masternodes (addresses)
    masternodes: Arc<RwLock<Vec<String>>>,

    /// Map node_id -> wallet_address
    wallet_addresses: Arc<RwLock<HashMap<String, String>>>,

    /// Current blockchain state
    state: Arc<RwLock<Option<BlockchainState>>>,

    /// Pending votes for current block - lock-free concurrent access
    pending_votes: Arc<DashMap<String, Vec<Vote>>>, // block_hash -> votes

    /// Pending votes for transactions (instant finality) - lock-free concurrent access
    transaction_votes: Arc<DashMap<String, Vec<Vote>>>, // txid -> votes

    /// Cached vote counts for O(1) consensus checks
    vote_counts: Arc<DashMap<String, VoteCount>>, // block_hash -> counts

    /// Cached transaction vote counts for O(1) consensus checks
    transaction_vote_counts: Arc<DashMap<String, VoteCount>>, // txid -> counts

    /// Proposal manager for treasury grants
    proposal_manager: Option<Arc<crate::proposals::ProposalManager>>,

    /// VRF selector for leader election
    vrf_selector: DefaultVRFSelector,
}

impl ConsensusEngine {
    pub fn new(dev_mode: bool) -> Self {
        Self::new_with_network(dev_mode, "mainnet".to_string())
    }

    pub fn new_with_network(dev_mode: bool, network: String) -> Self {
        let engine = Self {
            dev_mode,
            network: network.clone(),
            masternodes: Arc::new(RwLock::new(Vec::new())),
            wallet_addresses: Arc::new(RwLock::new(HashMap::new())),
            state: Arc::new(RwLock::new(None)),
            pending_votes: Arc::new(DashMap::new()),
            transaction_votes: Arc::new(DashMap::new()),
            vote_counts: Arc::new(DashMap::new()),
            transaction_vote_counts: Arc::new(DashMap::new()),
            proposal_manager: None,
            vrf_selector: DefaultVRFSelector,
        };

        // Log VRF configuration for debugging synchronization issues
        println!("🔍 VRF Configuration:");
        println!("   Network: {}", network);
        println!("   Dev mode: {}", dev_mode);
        println!("   Selector: DefaultVRFSelector (SHA256-based, height-only seed)");

        engine
    }

    /// Set the proposal manager
    pub fn set_proposal_manager(&mut self, manager: Arc<crate::proposals::ProposalManager>) {
        self.proposal_manager = Some(manager);
    }

    /// Get the proposal manager
    pub fn proposal_manager(&self) -> Option<Arc<crate::proposals::ProposalManager>> {
        self.proposal_manager.clone()
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

    /// Check if a node ID is a registered masternode
    pub async fn is_masternode(&self, node_id: &str) -> bool {
        let masternodes = self.masternodes.read().await;
        masternodes.contains(&node_id.to_string())
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

        masternodes
            .iter()
            .filter_map(|node_id| {
                wallets
                    .get(node_id)
                    .map(|wallet| (node_id.clone(), wallet.clone()))
            })
            .collect()
    }

    /// Sync masternode list with current peers (replaces existing list)
    pub async fn sync_masternodes(&self, peer_ips: Vec<String>) {
        let mut masternodes = self.masternodes.write().await;

        // Sort for deterministic ordering
        let mut sorted = peer_ips;
        sorted.sort();

        // Only accept if count >= 3 (BFT requirement), unless in dev mode
        if sorted.len() >= 3 || self.dev_mode {
            let old_count = masternodes.len();
            *masternodes = sorted;
            println!(
                "📋 Masternode list synced: {} → {} nodes",
                old_count,
                masternodes.len()
            );
        } else {
            println!(
                "⚠️  Rejecting masternode list: only {} nodes (need 3+ for BFT)",
                sorted.len()
            );
        }
    }

    /// Get block producer for a given block height using VRF-based selection
    pub async fn get_block_producer(&self, block_height: u64) -> Option<String> {
        let masternodes = self.masternodes.read().await;

        if masternodes.is_empty() {
            return None;
        }

        // Get previous block hash for VRF seed
        let previous_hash = if let Some(state) = self.state.read().await.as_ref() {
            state.chain_tip_hash().to_string()
        } else {
            // Use default for genesis or no state
            "genesis".to_string()
        };

        // VRF-based selection with weighted random
        let leader = self.select_leader_vrf(&masternodes, block_height, &previous_hash);
        Some(leader)
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

    /// Check if node is the leader for this block using VRF-based selection
    pub async fn is_leader(&self, block_height: u64, node_address: &str) -> bool {
        match self.get_leader(block_height).await {
            Some(leader) => leader == node_address,
            None => false,
        }
    }

    /// Get the leader for a block using VRF-based selection
    pub async fn get_leader(&self, block_height: u64) -> Option<String> {
        let masternodes = self.masternodes.read().await;

        if masternodes.is_empty() {
            return None;
        }

        // CRITICAL FIX: Use canonical seed independent of node state
        // The VRF selector uses ONLY block_height (not previous_hash) to ensure
        // all nodes agree on the leader regardless of their chain sync state
        let previous_hash = if let Some(state) = self.state.read().await.as_ref() {
            state.chain_tip_hash().to_string()
        } else {
            "genesis".to_string()
        };

        // Log the seed components for debugging divergence
        println!("🔐 Leader election for block {}:", block_height);
        println!(
            "   Prev hash: {}... (note: NOT used in VRF seed)",
            &previous_hash[..previous_hash.len().min(16)]
        );
        println!("   Masternode count: {}", masternodes.len());

        // VRF-based selection (uses ONLY height internally)
        let leader = self.select_leader_vrf(&masternodes, block_height, &previous_hash);

        println!("👑 Selected leader: {}", leader);

        Some(leader)
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
    ///
    /// # Maturity Check Required
    /// **IMPORTANT**: Callers should validate that the masternode has reached vote maturity
    /// before calling this function. Use the masternode module's `MasternodeStatus::can_vote_at_height()`
    /// to check if the masternode has waited the required number of blocks since registration.
    /// This prevents instant takeover by newly coordinated malicious nodes.
    ///
    /// Example:
    /// ```ignore
    /// // Before voting, check maturity:
    /// if masternode_status.can_vote_at_height(current_block_height, &tier) {
    ///     consensus.vote_on_block(block_hash, voter, approve).await?;
    /// }
    /// ```
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

        // Use DashMap for lock-free concurrent access
        self.pending_votes
            .entry(block_hash.clone())
            .or_default()
            .push(Vote {
                block_hash: block_hash.clone(),
                voter,
                approve,
                timestamp: chrono::Utc::now().timestamp(),
            });

        // Update vote count cache for O(1) consensus checks
        let mut count = self.vote_counts.entry(block_hash).or_default();
        if approve {
            count.approvals += 1;
        } else {
            count.rejections += 1;
        }

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
                let masternodes = self.masternodes.read().await;
                let total_nodes = masternodes.len();
                drop(masternodes);

                if total_nodes < 3 {
                    return true; // Shouldn't happen in BFT mode but handle gracefully
                }

                // O(1) lookup using cached vote counts
                if let Some(count) = self.vote_counts.get(block_hash) {
                    let required = crate::quorum::required_for_bft(total_nodes);
                    count.approvals >= required
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
            state
                .add_block(block)
                .map_err(|_| ConsensusError::InvalidBlock)?;
        }

        Ok(())
    }

    /// Get quorum status for a block
    pub async fn check_quorum(&self, block_hash: &str) -> (bool, usize, usize, usize) {
        let masternodes = self.masternodes.read().await;
        let total_nodes = masternodes.len();
        drop(masternodes);

        // O(1) lookup using cached vote counts
        let (approvals, rejections) = if let Some(count) = self.vote_counts.get(block_hash) {
            (count.approvals, count.rejections)
        } else {
            (0, 0)
        };

        let required = crate::quorum::required_for_bft(total_nodes);
        let has_quorum = approvals >= required;

        (has_quorum, approvals, rejections, total_nodes)
    }

    /// Get vote counts for a block
    pub async fn get_vote_count(&self, block_hash: &str) -> (usize, usize) {
        // O(1) lookup using cached vote counts
        if let Some(count) = self.vote_counts.get(block_hash) {
            (count.approvals, count.rejections)
        } else {
            (0, 0)
        }
    }

    /// Validate a transaction and initiate BFT voting for instant finality
    pub async fn validate_and_vote_transaction(
        &self,
        tx: &Transaction,
        voter: String,
    ) -> Result<(), ConsensusError> {
        // Verify voter is a registered masternode
        let masternodes = self.masternodes.read().await;
        if !masternodes.contains(&voter) {
            return Err(ConsensusError::UnauthorizedVoter);
        }
        drop(masternodes);

        // Basic transaction validation
        let is_valid = self.validate_transaction(tx).await;

        // Vote on the transaction
        self.vote_on_transaction(&tx.txid, voter, is_valid).await
    }

    /// Validate a transaction (basic checks)
    pub async fn validate_transaction(&self, tx: &Transaction) -> bool {
        // Basic validation checks
        if tx.outputs.is_empty() {
            return false;
        }

        // Check if amounts are valid
        for output in &tx.outputs {
            if output.amount == 0 {
                return false;
            }
        }

        // Coinbase transactions (no inputs) are ONLY valid on testnet for minting
        // Mainnet should reject coinbase transactions unless they're in blocks
        if tx.inputs.is_empty() {
            if self.network == "testnet" {
                println!("   ✓ Testnet coinbase transaction accepted for instant finality");
                return true;
            } else {
                println!("   ✗ Mainnet coinbase transaction rejected (only allowed in blocks)");
                return false;
            }
        }

        // For regular transactions, verify structure
        for input in &tx.inputs {
            if input.previous_output.txid.is_empty() {
                return false;
            }
        }

        true
    }

    /// Vote on a transaction for instant finality
    pub async fn vote_on_transaction(
        &self,
        txid: &str,
        voter: String,
        approve: bool,
    ) -> Result<(), ConsensusError> {
        // Verify voter is a registered masternode
        let masternodes = self.masternodes.read().await;
        if !masternodes.contains(&voter) {
            return Err(ConsensusError::UnauthorizedVoter);
        }
        drop(masternodes);

        // Use DashMap for lock-free concurrent access
        self.transaction_votes
            .entry(txid.to_string())
            .or_default()
            .push(Vote {
                block_hash: txid.to_string(), // Reuse block_hash field for txid
                voter,
                approve,
                timestamp: chrono::Utc::now().timestamp(),
            });

        // Update transaction vote count cache for O(1) consensus checks
        let mut count = self
            .transaction_vote_counts
            .entry(txid.to_string())
            .or_default();
        if approve {
            count.approvals += 1;
        } else {
            count.rejections += 1;
        }

        Ok(())
    }

    /// Check if transaction has reached consensus (2/3+ approval) for instant finality
    pub async fn has_transaction_consensus(&self, txid: &str) -> bool {
        let mode = self.consensus_mode().await;

        match mode {
            ConsensusMode::Development | ConsensusMode::BootstrapNoQuorum => {
                // Always true in dev/bootstrap mode for instant finality
                true
            }
            ConsensusMode::BFT => {
                let masternodes = self.masternodes.read().await;
                let total_nodes = masternodes.len();
                drop(masternodes);

                if total_nodes < 3 {
                    return true; // Accept in bootstrap mode
                }

                // O(1) lookup using cached transaction vote counts
                if let Some(count) = self.transaction_vote_counts.get(txid) {
                    let required = crate::quorum::required_for_bft(total_nodes);
                    count.approvals >= required
                } else {
                    false
                }
            }
        }
    }

    /// Get transaction vote counts
    pub async fn get_transaction_vote_count(&self, txid: &str) -> (usize, usize) {
        if let Some(vote_list) = self.transaction_votes.get(txid) {
            let approvals = vote_list.iter().filter(|v| v.approve).count();
            let rejections = vote_list.iter().filter(|v| !v.approve).count();
            (approvals, rejections)
        } else {
            (0, 0)
        }
    }

    /// Clear transaction votes after finalization
    pub async fn clear_transaction_votes(&self, txid: &str) {
        self.transaction_votes.remove(txid);
    }

    /// VRF-based leader selection with weighted random
    /// Uses SHA256-based VRF to provide unpredictable but verifiable selection
    fn select_leader_vrf(
        &self,
        masternodes: &[String],
        block_height: u64,
        previous_hash: &str,
    ) -> String {
        if masternodes.is_empty() {
            return String::new();
        }

        // Use the VRF selector trait
        self.vrf_selector
            .select_leader(masternodes, block_height, previous_hash)
            .unwrap_or_default()
    }

    /// Generate VRF proof for selected leader (for verification)
    pub fn generate_vrf_proof(
        &self,
        block_height: u64,
        previous_hash: &str,
        leader: &str,
    ) -> Vec<u8> {
        self.vrf_selector
            .generate_proof(block_height, previous_hash, leader)
    }

    /// Verify VRF proof for a leader selection
    pub fn verify_vrf_proof(
        &self,
        block_height: u64,
        previous_hash: &str,
        leader: &str,
        proof: &[u8],
    ) -> bool {
        self.vrf_selector
            .verify_proof(block_height, previous_hash, leader, proof)
    }

    /// Announce chain state to peers and check for mismatches
    pub async fn announce_chain_state(
        &self,
        height: u64,
        tip_hash: String,
        peers: Vec<String>,
    ) -> (bool, Vec<String>, Vec<String>) {
        // This would normally involve network communication
        // For now, just return empty lists
        let _ = (height, tip_hash, peers);
        (true, Vec::new(), Vec::new())
    }

    /// Get current block height from state
    pub async fn get_current_height(&self) -> u64 {
        if let Some(state) = self.state.read().await.as_ref() {
            state.chain_tip_height()
        } else {
            0
        }
    }

    /// Get block hash at specific height from state
    pub async fn get_block_hash_at_height(&self, height: u64) -> Option<String> {
        if let Some(state) = self.state.read().await.as_ref() {
            state
                .get_block_by_height(height)
                .map(|block| block.hash.clone())
        } else {
            None
        }
    }

    /// Verify block at height matches consensus across peers
    pub async fn verify_block_consensus(
        &self,
        height: u64,
        verification_threshold_percent: u64,
    ) -> Result<bool, String> {
        let our_hash = self
            .get_block_hash_at_height(height)
            .await
            .ok_or_else(|| format!("No block at height {}", height))?;

        let peers = self.get_masternodes().await;

        let verifier =
            crate::height_sync::BlockVerificationManager::new(verification_threshold_percent);

        verifier
            .verify_block_consensus(height, &our_hash, &peers)
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub block_hash: String,
    pub voter: String,
    pub approve: bool,
    pub timestamp: i64,
}

/// Cached vote counts for O(1) consensus checks
#[derive(Debug, Clone, Default)]
struct VoteCount {
    approvals: usize,
    rejections: usize,
}

#[derive(Debug)]
pub enum ConsensusError {
    UnauthorizedProposer,
    UnauthorizedVoter,
    DuplicateVote,
    InvalidBlock,
    QuorumNotReached,
    MasternodeNotMature,
}

impl std::fmt::Display for ConsensusError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ConsensusError::UnauthorizedProposer => write!(f, "Unauthorized block proposer"),
            ConsensusError::UnauthorizedVoter => write!(f, "Unauthorized voter"),
            ConsensusError::DuplicateVote => write!(f, "Duplicate vote detected"),
            ConsensusError::InvalidBlock => write!(f, "Invalid block"),
            ConsensusError::QuorumNotReached => write!(f, "Quorum not reached"),
            ConsensusError::MasternodeNotMature => {
                write!(f, "Masternode not mature enough to vote")
            }
        }
    }
}

impl std::error::Error for ConsensusError {}

// Transaction consensus module

/// Masternode performance status
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum NodeStatus {
    Active,      // Full consensus participant
    Degraded,    // Slow but participating
    Quarantined, // Temporarily excluded
    Downgraded,  // Demoted to seed node
    Offline,     // Not responding
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NodeStatus::Active => write!(f, "✅ ACTIVE"),
            NodeStatus::Degraded => write!(f, "⚠️  DEGRADED"),
            NodeStatus::Quarantined => write!(f, "🔒 QUARANTINED"),
            NodeStatus::Downgraded => write!(f, "⬇️  DOWNGRADED"),
            NodeStatus::Offline => write!(f, "❌ OFFLINE"),
        }
    }
}

/// Health metrics for a masternode
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MasternodeHealth {
    pub address: String,
    pub status: NodeStatus,
    pub avg_response_time_ms: u64,
    pub vote_participation_rate: f32,
    pub missed_votes: u32,
    pub consecutive_misses: u32,
    pub last_response: i64,
    pub status_changed_at: i64,
    pub total_votes_cast: u32,
    pub total_votes_expected: u32,
}

impl MasternodeHealth {
    pub fn new(address: String) -> Self {
        use chrono::Utc;
        Self {
            address,
            status: NodeStatus::Active,
            avg_response_time_ms: 0,
            vote_participation_rate: 1.0,
            missed_votes: 0,
            consecutive_misses: 0,
            last_response: Utc::now().timestamp(),
            status_changed_at: Utc::now().timestamp(),
            total_votes_cast: 0,
            total_votes_expected: 0,
        }
    }
}

// Performance thresholds
pub const RESPONSE_TIMEOUT_MS: u64 = 3000;
pub const DEGRADED_THRESHOLD_MS: u64 = 2000;
pub const MAX_CONSECUTIVE_MISSES: u32 = 3;
pub const MIN_PARTICIPATION_RATE: f32 = 0.70;
pub const QUARANTINE_DURATION_SECS: i64 = 3600; // 1 hour

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

    impl crate::core::collector::Vote for TxSetVote {
        fn voter(&self) -> &str {
            &self.voter
        }

        fn approve(&self) -> bool {
            self.approve
        }

        fn timestamp(&self) -> i64 {
            self.timestamp
        }
    }

    pub struct TxConsensusManager {
        proposals: Arc<RwLock<HashMap<u64, TransactionProposal>>>,
        vote_collector: crate::core::collector::VoteCollector<TxSetVote>,
        masternodes: Arc<RwLock<Vec<String>>>,
    }

    impl Default for TxConsensusManager {
        fn default() -> Self {
            Self::new()
        }
    }

    impl TxConsensusManager {
        pub fn new() -> Self {
            Self {
                proposals: Arc::new(RwLock::new(HashMap::new())),
                vote_collector: crate::core::collector::VoteCollector::new_bft(0), // Will be updated via set_masternodes
                masternodes: Arc::new(RwLock::new(Vec::new())),
            }
        }

        pub async fn set_masternodes(&self, nodes: Vec<String>) {
            let mut masternodes = self.masternodes.write().await;
            *masternodes = nodes.clone();
            drop(masternodes);

            // Update vote collector with new count
            self.vote_collector.set_total_voters(nodes.len()).await;
        }

        pub async fn propose_tx_set(&self, proposal: TransactionProposal) {
            let mut proposals = self.proposals.write().await;
            proposals.insert(proposal.block_height, proposal);
        }

        /// Vote on a transaction set
        ///
        /// # Maturity Check Required
        /// **IMPORTANT**: Callers should validate that the masternode has reached vote maturity
        /// before calling this function. Use the masternode module's `MasternodeStatus::can_vote_at_height()`
        /// to check if the masternode has waited the required number of blocks since registration.
        /// This prevents instant takeover by newly coordinated malicious nodes.
        pub async fn vote_on_tx_set(&self, vote: TxSetVote) -> Result<(), String> {
            let masternodes = self.masternodes.read().await;
            if !masternodes.contains(&vote.voter) {
                return Err("Unauthorized voter".to_string());
            }
            drop(masternodes);

            // Record vote using the vote collector
            self.vote_collector
                .record_vote(vote.block_height, vote.merkle_root.clone(), vote);

            Ok(())
        }

        pub async fn has_tx_consensus(
            &self,
            block_height: u64,
            merkle_root: &str,
        ) -> (bool, usize, usize) {
            self.vote_collector
                .check_consensus(block_height, merkle_root)
                .await
        }

        pub async fn get_agreed_tx_set(&self, block_height: u64) -> Option<TransactionProposal> {
            let proposals = self.proposals.read().await;
            let proposal = proposals.get(&block_height)?;

            let (has_consensus, _, _) = self
                .has_tx_consensus(block_height, &proposal.merkle_root)
                .await;

            if has_consensus {
                Some(proposal.clone())
            } else {
                None
            }
        }

        pub async fn cleanup_old(&self, current_height: u64) {
            let mut proposals = self.proposals.write().await;
            proposals.retain(|&h, _| h >= current_height.saturating_sub(10));
            self.vote_collector.cleanup_old(current_height, 10);
        }

        pub async fn get_proposal(&self, block_height: u64) -> Option<TransactionProposal> {
            let proposals = self.proposals.read().await;
            proposals.get(&block_height).cloned()
        }
        /// Get list of masternodes that voted on this block's transaction set
        pub async fn get_voters(&self, block_height: u64, merkle_root: &str) -> Vec<String> {
            self.vote_collector.get_approvers(block_height, merkle_root)
        }
    }
}

// Block consensus module - for voting on catch-up blocks
pub mod block_consensus {
    use super::*;
    use dashmap::DashMap;

    // Type alias for lock-free concurrent nested map
    type BlockVotesMap = Arc<DashMap<u64, DashMap<String, Vec<BlockVote>>>>;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BlockProposal {
        pub block_height: u64,
        pub proposer: String,
        pub block_hash: String,
        pub merkle_root: String,
        pub previous_hash: String,
        pub timestamp: i64,
        #[serde(default)]
        pub is_reward_only: bool,
        #[serde(default)]
        pub strategy: Option<String>, // Foolproof strategy name
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BlockVote {
        pub block_height: u64,
        pub block_hash: String,
        pub voter: String,
        pub approve: bool,
        pub timestamp: i64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PeerBuildInfo {
        pub version: String,
        pub build_timestamp: String,
        pub commit_count: u64,
    }

    pub struct BlockConsensusManager {
        proposals: Arc<RwLock<HashMap<u64, BlockProposal>>>,
        votes: BlockVotesMap,
        masternodes: Arc<RwLock<Vec<String>>>,
        health: Arc<RwLock<HashMap<String, MasternodeHealth>>>,
        peer_versions: Arc<RwLock<HashMap<String, String>>>,
        peer_build_info: Arc<RwLock<HashMap<String, PeerBuildInfo>>>,
    }

    impl Default for BlockConsensusManager {
        fn default() -> Self {
            Self::new()
        }
    }

    impl BlockConsensusManager {
        pub fn new() -> Self {
            Self {
                proposals: Arc::new(RwLock::new(HashMap::new())),
                votes: Arc::new(DashMap::new()),
                masternodes: Arc::new(RwLock::new(Vec::new())),
                health: Arc::new(RwLock::new(HashMap::new())),
                peer_versions: Arc::new(RwLock::new(HashMap::new())),
                peer_build_info: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        pub async fn set_masternodes(&self, nodes: Vec<String>) {
            let mut masternodes = self.masternodes.write().await;
            *masternodes = nodes;
        }

        pub async fn add_masternode(&self, address: String) {
            let mut masternodes = self.masternodes.write().await;
            if !masternodes.contains(&address) {
                masternodes.push(address);
                masternodes.sort(); // Keep deterministic ordering
            }
        }

        pub async fn propose_block(&self, proposal: BlockProposal) -> bool {
            let mut proposals = self.proposals.write().await;

            // First-proposal-wins: only accept if no proposal exists yet
            if proposals.contains_key(&proposal.block_height) {
                return false; // Proposal already exists, reject this one
            }

            proposals.insert(proposal.block_height, proposal);
            true // Proposal accepted
        }

        /// Vote on a proposed block
        ///
        /// # Maturity Check Required
        /// **IMPORTANT**: Callers should validate that the masternode has reached vote maturity
        /// before calling this function. Use the masternode module's `MasternodeStatus::can_vote_at_height()`
        /// to check if the masternode has waited the required number of blocks since registration.
        /// This prevents instant takeover by newly coordinated malicious nodes.
        pub async fn vote_on_block(&self, vote: BlockVote) -> Result<(), String> {
            let mut masternodes = self.masternodes.write().await;

            // Auto-register voter if not in list (handles race conditions during catch-up)
            if !masternodes.contains(&vote.voter) {
                println!("   ℹ️  Auto-registering voter: {}", vote.voter);
                masternodes.push(vote.voter.clone());
                masternodes.sort();
            }
            drop(masternodes);

            // Check for duplicate vote BEFORE adding
            if self
                .has_voted(&vote.voter, vote.block_height, &vote.block_hash)
                .await
            {
                // Silently ignore duplicate votes
                return Ok(());
            }

            // Use lock-free nested DashMap
            let height_votes = self.votes.entry(vote.block_height).or_default();

            height_votes
                .entry(vote.block_hash.clone())
                .or_default()
                .push(vote);

            Ok(())
        }

        pub async fn has_block_consensus(
            &self,
            block_height: u64,
            block_hash: &str,
        ) -> (bool, usize, usize) {
            let masternodes = self.masternodes.read().await;
            let total_nodes = masternodes.len();
            drop(masternodes);

            if total_nodes < 3 {
                return (true, 0, total_nodes);
            }

            if let Some(height_votes) = self.votes.get(&block_height) {
                if let Some(vote_list) = height_votes.get(block_hash) {
                    let approvals = vote_list.iter().filter(|v| v.approve).count();
                    let required = crate::quorum::required_for_bft(total_nodes);
                    let has_consensus = approvals >= required;
                    return (has_consensus, approvals, total_nodes);
                }
            }

            (false, 0, total_nodes)
        }

        /// Check if a voter has already voted on a specific block
        pub async fn has_voted(&self, voter: &str, block_height: u64, block_hash: &str) -> bool {
            if let Some(height_votes) = self.votes.get(&block_height) {
                if let Some(vote_list) = height_votes.get(block_hash) {
                    return vote_list.iter().any(|v| v.voter == voter);
                }
            }
            false
        }

        /// Clear finalized block consensus data to free memory
        pub async fn clear_block_consensus(&self, block_height: u64) {
            let mut proposals = self.proposals.write().await;
            proposals.remove(&block_height);
            self.votes.remove(&block_height);
        }

        /// Register peer version WITH build info when they connect
        pub async fn register_peer_version_with_build_info(
            &self,
            peer_ip: String,
            version: String,
            build_timestamp: String,
            commit_count: u64,
        ) {
            let mut versions = self.peer_versions.write().await;
            let mut build_info = self.peer_build_info.write().await;

            versions.insert(peer_ip.clone(), version.clone());
            build_info.insert(
                peer_ip,
                PeerBuildInfo {
                    version,
                    build_timestamp,
                    commit_count,
                },
            );
        }

        /// Get the latest version among active nodes using comprehensive comparison
        async fn get_latest_version_among_active(&self, active_nodes: &[String]) -> Option<String> {
            use chrono::NaiveDateTime;

            let build_info = self.peer_build_info.read().await;

            if active_nodes.is_empty() {
                return None;
            }

            // Collect build info for active nodes
            let mut node_builds: Vec<(String, PeerBuildInfo)> = active_nodes
                .iter()
                .filter_map(|node| {
                    build_info
                        .get(node)
                        .map(|info| (node.clone(), info.clone()))
                })
                .collect();

            if node_builds.is_empty() {
                return None;
            }

            // Sort by commit count (highest first), then by timestamp
            node_builds.sort_by(|a, b| {
                // First compare commit counts
                match b.1.commit_count.cmp(&a.1.commit_count) {
                    std::cmp::Ordering::Equal => {
                        // If equal, compare timestamps
                        let format = "%Y-%m-%d %H:%M:%S";
                        let a_time =
                            NaiveDateTime::parse_from_str(&a.1.build_timestamp, format).ok();
                        let b_time =
                            NaiveDateTime::parse_from_str(&b.1.build_timestamp, format).ok();
                        b_time.cmp(&a_time)
                    }
                    other => other,
                }
            });

            // Return the latest version
            node_builds.first().map(|(_, info)| info.version.clone())
        }

        /// Three-round BFT consensus with progressive fallback
        /// Round 1: All active nodes
        /// Round 2: Latest-version active nodes only (encourages upgrades)
        /// Round 3: Emergency - force block creation
        /// Returns: (has_consensus, approvals, total_nodes, round_used)
        pub async fn has_block_consensus_with_progressive_fallback(
            &self,
            block_height: u64,
            block_hash: &str,
        ) -> (bool, usize, usize, u8) {
            let all_masternodes = self.masternodes.read().await.clone();

            if all_masternodes.is_empty() {
                // No nodes registered - emergency
                println!("   🚨 EMERGENCY: No masternodes registered - forcing block");
                return (true, 0, 0, 3);
            }

            // Get vote information
            let vote_list = if let Some(height_votes) = self.votes.get(&block_height) {
                height_votes.get(block_hash).map(|v| v.clone())
            } else {
                None
            };

            // ═══════════════════════════════════════════════════════════════
            // ROUND 1: Try consensus with ALL ACTIVE masternodes
            // ═══════════════════════════════════════════════════════════════
            let active_nodes = self.get_active_masternodes(&all_masternodes).await;
            let active_count = active_nodes.len();

            if active_count >= 3 {
                let active_approvals = vote_list
                    .as_ref()
                    .map(|list| {
                        list.iter()
                            .filter(|v| v.approve && active_nodes.contains(&v.voter))
                            .count()
                    })
                    .unwrap_or(0);

                let required_active = crate::quorum::required_for_bft(active_count);

                if active_approvals >= required_active {
                    println!(
                        "   ✅ Round 1: BFT consensus achieved with active nodes ({}/{} votes, needed {})",
                        active_approvals, active_count, required_active
                    );
                    return (true, active_approvals, active_count, 1);
                }

                println!(
                    "   ⚠️  Round 1: BFT not reached with active nodes ({}/{} votes, needed {})",
                    active_approvals, active_count, required_active
                );
            } else {
                println!(
                    "   ⚠️  Round 1: Not enough active nodes ({} < 3)",
                    active_count
                );
            }

            println!("   🔄 Round 2: Falling back to latest-version active nodes only...");

            // ═══════════════════════════════════════════════════════════════
            // ROUND 2: Try consensus with LATEST-VERSION ACTIVE nodes only
            // ═══════════════════════════════════════════════════════════════

            // Find the latest version among all active nodes
            let latest_version = self.get_latest_version_among_active(&active_nodes).await;

            if let Some(ref version) = latest_version {
                println!("   📌 Latest version detected: {}", version);

                // Get nodes running the latest version
                let latest_version_nodes = self.get_masternodes_by_version(version).await;

                // Filter for both active AND latest version
                let latest_version_active: Vec<String> = active_nodes
                    .iter()
                    .filter(|node| latest_version_nodes.contains(node))
                    .cloned()
                    .collect();

                let latest_version_count = latest_version_active.len();

                if latest_version_count >= 3 {
                    let latest_version_approvals = vote_list
                        .as_ref()
                        .map(|list| {
                            list.iter()
                                .filter(|v| v.approve && latest_version_active.contains(&v.voter))
                                .count()
                        })
                        .unwrap_or(0);

                    let required_latest_version =
                        crate::quorum::required_for_bft(latest_version_count);

                    if latest_version_approvals >= required_latest_version {
                        println!(
                            "   ✅ Round 2: BFT consensus with latest-version nodes ({}/{} votes, needed {})",
                            latest_version_approvals, latest_version_count, required_latest_version
                        );
                        println!("   📊 Using only nodes running latest version: {}", version);

                        // Warn outdated nodes
                        let outdated_count = active_count - latest_version_count;
                        if outdated_count > 0 {
                            println!(
                                "   ⚠️  {} active node(s) running older versions excluded",
                                outdated_count
                            );
                            println!("   💡 Outdated nodes should update to version {}", version);
                        }

                        return (true, latest_version_approvals, latest_version_count, 2);
                    }

                    println!(
                        "   ⚠️  Round 2: BFT not reached with latest-version nodes ({}/{} votes, needed {})",
                        latest_version_approvals, latest_version_count, required_latest_version
                    );
                } else {
                    println!(
                        "   ⚠️  Round 2: Not enough latest-version active nodes ({} < 3)",
                        latest_version_count
                    );
                }
            } else {
                println!("   ⚠️  Round 2: Could not determine latest version from active nodes");
            }

            // ═══════════════════════════════════════════════════════════════
            // ROUND 3: EMERGENCY - Force block creation to prevent chain halt
            // ═══════════════════════════════════════════════════════════════
            println!();
            println!("   🚨 ════════════════════════════════════════════════════════");
            println!("   🚨 ROUND 3: EMERGENCY CONSENSUS ACTIVATED");
            println!("   🚨 ════════════════════════════════════════════════════════");
            println!("   📋 Reason: Unable to reach 2/3+ consensus in Rounds 1 & 2");
            println!("   ⚠️  Status: FORCING block creation to prevent chain halt");
            println!("   📊 Network Health:");
            println!("       - Total masternodes: {}", all_masternodes.len());
            println!("       - Active nodes: {}", active_count);

            if let Some(version) = latest_version {
                let latest_count = self.get_masternodes_by_version(&version).await.len();
                println!("       - Latest-version active: {}", latest_count);
                println!("       - Latest version: {}", version);
            }

            if let Some(list) = vote_list.as_ref() {
                let total_approvals = list.iter().filter(|v| v.approve).count();
                println!("       - Total approvals received: {}", total_approvals);
            }

            println!();
            println!("   ⚠️  OPERATOR WARNING:");
            println!("   - Emergency consensus should be RARE");
            println!("   - Check network connectivity between nodes");
            println!("   - Verify all nodes are running and responsive");
            println!("   - Consider restarting lagging nodes");
            println!("   - Review masternode health status");
            println!("   - Ensure nodes are updated to latest version");
            println!();
            println!("   ✅ Block will be created to maintain chain continuity");
            println!("   🚨 ════════════════════════════════════════════════════════");
            println!();

            // Return true to force block creation, with any approvals we got
            let emergency_approvals = vote_list
                .as_ref()
                .map(|list| list.iter().filter(|v| v.approve).count())
                .unwrap_or(0);

            (true, emergency_approvals, all_masternodes.len(), 3)
        }

        pub async fn get_agreed_block(&self, block_height: u64) -> Option<BlockProposal> {
            let proposals = self.proposals.read().await;
            let proposal = proposals.get(&block_height)?;

            let (has_consensus, _, _) = self
                .has_block_consensus(block_height, &proposal.block_hash)
                .await;

            if has_consensus {
                Some(proposal.clone())
            } else {
                None
            }
        }

        pub async fn cleanup_old(&self, current_height: u64) {
            let mut proposals = self.proposals.write().await;
            proposals.retain(|&h, _| h >= current_height.saturating_sub(10));
            self.votes
                .retain(|h, _| *h >= current_height.saturating_sub(10));
        }

        pub async fn get_proposal(&self, block_height: u64) -> Option<BlockProposal> {
            let proposals = self.proposals.read().await;
            proposals.get(&block_height).cloned()
        }

        /// Get list of masternodes that voted on this block
        pub async fn get_voters(&self, block_height: u64, block_hash: &str) -> Vec<String> {
            if let Some(height_votes) = self.votes.get(&block_height) {
                if let Some(vote_list) = height_votes.get(block_hash) {
                    return vote_list
                        .iter()
                        .filter(|v| v.approve)
                        .map(|v| v.voter.clone())
                        .collect();
                }
            }
            Vec::new()
        }
        pub async fn store_proposal(&self, proposal: BlockProposal) {
            let mut proposals = self.proposals.write().await;
            proposals.insert(proposal.block_height, proposal);
        }

        pub async fn store_vote(&self, vote: BlockVote) {
            let height_votes = self.votes.entry(vote.block_height).or_default();

            height_votes
                .entry(vote.block_hash.clone())
                .or_default()
                .push(vote);
        }

        pub async fn wait_for_proposal(&self, block_height: u64) -> Option<BlockProposal> {
            // Wait up to 60 seconds for a proposal
            for _ in 0..600 {
                let proposals = self.proposals.read().await;
                if let Some(proposal) = proposals.get(&block_height) {
                    return Some(proposal.clone());
                }
                drop(proposals);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
            None
        }

        pub async fn collect_votes(
            &self,
            block_height: u64,
            required_votes: usize,
        ) -> (usize, usize) {
            self.collect_votes_with_timeout(block_height, required_votes, 30)
                .await
        }

        /// Collect votes with a configurable timeout in seconds
        pub async fn collect_votes_with_timeout(
            &self,
            block_height: u64,
            required_votes: usize,
            timeout_secs: u64,
        ) -> (usize, usize) {
            // Get total masternodes that should be voting
            let masternodes = self.masternodes.read().await;
            let total_masternodes = masternodes.len();
            drop(masternodes);

            // Helper to count approved votes for a block height
            let count_approved = |block_height: u64, votes_map: &BlockVotesMap| -> usize {
                votes_map
                    .get(&block_height)
                    .map(|height_votes| {
                        height_votes
                            .iter()
                            .flat_map(|entry| entry.value().clone())
                            .filter(|vote| vote.approve)
                            .count()
                    })
                    .unwrap_or(0)
            };

            // Poll for votes with timeout
            let poll_future = async {
                loop {
                    let approved = count_approved(block_height, &self.votes);
                    if approved >= required_votes {
                        return approved;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            };

            match tokio::time::timeout(tokio::time::Duration::from_secs(timeout_secs), poll_future)
                .await
            {
                Ok(approved) => (approved, total_masternodes),
                Err(_) => {
                    // Timeout reached - return whatever votes we have
                    let approved = count_approved(block_height, &self.votes);
                    (approved, total_masternodes)
                }
            }
        }

        pub fn validate_proposal(
            &self,
            proposal: &BlockProposal,
            blockchain_tip_hash: &str,
            blockchain_height: u64,
        ) -> bool {
            // Validate previous hash
            if proposal.previous_hash != blockchain_tip_hash {
                println!("   ❌ REJECT: Previous hash mismatch");
                println!("      Expected: {}", blockchain_tip_hash);
                println!("      Got: {}", proposal.previous_hash);
                return false;
            }

            // Validate block height
            if proposal.block_height != blockchain_height + 1 {
                println!("   ❌ REJECT: Block height mismatch");
                println!("      Expected: {}", blockchain_height + 1);
                println!("      Got: {}", proposal.block_height);
                return false;
            }

            println!("   ✅ Proposal validation passed");
            println!("      Block height: {}", proposal.block_height);
            println!(
                "      Previous hash: {}...",
                &proposal.previous_hash[..proposal.previous_hash.len().min(16)]
            );
            println!(
                "      Merkle root: {}...",
                &proposal.merkle_root[..proposal.merkle_root.len().min(16)]
            );

            true
        }

        /// Initialize health tracking for a masternode
        pub async fn init_masternode_health(&self, address: String) {
            let mut health = self.health.write().await;
            if !health.contains_key(&address) {
                health.insert(address.clone(), MasternodeHealth::new(address.clone()));
                println!("📊 Health tracking initialized for {}", address);
            }
        }

        /// Record a vote with response time
        pub async fn record_vote_response(&self, address: &str, response_time_ms: u64) {
            let mut health = self.health.write().await;

            if let Some(node_health) = health.get_mut(address) {
                node_health.last_response = chrono::Utc::now().timestamp();
                node_health.consecutive_misses = 0;
                node_health.total_votes_cast += 1;

                // Update rolling average response time
                let alpha = 0.3; // Smoothing factor
                node_health.avg_response_time_ms =
                    ((1.0 - alpha) * node_health.avg_response_time_ms as f64
                        + alpha * response_time_ms as f64) as u64;

                // Update participation rate
                if node_health.total_votes_expected > 0 {
                    node_health.vote_participation_rate = node_health.total_votes_cast as f32
                        / node_health.total_votes_expected as f32;
                }

                // Check for state changes based on response time
                if node_health.status == NodeStatus::Active
                    && response_time_ms > DEGRADED_THRESHOLD_MS
                {
                    self.transition_to_degraded(address).await;
                } else if node_health.status == NodeStatus::Degraded && response_time_ms <= 1000 {
                    // Good response from degraded node - potential recovery
                    println!(
                        "   ✅ {} responding well ({}ms) - monitoring for recovery",
                        address, response_time_ms
                    );
                }
            }
        }

        /// Record a missed vote
        pub async fn record_missed_vote(&self, address: &str) {
            let mut health = self.health.write().await;

            if let Some(node_health) = health.get_mut(address) {
                node_health.missed_votes += 1;
                node_health.consecutive_misses += 1;
                node_health.total_votes_expected += 1;

                // Update participation rate
                node_health.vote_participation_rate =
                    node_health.total_votes_cast as f32 / node_health.total_votes_expected as f32;

                println!(
                    "   ⚠️  {} missed vote (consecutive: {}, participation: {:.1}%)",
                    address,
                    node_health.consecutive_misses,
                    node_health.vote_participation_rate * 100.0
                );

                // Check for state transitions
                if node_health.consecutive_misses >= MAX_CONSECUTIVE_MISSES
                    && (node_health.status == NodeStatus::Active
                        || node_health.status == NodeStatus::Degraded)
                {
                    self.transition_to_quarantined(address).await;
                }
            }
        }

        /// Transition masternode to DEGRADED status
        async fn transition_to_degraded(&self, address: &str) {
            let mut health = self.health.write().await;

            if let Some(node_health) = health.get_mut(address) {
                if node_health.status != NodeStatus::Degraded {
                    node_health.status = NodeStatus::Degraded;
                    node_health.status_changed_at = chrono::Utc::now().timestamp();

                    println!();
                    println!("⚠️  ═══════════════════════════════════════════════════════");
                    println!("⚠️  MASTERNODE DEGRADED: {}", address);
                    println!("⚠️  ═══════════════════════════════════════════════════════");
                    println!(
                        "   Reason: Slow response times (avg {}ms)",
                        node_health.avg_response_time_ms
                    );
                    println!("   Status: Still participating in consensus");
                    println!("   Action: Monitor network connection and server performance");
                    println!("   Impact: May be quarantined if performance doesn't improve");
                    println!("⚠️  ═══════════════════════════════════════════════════════");
                    println!();
                }
            }
        }

        /// Transition masternode to QUARANTINED status
        async fn transition_to_quarantined(&self, address: &str) {
            let mut health = self.health.write().await;

            if let Some(node_health) = health.get_mut(address) {
                node_health.status = NodeStatus::Quarantined;
                node_health.status_changed_at = chrono::Utc::now().timestamp();

                let quarantine_until =
                    chrono::Utc::now() + chrono::Duration::seconds(QUARANTINE_DURATION_SECS);

                println!();
                println!("🔒 ═══════════════════════════════════════════════════════");
                println!("🔒 MASTERNODE QUARANTINED: {}", address);
                println!("🔒 ═══════════════════════════════════════════════════════");
                println!(
                    "   Reason: {} consecutive missed votes",
                    node_health.consecutive_misses
                );
                println!(
                    "   Participation rate: {:.1}%",
                    node_health.vote_participation_rate * 100.0
                );
                println!("   Status: EXCLUDED from consensus");
                println!(
                    "   Duration: 1 hour (until {})",
                    quarantine_until.format("%H:%M:%S UTC")
                );
                println!("   ");
                println!("   ⚠️  OPERATOR ACTION REQUIRED:");
                println!("   1. Check server is online and responsive");
                println!("   2. Check network connectivity");
                println!("   3. Review server logs for errors");
                println!("   4. Verify firewall allows port 24101");
                println!("   ");
                println!("   If issues persist, node will be downgraded to seed-only");
                println!("🔒 ═══════════════════════════════════════════════════════");
                println!();
            }
        }

        /// Transition masternode to DOWNGRADED status (seed node only)
        async fn transition_to_downgraded(&self, address: &str) {
            let mut health = self.health.write().await;

            if let Some(node_health) = health.get_mut(address) {
                node_health.status = NodeStatus::Downgraded;
                node_health.status_changed_at = chrono::Utc::now().timestamp();

                println!();
                println!("⬇️  ═══════════════════════════════════════════════════════");
                println!("⬇️  MASTERNODE DOWNGRADED: {}", address);
                println!("⬇️  ═══════════════════════════════════════════════════════");
                println!("   Reason: Persistent poor performance after quarantine");
                println!("   Status: DEMOTED to seed node only");
                println!("   ");
                println!("   ❌ NO LONGER:");
                println!("   - Participating in consensus votes");
                println!("   - Earning block rewards");
                println!("   - Validating transactions");
                println!("   ");
                println!("   ✅ STILL:");
                println!("   - Serving as seed node for peer discovery");
                println!("   - Collateral locked");
                println!("   ");
                println!("   🔧 TO REGAIN MASTERNODE STATUS:");
                println!("   1. Fix performance issues (target <1000ms response)");
                println!("   2. Ensure 99%+ uptime");
                println!("   3. Node will automatically retest and promote if healthy");
                println!("   ");
                println!("   Contact: Check node logs and system resources");
                println!("⬇️  ═══════════════════════════════════════════════════════");
                println!();
            }
        }

        /// Get only active masternodes (for consensus)
        pub async fn get_active_masternodes(&self, all_masternodes: &[String]) -> Vec<String> {
            let health = self.health.read().await;

            let active: Vec<String> = all_masternodes
                .iter()
                .filter(|addr| {
                    health
                        .get(*addr)
                        .map(|h| h.status == NodeStatus::Active)
                        .unwrap_or(true) // Default to active if not tracked yet
                })
                .cloned()
                .collect();

            let excluded = all_masternodes.len() - active.len();
            if excluded > 0 {
                println!(
                    "   ℹ️  Consensus pool: {} active, {} excluded",
                    active.len(),
                    excluded
                );
            }

            active
        }

        /// Count only active masternodes
        pub async fn active_masternode_count(&self) -> usize {
            let masternodes = self.masternodes.read().await;
            let health = self.health.read().await;

            masternodes
                .iter()
                .filter(|addr| {
                    health
                        .get(*addr)
                        .map(|h| h.status == NodeStatus::Active)
                        .unwrap_or(true)
                })
                .count()
        }

        /// Sync masternodes with connected peers and update health status
        /// This ensures disconnected nodes are marked as Offline immediately
        pub async fn sync_with_connected_peers(&self, connected_peer_ips: Vec<String>) {
            let mut masternodes = self.masternodes.write().await;
            let mut health = self.health.write().await;

            // Convert connected peers to a set for quick lookup
            let connected_set: std::collections::HashSet<String> =
                std::collections::HashSet::from_iter(connected_peer_ips.iter().cloned());

            // Add any new connected peers to the masternode list if they're not already there
            for peer_ip in connected_peer_ips.iter() {
                if !masternodes.contains(peer_ip) {
                    masternodes.push(peer_ip.clone());
                }
            }
            masternodes.sort(); // Keep deterministic ordering

            // Update health status for all masternodes based on connection status
            for masternode in masternodes.iter() {
                if connected_set.contains(masternode) {
                    // Node is connected - mark as Active
                    if let Some(node_health) = health.get_mut(masternode) {
                        if node_health.status == NodeStatus::Offline {
                            // Node reconnected, restore to Active
                            node_health.status = NodeStatus::Active;
                            node_health.consecutive_misses = 0;
                            node_health.status_changed_at = chrono::Utc::now().timestamp();
                        }
                    } else {
                        // New node, initialize health tracking as Active
                        health.insert(
                            masternode.clone(),
                            MasternodeHealth::new(masternode.clone()),
                        );
                    }
                } else {
                    // Node is NOT connected - mark as Offline
                    if let Some(node_health) = health.get_mut(masternode) {
                        if node_health.status != NodeStatus::Offline {
                            node_health.status = NodeStatus::Offline;
                            node_health.status_changed_at = chrono::Utc::now().timestamp();
                        }
                    } else {
                        // Initialize health tracking for nodes we haven't seen before
                        let mut new_health = MasternodeHealth::new(masternode.clone());
                        new_health.status = NodeStatus::Offline;
                        health.insert(masternode.clone(), new_health);
                    }
                }
            }
        }

        /// Check for quarantine expiration and recovery
        pub async fn check_quarantine_expiration(&self) {
            let mut addresses_to_downgrade = Vec::new();

            {
                let mut health = self.health.write().await;
                let now = chrono::Utc::now().timestamp();

                for (address, node_health) in health.iter_mut() {
                    if node_health.status == NodeStatus::Quarantined {
                        let time_in_quarantine = now - node_health.status_changed_at;

                        if time_in_quarantine >= QUARANTINE_DURATION_SECS {
                            // Check if they've improved
                            if node_health.consecutive_misses < MAX_CONSECUTIVE_MISSES {
                                println!("✅ {} quarantine expired - restoring to ACTIVE", address);
                                node_health.status = NodeStatus::Active;
                                node_health.consecutive_misses = 0;
                                node_health.status_changed_at = now;
                            } else {
                                // Mark for downgrade (will be done after releasing lock)
                                addresses_to_downgrade.push(address.clone());
                            }
                        }
                    }
                }
            } // Release lock

            // Now downgrade nodes that need it
            for address in addresses_to_downgrade {
                self.transition_to_downgraded(&address).await;
            }
        }

        /// Print health status report
        pub async fn print_health_report(&self) {
            let health = self.health.read().await;

            if health.is_empty() {
                return;
            }

            println!();
            println!("📊 ═══════════════════════════════════════════════════════");
            println!("📊 MASTERNODE HEALTH REPORT");
            println!("📊 ═══════════════════════════════════════════════════════");

            for (address, node_health) in health.iter() {
                println!(
                    "   {} {} - {}ms avg - {:.0}% participation",
                    node_health.status,
                    address,
                    node_health.avg_response_time_ms,
                    node_health.vote_participation_rate * 100.0
                );

                if node_health.status != NodeStatus::Active {
                    println!(
                        "      Consecutive misses: {}",
                        node_health.consecutive_misses
                    );
                }
            }

            println!("📊 ═══════════════════════════════════════════════════════");
            println!();
        }

        /// Register peer version when they connect (legacy method, kept for compatibility)
        pub async fn register_peer_version(&self, peer_ip: String, version: String) {
            let mut versions = self.peer_versions.write().await;
            versions.insert(peer_ip, version);
        }

        /// Get masternodes matching a specific version
        pub async fn get_masternodes_by_version(&self, target_version: &str) -> Vec<String> {
            let masternodes = self.masternodes.read().await;
            let versions = self.peer_versions.read().await;

            masternodes
                .iter()
                .filter(|node| {
                    versions
                        .get(*node)
                        .map(|v| v == target_version)
                        .unwrap_or(false)
                })
                .cloned()
                .collect()
        }

        /// Check consensus with optional version filtering
        pub async fn has_block_consensus_with_version_filter(
            &self,
            block_height: u64,
            block_hash: &str,
            version_filter: Option<&str>,
        ) -> (bool, usize, usize) {
            let all_masternodes = self.masternodes.read().await;

            let eligible_nodes = if let Some(version) = version_filter {
                let versions = self.peer_versions.read().await;
                all_masternodes
                    .iter()
                    .filter(|node| versions.get(*node).map(|v| v == version).unwrap_or(false))
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                all_masternodes.clone()
            };

            let total_nodes = eligible_nodes.len();
            drop(all_masternodes);

            if total_nodes < 3 {
                return (true, 0, total_nodes);
            }

            if let Some(height_votes) = self.votes.get(&block_height) {
                if let Some(vote_list) = height_votes.get(block_hash) {
                    let approvals = vote_list
                        .iter()
                        .filter(|v| v.approve && eligible_nodes.contains(&v.voter))
                        .count();

                    let required = crate::quorum::required_for_bft(total_nodes);
                    let has_consensus = approvals >= required;
                    return (has_consensus, approvals, total_nodes);
                }
            }

            (false, 0, total_nodes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vrf_selection_deterministic() {
        let engine = ConsensusEngine::new(false);

        // Add masternodes
        engine.add_masternode("192.168.1.1".to_string()).await;
        engine.add_masternode("192.168.1.2".to_string()).await;
        engine.add_masternode("192.168.1.3".to_string()).await;

        // Same block height and previous hash should give same leader
        let leader1 = engine.get_leader(100).await.unwrap();
        let leader2 = engine.get_leader(100).await.unwrap();

        assert_eq!(leader1, leader2, "VRF selection should be deterministic");
    }

    #[tokio::test]
    async fn test_vrf_selection_different_blocks() {
        let engine = ConsensusEngine::new(false);

        // Add masternodes
        engine.add_masternode("192.168.1.1".to_string()).await;
        engine.add_masternode("192.168.1.2".to_string()).await;
        engine.add_masternode("192.168.1.3".to_string()).await;
        engine.add_masternode("192.168.1.4".to_string()).await;

        // Different block heights can produce different leaders
        let leader1 = engine.get_leader(100).await.unwrap();
        let leader2 = engine.get_leader(101).await.unwrap();

        // With 4 nodes, there's a good chance leaders will differ
        // Just verify both are valid masternodes
        let masternodes = engine.get_masternodes().await;
        assert!(masternodes.contains(&leader1));
        assert!(masternodes.contains(&leader2));
    }

    #[tokio::test]
    async fn test_vrf_selection_distribution() {
        let engine = ConsensusEngine::new(false);

        // Add masternodes
        engine.add_masternode("192.168.1.1".to_string()).await;
        engine.add_masternode("192.168.1.2".to_string()).await;
        engine.add_masternode("192.168.1.3".to_string()).await;
        engine.add_masternode("192.168.1.4".to_string()).await;

        // Test that VRF distributes selections over multiple blocks
        let mut selections = HashMap::new();
        for height in 1..=100 {
            let leader = engine.get_leader(height).await.unwrap();
            *selections.entry(leader).or_insert(0) += 1;
        }

        // All nodes should be selected at least once over 100 blocks
        let masternodes = engine.get_masternodes().await;
        for mn in masternodes {
            let count = selections.get(&mn).unwrap_or(&0);
            assert!(
                *count > 0,
                "Node {} should be selected at least once in 100 blocks",
                mn
            );
        }
    }

    #[tokio::test]
    async fn test_vrf_proof_generation_and_verification() {
        let engine = ConsensusEngine::new(false);

        let block_height = 100u64;
        let previous_hash = "test_hash_12345";
        let leader = "192.168.1.1";

        // Generate proof
        let proof = engine.generate_vrf_proof(block_height, previous_hash, leader);

        // Verify correct proof
        assert!(
            engine.verify_vrf_proof(block_height, previous_hash, leader, &proof),
            "Valid proof should verify"
        );

        // Verify invalid proof
        let invalid_proof = vec![0u8; 32];
        assert!(
            !engine.verify_vrf_proof(block_height, previous_hash, leader, &invalid_proof),
            "Invalid proof should not verify"
        );

        // Verify proof with wrong leader
        assert!(
            !engine.verify_vrf_proof(block_height, previous_hash, "wrong_leader", &proof),
            "Proof for wrong leader should not verify"
        );

        // Verify proof with wrong block height
        assert!(
            !engine.verify_vrf_proof(block_height + 1, previous_hash, leader, &proof),
            "Proof for wrong block height should not verify"
        );
    }

    #[tokio::test]
    async fn test_vrf_seed_changes_with_previous_hash() {
        let engine = ConsensusEngine::new(false);

        // Same block height but different previous hashes should produce different seeds
        let seed1 = engine.vrf_selector.generate_seed(100, "hash1");
        let seed2 = engine.vrf_selector.generate_seed(100, "hash2");

        assert_ne!(
            seed1, seed2,
            "Different previous hashes should produce different VRF seeds"
        );
    }

    #[tokio::test]
    async fn test_vrf_unpredictable_selection() {
        let engine = ConsensusEngine::new(false);

        // Add masternodes
        for i in 1..=10 {
            engine.add_masternode(format!("192.168.1.{}", i)).await;
        }

        // Verify that we can't easily predict the next leader
        // by looking at a pattern of previous selections
        let mut leaders = Vec::new();
        for height in 1..=20 {
            let leader = engine.get_leader(height).await.unwrap();
            leaders.push(leader);
        }

        // Check that selections vary (not all the same)
        let first_leader = &leaders[0];
        let all_same = leaders.iter().all(|l| l == first_leader);
        assert!(
            !all_same,
            "VRF should produce varied selections, not all the same leader"
        );
    }

    #[tokio::test]
    async fn test_is_my_turn_with_vrf() {
        let engine = ConsensusEngine::new(false);

        // Add masternodes
        engine.add_masternode("192.168.1.1".to_string()).await;
        engine.add_masternode("192.168.1.2".to_string()).await;
        engine.add_masternode("192.168.1.3".to_string()).await;

        // Check that exactly one node is selected per block
        let block_height = 100u64;
        let leader = engine.get_leader(block_height).await.unwrap();

        assert!(engine.is_my_turn(block_height, &leader).await);

        // Other nodes should not be the leader
        let masternodes = engine.get_masternodes().await;
        for mn in masternodes {
            if mn != leader {
                assert!(!engine.is_my_turn(block_height, &mn).await);
            }
        }
    }

    #[tokio::test]
    async fn test_get_block_producer_with_vrf() {
        let engine = ConsensusEngine::new(false);

        // Add masternodes
        engine.add_masternode("192.168.1.1".to_string()).await;
        engine.add_masternode("192.168.1.2".to_string()).await;

        // Test block producer selection
        let producer = engine.get_block_producer(100).await;
        assert!(producer.is_some());

        let masternodes = engine.get_masternodes().await;
        assert!(masternodes.contains(&producer.unwrap()));
    }

    #[tokio::test]
    async fn test_transaction_voting() {
        use time_core::transaction::{Transaction, TxOutput};

        let engine = ConsensusEngine::new(false);

        // Add masternodes
        engine.add_masternode("mn1".to_string()).await;
        engine.add_masternode("mn2".to_string()).await;
        engine.add_masternode("mn3".to_string()).await;

        let tx = Transaction {
            txid: "test_tx_1".to_string(),
            version: 1,
            inputs: vec![],
            outputs: vec![TxOutput {
                amount: 1000,
                address: "addr1".to_string(),
            }],
            lock_time: 0,
            timestamp: 1234567890,
        };

        // Vote from all masternodes
        engine
            .vote_on_transaction(&tx.txid, "mn1".to_string(), true)
            .await
            .unwrap();
        engine
            .vote_on_transaction(&tx.txid, "mn2".to_string(), true)
            .await
            .unwrap();
        engine
            .vote_on_transaction(&tx.txid, "mn3".to_string(), true)
            .await
            .unwrap();

        // Check vote counts
        let (approvals, rejections) = engine.get_transaction_vote_count(&tx.txid).await;
        assert_eq!(approvals, 3);
        assert_eq!(rejections, 0);

        // Check consensus (should have 3/3 = 100% approval)
        assert!(engine.has_transaction_consensus(&tx.txid).await);
    }

    #[tokio::test]
    async fn test_transaction_consensus_threshold() {
        use time_core::transaction::{Transaction, TxOutput};

        let engine = ConsensusEngine::new(false);

        // Add masternodes
        engine.add_masternode("mn1".to_string()).await;
        engine.add_masternode("mn2".to_string()).await;
        engine.add_masternode("mn3".to_string()).await;

        let tx = Transaction {
            txid: "test_tx_2".to_string(),
            version: 1,
            inputs: vec![],
            outputs: vec![TxOutput {
                amount: 1000,
                address: "addr1".to_string(),
            }],
            lock_time: 0,
            timestamp: 1234567890,
        };

        // Only 2 out of 3 approve (66.7%)
        engine
            .vote_on_transaction(&tx.txid, "mn1".to_string(), true)
            .await
            .unwrap();
        engine
            .vote_on_transaction(&tx.txid, "mn2".to_string(), true)
            .await
            .unwrap();
        engine
            .vote_on_transaction(&tx.txid, "mn3".to_string(), false)
            .await
            .unwrap();

        // Should reach consensus (2/3 = 67% meets the 2/3 threshold)
        assert!(engine.has_transaction_consensus(&tx.txid).await);
    }

    #[tokio::test]
    async fn test_transaction_consensus_not_reached() {
        use time_core::transaction::{Transaction, TxOutput};

        let engine = ConsensusEngine::new(false);

        // Add masternodes
        engine.add_masternode("mn1".to_string()).await;
        engine.add_masternode("mn2".to_string()).await;
        engine.add_masternode("mn3".to_string()).await;

        let tx = Transaction {
            txid: "test_tx_3".to_string(),
            version: 1,
            inputs: vec![],
            outputs: vec![TxOutput {
                amount: 1000,
                address: "addr1".to_string(),
            }],
            lock_time: 0,
            timestamp: 1234567890,
        };

        // Only 1 out of 3 approves (33.3%)
        engine
            .vote_on_transaction(&tx.txid, "mn1".to_string(), true)
            .await
            .unwrap();
        engine
            .vote_on_transaction(&tx.txid, "mn2".to_string(), false)
            .await
            .unwrap();
        engine
            .vote_on_transaction(&tx.txid, "mn3".to_string(), false)
            .await
            .unwrap();

        // Should NOT reach consensus (1/3 = 33% below the 67% threshold)
        assert!(!engine.has_transaction_consensus(&tx.txid).await);
    }
}

// Re-exports
pub use transaction_approval::{
    ApprovalDecision, TransactionApproval, TransactionApprovalManager, TransactionStatus,
};
