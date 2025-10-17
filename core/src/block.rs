//! Block structures and functionality

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub block_number: u64,
    pub timestamp: DateTime<Utc>,
    pub previous_hash: String,
    pub merkle_root: String,
    pub validator_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<String>, // Transaction IDs
    pub hash: String,
}

impl Block {
    pub fn new(block_number: u64, previous_hash: String) -> Self {
        let header = BlockHeader {
            block_number,
            timestamp: Utc::now(),
            previous_hash,
            merkle_root: String::new(),
            validator_signature: String::new(),
        };

        let mut block = Block {
            header,
            transactions: Vec::new(),
            hash: String::new(),
        };

        block.hash = block.calculate_hash();
        block
    }

    pub fn add_transaction(&mut self, tx_id: String) {
        self.transactions.push(tx_id);
        self.header.merkle_root = self.calculate_merkle_root();
        self.hash = self.calculate_hash();
    }

    pub fn calculate_hash(&self) -> String {
        let data = format!(
            "{}:{}:{}:{}",
            self.header.block_number,
            self.header.timestamp.timestamp(),
            self.header.previous_hash,
            self.header.merkle_root
        );

        let mut hasher = Sha3_256::new();
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn calculate_merkle_root(&self) -> String {
        if self.transactions.is_empty() {
            return String::from("0");
        }

        let mut hasher = Sha3_256::new();
        for tx in &self.transactions {
            hasher.update(tx.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }

    pub fn validate(&self) -> Result<(), String> {
        // Validate hash
        if self.hash != self.calculate_hash() {
            return Err("Invalid block hash".to_string());
        }

        // Validate merkle root
        if self.header.merkle_root != self.calculate_merkle_root() {
            return Err("Invalid merkle root".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_creation() {
        let block = Block::new(1, "genesis".to_string());
        assert_eq!(block.header.block_number, 1);
        assert_eq!(block.header.previous_hash, "genesis");
        assert!(!block.hash.is_empty());
    }

    #[test]
    fn test_add_transaction() {
        let mut block = Block::new(1, "genesis".to_string());
        let initial_hash = block.hash.clone();
        
        block.add_transaction("tx123".to_string());
        
        assert_eq!(block.transactions.len(), 1);
        assert_ne!(block.hash, initial_hash); // Hash should change
    }

    #[test]
    fn test_block_validation() {
        let block = Block::new(1, "genesis".to_string());
        assert!(block.validate().is_ok());
    }
}
