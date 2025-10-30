//! Blockchain State Manager for TIME Coin
//! 
//! Manages the blockchain state including:
//! - UTXO set
//! - Block chain
//! - Masternode tracking
//! - Chain tip and reorganization

use crate::block::{Block, BlockError, MasternodeCounts, MasternodeTier};
use crate::transaction::{Transaction, TransactionError};
use crate::utxo_set::UTXOSet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    
    /// Current masternode counts by tier
    masternode_counts: MasternodeCounts,
    
    /// Genesis block hash
    genesis_hash: String,
}

impl BlockchainState {
    /// Create a new blockchain state with genesis block
    /// Create a new blockchain state with genesis block and database
    pub fn new(genesis_block: Block, db_path: &str) -> Result<Self, StateError> {
        // Open database
        let db = crate::db::BlockchainDB::open(db_path)?;
        
        // Try to load blocks from disk
        let existing_blocks = db.load_all_blocks()?;
        
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
        };
        
        if existing_blocks.is_empty() {
            // No blocks on disk, add genesis
            genesis_block.validate_structure()?;
            // Apply genesis coinbase to UTXO set
            for tx in &genesis_block.transactions {
                state.utxo_set.apply_transaction(tx)?;
            }
            state.blocks.insert(genesis_block.hash.clone(), genesis_block.clone());
            state.blocks_by_height.insert(0, genesis_block.hash.clone());
            state.db.save_block(&genesis_block)?;
        } else {
            // Load blocks from disk into memory
            for block in existing_blocks {
                // Apply transactions to UTXO set
                for tx in &block.transactions {
                    state.utxo_set.apply_transaction(tx)?;
                }
                state.chain_tip_height = block.header.block_number;
                state.chain_tip_hash = block.hash.clone();
                state.blocks.insert(block.hash.clone(), block.clone());
                state.blocks_by_height.insert(block.header.block_number, block.hash.clone());
            }
        }
        
        Ok(state)
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

    /// Get masternode counts
    pub fn masternode_counts(&self) -> &MasternodeCounts {
        &self.masternode_counts
    }

    /// Get total supply
    pub fn total_supply(&self) -> u64 {
        self.utxo_set.total_supply()
    }

