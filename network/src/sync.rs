//! Blockchain sync with merkle verification
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Debug, Clone)]
pub struct BlockchainSync {
    pub local_height: u64,
    pub network_height: u64,
    pub verified_blocks: HashMap<u64, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub height: u64,
    pub hash: String,
    pub previous_hash: String,
    pub merkle_root: String,
    pub timestamp: i64,
    pub transactions: Vec<String>,
}
impl BlockchainSync {
    pub fn new(local_height: u64) -> Self {
        Self {
            local_height,
            network_height: 0,
            verified_blocks: HashMap::new(),
        }
    }
    pub fn verify_block(&self, block: &Block) -> Result<bool, String> {
        let calculated_merkle = self.calculate_merkle_root(&block.transactions);
        if calculated_merkle != block.merkle_root {
            return Err(format!("Block {}: Merkle root mismatch", block.height));
        }
        let calculated_hash = self.calculate_block_hash(block);
        if calculated_hash != block.hash {
            return Err(format!("Block {}: Hash mismatch", block.height));
        }
        if block.height > 0 {
            if let Some(prev_hash) = self.verified_blocks.get(&(block.height - 1)) {
                if prev_hash != &block.previous_hash {
                    return Err(format!("Block {}: Chain mismatch", block.height));
                }
            }
        }
        Ok(true)
    }
    fn calculate_merkle_root(&self, transactions: &[String]) -> String {
        if transactions.is_empty() { return "0".repeat(64); }
        let mut hashes: Vec<String> = transactions.iter().map(|tx| self.hash_string(tx)).collect();
        while hashes.len() > 1 {
            let mut next = Vec::new();
            for chunk in hashes.chunks(2) {
                let combined = if chunk.len() == 2 {
                    format!("{}{}", chunk[0], chunk[1])
                } else {
                    format!("{}{}", chunk[0], chunk[0])
                };
                next.push(self.hash_string(&combined));
            }
            hashes = next;
        }
        hashes[0].clone()
    }
    fn calculate_block_hash(&self, block: &Block) -> String {
        let data = format!("{}{}{}{}{}", block.height, block.previous_hash, block.merkle_root, block.timestamp, block.transactions.len());
        self.hash_string(&data)
    }
    fn hash_string(&self, data: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}
