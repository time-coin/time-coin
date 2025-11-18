//! Blockchain State Manager for TIME Coin
//!
//! Manages the blockchain state including:
//! - UTXO set
//! - Block chain
//! - Masternode tracking
//! - Chain tip and reorganization

use crate::block::{Block, BlockError, MasternodeCounts, MasternodeTier};
use crate::transaction::{OutPoint, Transaction, TransactionError};
use crate::utxo_set::UTXOSet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub enum StateError {
    IoError(String),
    BlockError(BlockError),
    TransactionError(TransactionError),
    InvalidBlockHeight,
    BlockNotFound,
    InvalidPreviousHash,
    DuplicateBlock,
    DuplicateTransaction,
    OrphanBlock,
    InvalidMasternodeCount,
    ChainValidationFailed(String),
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            StateError::BlockError(e) => write!(f, "Block error: {}", e),
            StateError::TransactionError(e) => write!(f, "Transaction error: {}", e),
            StateError::InvalidBlockHeight => write!(f, "Invalid block height"),
            StateError::BlockNotFound => write!(f, "Block not found"),
            StateError::InvalidPreviousHash => write!(f, "Invalid previous hash"),
            StateError::DuplicateBlock => write!(f, "Duplicate block"),
            StateError::DuplicateTransaction => write!(f, "Duplicate transaction"),
            StateError::OrphanBlock => write!(f, "Orphan block"),
            StateError::InvalidMasternodeCount => write!(f, "Invalid masternode count"),
            StateError::IoError(msg) => write!(f, "IO error: {}", msg),
            StateError::ChainValidationFailed(msg) => write!(f, "Chain validation failed: {}", msg),
        }
    }
}

impl std::error::Error for StateError {}

impl From<BlockError> for StateError {
    fn from(err: BlockError) -> Self {
        StateError::BlockError(err)
    }
}

impl From<TransactionError> for StateError {
    fn from(err: TransactionError) -> Self {
        StateError::TransactionError(err)
    }
}

/// Represents a masternode registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasternodeInfo {
    pub address: String,
    pub tier: MasternodeTier,
    pub collateral_tx: String,
    pub registered_height: u64,
    pub last_seen: i64,
    pub is_active: bool,
    pub wallet_address: String,
}

/// Transaction invalidation event
#[derive(Debug, Clone, Serialize)]
pub struct TxInvalidationEvent {
    pub txid: String,
    pub reason: String,
    pub timestamp: i64,
    pub affected_addresses: Vec<String>,
}

/// Treasury allocation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryAllocation {
    pub block_number: u64,
    pub amount: u64,
    pub source: TreasurySource,
    pub timestamp: i64,
}

/// Source of treasury funds
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TreasurySource {
    BlockReward,
    TransactionFees,
}

/// Treasury withdrawal record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryWithdrawal {
    pub proposal_id: String,
    pub amount: u64,
    pub recipient: String,
    pub block_number: u64,
    pub timestamp: i64,
}

/// Approved grant record for treasury proposals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovedGrant {
    pub proposal_id: String,
    pub amount: u64,
    pub approved_at: i64,
}

/// Protocol-managed treasury state (no wallet address or private key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Treasury {
    /// Current treasury balance (tracked in state, not UTXOs)
    balance: u64,

    /// Total amount ever allocated to treasury
    total_allocated: u64,

    /// Total amount distributed from treasury
    total_distributed: u64,

    /// History of all allocations
    allocations: Vec<TreasuryAllocation>,

    /// History of all withdrawals
    withdrawals: Vec<TreasuryWithdrawal>,

    /// Approved proposals that can withdraw funds
    approved_proposals: HashMap<String, ApprovedGrant>, // proposal_id -> ApprovedGrant

    /// Percentage of block rewards allocated to treasury (default 5%)
    block_reward_percentage: u64,

    /// Percentage of transaction fees allocated to treasury (default 50%)
    fee_percentage: u64,
}

impl Treasury {
    /// Create a new empty treasury
    pub fn new() -> Self {
        Self {
            balance: 0,
            total_allocated: 0,
            total_distributed: 0,
            allocations: Vec::new(),
            withdrawals: Vec::new(),
            approved_proposals: HashMap::new(),
            block_reward_percentage: 5, // 5% of block rewards
            fee_percentage: 50,         // 50% of transaction fees
        }
    }

    /// Get current balance
    pub fn balance(&self) -> u64 {
        self.balance
    }

    /// Get total allocated
    pub fn total_allocated(&self) -> u64 {
        self.total_allocated
    }

    /// Get total distributed
    pub fn total_distributed(&self) -> u64 {
        self.total_distributed
    }

    /// Get allocation history
    pub fn allocations(&self) -> &[TreasuryAllocation] {
        &self.allocations
    }

    /// Get withdrawal history
    pub fn withdrawals(&self) -> &[TreasuryWithdrawal] {
        &self.withdrawals
    }

    /// Allocate funds from block reward
    pub fn allocate_from_block_reward(
        &mut self,
        block_number: u64,
        block_reward: u64,
        timestamp: i64,
    ) -> Result<u64, StateError> {
        let allocation = (block_reward * self.block_reward_percentage) / 100;

        self.balance = self
            .balance
            .checked_add(allocation)
            .ok_or_else(|| StateError::IoError("Treasury balance overflow".to_string()))?;

        self.total_allocated = self
            .total_allocated
            .checked_add(allocation)
            .ok_or_else(|| StateError::IoError("Treasury total allocation overflow".to_string()))?;

        self.allocations.push(TreasuryAllocation {
            block_number,
            amount: allocation,
            source: TreasurySource::BlockReward,
            timestamp,
        });

        Ok(allocation)
    }

    /// Directly allocate a specific amount to treasury (used when amount is pre-calculated in coinbase)
    pub fn allocate_direct(
        &mut self,
        block_number: u64,
        amount: u64,
        source: TreasurySource,
        timestamp: i64,
    ) -> Result<(), StateError> {
        self.balance = self
            .balance
            .checked_add(amount)
            .ok_or_else(|| StateError::IoError("Treasury balance overflow".to_string()))?;

        self.total_allocated = self
            .total_allocated
            .checked_add(amount)
            .ok_or_else(|| StateError::IoError("Treasury total allocation overflow".to_string()))?;

        self.allocations.push(TreasuryAllocation {
            block_number,
            amount,
            source,
            timestamp,
        });

        Ok(())
    }

    /// Allocate funds from transaction fees
    pub fn allocate_from_fees(
        &mut self,
        block_number: u64,
        total_fees: u64,
        timestamp: i64,
    ) -> Result<u64, StateError> {
        if total_fees == 0 {
            return Ok(0);
        }

        let allocation = (total_fees * self.fee_percentage) / 100;

        self.balance = self
            .balance
            .checked_add(allocation)
            .ok_or_else(|| StateError::IoError("Treasury balance overflow".to_string()))?;

        self.total_allocated = self
            .total_allocated
            .checked_add(allocation)
            .ok_or_else(|| StateError::IoError("Treasury total allocation overflow".to_string()))?;

        self.allocations.push(TreasuryAllocation {
            block_number,
            amount: allocation,
            source: TreasurySource::TransactionFees,
            timestamp,
        });

        Ok(allocation)
    }

    /// Approve a proposal for spending (called by governance)
    pub fn approve_proposal(&mut self, proposal_id: String, amount: u64) -> Result<(), StateError> {
        if amount > self.balance {
            return Err(StateError::IoError(format!(
                "Insufficient treasury balance: requested {}, available {}",
                amount, self.balance
            )));
        }

        let timestamp = chrono::Utc::now().timestamp();
        let grant = ApprovedGrant {
            proposal_id: proposal_id.clone(),
            amount,
            approved_at: timestamp,
        };
        self.approved_proposals.insert(proposal_id, grant);
        Ok(())
    }

