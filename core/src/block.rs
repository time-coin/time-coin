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
        let mut block = Block {
            header: BlockHeader {
                block_number,
                timestamp: Utc::now(),
                previous_hash,
                merkle_root: String::new(),
                validator_signature: String::new(),
            },
            transactions: Vec::new(),
            hash: String::new(),
        };

        // Calculate merkle root first (will be "0" for empty block)
        block.header.merkle_root = block.calculate_merkle_root();
        // Then calculate hash
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
        let calculated_hash = self.calculate_hash();
        if self.hash != calculated_hash {
            return Err(format!(
                "Invalid block hash. Expected: {}, Got: {}",
                calculated_hash, self.hash
            ));
        }

        // Validate merkle root
        let calculated_merkle = self.calculate_merkle_root();
        if self.header.merkle_root != calculated_merkle {
            return Err(format!(
                "Invalid merkle root. Expected: {}, Got: {}",
                calculated_merkle, self.header.merkle_root
            ));
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
        assert_eq!(block.header.merkle_root, "0"); // Empty block has merkle root "0"
    }

    #[test]
    fn test_add_transaction() {
        let mut block = Block::new(1, "genesis".to_string());
        let initial_hash = block.hash.clone();
        let initial_merkle = block.header.merkle_root.clone();
        
        block.add_transaction("tx123".to_string());
        
        assert_eq!(block.transactions.len(), 1);
        assert_ne!(block.hash, initial_hash); // Hash should change
        assert_ne!(block.header.merkle_root, initial_merkle); // Merkle root should change
    }

    #[test]
    fn test_block_validation() {
        let block = Block::new(1, "genesis".to_string());
        assert!(block.validate().is_ok());
    }

    #[test]
    fn test_block_with_transactions_validation() {
        let mut block = Block::new(1, "genesis".to_string());
        block.add_transaction("tx1".to_string());
        block.add_transaction("tx2".to_string());
        
        assert!(block.validate().is_ok());
    }

    #[test]
    fn test_invalid_hash_detection() {
        let mut block = Block::new(1, "genesis".to_string());
        block.hash = "invalid_hash".to_string();
        
        assert!(block.validate().is_err());
    }

    #[test]
    fn test_invalid_merkle_root_detection() {
        let mut block = Block::new(1, "genesis".to_string());
        block.header.merkle_root = "invalid_merkle".to_string();
        
        assert!(block.validate().is_err());
    }

    #[test]
    fn test_merkle_root_changes_with_transactions() {
        let mut block = Block::new(1, "genesis".to_string());
        let empty_merkle = block.header.merkle_root.clone();
        
        block.add_transaction("tx1".to_string());
        let one_tx_merkle = block.header.merkle_root.clone();
        
        block.add_transaction("tx2".to_string());
        let two_tx_merkle = block.header.merkle_root.clone();
        
        // All should be different
        assert_ne!(empty_merkle, one_tx_merkle);
        assert_ne!(one_tx_merkle, two_tx_merkle);
        assert_ne!(empty_merkle, two_tx_merkle);
    }
}
