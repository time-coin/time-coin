use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub block_number: u64,
    pub timestamp: u64,
    pub previous_hash: String,
    pub transactions: Vec<String>, // Simplified for now
}

impl Block {
    pub fn new(block_number: u64, previous_hash: String) -> Self {
        Block {
            block_number,
            timestamp: current_timestamp(),
            previous_hash,
            transactions: Vec::new(),
        }
    }
}

fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_creation() {
        let block = Block::new(1, "genesis".to_string());
        assert_eq!(block.block_number, 1);
        assert_eq!(block.previous_hash, "genesis");
    }
}