    /// Distribute funds for an approved proposal
    pub fn distribute(
        &mut self,
        proposal_id: String,
        recipient: String,
        amount: u64,
        block_number: u64,
        timestamp: i64,
    ) -> Result<(), StateError> {
        // Check if proposal is approved
        let approved_grant = self
            .approved_proposals
            .get(&proposal_id)
            .ok_or_else(|| StateError::IoError(format!("Proposal {} not approved", proposal_id)))?;

        if amount > approved_grant.amount {
            return Err(StateError::IoError(format!(
                "Requested amount {} exceeds approved amount {}",
                amount, approved_grant.amount
            )));
        }

        if amount > self.balance {
            return Err(StateError::IoError(format!(
                "Insufficient treasury balance: requested {}, available {}",
                amount, self.balance
            )));
        }

        // Deduct from balance
        self.balance = self
            .balance
            .checked_sub(amount)
            .ok_or_else(|| StateError::IoError("Treasury balance underflow".to_string()))?;

        self.total_distributed = self.total_distributed.checked_add(amount).ok_or_else(|| {
            StateError::IoError("Treasury total distribution overflow".to_string())
        })?;

        // Record withdrawal
        self.withdrawals.push(TreasuryWithdrawal {
            proposal_id: proposal_id.clone(),
            amount,
            recipient,
            block_number,
            timestamp,
        });

        // Update or remove approved amount
        let remaining = approved_grant
            .amount
            .checked_sub(amount)
            .ok_or_else(|| StateError::IoError("Approved amount underflow".to_string()))?;

        if remaining == 0 {
            self.approved_proposals.remove(&proposal_id);
        } else {
            let updated_grant = ApprovedGrant {
                proposal_id: proposal_id.clone(),
                amount: remaining,
                approved_at: approved_grant.approved_at,
            };
            self.approved_proposals.insert(proposal_id, updated_grant);
        }

        Ok(())
    }

    /// Get statistics
    pub fn get_stats(&self) -> TreasuryStats {
        TreasuryStats {
            balance: self.balance,
            total_allocated: self.total_allocated,
            total_distributed: self.total_distributed,
            allocation_count: self.allocations.len(),
            withdrawal_count: self.withdrawals.len(),
            pending_proposals: self.approved_proposals.len(),
        }
    }

    /// Set block reward percentage (for governance parameter changes)
    pub fn set_block_reward_percentage(&mut self, percentage: u64) -> Result<(), StateError> {
        if percentage > 100 {
            return Err(StateError::IoError(
                "Percentage cannot exceed 100".to_string(),
            ));
        }
        self.block_reward_percentage = percentage;
        Ok(())
    }

    /// Set fee percentage (for governance parameter changes)
    pub fn set_fee_percentage(&mut self, percentage: u64) -> Result<(), StateError> {
        if percentage > 100 {
            return Err(StateError::IoError(
                "Percentage cannot exceed 100".to_string(),
            ));
        }
        self.fee_percentage = percentage;
        Ok(())
    }

    /// Check if a proposal has been executed (by checking if it has a withdrawal record)
    pub fn is_proposal_executed(&self, proposal_id: &str) -> bool {
        self.withdrawals
            .iter()
            .any(|w| w.proposal_id == proposal_id)
    }

    /// Get all approved grants (for block producer)
    pub fn get_approved_grants(&self) -> Vec<ApprovedGrant> {
        self.approved_proposals.values().cloned().collect()
    }

    /// Get approved proposal amount (None if not approved or already executed)
    pub fn get_approved_amount(&self, proposal_id: &str) -> Option<u64> {
        self.approved_proposals.get(proposal_id).map(|g| g.amount)
    }

    /// Remove an approved proposal (for cleanup of expired proposals)
    /// This should only be called by governance logic when a proposal expires
    pub fn remove_approved_proposal(&mut self, proposal_id: &str) -> Result<(), StateError> {
        if self.approved_proposals.remove(proposal_id).is_some() {
            Ok(())
        } else {
            Err(StateError::IoError(format!(
                "Proposal {} is not in approved proposals",
                proposal_id
            )))
        }
    }
}

impl Default for Treasury {
    fn default() -> Self {
        Self::new()
    }
}

/// Treasury statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryStats {
    pub balance: u64,
    pub total_allocated: u64,
    pub total_distributed: u64,
    pub allocation_count: usize,
    pub withdrawal_count: usize,
    pub pending_proposals: usize,
}

/// Main blockchain state
#[derive(Debug, Clone)]
pub struct BlockchainState {
    /// Current UTXO set
    utxo_set: UTXOSet,

    /// All blocks by hash
    blocks: HashMap<String, Block>,

    /// Block hash by height
    blocks_by_height: HashMap<u64, String>,

    /// Current chain tip (best block height)
    chain_tip_height: u64,

    /// Database for persistence
    db: crate::db::BlockchainDB,

    /// Current chain tip hash
    chain_tip_hash: String,

    /// Registered masternodes
    masternodes: HashMap<String, MasternodeInfo>,

    /// Track invalidated transactions for wallet notification
    invalidated_transactions: Arc<RwLock<Vec<TxInvalidationEvent>>>,

    /// Current masternode counts by tier
    masternode_counts: MasternodeCounts,

    /// Genesis block hash
    genesis_hash: String,

    /// Protocol-managed treasury (no wallet/private key)
    treasury: Treasury,
}

impl BlockchainState {
    /// Create a new blockchain state with genesis block
    pub fn new(genesis_block: Block, db_path: &str) -> Result<Self, StateError> {
        let db = crate::db::BlockchainDB::open(db_path)?;
        let existing_blocks = db.load_all_blocks()?;

        // Validate genesis block matches if we have existing blocks
        if !existing_blocks.is_empty() {
            let first_block = &existing_blocks[0];
            if first_block.hash != genesis_block.hash {
                eprintln!(
                    "⚠️  Genesis block mismatch detected!\n   Expected: {}...\n   Found:    {}...",
                    &genesis_block.hash[..16],
                    &first_block.hash[..16]
                );
                eprintln!("   Rebuilding blockchain from new genesis block...");

                // Clear the database
                db.clear_all()?;
            }
        }

        let mut state = Self {
            utxo_set: UTXOSet::new(),
            blocks: HashMap::new(),
            blocks_by_height: HashMap::new(),
            chain_tip_height: 0,
            chain_tip_hash: genesis_block.hash.clone(),
            masternodes: HashMap::new(),
            masternode_counts: MasternodeCounts {
                free: 0,
                bronze: 0,
                silver: 0,
                gold: 0,
            },
            genesis_hash: genesis_block.hash.clone(),
            db,
            invalidated_transactions: Arc::new(RwLock::new(Vec::new())),
            treasury: Treasury::new(),
        };

        // Load blocks from disk if they match the genesis
        let existing_blocks = state.db.load_all_blocks()?;

        if existing_blocks.is_empty() {
            genesis_block.validate_structure()?;
            for tx in &genesis_block.transactions {
                state.utxo_set.apply_transaction(tx)?;
            }
            state
                .blocks
                .insert(genesis_block.hash.clone(), genesis_block.clone());
            state.blocks_by_height.insert(0, genesis_block.hash.clone());
            state.db.save_block(&genesis_block)?;
            eprintln!(
                "✅ Genesis block initialized: {}...",
                &genesis_block.hash[..16]
            );
        } else {
            eprintln!("🔍 Validating blockchain on startup...");
            let validation_start = std::time::Instant::now();

            // Perform comprehensive validation
            let mut validated_blocks = Vec::new();
            for block in existing_blocks {
                validated_blocks.push(block);
            }

            // Validate the entire chain
            match Self::validate_blockchain_integrity(&validated_blocks) {
                Ok(()) => {
                    eprintln!(
                        "✅ Blockchain validation passed ({} blocks verified in {:?})",
                        validated_blocks.len(),
                        validation_start.elapsed()
                    );

                    // Apply validated blocks to state
                    for block in validated_blocks {
                        for tx in &block.transactions {
                            state.utxo_set.apply_transaction(tx)?;
                        }
                        state.chain_tip_height = block.header.block_number;
                        state.chain_tip_hash = block.hash.clone();
                        state.blocks.insert(block.hash.clone(), block.clone());
                        state
                            .blocks_by_height
                            .insert(block.header.block_number, block.hash.clone());
                    }
                }
                Err(e) => {
                    eprintln!("❌ Blockchain validation failed: {}", e);
                    eprintln!("   Rebuilding blockchain from genesis...");

                    // Clear corrupted data
                    state.db.clear_all()?;

                    // Initialize with genesis
                    genesis_block.validate_structure()?;
                    for tx in &genesis_block.transactions {
                        state.utxo_set.apply_transaction(tx)?;
                    }
                    state
                        .blocks
                        .insert(genesis_block.hash.clone(), genesis_block.clone());
                    state.blocks_by_height.insert(0, genesis_block.hash.clone());
                    state.db.save_block(&genesis_block)?;
                    eprintln!(
                        "✅ Genesis block re-initialized: {}...",
                        &genesis_block.hash[..16]
                    );
                }
            }

            eprintln!(
                "✅ Blockchain ready: {} blocks (genesis: {}...)",
                state.blocks.len(),
                &state.genesis_hash[..16]
            );
        }

        // Load UTXO snapshot to restore finalized transactions not yet in blocks
        eprintln!("🔍 Checking for UTXO snapshot...");
        if let Err(e) = state.load_and_merge_utxo_snapshot() {
            eprintln!("⚠️  Failed to load UTXO snapshot: {}", e);
            eprintln!("   Continuing with UTXO set from blocks only");
        }

        Ok(state)
    }

