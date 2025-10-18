//! Blockchain synchronization with merkle root verification

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct BlockchainSync {
    pub local_height: u64,
    pub network_height: u64,
    pub peers: Vec<String>,
    pub verified_blocks: HashMap<u64, String>, // height -> hash
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub start_height: u64,
    pub end_height: u64,
    pub requester: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    pub blocks: Vec<Block>,
    pub current_height: u64,
}

impl BlockchainSync {
    pub fn new(local_height: u64) -> Self {
        Self {
            local_height,
            network_height: 0,
            peers: Vec::new(),
            verified_blocks: HashMap::new(),
        }
    }
    
    /// Sync blockchain from network peers
    pub async fn sync_from_network(&mut self) -> Result<u64, String> {
        println!("⏳ Starting blockchain sync...");
        println!("  Local height: {}", self.local_height);
        
        // 1. Query all peers for their block height
        let peer_heights = self.query_peer_heights().await?;
        if peer_heights.is_empty() {
            return Err("No peers available for sync".to_string());
        }
        
        // 2. Find consensus height (most common height among peers)
        self.network_height = self.find_consensus_height(&peer_heights)?;
        println!("  Network height: {}", self.network_height);
        
        if self.local_height >= self.network_height {
            println!("✓ Already synced!");
            return Ok(self.local_height);
        }
        
        // 3. Download missing blocks in batches
        let mut current = self.local_height + 1;
        let batch_size = 100;
        
        while current <= self.network_height {
            let end = std::cmp::min(current + batch_size - 1, self.network_height);
            
            println!("  Downloading blocks {} to {}", current, end);
            let blocks = self.request_block_range(current, end).await?;
            
            // 4. Verify each block
            for block in blocks {
                self.verify_and_apply_block(block)?;
                current = block.height + 1;
            }
            
            if current % 1000 == 0 {
                println!("  Progress: {}/{}", current - 1, self.network_height);
            }
        }
        
        self.local_height = self.network_height;
        println!("✓ Sync complete! Height: {}", self.local_height);
        Ok(self.local_height)
    }
    
    /// Verify block integrity
    fn verify_and_apply_block(&mut self, block: Block) -> Result<(), String> {
        // 1. Verify merkle root
        let calculated_merkle = self.calculate_merkle_root(&block.transactions);
        if calculated_merkle != block.merkle_root {
            return Err(format!(
                "Block {}: Merkle root mismatch. Expected {}, got {}",
                block.height, block.merkle_root, calculated_merkle
            ));
        }
        
        // 2. Verify block hash
        let calculated_hash = self.calculate_block_hash(&block);
        if calculated_hash != block.hash {
            return Err(format!(
                "Block {}: Hash mismatch. Expected {}, got {}",
                block.height, block.hash, calculated_hash
            ));
        }
        
        // 3. Verify chain continuity
        if block.height > 0 {
            if let Some(prev_hash) = self.verified_blocks.get(&(block.height - 1)) {
                if prev_hash != &block.previous_hash {
                    return Err(format!(
                        "Block {}: Previous hash mismatch. Wrong chain!",
                        block.height
                    ));
                }
            }
        }
        
        // 4. Store verified block
        self.verified_blocks.insert(block.height, block.hash.clone());
        self.apply_block_to_chain(block)?;
        
        Ok(())
    }
    
    /// Calculate merkle root from transactions
    fn calculate_merkle_root(&self, transactions: &[String]) -> String {
        if transactions.is_empty() {
            return "0".repeat(64);
        }
        
        let mut hashes: Vec<String> = transactions
            .iter()
            .map(|tx| self.hash_string(tx))
            .collect();
        
        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            
            for chunk in hashes.chunks(2) {
                let combined = if chunk.len() == 2 {
                    format!("{}{}", chunk[0], chunk[1])
                } else {
                    format!("{}{}", chunk[0], chunk[0])
                };
                next_level.push(self.hash_string(&combined));
            }
            
            hashes = next_level;
        }
        
        hashes[0].clone()
    }
    
    /// Calculate block hash
    fn calculate_block_hash(&self, block: &Block) -> String {
        let data = format!(
            "{}{}{}{}{}",
            block.height,
            block.previous_hash,
            block.merkle_root,
            block.timestamp,
            block.transactions.len()
        );
        self.hash_string(&data)
    }
    
    /// Simple hash function (replace with actual crypto hash)
    fn hash_string(&self, data: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
    
    /// Apply verified block to local chain
    fn apply_block_to_chain(&self, _block: Block) -> Result<(), String> {
        // TODO: Actually store block in database
        Ok(())
    }
    
    /// Query peer heights (stub - implement with actual network calls)
    async fn query_peer_heights(&self) -> Result<Vec<u64>, String> {
        // TODO: Implement actual peer queries
        Ok(vec![self.network_height])
    }
    
    /// Find consensus height from peer responses
    fn find_consensus_height(&self, heights: &[u64]) -> Result<u64, String> {
        if heights.is_empty() {
            return Err("No heights to determine consensus".to_string());
        }
        
        // Return the most common height
        let mut counts: HashMap<u64, usize> = HashMap::new();
        for &height in heights {
            *counts.entry(height).or_insert(0) += 1;
        }
        
        counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(height, _)| height)
            .ok_or_else(|| "Could not determine consensus height".to_string())
    }
    
    /// Request block range from peers (stub)
    async fn request_block_range(&self, _start: u64, _end: u64) -> Result<Vec<Block>, String> {
        // TODO: Implement actual network requests
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_root_calculation() {
        let sync = BlockchainSync::new(0);
        let txs = vec!["tx1".to_string(), "tx2".to_string()];
        let root = sync.calculate_merkle_root(&txs);
        assert!(!root.is_empty());
    }

    #[test]
    fn test_consensus_height() {
        let sync = BlockchainSync::new(0);
        let heights = vec![100, 100, 100, 99, 101];
        let consensus = sync.find_consensus_height(&heights).unwrap();
        assert_eq!(consensus, 100);
    }
}
