//! Block structures and functionality for TIME Coin

use crate::transaction::{Transaction, TransactionError, TxOutput};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
            MasternodeTier::Free => false, // Free tier cannot vote
            _ => true,                     // All paid tiers can vote
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
        (self.free * MasternodeTier::Free.weight())
            + (self.bronze * MasternodeTier::Bronze.weight())
            + (self.silver * MasternodeTier::Silver.weight())
            + (self.gold * MasternodeTier::Gold.weight())
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
        let hash2 = Sha3_256::digest(hash1);

        hex::encode(hash2)
    }

    /// Calculate merkle root of all transactions
    pub fn calculate_merkle_root(&self) -> String {
        if self.transactions.is_empty() {
            return "0".repeat(64);
        }

        let mut hashes: Vec<String> = self.transactions.iter().map(|tx| tx.txid.clone()).collect();

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

        // Only first transaction can be coinbase (treasury grants can appear anywhere)
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
    pub fn validate_and_apply(
        &self,
        utxo_set: &mut UTXOSet,
        masternode_counts: &MasternodeCounts,
    ) -> Result<(), BlockError> {
        // First validate structure
        self.validate_structure()?;

        // Calculate expected rewards (no treasury pre-allocation)
        let total_masternode_reward = calculate_total_masternode_reward(masternode_counts);

        // Validate coinbase reward
        let coinbase = self.coinbase().ok_or(BlockError::InvalidCoinbase)?;
        let coinbase_total: u64 = coinbase.outputs.iter().map(|o| o.amount).sum();

        // Calculate total fees from regular transactions (excluding treasury grants)
        let mut total_fees = 0u64;
        for tx in self.regular_transactions() {
            if !tx.is_treasury_grant() {
                let fee = tx.fee(utxo_set.utxos())?;
                total_fees += fee;
            }
        }

        // Coinbase should be masternode rewards + fees (no treasury)
        let max_coinbase = total_masternode_reward + total_fees;
        if coinbase_total > max_coinbase {
            return Err(BlockError::InvalidCoinbase);
        }

        // Apply coinbase first
        utxo_set.apply_transaction(coinbase)?;

        // Validate and apply all regular transactions (including treasury grants)
        for tx in self.regular_transactions() {
            utxo_set.apply_transaction(tx)?;
        }

        Ok(())
    }

    /// Get total transaction fees in the block
    pub fn total_fees(&self, utxo_set: &UTXOSet) -> Result<u64, BlockError> {
        let mut total = 0u64;
        for tx in self.regular_transactions() {
            // Skip treasury grants as they don't have fees
            if !tx.is_treasury_grant() {
                let fee = tx.fee(utxo_set.utxos())?;
                total += fee;
            }
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
    const BASE_REWARD: f64 = 2000.0; // 95 TIME base
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
pub fn calculate_tier_reward(tier: MasternodeTier, counts: &MasternodeCounts) -> u64 {
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

/// Calculate total block reward (masternodes + fees only, no treasury pre-allocation)
pub fn calculate_total_block_reward(
    masternode_counts: &MasternodeCounts,
    transaction_fees: u64,
) -> u64 {
    let masternodes = calculate_total_masternode_reward(masternode_counts);
    masternodes + transaction_fees
}

/// Distribute masternode rewards to all active masternodes
/// Returns a vector of TxOutput for the coinbase transaction
pub fn distribute_masternode_rewards(
    active_masternodes: &[(String, MasternodeTier)],
    counts: &MasternodeCounts,
) -> Vec<crate::transaction::TxOutput> {
    let mut outputs = Vec::new();

    // Calculate total pool
    let total_pool = calculate_total_masternode_reward(counts);
    let total_weight = counts.total_weight();

    if total_weight == 0 || active_masternodes.is_empty() {
        return outputs;
    }

    // Calculate reward per weight unit
    let per_weight = total_pool / total_weight;

    // Distribute to each masternode based on their tier weight
    for (address, tier) in active_masternodes {
        let reward = per_weight * tier.weight();
        if reward > 0 {
            outputs.push(crate::transaction::TxOutput::new(reward, address.clone()));
        }
    }

    outputs
}

/// Create a complete coinbase transaction with all block rewards
/// Uses block_timestamp to ensure deterministic transaction across all nodes
/// Note: Treasury funds are no longer pre-allocated; all rewards go to masternodes
pub fn create_coinbase_transaction(
    block_number: u64,
    active_masternodes: &[(String, MasternodeTier)],
    counts: &MasternodeCounts,
    transaction_fees: u64,
    block_timestamp: i64, // Use block timestamp for determinism
) -> crate::transaction::Transaction {
    let mut outputs = Vec::new();

    // Masternode rewards - sorted by address for determinism
    let mut masternode_list: Vec<(String, MasternodeTier)> = active_masternodes.to_vec();
    masternode_list.sort_by(|a, b| a.0.cmp(&b.0)); // Sort by wallet address

    let masternode_outputs = distribute_masternode_rewards(&masternode_list, counts);
    outputs.extend(masternode_outputs);

    // Transaction fees go to block producer (if any)
    if transaction_fees > 0 && !masternode_list.is_empty() {
        // Give fees to the first masternode (block producer) after sorting
        if let Some((producer_address, _)) = masternode_list.first() {
            outputs.push(crate::transaction::TxOutput::new(
                transaction_fees,
                producer_address.clone(),
            ));
        }
    }

    // Create coinbase transaction with DETERMINISTIC timestamp
    crate::transaction::Transaction {
        txid: format!("coinbase_{}", block_number), // This will be recalculated
        version: 1,
        inputs: vec![],
        outputs,
        lock_time: 0,
        timestamp: block_timestamp, // Use block timestamp for determinism!
    }
}

/// Create a deterministic reward-only block (no mempool transactions)
/// Uses a normalized timestamp (block_number * 86400) to ensure all nodes create identical blocks
/// This function is specifically designed for empty mempool scenarios to achieve instant consensus
pub fn create_reward_only_block(
    block_number: u64,
    previous_hash: String,
    validator_address: String,
    active_masternodes: &[(String, MasternodeTier)],
    counts: &MasternodeCounts,
) -> Block {
    // Use normalized timestamp based on block number (seconds since genesis)
    // This ensures ALL nodes create the exact same timestamp
    let normalized_timestamp = (block_number * 86400) as i64;

    // Sort masternodes by address for complete determinism
    let mut sorted_masternodes = active_masternodes.to_vec();
    sorted_masternodes.sort_by(|a, b| a.0.cmp(&b.0));

    // Create deterministic coinbase transaction (no treasury address needed)
    let coinbase_tx = create_coinbase_transaction(
        block_number,
        &sorted_masternodes,
        counts,
        0, // No transaction fees in reward-only block
        normalized_timestamp,
    );

    // Create normalized datetime from timestamp
    let datetime =
        chrono::DateTime::from_timestamp(normalized_timestamp, 0).unwrap_or_else(Utc::now);

    // Create block with deterministic values
    let mut block = Block {
        header: BlockHeader {
            block_number,
            timestamp: datetime,
            previous_hash,
            merkle_root: String::new(),
            validator_signature: String::new(),
            validator_address,
        },
        transactions: vec![coinbase_tx],
        hash: String::new(),
    };

    // Calculate merkle root and hash
    block.header.merkle_root = block.calculate_merkle_root();
    block.hash = block.calculate_hash();

    block
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_masternode_tier_collateral() {
        assert_eq!(MasternodeTier::Free.collateral_requirement(), 0);
        assert_eq!(
            MasternodeTier::Bronze.collateral_requirement(),
            1_000 * 100_000_000
        );
        assert_eq!(
            MasternodeTier::Silver.collateral_requirement(),
            10_000 * 100_000_000
        );
        assert_eq!(
            MasternodeTier::Gold.collateral_requirement(),
            100_000 * 100_000_000
        );
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
        let counts1 = MasternodeCounts {
            free: 100,
            bronze: 0,
            silver: 0,
            gold: 0,
        };
        let counts2 = MasternodeCounts {
            free: 500,
            bronze: 0,
            silver: 0,
            gold: 0,
        };
        let counts3 = MasternodeCounts {
            free: 1000,
            bronze: 0,
            silver: 0,
            gold: 0,
        };

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
        let outputs = vec![TxOutput::new(
            10_000_000_000,
            "validator_address".to_string(),
        )];
        let block = Block::new(
            1,
            "previous_hash".to_string(),
            "validator".to_string(),
            outputs,
        );

        assert_eq!(block.header.block_number, 1);
        assert_eq!(block.transactions.len(), 1);
        assert!(block.transactions[0].is_coinbase());
        assert!(!block.hash.is_empty());
    }
    #[test]
    fn test_tier_economics() {
        use super::*;
        const TIME_UNIT: u64 = 100_000_000;

        // Test different network scenarios
        let scenarios = vec![
            (
                "Early network",
                MasternodeCounts {
                    free: 50,
                    bronze: 10,
                    silver: 3,
                    gold: 1,
                },
            ),
            (
                "Growing network",
                MasternodeCounts {
                    free: 200,
                    bronze: 50,
                    silver: 20,
                    gold: 10,
                },
            ),
            (
                "Mature network",
                MasternodeCounts {
                    free: 1000,
                    bronze: 200,
                    silver: 50,
                    gold: 20,
                },
            ),
        ];

        for (name, counts) in scenarios {
            println!("\n{}: {} total nodes", name, counts.total());
            println!(
                "Total pool: {} TIME",
                calculate_total_masternode_reward(&counts) / TIME_UNIT
            );

            let free_reward = calculate_tier_reward(MasternodeTier::Free, &counts);
            let bronze_reward = calculate_tier_reward(MasternodeTier::Bronze, &counts);
            let silver_reward = calculate_tier_reward(MasternodeTier::Silver, &counts);
            let gold_reward = calculate_tier_reward(MasternodeTier::Gold, &counts);

            println!(
                "  Free:   {:.2} TIME/day",
                free_reward as f64 / TIME_UNIT as f64
            );
            println!(
                "  Bronze: {:.2} TIME/day (APY: {}%)",
                bronze_reward as f64 / TIME_UNIT as f64,
                (bronze_reward * 365 / TIME_UNIT / 1000)
            );
            println!(
                "  Silver: {:.2} TIME/day (APY: {}%)",
                silver_reward as f64 / TIME_UNIT as f64,
                (silver_reward * 365 / TIME_UNIT / 10000)
            );
            println!(
                "  Gold:   {:.2} TIME/day (APY: {}%)",
                gold_reward as f64 / TIME_UNIT as f64,
                (gold_reward * 365 / TIME_UNIT / 100000)
            );
        }
    }
    #[test]
    fn test_distribute_masternode_rewards() {
        let masternodes = vec![
            ("addr1".to_string(), MasternodeTier::Free),
            ("addr2".to_string(), MasternodeTier::Free),
            ("addr3".to_string(), MasternodeTier::Bronze),
            ("addr4".to_string(), MasternodeTier::Silver),
            ("addr5".to_string(), MasternodeTier::Gold),
        ];

        let counts = MasternodeCounts {
            free: 2,
            bronze: 1,
            silver: 1,
            gold: 1,
        };

        let outputs = distribute_masternode_rewards(&masternodes, &counts);

        // Should have 5 outputs (one per masternode)
        assert_eq!(outputs.len(), 5);

        // Calculate expected values
        let total_pool = calculate_total_masternode_reward(&counts);
        let total_weight = counts.total_weight(); // 2*1 + 1*10 + 1*25 + 1*50 = 87
        let per_weight = total_pool / total_weight;

        // Verify each tier gets correct reward
        assert_eq!(outputs[0].amount, per_weight); // Free
        assert_eq!(outputs[1].amount, per_weight); // Free
        assert_eq!(outputs[2].amount, per_weight * 10); // Bronze
        assert_eq!(outputs[3].amount, per_weight * 25); // Silver
        assert_eq!(outputs[4].amount, per_weight * 50); // Gold
    }

    #[test]
    fn test_create_coinbase_transaction() {
        let masternodes = vec![
            ("masternode1".to_string(), MasternodeTier::Bronze),
            ("masternode2".to_string(), MasternodeTier::Silver),
        ];

        let counts = MasternodeCounts {
            free: 0,
            bronze: 1,
            silver: 1,
            gold: 0,
        };

        let block_timestamp = 1700000000; // Fixed timestamp for testing
        let tx = create_coinbase_transaction(
            100,
            &masternodes,
            &counts,
            50_000_000, // 0.5 TIME in fees
            block_timestamp,
        );

        // Verify it's a coinbase
        assert!(tx.is_coinbase());

        // Should have: 2 masternodes + 1 fee output = 3 outputs (no treasury)
        assert_eq!(tx.outputs.len(), 3);

        // Last output is fees to block producer (first masternode after sorting)
        assert_eq!(tx.outputs[2].amount, 50_000_000);
        assert_eq!(tx.outputs[2].address, "masternode1");
    }

    #[test]
    fn test_reward_scaling_with_growth() {
        // Test that rewards scale logarithmically
        let scenarios = vec![
            (
                10,
                MasternodeCounts {
                    free: 10,
                    bronze: 0,
                    silver: 0,
                    gold: 0,
                },
            ),
            (
                100,
                MasternodeCounts {
                    free: 100,
                    bronze: 0,
                    silver: 0,
                    gold: 0,
                },
            ),
            (
                1000,
                MasternodeCounts {
                    free: 1000,
                    bronze: 0,
                    silver: 0,
                    gold: 0,
                },
            ),
        ];

        for (count, counts) in &scenarios {
            let total = calculate_total_masternode_reward(counts);
            println!("{} masternodes: {} TIME total", count, total / 100_000_000);
        }

        // Verify logarithmic growth (not linear)
        let pool_10 = calculate_total_masternode_reward(&scenarios[0].1);
        let pool_100 = calculate_total_masternode_reward(&scenarios[1].1);
        let pool_1000 = calculate_total_masternode_reward(&scenarios[2].1);

        // 10x increase in nodes should NOT be 10x increase in rewards
        assert!(pool_100 < pool_10 * 10);
        assert!(pool_1000 < pool_100 * 10);
    }

    #[test]
    fn test_create_reward_only_block_deterministic() {
        // Test that create_reward_only_block produces identical blocks
        let block_number = 100;
        let previous_hash = "test_prev_hash".to_string();
        let validator_address = "validator1".to_string();

        let active_masternodes = vec![
            ("wallet_a".to_string(), MasternodeTier::Bronze),
            ("wallet_b".to_string(), MasternodeTier::Gold),
            ("wallet_c".to_string(), MasternodeTier::Silver),
        ];

        let counts = MasternodeCounts {
            free: 10,
            bronze: 5,
            silver: 3,
            gold: 2,
        };

        // Create block twice with same inputs
        let block1 = create_reward_only_block(
            block_number,
            previous_hash.clone(),
            validator_address.clone(),
            &active_masternodes,
            &counts,
        );

        let block2 = create_reward_only_block(
            block_number,
            previous_hash.clone(),
            validator_address.clone(),
            &active_masternodes,
            &counts,
        );

        // Blocks should be identical
        assert_eq!(block1.hash, block2.hash);
        assert_eq!(block1.header.merkle_root, block2.header.merkle_root);
        assert_eq!(block1.header.timestamp, block2.header.timestamp);
        assert_eq!(block1.transactions.len(), 1); // Only coinbase
        assert!(block1.transactions[0].is_coinbase());

        // Verify normalized timestamp
        let expected_timestamp = (block_number * 86400) as i64;
        assert_eq!(block1.transactions[0].timestamp, expected_timestamp);
    }

    #[test]
    fn test_reward_only_block_different_order() {
        // Test that masternode order doesn't affect the final block
        let block_number = 100;
        let previous_hash = "test_prev_hash".to_string();
        let validator_address = "validator1".to_string();

        // Same masternodes in different order
        let masternodes1 = vec![
            ("wallet_a".to_string(), MasternodeTier::Bronze),
            ("wallet_b".to_string(), MasternodeTier::Gold),
            ("wallet_c".to_string(), MasternodeTier::Silver),
        ];

        let masternodes2 = vec![
            ("wallet_c".to_string(), MasternodeTier::Silver),
            ("wallet_a".to_string(), MasternodeTier::Bronze),
            ("wallet_b".to_string(), MasternodeTier::Gold),
        ];

        let counts = MasternodeCounts {
            free: 10,
            bronze: 5,
            silver: 3,
            gold: 2,
        };

        let block1 = create_reward_only_block(
            block_number,
            previous_hash.clone(),
            validator_address.clone(),
            &masternodes1,
            &counts,
        );

        let block2 = create_reward_only_block(
            block_number,
            previous_hash.clone(),
            validator_address.clone(),
            &masternodes2,
            &counts,
        );

        // Blocks should be identical despite different input order
        assert_eq!(block1.hash, block2.hash);
        assert_eq!(block1.header.merkle_root, block2.header.merkle_root);
    }

    #[test]
    fn test_genesis_block_hash() {
        use crate::transaction::TxOutput;

        // Create the exact genesis block as specified
        let timestamp = 1760227200i64;
        let datetime = chrono::DateTime::from_timestamp(timestamp, 0).expect("Valid timestamp");

        let amount = 11653781624u64;

        let coinbase = crate::transaction::Transaction {
            txid: "coinbase_0".to_string(),
            version: 1,
            inputs: vec![],
            outputs: vec![TxOutput::new(amount, "genesis".to_string())],
            lock_time: 0,
            timestamp,
        };

        let mut block = Block {
            header: BlockHeader {
                block_number: 0,
                timestamp: datetime,
                previous_hash: "0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
                merkle_root: String::new(),
                validator_signature: "genesis".to_string(),
                validator_address: "genesis".to_string(),
            },
            transactions: vec![coinbase],
            hash: String::new(),
        };

        // Calculate merkle root and hash
        block.header.merkle_root = block.calculate_merkle_root();
        block.hash = block.calculate_hash();

        println!("\n=== Genesis Block ===");
        println!("Block Number: {}", block.header.block_number);
        println!("Timestamp: {}", block.header.timestamp);
        println!("Previous Hash: {}", block.header.previous_hash);
        println!("Merkle Root: {}", block.header.merkle_root);
        println!("Validator: {}", block.header.validator_address);
        println!("Hash: {}", block.hash);
        println!("=====================\n");

        // Verify structure
        assert_eq!(block.header.block_number, 0);
        assert_eq!(block.transactions.len(), 1);
        assert!(block.transactions[0].is_coinbase());
    }
}
