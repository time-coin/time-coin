//! Sled-based persistence for blockchain data
use crate::block::Block;
use crate::state::StateError;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct BlockchainDB {
    db: sled::Db,
    path: String,
}

impl BlockchainDB {
    /// Open or create the database
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StateError> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let db = sled::open(&path)
            .map_err(|e| StateError::IoError(format!("Failed to open database: {}", e)))?;

        Ok(BlockchainDB { db, path: path_str })
    }

    /// Get the database path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Save a block to disk
    pub fn save_block(&self, block: &Block) -> Result<(), StateError> {
        let key = format!("block:{}", block.header.block_number);
        let value = bincode::serialize(block)
            .map_err(|e| StateError::IoError(format!("Failed to serialize block: {}", e)))?;

        self.db
            .insert(key.as_bytes(), value)
            .map_err(|e| StateError::IoError(format!("Failed to save block: {}", e)))?;

        // Flush to disk to ensure durability - critical for persistence after reboot
        self.db
            .flush()
            .map_err(|e| StateError::IoError(format!("Failed to flush block to disk: {}", e)))?;

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

    /// Save UTXO state snapshot for persistence between blocks
    pub fn save_utxo_snapshot(
        &self,
        utxo_set: &crate::utxo_set::UTXOSet,
    ) -> Result<(), StateError> {
        let snapshot = utxo_set.snapshot();
        let data = bincode::serialize(&snapshot).map_err(|e| {
            StateError::IoError(format!("Failed to serialize UTXO snapshot: {}", e))
        })?;

        self.db
            .insert(b"snapshot:utxo_state", data)
            .map_err(|e| StateError::IoError(format!("Failed to save UTXO snapshot: {}", e)))?;

        // Flush to ensure it's on disk
        self.db
            .flush()
            .map_err(|e| StateError::IoError(format!("Failed to flush UTXO snapshot: {}", e)))?;

        Ok(())
    }

    /// Load UTXO state snapshot
    pub fn load_utxo_snapshot(
        &self,
    ) -> Result<Option<crate::utxo_set::UTXOSetSnapshot>, StateError> {
        match self.db.get(b"snapshot:utxo_state") {
            Ok(Some(data)) => {
                let snapshot = bincode::deserialize(&data).map_err(|e| {
                    StateError::IoError(format!("Failed to deserialize UTXO snapshot: {}", e))
                })?;
                Ok(Some(snapshot))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(StateError::IoError(format!(
                "Failed to load UTXO snapshot: {}",
                e
            ))),
        }
    }

    /// Save a finalized transaction to database
    pub fn save_finalized_tx(
        &self,
        tx: &crate::Transaction,
        votes: usize,
        total: usize,
    ) -> Result<(), StateError> {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        struct FinalizedTx {
            transaction: crate::Transaction,
            finalized_at: i64,
            votes_received: usize,
            total_voters: usize,
        }

        let finalized = FinalizedTx {
            transaction: tx.clone(),
            finalized_at: chrono::Utc::now().timestamp(),
            votes_received: votes,
            total_voters: total,
        };

        let key = format!("finalized_tx:{}", tx.txid);
        let value = bincode::serialize(&finalized)
            .map_err(|e| StateError::IoError(format!("Failed to serialize finalized tx: {}", e)))?;

        self.db
            .insert(key.as_bytes(), value)
            .map_err(|e| StateError::IoError(format!("Failed to save finalized tx: {}", e)))?;

        self.db
            .flush()
            .map_err(|e| StateError::IoError(format!("Failed to flush finalized tx: {}", e)))?;

        Ok(())
    }

    /// Remove a finalized transaction (when it's been included in a block)
    pub fn remove_finalized_tx(&self, txid: &str) -> Result<(), StateError> {
        let key = format!("finalized_tx:{}", txid);
        self.db
            .remove(key.as_bytes())
            .map_err(|e| StateError::IoError(format!("Failed to remove finalized tx: {}", e)))?;
        Ok(())
    }

    /// Load all finalized transactions
    pub fn load_finalized_txs(&self) -> Result<Vec<crate::Transaction>, StateError> {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        struct FinalizedTx {
            transaction: crate::Transaction,
            finalized_at: i64,
            votes_received: usize,
            total_voters: usize,
        }

        let mut txs = Vec::new();
        let prefix = b"finalized_tx:";

        for item in self.db.scan_prefix(prefix) {
            match item {
                Ok((_key, value)) => {
                    let finalized: FinalizedTx = bincode::deserialize(&value).map_err(|e| {
                        StateError::IoError(format!("Failed to deserialize finalized tx: {}", e))
                    })?;
                    txs.push(finalized.transaction);
                }
                Err(e) => {
                    return Err(StateError::IoError(format!(
                        "Failed to scan finalized txs: {}",
                        e
                    )))
                }
            }
        }

        Ok(txs)
    }

    /// Clear all blocks from the database
    pub fn clear_all(&self) -> Result<(), StateError> {
        // Get all keys that start with "block:"
        let keys_to_delete: Vec<_> = self
            .db
            .scan_prefix(b"block:")
            .keys()
            .filter_map(|k| k.ok())
            .collect();

        // Delete all block entries
        for key in keys_to_delete {
            self.db
                .remove(key)
                .map_err(|e| StateError::IoError(format!("Failed to clear database: {}", e)))?;
        }

        // Also clear snapshots
        let _ = self.db.remove(b"snapshot:hot_state");

        // Flush to ensure changes are persisted
        self.db
            .flush()
            .map_err(|e| StateError::IoError(format!("Failed to flush database: {}", e)))?;

        Ok(())
    }

    /// Save wallet balance to database
    pub fn save_wallet_balance(&self, address: &str, balance: u64) -> Result<(), StateError> {
        let key = format!("wallet_balance:{}", address);
        let value = balance.to_le_bytes();

        self.db
            .insert(key.as_bytes(), &value)
            .map_err(|e| StateError::IoError(format!("Failed to save wallet balance: {}", e)))?;

        // Flush to ensure it's on disk
        self.db
            .flush()
            .map_err(|e| StateError::IoError(format!("Failed to flush wallet balance: {}", e)))?;

        Ok(())
    }

    /// Load wallet balance from database
    pub fn load_wallet_balance(&self, address: &str) -> Result<Option<u64>, StateError> {
        let key = format!("wallet_balance:{}", address);

        match self.db.get(key.as_bytes()) {
            Ok(Some(data)) => {
                if data.len() == 8 {
                    let mut bytes = [0u8; 8];
                    bytes.copy_from_slice(&data);
                    Ok(Some(u64::from_le_bytes(bytes)))
                } else {
                    Err(StateError::IoError(
                        "Invalid wallet balance data".to_string(),
                    ))
                }
            }
            Ok(None) => Ok(None),
            Err(e) => Err(StateError::IoError(format!(
                "Failed to load wallet balance: {}",
                e
            ))),
        }
    }
}
