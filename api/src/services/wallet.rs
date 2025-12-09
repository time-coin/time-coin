//! Wallet service - encapsulates wallet and transaction operations

use crate::{ApiError, ApiResult};
use std::sync::Arc;
use time_core::state::BlockchainState;
use time_core::transaction::Transaction;
use tokio::sync::RwLock;

/// Service for wallet operations
#[derive(Clone)]
pub struct WalletService {
    blockchain: Arc<RwLock<BlockchainState>>,
}

impl WalletService {
    /// Create a new wallet service
    pub fn new(blockchain: Arc<RwLock<BlockchainState>>) -> Self {
        Self { blockchain }
    }

    /// Get wallet balance
    pub async fn get_wallet_balance(&self, address: &str) -> ApiResult<WalletBalanceInfo> {
        let blockchain = self.blockchain.read().await;
        let confirmed_balance = blockchain.get_balance(address);
        let utxo_set = blockchain.utxo_set();
        let utxo_manager = blockchain.utxo_state_manager();

        // Calculate available (unlocked) balance
        let mut available_balance = 0u64;
        for (outpoint, output) in utxo_set.get_utxos_by_address(address) {
            if output.address == address {
                let utxo_state = utxo_manager.get_state(&outpoint).await;
                let is_available = matches!(
                    utxo_state,
                    None | Some(time_core::utxo_state_manager::UTXOState::Unspent)
                        | Some(time_core::utxo_state_manager::UTXOState::Confirmed { .. })
                );
                if is_available {
                    available_balance = available_balance.saturating_add(output.amount);
                }
            }
        }

        Ok(WalletBalanceInfo {
            address: address.to_string(),
            confirmed_balance,
            available_balance,
            pending_balance: 0, // TODO: Calculate from mempool
        })
    }

    /// Check if address has sufficient balance
    pub async fn check_sufficient_balance(&self, address: &str, amount: u64) -> ApiResult<bool> {
        let blockchain = self.blockchain.read().await;
        let utxo_set = blockchain.utxo_set();
        let utxo_manager = blockchain.utxo_state_manager();

        // Check available (spendable) balance, not just total balance
        let mut available = 0u64;
        for (outpoint, output) in utxo_set.get_utxos_by_address(address) {
            if output.address == address {
                let utxo_state = utxo_manager.get_state(&outpoint).await;
                let is_available = matches!(
                    utxo_state,
                    None | Some(time_core::utxo_state_manager::UTXOState::Unspent)
                        | Some(time_core::utxo_state_manager::UTXOState::Confirmed { .. })
                );
                if is_available {
                    available = available.saturating_add(output.amount);
                }
            }
        }

        if available < amount {
            return Err(ApiError::InsufficientBalance {
                have: available,
                need: amount,
            });
        }

        Ok(true)
    }

    /// Validate transaction inputs
    pub async fn validate_transaction(&self, tx: &Transaction) -> ApiResult<bool> {
        let blockchain = self.blockchain.read().await;

        // Basic validation
        if tx.inputs.is_empty() {
            return Err(ApiError::BadRequest(
                "Transaction has no inputs".to_string(),
            ));
        }

        if tx.outputs.is_empty() {
            return Err(ApiError::BadRequest(
                "Transaction has no outputs".to_string(),
            ));
        }

        // Verify all inputs exist in UTXO set
        let utxo_set = blockchain.utxo_set();
        for input in &tx.inputs {
            if utxo_set.get(&input.previous_output).is_none() {
                return Err(ApiError::BadRequest(format!(
                    "Input references non-existent UTXO: {}:{}",
                    input.previous_output.txid, input.previous_output.vout
                )));
            }
        }

        // Calculate input and output amounts
        let input_sum: u64 = tx
            .inputs
            .iter()
            .filter_map(|input| utxo_set.get(&input.previous_output))
            .map(|output| output.amount)
            .sum();

        let output_sum: u64 = tx.outputs.iter().map(|output| output.amount).sum();

        if output_sum > input_sum {
            return Err(ApiError::BadRequest(format!(
                "Transaction outputs ({}) exceed inputs ({})",
                output_sum, input_sum
            )));
        }

        Ok(true)
    }

    /// Get UTXOs for wallet (helper for transaction creation)
    pub async fn get_wallet_utxos(
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

        if utxos.is_empty() {
            return Err(ApiError::BadRequest(format!(
                "No UTXOs available for address: {}",
                address
            )));
        }

        Ok(utxos)
    }
}

/// Wallet balance information
#[derive(Debug, Clone)]
pub struct WalletBalanceInfo {
    pub address: String,
    pub confirmed_balance: u64,
    pub available_balance: u64,
    pub pending_balance: u64,
}
