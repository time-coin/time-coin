//! Block structures and functionality for TIME Coin

use crate::transaction::{Transaction, TxOutput, TransactionError};
use crate::utxo_set::UTXOSet;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

#[derive(Debug, Clone)]
pub enum BlockError {
    InvalidHash,
    InvalidMerkleRoot,
    InvalidTimestamp,
    InvalidBlockNumber,
    InvalidCoinbase,
    InvalidTransactions,
    TransactionError(TransactionError),
    NoTransactions,
}

impl std::fmt::Display for BlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BlockError::InvalidHash => write!(f, "Invalid block hash"),
            BlockError::InvalidMerkleRoot => write!(f, "Invalid merkle root"),
            BlockError::InvalidTimestamp => write!(f, "Invalid timestamp"),
            BlockError::InvalidBlockNumber => write!(f, "Invalid block number"),
            BlockError::InvalidCoinbase => write!(f, "Invalid coinbase transaction"),
            BlockError::InvalidTransactions => write!(f, "Invalid transactions"),
            BlockError::TransactionError(e) => write!(f, "Transaction error: {}", e),
            BlockError::NoTransactions => write!(f, "Block has no transactions"),
        }
    }
}

impl std::error::Error for BlockError {}

impl From<TransactionError> for BlockError {
    fn from(err: TransactionError) -> Self {
        BlockError::TransactionError(err)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Block height/number
    pub block_number: u64,
    /// Timestamp when block was created
    pub timestamp: DateTime<Utc>,
    /// Hash of the previous block
    pub previous_hash: String,
    /// Merkle root of all transactions
    pub merkle_root: String,
    /// Validator/masternode signature
    pub validator_signature: String,
    /// Validator address
    pub validator_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Block header
    pub header: BlockHeader,
    /// All transactions in the block (first one must be coinbase)
    pub transactions: Vec<Transaction>,
    /// Block hash
    pub hash: String,
}

/// Masternode tier definitions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MasternodeTier {
    Free,
    Bronze,
    Silver,
    Gold,
}

impl MasternodeTier {
    /// Get the collateral requirement for this tier
    pub fn collateral_requirement(&self) -> u64 {
        const TIME_UNIT: u64 = 100_000_000;
        match self {
            MasternodeTier::Free => 0,
            MasternodeTier::Bronze => 1_000 * TIME_UNIT,
            MasternodeTier::Silver => 10_000 * TIME_UNIT,
            MasternodeTier::Gold => 100_000 * TIME_UNIT,
        }
    }

    /// Get the reward weight multiplier for this tier
    /// Check if this tier can vote in governance
    pub fn can_vote(&self) -> bool {
        match self {
            MasternodeTier::Free => false,  // Free tier cannot vote
            _ => true,  // All paid tiers can vote
        }
    }

    pub fn weight(&self) -> u64 {
        match self {
            MasternodeTier::Free => 1,
            MasternodeTier::Bronze => 10,
            MasternodeTier::Silver => 25,
            MasternodeTier::Gold => 50,
        }
    }
}

/// Masternode count breakdown by tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasternodeCounts {
    pub free: u64,
    pub bronze: u64,
    pub silver: u64,
    pub gold: u64,
}

impl MasternodeCounts {
    pub fn total(&self) -> u64 {
        self.free + self.bronze + self.silver + self.gold
    }

    pub fn total_weight(&self) -> u64 {
        (self.free * MasternodeTier::Free.weight()) +
        (self.bronze * MasternodeTier::Bronze.weight()) +
        (self.silver * MasternodeTier::Silver.weight()) +
        (self.gold * MasternodeTier::Gold.weight())
    }
}

impl Block {
    /// Create a new block with a coinbase transaction
    pub fn new(
        block_number: u64,
        previous_hash: String,
        validator_address: String,
        coinbase_outputs: Vec<TxOutput>,
    ) -> Self {
        // Create coinbase transaction (no inputs, generates new coins)
        let coinbase = Transaction {
            txid: format!("coinbase_{}", block_number),
            version: 1,
            inputs: vec![], // Coinbase has no inputs
            outputs: coinbase_outputs,
            lock_time: 0,
            timestamp: Utc::now().timestamp(),
        };

        let mut block = Block {
            header: BlockHeader {
                block_number,
                timestamp: Utc::now(),
                previous_hash,
                merkle_root: String::new(),
                validator_signature: String::new(),
                validator_address,
            },
            transactions: vec![coinbase],
            hash: String::new(),
        };

        // Calculate merkle root and hash
        block.header.merkle_root = block.calculate_merkle_root();
        block.hash = block.calculate_hash();

        block
    }

    /// Add a transaction to the block
    pub fn add_transaction(&mut self, tx: Transaction) -> Result<(), BlockError> {
        // Validate transaction structure
        tx.validate_structure()?;

        self.transactions.push(tx);
        self.header.merkle_root = self.calculate_merkle_root();
        self.hash = self.calculate_hash();

        Ok(())
    }