    /// Validate the integrity of an entire blockchain
    /// Verifies: block structure, merkle roots, coinbase transactions, block linkage, and hashes
    fn validate_blockchain_integrity(blocks: &[Block]) -> Result<(), StateError> {
        if blocks.is_empty() {
            return Ok(());
        }

        eprintln!("   📋 Validating {} blocks...", blocks.len());

        // Validate genesis block (block 0)
        let genesis = &blocks[0];
        if genesis.header.block_number != 0 {
            return Err(StateError::ChainValidationFailed(
                "First block is not genesis (height != 0)".to_string(),
            ));
        }

        genesis.validate_structure()?;
        eprintln!("   ✓ Genesis block valid");

        // Validate each subsequent block
        for i in 1..blocks.len() {
            let block = &blocks[i];
            let prev_block = &blocks[i - 1];

            // 1. Validate block structure (merkle root, coinbase, hash)
            block.validate_structure().map_err(|e| {
                StateError::ChainValidationFailed(format!(
                    "Block {} structure invalid: {}",
                    block.header.block_number, e
                ))
            })?;

            // 2. Validate block height is sequential
            if block.header.block_number != prev_block.header.block_number + 1 {
                return Err(StateError::ChainValidationFailed(format!(
                    "Block height gap: expected {}, got {}",
                    prev_block.header.block_number + 1,
                    block.header.block_number
                )));
            }

            // 3. Validate previous hash linkage
            if block.header.previous_hash != prev_block.hash {
                return Err(StateError::ChainValidationFailed(format!(
                    "Block {} previous hash mismatch: expected {}..., got {}...",
                    block.header.block_number,
                    &prev_block.hash[..16],
                    &block.header.previous_hash[..16]
                )));
            }

            // 4. Validate coinbase transaction exists and is first
            if block.transactions.is_empty() {
                return Err(StateError::ChainValidationFailed(format!(
                    "Block {} has no transactions",
                    block.header.block_number
                )));
            }

            if !block.transactions[0].is_coinbase() {
                return Err(StateError::ChainValidationFailed(format!(
                    "Block {} first transaction is not coinbase",
                    block.header.block_number
                )));
            }

            // 5. Verify merkle root
            let calculated_merkle = block.calculate_merkle_root();
            if calculated_merkle != block.header.merkle_root {
                return Err(StateError::ChainValidationFailed(format!(
                    "Block {} merkle root mismatch: expected {}..., got {}...",
                    block.header.block_number,
                    &calculated_merkle[..16],
                    &block.header.merkle_root[..16]
                )));
            }

            // Log progress every 100 blocks
            if i % 100 == 0 {
                eprintln!("   ✓ Validated {} blocks...", i);
            }
        }

        eprintln!("   ✓ All blocks validated successfully");
        Ok(())
    }
    /// Get all registered masternodes
    pub fn get_all_masternodes(&self) -> Vec<&MasternodeInfo> {
        self.masternodes.values().collect()
    }

    /// Get current chain tip height
    pub fn chain_tip_height(&self) -> u64 {
        self.chain_tip_height
    }

    /// Get current chain tip hash
    pub fn chain_tip_hash(&self) -> &str {
        &self.chain_tip_hash
    }

    /// Get genesis hash
    pub fn genesis_hash(&self) -> &str {
        &self.genesis_hash
    }

    /// Get block by hash
    pub fn get_block(&self, hash: &str) -> Option<&Block> {
        self.blocks.get(hash)
    }

    /// Get block by height
    pub fn get_block_by_height(&self, height: u64) -> Option<&Block> {
        self.blocks_by_height
            .get(&height)
            .and_then(|hash| self.blocks.get(hash))
    }

    /// Check if block exists
    pub fn has_block(&self, hash: &str) -> bool {
        self.blocks.contains_key(hash)
    }

    /// Get balance for an address
    pub fn get_balance(&self, address: &str) -> u64 {
        self.utxo_set.get_balance(address)
    }

    /// Get UTXO set reference
    pub fn utxo_set(&self) -> &UTXOSet {
        &self.utxo_set
    }

    /// Get mutable UTXO set reference (for applying finalized transactions)
    pub fn utxo_set_mut(&mut self) -> &mut UTXOSet {
        &mut self.utxo_set
    }

    /// Get masternode counts
    pub fn masternode_counts(&self) -> &MasternodeCounts {
        &self.masternode_counts
    }

    /// Get total supply
    pub fn total_supply(&self) -> u64 {
        self.utxo_set.total_supply()
    }

    /// Save current UTXO state to disk for persistence between blocks
    /// This allows finalized transactions to persist across restarts
    pub fn save_utxo_snapshot(&self) -> Result<(), StateError> {
        self.db.save_utxo_snapshot(&self.utxo_set)?;
        eprintln!("💾 UTXO state snapshot saved to disk");
        Ok(())
    }

    /// Get database path (for storing related files)
    pub fn db_path(&self) -> String {
        self.db.path().to_string()
    }

    /// Save a finalized transaction to database
    pub fn save_finalized_tx(
        &self,
        tx: &Transaction,
        votes: usize,
        total: usize,
    ) -> Result<(), StateError> {
        self.db.save_finalized_tx(tx, votes, total)
    }

    /// Remove a finalized transaction (when it's been included in a block)
    pub fn remove_finalized_tx(&self, txid: &str) -> Result<(), StateError> {
        self.db.remove_finalized_tx(txid)
    }

    /// Load all finalized transactions from database
    pub fn load_finalized_txs(&self) -> Result<Vec<Transaction>, StateError> {
        self.db.load_finalized_txs()
    }

