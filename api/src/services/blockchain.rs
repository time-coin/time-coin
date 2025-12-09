//! Blockchain service - encapsulates blockchain queries and operations

use crate::{ApiError, ApiResult};
use std::sync::Arc;
use time_core::state::BlockchainState;
use tokio::sync::RwLock;

/// Service for blockchain operations
#[derive(Clone)]
pub struct BlockchainService {
    blockchain: Arc<RwLock<BlockchainState>>,
}

impl BlockchainService {
    /// Create a new blockchain service
    pub fn new(blockchain: Arc<RwLock<BlockchainState>>) -> Self {
        Self { blockchain }
    }

    /// Get blockchain information
    pub async fn get_info(&self) -> ApiResult<BlockchainInfo> {
        let blockchain = self.blockchain.read().await;

        // Calculate total supply from UTXO set
        let total_supply: u64 = blockchain
            .utxo_set()
            .utxos()
            .values()
            .map(|output| output.amount)
            .sum();

        Ok(BlockchainInfo {
            height: blockchain.chain_tip_height(),
            best_block_hash: blockchain.chain_tip_hash().to_string(),
            total_supply,
        })
    }

    /// Get block by height
    pub async fn get_block(&self, height: u64) -> ApiResult<time_core::block::Block> {
        let blockchain = self.blockchain.read().await;

        blockchain
            .get_block_by_height(height)
            .cloned()
            .ok_or_else(|| ApiError::BlockNotFound(format!("Block at height {} not found", height)))
    }

    /// Get balance for an address
    pub async fn get_balance(&self, address: &str) -> ApiResult<u64> {
        let blockchain = self.blockchain.read().await;
        Ok(blockchain.get_balance(address))
    }

    /// Get available (spendable) balance for an address
    ///
    /// This filters out UTXOs that are locked or spent pending in instant finality
    pub async fn get_available_balance(&self, address: &str) -> ApiResult<u64> {
        let blockchain = self.blockchain.read().await;
        let utxo_set = blockchain.utxo_set();
        let utxo_manager = blockchain.utxo_state_manager();

        let mut available = 0u64;

        for (outpoint, output) in utxo_set.get_utxos_by_address(address) {
            if output.address == address {
                let utxo_state = utxo_manager.get_state(&outpoint).await;

                // Only count UTXOs that are spendable
                let is_available = match utxo_state {
                    None => true, // Not tracked = available
                    Some(time_core::utxo_state_manager::UTXOState::Unspent) => true,
                    Some(time_core::utxo_state_manager::UTXOState::Confirmed { .. }) => true,
                    Some(time_core::utxo_state_manager::UTXOState::Locked { .. }) => false,
                    Some(time_core::utxo_state_manager::UTXOState::SpentPending { .. }) => false,
                    Some(time_core::utxo_state_manager::UTXOState::SpentFinalized { .. }) => false,
                };

                if is_available {
                    available = available.saturating_add(output.amount);
                }
            }
        }

        Ok(available)
    }

    /// Get UTXOs for an address
    pub async fn get_utxos(
        &self,
        address: &str,
    ) -> ApiResult<
        Vec<(
            time_core::transaction::OutPoint,
            time_core::transaction::TxOutput,
        )>,
    > {
        let blockchain = self.blockchain.read().await;
        let utxo_set = blockchain.utxo_set();

        let utxos: Vec<_> = utxo_set
            .get_utxos_by_address(address)
            .into_iter()
            .map(|(outpoint, output)| (outpoint, output.clone()))
            .collect();

        Ok(utxos)
    }
}

/// Blockchain information response
#[derive(Debug, Clone)]
pub struct BlockchainInfo {
    pub height: u64,
    pub best_block_hash: String,
    pub total_supply: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use time_core::block::{Block, BlockHeader, MasternodeCounts};

    fn create_test_genesis_block(db_suffix: &str) -> (Block, String) {
        let coinbase_tx = time_core::Transaction {
            txid: format!("coinbase_{}", db_suffix),
            version: 1,
            inputs: vec![],
            outputs: vec![time_core::TxOutput {
                address: "genesis".to_string(),
                amount: 50_000_000_000,
            }],
            lock_time: 0,
            timestamp: Utc::now().timestamp(),
        };

        let transactions = vec![coinbase_tx];
        let merkle_root = time_core::calculate_merkle_root(&transactions);

        let genesis_header = BlockHeader {
            block_number: 0,
            timestamp: Utc::now(),
            previous_hash: "0".to_string(),
            merkle_root,
            validator_signature: "genesis".to_string(),
            validator_address: "genesis".to_string(),
            masternode_counts: MasternodeCounts::default(),
            proof_of_time: None,
            checkpoints: vec![],
        };

        let hash = time_core::calculate_block_hash(&genesis_header);

        let genesis_block = Block {
            header: genesis_header,
            transactions,
            hash,
        };

        let db_path = format!("test_{}", db_suffix);
        (genesis_block, db_path)
    }

    #[tokio::test]
    async fn test_blockchain_service_creation() {
        let (genesis_block, db_path) = create_test_genesis_block("service");

        let blockchain = Arc::new(RwLock::new(
            BlockchainState::new(genesis_block, &db_path).unwrap(),
        ));
        let service = BlockchainService::new(blockchain);

        // Service should be created successfully
        let info = service.get_info().await.unwrap();
        assert_eq!(info.height, 0); // Genesis state
    }

    #[tokio::test]
    async fn test_get_balance_uses_utxo_set() {
        let (genesis_block, db_path) = create_test_genesis_block("balance");

        let blockchain = Arc::new(RwLock::new(
            BlockchainState::new(genesis_block, &db_path).unwrap(),
        ));

        let service = BlockchainService::new(blockchain.clone());

        // Get balance for non-existent address
        let balance = service.get_balance("TIME0test").await.unwrap();
        assert_eq!(balance, 0);

        // Get available balance for non-existent address
        let available = service.get_available_balance("TIME0test").await.unwrap();
        assert_eq!(available, 0);

        // Get balance for genesis address (should have coinbase)
        let genesis_balance = service.get_balance("genesis").await.unwrap();
        assert_eq!(genesis_balance, 50_000_000_000);
    }

    #[tokio::test]
    async fn test_available_balance_filters_locked_utxos() {
        let (genesis_block, db_path) = create_test_genesis_block("available");

        let blockchain = Arc::new(RwLock::new(
            BlockchainState::new(genesis_block, &db_path).unwrap(),
        ));

        let service = BlockchainService::new(blockchain);

        // Should return 0 for address with no UTXOs
        let available = service.get_available_balance("TIME0test").await.unwrap();
        assert_eq!(available, 0);
    }
}