    /// Calculate the block hash (double SHA3-256)
    pub fn calculate_hash(&self) -> String {
        let mut hasher = Sha3_256::new();
        
        // Hash header data
        hasher.update(self.header.block_number.to_le_bytes());
        hasher.update(self.header.timestamp.to_rfc3339().as_bytes());
        hasher.update(self.header.previous_hash.as_bytes());
        hasher.update(self.header.merkle_root.as_bytes());
        hasher.update(self.header.validator_address.as_bytes());
        
        let hash1 = hasher.finalize();
        let hash2 = Sha3_256::digest(&hash1);
        
        hex::encode(hash2)
    }

    /// Calculate merkle root of all transactions
    pub fn calculate_merkle_root(&self) -> String {
        if self.transactions.is_empty() {
            return "0".repeat(64);
        }

        let mut hashes: Vec<String> = self.transactions
            .iter()
            .map(|tx| tx.txid.clone())
            .collect();

        // Build merkle tree
        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            
            for i in (0..hashes.len()).step_by(2) {
                let left = &hashes[i];
                let right = if i + 1 < hashes.len() {
                    &hashes[i + 1]
                } else {
                    left // Duplicate if odd number
                };

                let combined = format!("{}{}", left, right);
                let hash = Sha3_256::digest(combined.as_bytes());
                next_level.push(hex::encode(hash));
            }

            hashes = next_level;
        }

        hashes[0].clone()
    }

    /// Get the coinbase transaction
    pub fn coinbase(&self) -> Option<&Transaction> {
        self.transactions.first()
    }

    /// Get all transactions except coinbase
    pub fn regular_transactions(&self) -> &[Transaction] {
        if self.transactions.len() > 1 {
            &self.transactions[1..]
        } else {
            &[]
        }
    }

    /// Validate block structure (not including transaction validation against UTXO)
    pub fn validate_structure(&self) -> Result<(), BlockError> {
        // Must have at least one transaction (coinbase)
        if self.transactions.is_empty() {
            return Err(BlockError::NoTransactions);
        }

        // First transaction must be coinbase
        if !self.transactions[0].is_coinbase() {
            return Err(BlockError::InvalidCoinbase);
        }

        // Only first transaction can be coinbase
        for tx in &self.transactions[1..] {
            if tx.is_coinbase() {
                return Err(BlockError::InvalidCoinbase);
            }
        }

        // Verify merkle root
        let calculated_merkle = self.calculate_merkle_root();
        if calculated_merkle != self.header.merkle_root {
            return Err(BlockError::InvalidMerkleRoot);
        }

        // Verify block hash
        let calculated_hash = self.calculate_hash();
        // Skip hash validation for genesis block
        if self.header.block_number == 0 {
            return Ok(());
        }
        if calculated_hash != self.hash {
            return Err(BlockError::InvalidHash);
        }

        // Validate all transaction structures
        for tx in &self.transactions {
            tx.validate_structure()?;
        }

        Ok(())
    }

    /// Validate block against UTXO set and apply it
    pub fn validate_and_apply(&self, utxo_set: &mut UTXOSet, masternode_counts: &MasternodeCounts) -> Result<(), BlockError> {
        // First validate structure
        self.validate_structure()?;

        // Calculate expected rewards
        let treasury_reward = calculate_treasury_reward();
        let total_masternode_reward = calculate_total_masternode_reward(masternode_counts);

        // Validate coinbase reward
        let coinbase = self.coinbase().ok_or(BlockError::InvalidCoinbase)?;
        let coinbase_total: u64 = coinbase.outputs.iter().map(|o| o.amount).sum();
        
        // Calculate total fees from regular transactions
        let mut total_fees = 0u64;
        for tx in self.regular_transactions() {
            let fee = tx.fee(utxo_set.utxos())?;
            total_fees += fee;
        }

        // Coinbase should be treasury + masternode rewards + fees
        let max_coinbase = treasury_reward + total_masternode_reward + total_fees;
        if coinbase_total > max_coinbase {
            return Err(BlockError::InvalidCoinbase);
        }

        // Apply coinbase first
        utxo_set.apply_transaction(coinbase)?;

        // Validate and apply all regular transactions
        for tx in self.regular_transactions() {
            utxo_set.apply_transaction(tx)?;
        }

        Ok(())
    }

    /// Get total transaction fees in the block
    pub fn total_fees(&self, utxo_set: &UTXOSet) -> Result<u64, BlockError> {
        let mut total = 0u64;
        for tx in self.regular_transactions() {
            let fee = tx.fee(utxo_set.utxos())?;
            total += fee;
        }
        Ok(total)
    }

    /// Sign the block (for masternode validators)
    pub fn sign(&mut self, signature: String) {
        self.header.validator_signature = signature;
        // Note: Signature is not included in hash calculation
    }

    /// Get block size in bytes (approximate)
    pub fn size(&self) -> usize {
        serde_json::to_string(self).map(|s| s.len()).unwrap_or(0)
    }

    /// Get transaction count
    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }
}