    /// Load UTXO state from disk snapshot and merge with blockchain state
    /// This restores finalized transactions that aren't yet in blocks
    pub fn load_and_merge_utxo_snapshot(&mut self) -> Result<(), StateError> {
        if let Some(snapshot) = self.db.load_utxo_snapshot()? {
            // The snapshot contains UTXOs from finalized transactions
            // that aren't in blocks yet, plus UTXOs from blocks that existed
            // when the snapshot was saved.
            // We merge (not replace) to preserve UTXOs from blocks that were
            // added after the snapshot was saved.
            eprintln!("📥 Loading UTXO snapshot from disk...");

            // Merge snapshot into current UTXO set
            // Current set (from blocks) takes precedence over snapshot for duplicate UTXOs
            self.utxo_set.merge_snapshot(snapshot);
            eprintln!("✅ UTXO snapshot merged - finalized transactions restored");
            Ok(())
        } else {
            eprintln!("ℹ️  No UTXO snapshot found");
            Ok(())
        }
    }

    /// Add a new block to the chain
    pub fn add_block(&mut self, block: Block) -> Result<(), StateError> {
        if self.has_block(&block.hash) {
            return Err(StateError::DuplicateBlock);
        }
        block.validate_structure()?;
        if block.header.block_number == 0 {
            return Err(StateError::InvalidBlockHeight);
        }
        if !self.has_block(&block.header.previous_hash) {
            return Err(StateError::OrphanBlock);
        }
        if block.header.block_number != self.chain_tip_height + 1 {
            return Err(StateError::InvalidBlockHeight);
        }
        if block.header.previous_hash != self.chain_tip_hash {
            return Err(StateError::InvalidPreviousHash);
        }

        let utxo_snapshot = self.utxo_set.snapshot();

        match block.validate_and_apply(&mut self.utxo_set, &self.masternode_counts) {
            Ok(_) => {
                // Extract and allocate treasury funds from coinbase transaction
                let timestamp = block.header.timestamp.timestamp();
                let block_number = block.header.block_number;

                // Find treasury allocation in coinbase transaction (marked with "TREASURY" address)
                if let Some(coinbase) = block.coinbase() {
                    if let Some(treasury_output) =
                        coinbase.outputs.iter().find(|o| o.address == "TREASURY")
                    {
                        // Allocate treasury funds from the coinbase output
                        // This represents 10% of total block rewards (base rewards + fees)
                        self.treasury.allocate_direct(
                            block_number,
                            treasury_output.amount,
                            TreasurySource::BlockReward,
                            timestamp,
                        )?;
                    }
                }

                // Process treasury grant transactions
                for tx in &block.transactions {
                    if tx.is_treasury_grant() {
                        // Extract proposal ID from the transaction
                        let proposal_id = tx.treasury_grant_proposal_id().ok_or_else(|| {
                            StateError::IoError(
                                "Invalid treasury grant transaction format".to_string(),
                            )
                        })?;

                        // Validate transaction structure for treasury grants
                        if tx.outputs.len() != 1 {
                            return Err(StateError::IoError(
                                "Treasury grant must have exactly one output".to_string(),
                            ));
                        }

                        let output = &tx.outputs[0];
                        let recipient = &output.address;
                        let amount = output.amount;

                        // Check if proposal has already been executed (double execution prevention)
                        if self.treasury.is_proposal_executed(&proposal_id) {
                            return Err(StateError::IoError(format!(
                                "Treasury grant for proposal {} has already been executed",
                                proposal_id
                            )));
                        }

                        // Validate against approved proposal
                        let approved_amount = self
                            .treasury
                            .get_approved_amount(&proposal_id)
                            .ok_or_else(|| {
                                StateError::IoError(format!(
                                    "Treasury grant for proposal {} is not approved",
                                    proposal_id
                                ))
                            })?;

                        // Validate amount matches approved amount
                        if amount != approved_amount {
                            return Err(StateError::IoError(format!(
                                "Treasury grant amount {} does not match approved amount {}",
                                amount, approved_amount
                            )));
                        }

                        // Execute the distribution through the treasury
                        self.treasury.distribute(
                            proposal_id,
                            recipient.clone(),
                            amount,
                            block_number,
                            timestamp,
                        )?;
                    }
                }

                self.db.save_block(&block)?;
                self.blocks_by_height
                    .insert(block.header.block_number, block.hash.clone());
                self.chain_tip_height = block.header.block_number;
                self.chain_tip_hash = block.hash.clone();
                self.blocks.insert(block.hash.clone(), block);
                Ok(())
            }
            Err(e) => {
                self.utxo_set.restore(utxo_snapshot);
                Err(e.into())
            }
        }
    }

    /// Register a new masternode
    pub fn register_masternode(
        &mut self,
        address: String,
        tier: MasternodeTier,
        collateral_tx: String,
        wallet_address: String,
    ) -> Result<(), StateError> {
        // For non-Free tiers, validate collateral UTXO
        if tier != MasternodeTier::Free {
            let required_collateral = tier.collateral_requirement();

            // Parse collateral transaction (format: "txid:vout")
            if collateral_tx.contains(':') {
                let parts: Vec<&str> = collateral_tx.split(':').collect();
                if parts.len() == 2 {
                    let txid = parts[0].to_string();
                    let vout: u32 = parts[1].parse().unwrap_or(0);

                    // Create OutPoint and verify UTXO exists
                    let outpoint = crate::transaction::OutPoint::new(txid, vout);
                    let utxo_valid = self
                        .utxo_set
                        .get(&outpoint)
                        .map(|utxo| utxo.amount >= required_collateral)
                        .unwrap_or(false);

                    if !utxo_valid {
                        return Err(StateError::InvalidMasternodeCount); // TODO: Add proper error type
                    }

                    // TODO: Lock the UTXO so it cannot be spent while masternode is active
                    // This will require tracking locked UTXOs in the state
                } else {
                    return Err(StateError::InvalidMasternodeCount);
                }
            } else {
                return Err(StateError::InvalidMasternodeCount);
            }
        }

        let masternode = MasternodeInfo {
            address: address.clone(),
            tier,
            collateral_tx,
            registered_height: self.chain_tip_height,
            last_seen: chrono::Utc::now().timestamp(),
            is_active: true,
            wallet_address,
        };

        match tier {
            MasternodeTier::Free => self.masternode_counts.free += 1,
            MasternodeTier::Bronze => self.masternode_counts.bronze += 1,
            MasternodeTier::Silver => self.masternode_counts.silver += 1,
            MasternodeTier::Gold => self.masternode_counts.gold += 1,
        }

        self.masternodes.insert(address, masternode);
        Ok(())
    }

    /// Deactivate a masternode
    pub fn deactivate_masternode(&mut self, address: &str) -> Result<(), StateError> {
        let masternode = self
            .masternodes
            .get_mut(address)
            .ok_or(StateError::InvalidMasternodeCount)?;

        if !masternode.is_active {
            return Ok(());
        }

        masternode.is_active = false;

        match masternode.tier {
            MasternodeTier::Free => {
                self.masternode_counts.free = self.masternode_counts.free.saturating_sub(1)
            }
            MasternodeTier::Bronze => {
                self.masternode_counts.bronze = self.masternode_counts.bronze.saturating_sub(1)
            }
            MasternodeTier::Silver => {
                self.masternode_counts.silver = self.masternode_counts.silver.saturating_sub(1)
            }
            MasternodeTier::Gold => {
                self.masternode_counts.gold = self.masternode_counts.gold.saturating_sub(1)
            }
        }

        Ok(())
    }

    /// Get masternode info
    pub fn get_masternode(&self, address: &str) -> Option<&MasternodeInfo> {
        self.masternodes.get(address)
    }

    /// Get all active masternodes
    pub fn get_active_masternodes(&self) -> Vec<&MasternodeInfo> {
        self.masternodes
            .values()
            .filter(|mn| mn.is_active)
            .collect()
    }