    /// Add a new block to the chain
    pub fn add_block(&mut self, block: Block) -> Result<(), StateError> {
        // Check if block already exists
        if self.has_block(&block.hash) {
            return Err(StateError::DuplicateBlock);
        }
        // Validate block structure
        block.validate_structure()?;
        // Check if this connects to our chain
        if block.header.block_number == 0 {
            // Cant add another genesis block
            return Err(StateError::InvalidBlockHeight);
        }
        // Verify previous block exists
        if !self.has_block(&block.header.previous_hash) {
            return Err(StateError::OrphanBlock);
        }
        // Verify block height is correct
        let expected_height = self.chain_tip_height + 1;
        if block.header.block_number != expected_height {
            return Err(StateError::InvalidBlockHeight);
        }
        // Verify previous hash matches chain tip
        if block.header.previous_hash != self.chain_tip_hash {
            return Err(StateError::InvalidPreviousHash);
        }
        // Create UTXO snapshot for potential rollback
        let utxo_snapshot = self.utxo_set.snapshot();
        // Validate and apply block to UTXO set
        match block.validate_and_apply(&mut self.utxo_set, &self.masternode_counts) {
            Ok(_) => {
                // Save block to disk FIRST (before moving into HashMap)
                self.db.save_block(&block)?;
                // Success! Add block to chain
                self.blocks_by_height.insert(block.header.block_number, block.hash.clone());
                self.chain_tip_height = block.header.block_number;
                self.chain_tip_hash = block.hash.clone();
                self.blocks.insert(block.hash.clone(), block);
                Ok(())
            }
            Err(e) => {
                // Rollback UTXO changes
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
    ) -> Result<(), StateError> {
        // Verify collateral requirement
        let required_collateral = tier.collateral_requirement();
        
        if required_collateral > 0 {
            // For paid tiers, verify the address has sufficient balance
            let balance = self.get_balance(&address);
            if balance < required_collateral {
                return Err(StateError::InvalidMasternodeCount);
            }
        }

        // Create masternode info
        let masternode = MasternodeInfo {
            address: address.clone(),
            tier,
            collateral_tx,
            registered_height: self.chain_tip_height,
            last_seen: chrono::Utc::now().timestamp(),
            is_active: true,
        };

        // Update counts
        match tier {
            MasternodeTier::Free => self.masternode_counts.free += 1,
            MasternodeTier::Bronze => self.masternode_counts.bronze += 1,
            MasternodeTier::Silver => self.masternode_counts.silver += 1,
            MasternodeTier::Gold => self.masternode_counts.gold += 1,
        }

        // Store masternode
        self.masternodes.insert(address, masternode);

        Ok(())
    }

    /// Deactivate a masternode
    pub fn deactivate_masternode(&mut self, address: &str) -> Result<(), StateError> {
        let masternode = self.masternodes
            .get_mut(address)
            .ok_or(StateError::InvalidMasternodeCount)?;

        if !masternode.is_active {
            return Ok(()); // Already inactive
        }

        masternode.is_active = false;

        // Update counts
        match masternode.tier {
            MasternodeTier::Free => self.masternode_counts.free = self.masternode_counts.free.saturating_sub(1),
            MasternodeTier::Bronze => self.masternode_counts.bronze = self.masternode_counts.bronze.saturating_sub(1),
            MasternodeTier::Silver => self.masternode_counts.silver = self.masternode_counts.silver.saturating_sub(1),
            MasternodeTier::Gold => self.masternode_counts.gold = self.masternode_counts.gold.saturating_sub(1),
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

    /// Validate a transaction against current UTXO set
    pub fn validate_transaction(&self, tx: &Transaction) -> Result<(), StateError> {
        // Validate structure
        tx.validate_structure()?;

        // Skip UTXO validation for coinbase
        if tx.is_coinbase() {
            return Ok(());
        }

        // Verify all inputs exist in UTXO set
        for input in &tx.inputs {
            if !self.utxo_set.contains(&input.previous_output) {
                return Err(StateError::TransactionError(TransactionError::InvalidInput));
            }
        }

        // Verify input amounts >= output amounts (implicit fee check)
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
        }
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
}

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
        
        let state = BlockchainState::new(genesis, "/tmp/test_blockchain_1").unwrap();
        
        assert_eq!(state.chain_tip_height(), 0);
        assert_eq!(state.chain_tip_hash(), genesis_hash);
        assert_eq!(state.total_supply(), 100_000_000_000);
    }

    #[test]
    fn test_add_block() {
        let genesis = create_genesis_block();
        let genesis_hash = genesis.hash.clone();
        let mut state = BlockchainState::new(genesis, "/tmp/test_blockchain_2").unwrap();
        
        // Create block 1
        let outputs = vec![TxOutput::new(10_000_000_000, "miner1".to_string())];
        let block1 = Block::new(1, genesis_hash.clone(), "miner1".to_string(), outputs);
        
        state.add_block(block1).unwrap();
        
        assert_eq!(state.chain_tip_height(), 1);
        assert!(state.get_block_by_height(1).is_some());
    }

    #[test]
    fn test_masternode_registration() {
        let genesis = create_genesis_block();
        let mut state = BlockchainState::new(genesis, "/tmp/test_blockchain_3").unwrap();
        
        // Register free tier masternode
        state.register_masternode(
            "masternode1".to_string(),
            MasternodeTier::Free,
            "collateral_tx".to_string(),
        ).unwrap();
        
        assert_eq!(state.masternode_counts().free, 1);
        assert!(state.get_masternode("masternode1").is_some());
    }

    #[test]
    fn test_get_balance() {
        let genesis = create_genesis_block();
        let state = BlockchainState::new(genesis, "/tmp/test_blockchain_4").unwrap();
        
        // Genesis address should have balance
        assert_eq!(state.get_balance("genesis"), 100_000_000_000);
        assert_eq!(state.get_balance("nonexistent"), 0);
    }
}
