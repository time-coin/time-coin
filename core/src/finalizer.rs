//! Block Finalizer
//! 
//! Runs at midnight UTC to finalize daily state into a block

use crate::state::{DailyState, StateSnapshot};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalizedBlock {
    pub height: u64,
    pub timestamp: i64,
    pub previous_hash: String,
    pub hash: String,
    pub state_snapshot: StateSnapshot,
    pub transaction_count: u64,
    pub merkle_root: String,
}

pub struct BlockFinalizer;

impl BlockFinalizer {
    /// Finalize current state into a block
    pub fn finalize(
        state: &DailyState,
        previous_hash: String,
    ) -> FinalizedBlock {
        let snapshot = state.create_snapshot();
        let timestamp = Utc::now().timestamp();
        
        let merkle_root = Self::calculate_merkle_root(&state.transactions);
        
        let mut block = FinalizedBlock {
            height: state.current_height,
            timestamp,
            previous_hash,
            hash: String::new(),
            state_snapshot: snapshot,
            transaction_count: state.transactions.len() as u64,
            merkle_root,
        };
        
        block.hash = Self::calculate_hash(&block);
        
        block
    }
    
    fn calculate_hash(block: &FinalizedBlock) -> String {
        let mut hasher = Sha256::new();
        
        let data = format!(
            "{}{}{}{}{}",
            block.height,
            block.timestamp,
            block.previous_hash,
            block.merkle_root,
            block.transaction_count
        );
        
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    fn calculate_merkle_root(transactions: &[crate::state::Transaction]) -> String {
        if transactions.is_empty() {
            return "0".repeat(64);
        }
        
        let mut hasher = Sha256::new();
        for tx in transactions {
            hasher.update(tx.txid.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_finalization() {
        let state = DailyState::new(1);
        let block = BlockFinalizer::finalize(&state, "genesis".to_string());
        
        assert_eq!(block.height, 1);
        assert!(!block.hash.is_empty());
        assert_eq!(block.previous_hash, "genesis");
    }
}