    /// Get masternodes by tier
    pub fn get_masternodes_by_tier(&self, tier: MasternodeTier) -> Vec<&MasternodeInfo> {
        self.masternodes
            .values()
            .filter(|mn| mn.is_active && mn.tier == tier)
            .collect()
    }

    /// Validate a transaction
    pub fn validate_transaction(&self, tx: &Transaction) -> Result<(), StateError> {
        tx.validate_structure()?;
        if tx.is_coinbase() {
            return Ok(());
        }
        for input in &tx.inputs {
            if !self.utxo_set.contains(&input.previous_output) {
                return Err(StateError::TransactionError(TransactionError::InvalidInput));
            }
        }
        let _fee = tx.fee(self.utxo_set.utxos())?;
        Ok(())
    }

    /// Get chain statistics
    pub fn get_stats(&self) -> ChainStats {
        ChainStats {
            chain_height: self.chain_tip_height,
            total_blocks: self.blocks.len(),
            total_supply: self.total_supply(),
            utxo_count: self.utxo_set.len(),
            active_masternodes: self.masternode_counts.total(),
            free_masternodes: self.masternode_counts.free,
            bronze_masternodes: self.masternode_counts.bronze,
            silver_masternodes: self.masternode_counts.silver,
            gold_masternodes: self.masternode_counts.gold,
            treasury_balance: self.treasury.balance(),
            treasury_total_allocated: self.treasury.total_allocated(),
            treasury_total_distributed: self.treasury.total_distributed(),
        }
    }

    /// Replace a block at a specific height
    pub fn replace_block(&mut self, height: u64, new_block: Block) -> Result<(), StateError> {
        new_block.validate_structure()?;
        if let Some(old_hash) = self.blocks_by_height.get(&height) {
            let old_hash = old_hash.clone();
            self.blocks.remove(&old_hash);
            if self.chain_tip_hash == old_hash {
                self.chain_tip_hash = new_block.hash.clone();
            }
            self.blocks_by_height.insert(height, new_block.hash.clone());
            self.blocks
                .insert(new_block.hash.clone(), new_block.clone());
            self.db.save_block(&new_block)?;
            Ok(())
        } else {
            Err(StateError::BlockNotFound)
        }
    }

    /// Process orphaned transaction
    pub fn process_orphaned_transaction(&mut self, tx: Transaction) -> Result<bool, StateError> {
        match self.validate_transaction(&tx) {
            Ok(_) => Ok(true),
            Err(e) => {
                let event = TxInvalidationEvent {
                    txid: tx.txid.clone(),
                    reason: format!("Chain fork: {}", e),
                    timestamp: chrono::Utc::now().timestamp(),
                    affected_addresses: self.get_affected_addresses(&tx),
                };
                self.invalidated_transactions
                    .write()
                    .unwrap()
                    .push(event.clone());
                println!("   ❌ Transaction {} invalidated: {}", tx.txid, e);
                Ok(false)
            }
        }
    }

    fn get_address_for_utxo(&self, outpoint: &OutPoint) -> Option<String> {
        self.utxo_set.get(outpoint).map(|u| u.address.clone())
    }

    fn get_affected_addresses(&self, tx: &Transaction) -> Vec<String> {
        let mut addresses = Vec::new();
        for input in &tx.inputs {
            if let Some(addr) = self.get_address_for_utxo(&input.previous_output) {
                addresses.push(addr);
            }
        }
        for output in &tx.outputs {
            addresses.push(output.address.clone());
        }
        addresses
    }

    pub fn get_invalidated_txs_for_address(&self, address: &str) -> Vec<TxInvalidationEvent> {
        self.invalidated_transactions
            .read()
            .unwrap()
            .iter()
            .filter(|event| event.affected_addresses.contains(&address.to_string()))
            .cloned()
            .collect()
    }

    /// Get treasury reference
    pub fn treasury(&self) -> &Treasury {
        &self.treasury
    }

    /// Get treasury statistics
    pub fn treasury_stats(&self) -> TreasuryStats {
        self.treasury.get_stats()
    }

    /// Approve a governance proposal for treasury spending
    pub fn approve_treasury_proposal(
        &mut self,
        proposal_id: String,
        amount: u64,
    ) -> Result<(), StateError> {
        self.treasury.approve_proposal(proposal_id, amount)
    }

    /// Distribute treasury funds for an approved proposal
    /// This should only be called after governance consensus is reached
    pub fn distribute_treasury_funds(
        &mut self,
        proposal_id: String,
        recipient: String,
        amount: u64,
    ) -> Result<(), StateError> {
        let timestamp = chrono::Utc::now().timestamp();
        self.treasury.distribute(
            proposal_id,
            recipient,
            amount,
            self.chain_tip_height,
            timestamp,
        )
    }

    /// Get all approved treasury grants (for block producer)
    pub fn get_approved_treasury_grants(&self) -> Vec<ApprovedGrant> {
        self.treasury.get_approved_grants()
    }

    /// Remove an expired approved proposal from treasury
    /// This should be called by governance logic when a proposal expires
    pub fn cleanup_expired_treasury_proposal(
        &mut self,
        proposal_id: &str,
    ) -> Result<(), StateError> {
        self.treasury.remove_approved_proposal(proposal_id)
    }
}

/// Chain statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStats {
    pub chain_height: u64,
    pub total_blocks: usize,
    pub total_supply: u64,
    pub utxo_count: usize,
    pub active_masternodes: u64,
    pub free_masternodes: u64,
    pub bronze_masternodes: u64,
    pub silver_masternodes: u64,
    pub gold_masternodes: u64,
    pub treasury_balance: u64,
    pub treasury_total_allocated: u64,
    pub treasury_total_distributed: u64,
}

