//! TIME Coin Storage Layer
//! 
//! Persistent storage for finalized daily blocks

use rocksdb::{DB, Options};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] rocksdb::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Block not found: {0}")]
    BlockNotFound(u64),
}

/// Block storage database
pub struct BlockStorage {
    db: DB,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredBlock {
    pub height: u64,
    pub timestamp: i64,
    pub hash: String,
    pub previous_hash: String,
    pub state_snapshot: Vec<u8>,
    pub transaction_count: u64,
}

impl BlockStorage {
    /// Open or create block storage
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        
        let db = DB::open(&opts, path)?;
        
        Ok(Self { db })
    }
    
    /// Save a block
    pub fn save_block(&self, block: &StoredBlock) -> Result<(), StorageError> {
        let key = format!("block:{}", block.height);
        let value = bincode::serialize(block)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        
        self.db.put(key.as_bytes(), value)?;
        
        // Update tip
        self.db.put(b"tip", block.height.to_be_bytes())?;
        
        // Index by hash
        let hash_key = format!("hash:{}", block.hash);
        self.db.put(hash_key.as_bytes(), block.height.to_be_bytes())?;
        
        Ok(())
    }
    
    /// Get block by height
    pub fn get_block(&self, height: u64) -> Result<StoredBlock, StorageError> {
        let key = format!("block:{}", height);
        
        let value = self.db.get(key.as_bytes())?
            .ok_or(StorageError::BlockNotFound(height))?;
        
        let block: StoredBlock = bincode::deserialize(&value)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        
        Ok(block)
    }
    
    /// Get block by hash
    pub fn get_block_by_hash(&self, hash: &str) -> Result<StoredBlock, StorageError> {
        let hash_key = format!("hash:{}", hash);
        
        let height_bytes = self.db.get(hash_key.as_bytes())?
            .ok_or(StorageError::BlockNotFound(0))?;
        
        let height = u64::from_be_bytes(height_bytes.try_into().unwrap());
        
        self.get_block(height)
    }
    
    /// Get current chain tip (latest block height)
    pub fn get_tip(&self) -> Result<u64, StorageError> {
        let tip_bytes = self.db.get(b"tip")?
            .unwrap_or_else(|| vec![0u8; 8]);
        
        Ok(u64::from_be_bytes(tip_bytes.try_into().unwrap()))
    }
    
    /// Get latest block
    pub fn get_latest_block(&self) -> Result<Option<StoredBlock>, StorageError> {
        let tip = self.get_tip()?;
        
        if tip == 0 {
            return Ok(None);
        }
        
        Ok(Some(self.get_block(tip)?))
    }
    
    /// Check if block exists
    pub fn has_block(&self, height: u64) -> bool {
        let key = format!("block:{}", height);
        self.db.get(key.as_bytes()).ok().flatten().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_storage_basic() {
        let dir = tempdir().unwrap();
        let storage = BlockStorage::open(dir.path()).unwrap();
        
        let block = StoredBlock {
            height: 1,
            timestamp: 1729123200,
            hash: "test_hash".to_string(),
            previous_hash: "genesis".to_string(),
            state_snapshot: vec![1, 2, 3],
            transaction_count: 5,
        };
        
        storage.save_block(&block).unwrap();
        
        let loaded = storage.get_block(1).unwrap();
        assert_eq!(loaded.height, 1);
        assert_eq!(loaded.hash, "test_hash");
        
        let by_hash = storage.get_block_by_hash("test_hash").unwrap();
        assert_eq!(by_hash.height, 1);
        
        assert_eq!(storage.get_tip().unwrap(), 1);
    }
}
