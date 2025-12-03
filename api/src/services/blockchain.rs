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

    #[tokio::test]
    async fn test_blockchain_service_creation() {
        let blockchain = Arc::new(RwLock::new(BlockchainState::new("test".to_string())));
        let service = BlockchainService::new(blockchain);

        // Service should be created successfully
        let info = service.get_info().await.unwrap();
        assert_eq!(info.height, 0); // Genesis state
    }
}