/// Test module
#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::TxOutput;

    fn create_genesis_block() -> Block {
        let outputs = vec![TxOutput::new(100_000_000_000, "genesis".to_string())];
        Block::new(0, "0".repeat(64), "genesis_validator".to_string(), outputs)
    }

    #[test]
    fn test_blockchain_initialization() {
        let genesis = create_genesis_block();
        let genesis_hash = genesis.hash.clone();
        let db_dir = std::env::temp_dir().join(format!(
            "time_coin_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path = db_dir.to_str().unwrap().to_string();
        let _ = std::fs::remove_dir_all(&db_path);
        let state = BlockchainState::new(genesis, &db_path).unwrap();
        assert_eq!(state.chain_tip_height(), 0);
        assert_eq!(state.chain_tip_hash(), genesis_hash);
        assert_eq!(state.total_supply(), 100_000_000_000);
    }

    #[test]
    fn test_add_block() {
        let genesis = create_genesis_block();
        let genesis_hash = genesis.hash.clone();
        let db_dir = std::env::temp_dir().join(format!(
            "time_coin_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path = db_dir.to_str().unwrap().to_string();
        let _ = std::fs::remove_dir_all(&db_path);
        let mut state = BlockchainState::new(genesis, &db_path).unwrap();

        // Register a Free tier masternode (no collateral required)
        state
            .register_masternode(
                "node1".to_string(),
                MasternodeTier::Free,
                "collateral_tx_1".to_string(),
                "miner1".to_string(),
            )
            .unwrap();

        // Calculate the expected masternode reward for 1 Free node
        let counts = MasternodeCounts {
            free: 1,
            bronze: 0,
            silver: 0,
            gold: 0,
        };
        let base_reward = crate::block::calculate_total_masternode_reward(&counts);

        // Calculate total rewards and split properly
        let total_rewards = base_reward;
        let treasury_amount = crate::block::calculate_treasury_allocation(total_rewards);
        let masternode_share = crate::block::calculate_masternode_share(total_rewards);

        // Create block with proper treasury split
        let outputs = vec![
            TxOutput::new(treasury_amount, "TREASURY".to_string()),
            TxOutput::new(masternode_share, "miner1".to_string()),
        ];
        let block1 = Block::new(1, genesis_hash.clone(), "miner1".to_string(), outputs);
        state.add_block(block1).unwrap();
        assert_eq!(state.chain_tip_height(), 1);
        assert!(state.get_block_by_height(1).is_some());
    }

    #[test]
    fn test_masternode_registration() {
        let genesis = create_genesis_block();
        let db_dir = std::env::temp_dir().join(format!(
            "time_coin_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path = db_dir.to_str().unwrap().to_string();
        let _ = std::fs::remove_dir_all(&db_path);
        let mut state = BlockchainState::new(genesis, &db_path).unwrap();
        state
            .register_masternode(
                "masternode1".to_string(),
                MasternodeTier::Free,
                "collateral_tx".to_string(),
                "wallet_address".to_string(),
            )
            .unwrap();
        assert_eq!(state.masternode_counts().free, 1);
        assert!(state.get_masternode("masternode1").is_some());
    }

    #[test]
    fn test_get_balance() {
        let genesis = create_genesis_block();
        let db_dir = std::env::temp_dir().join(format!(
            "time_coin_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path = db_dir.to_str().unwrap().to_string();
        let _ = std::fs::remove_dir_all(&db_path);
        let state = BlockchainState::new(genesis, &db_path).unwrap();
        assert_eq!(state.get_balance("genesis"), 100_000_000_000);
        assert_eq!(state.get_balance("nonexistent"), 0);
    }

    #[test]
    fn test_empty_transactions_block_rejected() {
        use crate::block::BlockHeader;
        use chrono::Utc;

        // Create a block with no transactions (invalid)
        let invalid_block = Block {
            header: BlockHeader {
                block_number: 0,
                timestamp: Utc::now(),
                previous_hash: "0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
                merkle_root: String::new(),
                validator_signature: "genesis".to_string(),
                validator_address: "genesis".to_string(),
            },
            transactions: vec![], // Empty transactions - invalid!
            hash: "invalid".to_string(),
        };

        let db_dir = std::env::temp_dir().join(format!(
            "time_coin_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path = db_dir.to_str().unwrap().to_string();
        let _ = std::fs::remove_dir_all(&db_path);

        // Should fail with BlockError(NoTransactions) converted to StateError
        let result = BlockchainState::new(invalid_block, &db_path);
        assert!(result.is_err());

        // Verify it's the right error
        if let Err(e) = result {
            let error_msg = format!("{:?}", e);
            assert!(
                error_msg.contains("NoTransactions") || error_msg.contains("BlockError"),
                "Expected NoTransactions error, got: {}",
                error_msg
            );
        }
    }

    #[test]
    fn test_treasury_initialization() {
        let genesis = create_genesis_block();
        let db_dir = std::env::temp_dir().join(format!(
            "time_coin_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path = db_dir.to_str().unwrap().to_string();
        let _ = std::fs::remove_dir_all(&db_path);
        let state = BlockchainState::new(genesis, &db_path).unwrap();

        // Treasury should be initialized with zero balance
        assert_eq!(state.treasury().balance(), 0);
        assert_eq!(state.treasury().total_allocated(), 0);
        assert_eq!(state.treasury().total_distributed(), 0);
    }

    #[test]
    fn test_treasury_allocation_from_block() {
        let genesis = create_genesis_block();
        let genesis_hash = genesis.hash.clone();
        let db_dir = std::env::temp_dir().join(format!(
            "time_coin_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path = db_dir.to_str().unwrap().to_string();
        let _ = std::fs::remove_dir_all(&db_path);
        let mut state = BlockchainState::new(genesis, &db_path).unwrap();

        // Register a Free tier masternode
        state
            .register_masternode(
                "node1".to_string(),
                MasternodeTier::Free,
                "collateral_tx_1".to_string(),
                "miner1".to_string(),
            )
            .unwrap();

        let counts = MasternodeCounts {
            free: 1,
            bronze: 0,
            silver: 0,
            gold: 0,
        };
        let base_reward = crate::block::calculate_total_masternode_reward(&counts);

        // Calculate treasury allocation (10% of total)
        let total_rewards = base_reward;
        let treasury_allocation = crate::block::calculate_treasury_allocation(total_rewards);
        let masternode_share = crate::block::calculate_masternode_share(total_rewards);

        // Create block with treasury split
        let outputs = vec![
            TxOutput::new(treasury_allocation, "TREASURY".to_string()),
            TxOutput::new(masternode_share, "miner1".to_string()),
        ];
        let block1 = Block::new(1, genesis_hash.clone(), "miner1".to_string(), outputs);

        state.add_block(block1).unwrap();

        // Check treasury received allocation (10% of total reward)
        assert_eq!(state.treasury().balance(), treasury_allocation);
        assert_eq!(state.treasury().total_allocated(), treasury_allocation);
        assert_eq!(state.treasury().allocations().len(), 1);
    }

    #[test]
    fn test_treasury_proposal_approval_and_distribution() {
        let genesis = create_genesis_block();
        let genesis_hash = genesis.hash.clone();
        let db_dir = std::env::temp_dir().join(format!(
            "time_coin_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path = db_dir.to_str().unwrap().to_string();
        let _ = std::fs::remove_dir_all(&db_path);
        let mut state = BlockchainState::new(genesis, &db_path).unwrap();

        // Register masternode and add block to fund treasury
        state
            .register_masternode(
                "node1".to_string(),
                MasternodeTier::Free,
                "collateral_tx_1".to_string(),
                "miner1".to_string(),
            )
            .unwrap();

        let counts = MasternodeCounts {
            free: 1,
            bronze: 0,
            silver: 0,
            gold: 0,
        };
        let base_reward = crate::block::calculate_total_masternode_reward(&counts);
        let total_rewards = base_reward;
        let treasury_allocation = crate::block::calculate_treasury_allocation(total_rewards);
        let masternode_share = crate::block::calculate_masternode_share(total_rewards);

        let outputs = vec![
            TxOutput::new(treasury_allocation, "TREASURY".to_string()),
            TxOutput::new(masternode_share, "miner1".to_string()),
        ];
        let block1 = Block::new(1, genesis_hash.clone(), "miner1".to_string(), outputs);
        state.add_block(block1).unwrap();

        let treasury_balance = state.treasury().balance();
        assert!(treasury_balance > 0);

        // Approve a proposal
        let proposal_amount = treasury_balance / 2;
        state
            .approve_treasury_proposal("proposal-1".to_string(), proposal_amount)
            .unwrap();

        // Distribute funds
        state
            .distribute_treasury_funds(
                "proposal-1".to_string(),
                "recipient".to_string(),
                proposal_amount,
            )
            .unwrap();

        // Check treasury balance decreased
        assert_eq!(
            state.treasury().balance(),
            treasury_balance - proposal_amount
        );
        assert_eq!(state.treasury().total_distributed(), proposal_amount);
        assert_eq!(state.treasury().withdrawals().len(), 1);
    }

    #[test]
    fn test_treasury_insufficient_balance() {
        let genesis = create_genesis_block();
        let db_dir = std::env::temp_dir().join(format!(
            "time_coin_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path = db_dir.to_str().unwrap().to_string();
        let _ = std::fs::remove_dir_all(&db_path);
        let mut state = BlockchainState::new(genesis, &db_path).unwrap();

        // Try to approve proposal with more than treasury balance
        let result = state.approve_treasury_proposal("proposal-1".to_string(), 1_000_000);
        assert!(result.is_err());
    }

    #[test]
    fn test_treasury_unapproved_distribution() {
        let genesis = create_genesis_block();
        let db_dir = std::env::temp_dir().join(format!(
            "time_coin_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path = db_dir.to_str().unwrap().to_string();
        let _ = std::fs::remove_dir_all(&db_path);
        let mut state = BlockchainState::new(genesis, &db_path).unwrap();

        // Try to distribute funds without approval
        let result = state.distribute_treasury_funds(
            "unapproved-proposal".to_string(),
            "recipient".to_string(),
            1000,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_treasury_stats() {
        let genesis = create_genesis_block();
        let genesis_hash = genesis.hash.clone();
        let db_dir = std::env::temp_dir().join(format!(
            "time_coin_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path = db_dir.to_str().unwrap().to_string();
        let _ = std::fs::remove_dir_all(&db_path);
        let mut state = BlockchainState::new(genesis, &db_path).unwrap();

        // Register masternode and add block
        state
            .register_masternode(
                "node1".to_string(),
                MasternodeTier::Free,
                "collateral_tx_1".to_string(),
                "miner1".to_string(),
            )
            .unwrap();

        let counts = MasternodeCounts {
            free: 1,
            bronze: 0,
            silver: 0,
            gold: 0,
        };
        let base_reward = crate::block::calculate_total_masternode_reward(&counts);
        let total_rewards = base_reward;
        let treasury_allocation = crate::block::calculate_treasury_allocation(total_rewards);
        let masternode_share = crate::block::calculate_masternode_share(total_rewards);

        let outputs = vec![
            TxOutput::new(treasury_allocation, "TREASURY".to_string()),
            TxOutput::new(masternode_share, "miner1".to_string()),
        ];
        let block1 = Block::new(1, genesis_hash.clone(), "miner1".to_string(), outputs);
        state.add_block(block1).unwrap();

        // Check stats
        let stats = state.treasury_stats();
        assert!(stats.balance > 0);
        assert_eq!(stats.allocation_count, 1);
        assert_eq!(stats.withdrawal_count, 0);
        assert_eq!(stats.pending_proposals, 0);

        // Check chain stats include treasury info
        let chain_stats = state.get_stats();
        assert_eq!(chain_stats.treasury_balance, stats.balance);
        assert_eq!(chain_stats.treasury_total_allocated, stats.total_allocated);
    }

    #[test]
    fn test_treasury_grant_transaction_creation() {
        use crate::transaction::Transaction;

        // Create a treasury grant transaction
        let grant = Transaction::create_treasury_grant(
            "proposal-123".to_string(),
            "recipient_address".to_string(),
            1000000,
            100,
            1234567890,
        );

        // Verify it's identified as a treasury grant
        assert!(grant.is_treasury_grant());
        assert!(!grant.is_coinbase());

        // Verify proposal ID can be extracted
        assert_eq!(
            grant.treasury_grant_proposal_id(),
            Some("proposal-123".to_string())
        );

        // Verify structure
        assert_eq!(grant.inputs.len(), 0);
        assert_eq!(grant.outputs.len(), 1);
        assert_eq!(grant.outputs[0].amount, 1000000);
        assert_eq!(grant.outputs[0].address, "recipient_address");

        // Verify txid format
        assert!(grant.txid.starts_with("treasury_grant_proposal-123_"));
    }

    #[test]
    fn test_treasury_grant_in_block_processing() {
        use crate::transaction::Transaction;

        let genesis = create_genesis_block();
        let genesis_hash = genesis.hash.clone();
        let db_dir = std::env::temp_dir().join(format!(
            "time_coin_test_treasury_grant_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path = db_dir.to_str().unwrap().to_string();
        let _ = std::fs::remove_dir_all(&db_path);
        let mut state = BlockchainState::new(genesis, &db_path).unwrap();

        // Register a masternode to set proper counts
        state
            .register_masternode(
                "node1".to_string(),
                MasternodeTier::Free,
                "collateral1".to_string(),
                "miner1".to_string(),
            )
            .unwrap();

        // Add first block to generate treasury funds
        let counts = MasternodeCounts {
            free: 1,
            bronze: 0,
            silver: 0,
            gold: 0,
        };
        let base_reward = crate::block::calculate_total_masternode_reward(&counts);
        let total_rewards = base_reward;
        let treasury_allocation = crate::block::calculate_treasury_allocation(total_rewards);
        let masternode_share = crate::block::calculate_masternode_share(total_rewards);

        let outputs = vec![
            TxOutput::new(treasury_allocation, "TREASURY".to_string()),
            TxOutput::new(masternode_share, "miner1".to_string()),
        ];
        let block1 = Block::new(1, genesis_hash.clone(), "miner1".to_string(), outputs);
        state.add_block(block1).unwrap();

        // Approve a proposal
        let proposal_id = "test-proposal-1".to_string();
        let recipient = "project_developer".to_string();
        let amount = 100_000_000u64; // 1 TIME

        state
            .approve_treasury_proposal(proposal_id.clone(), amount)
            .unwrap();

        // Create a block with a treasury grant transaction
        let grant_tx = Transaction::create_treasury_grant(
            proposal_id.clone(),
            recipient.clone(),
            amount,
            2,
            1234567890,
        );

        let block_hash = state.chain_tip_hash().to_string();
        // Need to provide valid coinbase outputs (masternode rewards + treasury)
        let coinbase_outputs = vec![
            TxOutput::new(treasury_allocation, "TREASURY".to_string()),
            TxOutput::new(masternode_share, "miner1".to_string()),
        ];
        let mut block2 = Block::new(2, block_hash, "miner1".to_string(), coinbase_outputs);

        // Add the grant transaction to the block
        block2.add_transaction(grant_tx).unwrap();

        // Process the block
        state.add_block(block2).unwrap();

        // Verify the treasury distribution happened
        let stats = state.treasury_stats();
        assert_eq!(stats.withdrawal_count, 1);
        assert_eq!(stats.pending_proposals, 0); // Should be removed after execution

        // Verify the recipient received the funds
        let recipient_balance = state.get_balance(&recipient);
        assert_eq!(recipient_balance, amount);
    }

    #[test]
    fn test_treasury_grant_double_execution_prevention() {
        use crate::transaction::Transaction;

        let genesis = create_genesis_block();
        let genesis_hash = genesis.hash.clone();
        let db_dir = std::env::temp_dir().join(format!(
            "time_coin_test_double_exec_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path = db_dir.to_str().unwrap().to_string();
        let _ = std::fs::remove_dir_all(&db_path);
        let mut state = BlockchainState::new(genesis, &db_path).unwrap();

        // Register a masternode to set proper counts
        state
            .register_masternode(
                "node1".to_string(),
                MasternodeTier::Free,
                "collateral1".to_string(),
                "miner1".to_string(),
            )
            .unwrap();

        // Add first block to generate treasury funds
        let counts = MasternodeCounts {
            free: 1,
            bronze: 0,
            silver: 0,
            gold: 0,
        };
        let base_reward = crate::block::calculate_total_masternode_reward(&counts);
        let total_rewards = base_reward;
        let treasury_allocation = crate::block::calculate_treasury_allocation(total_rewards);
        let masternode_share = crate::block::calculate_masternode_share(total_rewards);

        let outputs = vec![
            TxOutput::new(treasury_allocation, "TREASURY".to_string()),
            TxOutput::new(masternode_share, "miner1".to_string()),
        ];
        let block1 = Block::new(1, genesis_hash.clone(), "miner1".to_string(), outputs);
        state.add_block(block1).unwrap();

        // Approve a proposal
        let proposal_id = "test-proposal-2".to_string();
        let amount = 100_000_000u64;
        state
            .approve_treasury_proposal(proposal_id.clone(), amount)
            .unwrap();

        // Create first block with treasury grant
        let grant_tx1 = Transaction::create_treasury_grant(
            proposal_id.clone(),
            "recipient1".to_string(),
            amount,
            2,
            1234567890,
        );

        let block_hash = state.chain_tip_hash().to_string();
        let coinbase_outputs2 = vec![
            TxOutput::new(treasury_allocation, "TREASURY".to_string()),
            TxOutput::new(masternode_share, "miner1".to_string()),
        ];
        let mut block2 = Block::new(
            2,
            block_hash.clone(),
            "miner1".to_string(),
            coinbase_outputs2,
        );
        block2.add_transaction(grant_tx1).unwrap();
        state.add_block(block2).unwrap();

        // Try to create another block with the same grant (double execution attempt)
        let grant_tx2 = Transaction::create_treasury_grant(
            proposal_id.clone(),
            "recipient1".to_string(),
            amount,
            3,
            1234567891,
        );

        let block_hash2 = state.chain_tip_hash().to_string();
        let coinbase_outputs3 = vec![
            TxOutput::new(treasury_allocation, "TREASURY".to_string()),
            TxOutput::new(masternode_share, "miner1".to_string()),
        ];
        let mut block3 = Block::new(3, block_hash2, "miner1".to_string(), coinbase_outputs3);
        block3.add_transaction(grant_tx2).unwrap();

        // This should fail because the proposal was already executed
        let result = state.add_block(block3);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("already been executed"));
    }

    #[test]
    fn test_treasury_grant_unapproved_proposal() {
        use crate::transaction::Transaction;

        let genesis = create_genesis_block();
        let genesis_hash = genesis.hash.clone();
        let db_dir = std::env::temp_dir().join(format!(
            "time_coin_test_unapproved_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path = db_dir.to_str().unwrap().to_string();
        let _ = std::fs::remove_dir_all(&db_path);
        let mut state = BlockchainState::new(genesis, &db_path).unwrap();

        // Register a masternode to set proper counts
        state
            .register_masternode(
                "node1".to_string(),
                MasternodeTier::Free,
                "collateral1".to_string(),
                "miner1".to_string(),
            )
            .unwrap();

        // Add first block
        let counts = MasternodeCounts {
            free: 1,
            bronze: 0,
            silver: 0,
            gold: 0,
        };
        let base_reward = crate::block::calculate_total_masternode_reward(&counts);
        let total_rewards = base_reward;
        let treasury_allocation = crate::block::calculate_treasury_allocation(total_rewards);
        let masternode_share = crate::block::calculate_masternode_share(total_rewards);

        let outputs = vec![
            TxOutput::new(treasury_allocation, "TREASURY".to_string()),
            TxOutput::new(masternode_share, "miner1".to_string()),
        ];
        let block1 = Block::new(1, genesis_hash.clone(), "miner1".to_string(), outputs);
        state.add_block(block1).unwrap();

        // Try to create a grant for an unapproved proposal
        let grant_tx = Transaction::create_treasury_grant(
            "unapproved-proposal".to_string(),
            "recipient".to_string(),
            1000000,
            2,
            1234567890,
        );

        let block_hash = state.chain_tip_hash().to_string();
        let coinbase_outputs = vec![
            TxOutput::new(treasury_allocation, "TREASURY".to_string()),
            TxOutput::new(masternode_share, "miner1".to_string()),
        ];
        let mut block2 = Block::new(2, block_hash, "miner1".to_string(), coinbase_outputs);
        block2.add_transaction(grant_tx).unwrap();

        // This should fail because the proposal is not approved
        let result = state.add_block(block2);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not approved"));
    }

    #[test]
    fn test_treasury_grant_amount_mismatch() {
        use crate::transaction::Transaction;

        let genesis = create_genesis_block();
        let genesis_hash = genesis.hash.clone();
        let db_dir = std::env::temp_dir().join(format!(
            "time_coin_test_amount_mismatch_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path = db_dir.to_str().unwrap().to_string();
        let _ = std::fs::remove_dir_all(&db_path);
        let mut state = BlockchainState::new(genesis, &db_path).unwrap();

        // Register a masternode to set proper counts
        state
            .register_masternode(
                "node1".to_string(),
                MasternodeTier::Free,
                "collateral1".to_string(),
                "miner1".to_string(),
            )
            .unwrap();

        // Add first block
        let counts = MasternodeCounts {
            free: 1,
            bronze: 0,
            silver: 0,
            gold: 0,
        };
        let base_reward = crate::block::calculate_total_masternode_reward(&counts);
        let total_rewards = base_reward;
        let treasury_allocation = crate::block::calculate_treasury_allocation(total_rewards);
        let masternode_share = crate::block::calculate_masternode_share(total_rewards);

        let outputs = vec![
            TxOutput::new(treasury_allocation, "TREASURY".to_string()),
            TxOutput::new(masternode_share, "miner1".to_string()),
        ];
        let block1 = Block::new(1, genesis_hash.clone(), "miner1".to_string(), outputs);
        state.add_block(block1).unwrap();

        // Approve a proposal with a specific amount
        let proposal_id = "test-proposal-3".to_string();
        let approved_amount = 100_000_000u64;
        state
            .approve_treasury_proposal(proposal_id.clone(), approved_amount)
            .unwrap();

        // Try to create a grant with a different amount
        let wrong_amount = 150_000_000u64;
        let grant_tx = Transaction::create_treasury_grant(
            proposal_id.clone(),
            "recipient".to_string(),
            wrong_amount,
            2,
            1234567890,
        );

        let block_hash = state.chain_tip_hash().to_string();
        let coinbase_outputs = vec![
            TxOutput::new(treasury_allocation, "TREASURY".to_string()),
            TxOutput::new(masternode_share, "miner1".to_string()),
        ];
        let mut block2 = Block::new(2, block_hash, "miner1".to_string(), coinbase_outputs);
        block2.add_transaction(grant_tx).unwrap();

        // This should fail because the amount doesn't match
        let result = state.add_block(block2);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not match"));
    }

    #[test]
    fn test_cleanup_expired_treasury_proposal() {
        let genesis = create_genesis_block();
        let genesis_hash = genesis.hash.clone();
        let db_dir = std::env::temp_dir().join(format!(
            "time_coin_test_cleanup_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path = db_dir.to_str().unwrap().to_string();
        let _ = std::fs::remove_dir_all(&db_path);
        let mut state = BlockchainState::new(genesis, &db_path).unwrap();

        // Register a masternode to set proper counts
        state
            .register_masternode(
                "node1".to_string(),
                MasternodeTier::Free,
                "collateral1".to_string(),
                "miner1".to_string(),
            )
            .unwrap();

        // Add a block to generate treasury funds first
        let counts = MasternodeCounts {
            free: 1,
            bronze: 0,
            silver: 0,
            gold: 0,
        };
        let base_reward = crate::block::calculate_total_masternode_reward(&counts);
        let total_rewards = base_reward;
        let treasury_allocation = crate::block::calculate_treasury_allocation(total_rewards);
        let masternode_share = crate::block::calculate_masternode_share(total_rewards);

        let outputs = vec![
            TxOutput::new(treasury_allocation, "TREASURY".to_string()),
            TxOutput::new(masternode_share, "miner1".to_string()),
        ];
        let block1 = Block::new(1, genesis_hash.clone(), "miner1".to_string(), outputs);
        state.add_block(block1).unwrap();

        // Approve a proposal (should have treasury funds now)
        let proposal_id = "expired-proposal".to_string();
        let amount = 100_000u64; // Small amount
        state
            .approve_treasury_proposal(proposal_id.clone(), amount)
            .unwrap();

        // Verify it's approved
        let stats = state.treasury_stats();
        assert_eq!(stats.pending_proposals, 1);

        // Clean up the expired proposal
        state
            .cleanup_expired_treasury_proposal(&proposal_id)
            .unwrap();

        // Verify it's been removed
        let stats = state.treasury_stats();
        assert_eq!(stats.pending_proposals, 0);

        // Trying to clean up again should fail
        let result = state.cleanup_expired_treasury_proposal(&proposal_id);
        assert!(result.is_err());
    }
}