/// Calculate fixed treasury reward per block (5 TIME)
pub fn calculate_treasury_reward() -> u64 {
    const TIME_UNIT: u64 = 100_000_000;
    5 * TIME_UNIT
}

/// Calculate total masternode reward pool using logarithmic scaling
/// Formula: BASE * ln(1 + total_masternodes / SCALE)
pub fn calculate_total_masternode_reward(counts: &MasternodeCounts) -> u64 {
    const TIME_UNIT: u64 = 100_000_000;
    const BASE_REWARD: f64 = 95.0; // 95 TIME base
    const SCALE_FACTOR: f64 = 50.0; // Controls growth speed
    
    let total_nodes = counts.total() as f64;
    
    if total_nodes == 0.0 {
        return 0;
    }
    
    // Logarithmic scaling: BASE * ln(1 + count / SCALE)
    let multiplier = (1.0 + (total_nodes / SCALE_FACTOR)).ln();
    let reward = BASE_REWARD * multiplier * (TIME_UNIT as f64);
    
    reward as u64
}

/// Calculate reward for a specific masternode tier
pub fn calculate_tier_reward(
    tier: MasternodeTier,
    counts: &MasternodeCounts,
) -> u64 {
    let total_pool = calculate_total_masternode_reward(counts);
    let total_weight = counts.total_weight();
    
    if total_weight == 0 {
        return 0;
    }
    
    // Reward per weight unit
    let per_weight = total_pool / total_weight;
    
    // Multiply by tier weight
    per_weight * tier.weight()
}

/// Calculate total block reward (treasury + masternodes + fees)
pub fn calculate_total_block_reward(
    masternode_counts: &MasternodeCounts,
    transaction_fees: u64,
) -> u64 {
    let treasury = calculate_treasury_reward();
    let masternodes = calculate_total_masternode_reward(masternode_counts);
    treasury + masternodes + transaction_fees
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_masternode_tier_collateral() {
        assert_eq!(MasternodeTier::Free.collateral_requirement(), 0);
        assert_eq!(MasternodeTier::Bronze.collateral_requirement(), 1_000 * 100_000_000);
        assert_eq!(MasternodeTier::Silver.collateral_requirement(), 10_000 * 100_000_000);
        assert_eq!(MasternodeTier::Gold.collateral_requirement(), 100_000 * 100_000_000);
    }

    #[test]
    fn test_masternode_tier_weights() {
        assert_eq!(MasternodeTier::Free.weight(), 1);
        assert_eq!(MasternodeTier::Bronze.weight(), 10);
        assert_eq!(MasternodeTier::Silver.weight(), 25);
        assert_eq!(MasternodeTier::Gold.weight(), 50);
    }

    #[test]
    fn test_treasury_reward() {
        assert_eq!(calculate_treasury_reward(), 5 * 100_000_000);
    }

    #[test]
    fn test_logarithmic_scaling() {
        let counts1 = MasternodeCounts { free: 100, bronze: 0, silver: 0, gold: 0 };
        let counts2 = MasternodeCounts { free: 500, bronze: 0, silver: 0, gold: 0 };
        let counts3 = MasternodeCounts { free: 1000, bronze: 0, silver: 0, gold: 0 };
        
        let reward1 = calculate_total_masternode_reward(&counts1);
        let reward2 = calculate_total_masternode_reward(&counts2);
        let reward3 = calculate_total_masternode_reward(&counts3);
        
        // Rewards should increase but with diminishing returns
        assert!(reward2 > reward1);
        assert!(reward3 > reward2);
        assert!(reward2 - reward1 > reward3 - reward2); // Diminishing returns
    }

    #[test]
    fn test_tier_reward_distribution() {
        let counts = MasternodeCounts {
            free: 100,
            bronze: 50,
            silver: 20,
            gold: 10,
        };
        
        let free_reward = calculate_tier_reward(MasternodeTier::Free, &counts);
        let bronze_reward = calculate_tier_reward(MasternodeTier::Bronze, &counts);
        let silver_reward = calculate_tier_reward(MasternodeTier::Silver, &counts);
        let gold_reward = calculate_tier_reward(MasternodeTier::Gold, &counts);
        
        // Higher tiers should get proportionally more
        assert!(bronze_reward > free_reward);
        assert!(silver_reward > bronze_reward);
        assert!(gold_reward > silver_reward);
        
        // Check proportions match weights
        assert_eq!(bronze_reward / free_reward, 10); // 10x weight
        assert_eq!(silver_reward / free_reward, 25); // 25x weight
        assert_eq!(gold_reward / free_reward, 50); // 50x weight
    }

    #[test]
    fn test_block_creation() {
        let outputs = vec![TxOutput::new(10_000_000_000, "validator_address".to_string())];
        let block = Block::new(1, "previous_hash".to_string(), "validator".to_string(), outputs);
        
        assert_eq!(block.header.block_number, 1);
        assert_eq!(block.transactions.len(), 1);
        assert!(block.transactions[0].is_coinbase());
        assert!(!block.hash.is_empty());
    }
}
