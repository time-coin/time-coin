//! Sled-based persistence for blockchain data
use crate::block::Block;
use crate::state::StateError;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct BlockchainDB {
    db: sled::Db,
}

impl BlockchainDB {
    /// Open or create the database
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StateError> {
        let db = sled::open(path)
            .map_err(|e| StateError::IoError(format!("Failed to open database: {}", e)))?;

        Ok(BlockchainDB { db })
    }

    /// Save a block to disk
    pub fn save_block(&self, block: &Block) -> Result<(), StateError> {
        let key = format!("block:{}", block.header.block_number);
        let value = bincode::serialize(block)
            .map_err(|e| StateError::IoError(format!("Failed to serialize block: {}", e)))?;

        self.db
            .insert(key.as_bytes(), value)
            .map_err(|e| StateError::IoError(format!("Failed to save block: {}", e)))?;

        Ok(())
    }

    /// Load a block from disk by height
    pub fn load_block(&self, height: u64) -> Result<Option<Block>, StateError> {
        let key = format!("block:{}", height);

        match self.db.get(key.as_bytes()) {
            Ok(Some(data)) => {
                let block = bincode::deserialize(&data).map_err(|e| {
                    StateError::IoError(format!("Failed to deserialize block: {}", e))
                })?;
                Ok(Some(block))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(StateError::IoError(format!("Failed to load block: {}", e))),
        }
    }

    /// Load all blocks from disk
    pub fn load_all_blocks(&self) -> Result<Vec<Block>, StateError> {
        let mut blocks = Vec::new();
        let mut height = 0u64;

        while let Some(block) = self.load_block(height)? {
            blocks.push(block);
            height += 1;
        }

        Ok(blocks)
    }

    /// Save hot state snapshot
    pub fn save_snapshot(
        &self,
        snapshot: &crate::snapshot::HotStateSnapshot,
    ) -> Result<(), StateError> {
        let data = bincode::serialize(snapshot)
            .map_err(|e| StateError::IoError(format!("Failed to serialize snapshot: {}", e)))?;

        self.db
            .insert(b"snapshot:hot_state", data)
            .map_err(|e| StateError::IoError(format!("Failed to save snapshot: {}", e)))?;

        // Flush to ensure it's on disk
        self.db
            .flush()
            .map_err(|e| StateError::IoError(format!("Failed to flush snapshot: {}", e)))?;

        Ok(())
    }

    /// Load latest hot state snapshot
    pub fn load_snapshot(&self) -> Result<Option<crate::snapshot::HotStateSnapshot>, StateError> {
        match self.db.get(b"snapshot:hot_state") {
            Ok(Some(data)) => {
                let snapshot = bincode::deserialize(&data).map_err(|e| {
                    StateError::IoError(format!("Failed to deserialize snapshot: {}", e))
                })?;
                Ok(Some(snapshot))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(StateError::IoError(format!(
                "Failed to load snapshot: {}",
                e
            ))),
        }
    }
}
