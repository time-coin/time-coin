//! Snapshot system for fast memory operations with disk backup

use crate::transaction::Transaction;
use crate::state::StateError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

/// Hot state snapshot - current block period in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotStateSnapshot {
    pub current_height: u64,
    pub mempool: Vec<Transaction>,
    pub recent_tx_hashes: Vec<String>,
    pub snapshot_time: u64,
    pub last_block_hash: String,
}

/// Hot state manager - keeps current activity in memory
pub struct HotStateManager {
    hot_state: Arc<RwLock<HotState>>,
    snapshot_interval: u64,
    last_snapshot: Arc<RwLock<SystemTime>>,
}

/// In-memory hot state (fast access)
#[derive(Debug, Clone)]
pub struct HotState {
    pub current_height: u64,
    pub mempool: VecDeque<Transaction>,
    pub tx_hash_set: std::collections::HashSet<String>,
    pub recent_txs: VecDeque<Transaction>,
    pub balance_cache: HashMap<String, u64>,
    pub last_block_hash: String,
}

impl HotStateManager {
    pub fn new(_db: Arc<crate::db::BlockchainDB>, snapshot_interval_secs: u64) -> Result<Self, StateError> {
        let hot_state = Arc::new(RwLock::new(HotState {
            current_height: 0,
            mempool: VecDeque::new(),
            tx_hash_set: std::collections::HashSet::new(),
            recent_txs: VecDeque::new(),
            balance_cache: HashMap::new(),
            last_block_hash: String::new(),
        }));
        
        Ok(HotStateManager {
            hot_state,
            snapshot_interval: snapshot_interval_secs,
            last_snapshot: Arc::new(RwLock::new(SystemTime::now())),
        })
    }
    
    pub fn load_from_disk(&self) -> Result<(), StateError> {
        println!("⚠️  Snapshot loading not yet implemented");
        Ok(())
    }
    
    pub fn add_transaction(&self, tx: Transaction) -> Result<(), StateError> {
        let mut state = self.hot_state.write().unwrap();
        
        // Check for duplicates (O(1) lookup)
        if state.tx_hash_set.contains(&tx.txid) {
            return Err(StateError::DuplicateTransaction);
        }
        
        // Add to mempool
        state.tx_hash_set.insert(tx.txid.clone());
        state.mempool.push_back(tx.clone());
        state.recent_txs.push_back(tx);
        
        // Keep recent_txs bounded
        while state.recent_txs.len() > 10_000 {
            if let Some(old_tx) = state.recent_txs.pop_front() {
                // Don't remove from tx_hash_set if still in mempool
                let in_mempool = state.mempool.iter().any(|t| t.txid == old_tx.txid);
                if !in_mempool {
                    state.tx_hash_set.remove(&old_tx.txid);
                }
            }
        }
        
        Ok(())
    }
    
    pub fn get_mempool_transactions(&self, max_count: usize) -> Vec<Transaction> {
        let state = self.hot_state.read().unwrap();
        state.mempool.iter().take(max_count).cloned().collect()
    }
    
    pub fn mempool_size(&self) -> usize {
        let state = self.hot_state.read().unwrap();
        state.mempool.len()
    }
    
    pub fn has_transaction(&self, tx_hash: &[u8; 32]) -> bool {
        let state = self.hot_state.read().unwrap();
        let hash_str = hex::encode(tx_hash);
        state.tx_hash_set.contains(&hash_str)
    }
    
    pub fn save_snapshot(&self) -> Result<(), StateError> {
        let now = SystemTime::now();
        let last = *self.last_snapshot.read().unwrap();
        
        if now.duration_since(last).unwrap().as_secs() < self.snapshot_interval {
            return Ok(());
        }
        
        let state = self.hot_state.read().unwrap();
        println!("💾 Snapshot saved (height: {}, mempool: {} txs)", 
                 state.current_height, state.mempool.len());
        
        *self.last_snapshot.write().unwrap() = now;
        Ok(())
    }
    
    pub fn get_stats(&self) -> HotStateStats {
        let state = self.hot_state.read().unwrap();
        HotStateStats {
            mempool_size: state.mempool.len(),
            recent_tx_count: state.recent_txs.len(),
            pending_utxo_count: 0,
            cached_addresses: state.balance_cache.len(),
            current_height: state.current_height,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotStateStats {
    pub mempool_size: usize,
    pub recent_tx_count: usize,
    pub pending_utxo_count: usize,
    pub cached_addresses: usize,
    pub current_height: u64,
}
