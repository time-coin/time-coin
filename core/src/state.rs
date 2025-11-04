//! Blockchain State Manager for TIME Coin
//! 
//! Manages the blockchain state including:
//! - UTXO set
//! - Block chain
//! - Masternode tracking
//! - Chain tip and reorganization

use crate::block::{Block, BlockError, MasternodeCounts, MasternodeTier};
use std::sync::{Arc, RwLock};
use crate::transaction::{Transaction, TransactionError, OutPoint};
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
}

impl BlockchainState {
    /// Create a new blockchain state with genesis block
    pub fn new(genesis_block: Block, db_path: &str) -> Result<Self, StateError> {
        let db = crate::db::BlockchainDB::open(db_path)?;
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
            invalidated_transactions: Arc::new(RwLock::new(Vec::new())),
        };
        
        if existing_blocks.is_empty() {
            genesis_block.validate_structure()?;
            for tx in &genesis_block.transactions {
                state.utxo_set.apply_transaction(tx)?;
            }
            state.blocks.insert(genesis_block.hash.clone(), genesis_block.clone());
            state.blocks_by_height.insert(0, genesis_block.hash.clone());
            state.db.save_block(&genesis_block)?;
        } else {
            for block in existing_blocks {
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
                self.db.save_block(&block)?;
                self.blocks_by_height.insert(block.header.block_number, block.hash.clone());
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
        let required_collateral = tier.collateral_requirement();
        if required_collateral > 0 {
            let balance = self.get_balance(&address);
            if balance < required_collateral {
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
        let masternode = self.masternodes
            .get_mut(address)
            .ok_or(StateError::InvalidMasternodeCount)?;

        if !masternode.is_active {
            return Ok(());
        }

        masternode.is_active = false;

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
        self.masternodes.values().filter(|mn| mn.is_active).collect()
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
            self.blocks.insert(new_block.hash.clone(), new_block.clone());
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
                self.invalidated_transactions.write().unwrap().push(event.clone());
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
        state.register_masternode(
            "masternode1".to_string(),
            MasternodeTier::Free,
            "collateral_tx".to_string(),
            "wallet_address".to_string(),
        ).unwrap();
        assert_eq!(state.masternode_counts().free, 1);
        assert!(state.get_masternode("masternode1").is_some());
    }

    #[test]
    fn test_get_balance() {
        let genesis = create_genesis_block();
        let state = BlockchainState::new(genesis, "/tmp/test_blockchain_4").unwrap();
        assert_eq!(state.get_balance("genesis"), 100_000_000_000);
        assert_eq!(state.get_balance("nonexistent"), 0);
    }
}
