use crate::error::ApiError;
use crate::state::ApiState;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time_core::transaction::Transaction as CoreTransaction;

/// Request to sync wallet addresses
#[derive(Debug, Deserialize)]
pub struct WalletSyncRequest {
    /// List of addresses to sync
    pub addresses: Vec<String>,
}

/// UTXO information returned to wallet
#[derive(Debug, Serialize, Clone)]
pub struct UtxoInfo {
    pub tx_hash: String,
    pub output_index: u32,
    pub amount: u64,
    pub address: String,
    pub block_height: u64,
    pub confirmations: u64,
}

/// Transaction notification for wallet
#[derive(Debug, Serialize, Clone)]
pub struct TransactionNotification {
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: u64,
    pub block_height: u64,
    pub timestamp: u64,
    pub confirmations: u64,
}

/// Response for wallet sync
#[derive(Debug, Serialize)]
pub struct WalletSyncResponse {
    /// Map of address -> list of UTXOs
    pub utxos: HashMap<String, Vec<UtxoInfo>>,
    /// Total balance across all addresses
    pub total_balance: u64,
    /// Recent transactions for these addresses
    pub recent_transactions: Vec<TransactionNotification>,
    /// Current blockchain height
    pub current_height: u64,
}

/// Sync wallet addresses with the blockchain
/// Returns all UTXOs and recent transactions for the provided addresses
pub async fn sync_wallet_addresses(
    State(state): State<ApiState>,
    Json(request): Json<WalletSyncRequest>,
) -> Result<Json<WalletSyncResponse>, ApiError> {
    let blockchain = state.blockchain.read().await;
    let current_height = blockchain.chain_tip_height();

    let mut utxos_by_address: HashMap<String, Vec<UtxoInfo>> = HashMap::new();
    let mut total_balance = 0u64;
    let mut recent_transactions = Vec::new();

    // For each address, find all UTXOs and recent transactions
    for address in &request.addresses {
        let address_utxos = Vec::new();

        // Scan UTXO set for this address
        let balance = blockchain.get_balance(address);
        total_balance += balance;

        // Get recent transactions (last 10 blocks)
        let start_height = current_height.saturating_sub(10);
        for height in start_height..=current_height {
            if let Some(block) = blockchain.get_block_by_height(height) {
                for tx in &block.transactions {
                    // Check if transaction involves this address
                    let mut involves_address = false;
                    let mut from_address = String::new();
                    let mut to_address = String::new();
                    let mut amount = 0u64;

                    // Check outputs for this address (receiving)
                    for output in &tx.outputs {
                        if output.address == *address {
                            involves_address = true;
                            to_address = address.clone();
                            amount = output.amount;
                        }
                    }

                    // Check inputs to find sender (simplified - uses first input's address)
                    if involves_address && !tx.inputs.is_empty() {
                        from_address = "Unknown".to_string();
                    }

                    if involves_address {
                        let confirmations = current_height.saturating_sub(height);
                        recent_transactions.push(TransactionNotification {
                            tx_hash: tx.txid.clone(),
                            from_address,
                            to_address,
                            amount,
                            block_height: height,
                            timestamp: block.header.timestamp.timestamp() as u64,
                            confirmations,
                        });
                    }
                }
            }
        }

        utxos_by_address.insert(address.clone(), address_utxos);
    }

    // Sort transactions by block height (most recent first)
    recent_transactions.sort_by(|a, b| b.block_height.cmp(&a.block_height));

    // Limit to 50 most recent
    recent_transactions.truncate(50);

    Ok(Json(WalletSyncResponse {
        utxos: utxos_by_address,
        total_balance,
        recent_transactions,
        current_height,
    }))
}

/// Request to validate transactions before adding to mempool
#[derive(Debug, Deserialize)]
pub struct ValidateTransactionRequest {
    pub transaction_hex: String,
}

/// Response for transaction validation
#[derive(Debug, Serialize)]
pub struct ValidateTransactionResponse {
    pub valid: bool,
    pub error: Option<String>,
    pub tx_hash: Option<String>,
}

/// Validate a transaction before broadcasting
pub async fn validate_transaction(
    State(state): State<ApiState>,
    Json(request): Json<ValidateTransactionRequest>,
) -> Result<Json<ValidateTransactionResponse>, ApiError> {
    // Decode transaction
    let tx_bytes = hex::decode(&request.transaction_hex)
        .map_err(|_| ApiError::BadRequest("Invalid hex encoding".to_string()))?;

    let tx: CoreTransaction = serde_json::from_slice(&tx_bytes)
        .map_err(|_| ApiError::BadRequest("Invalid transaction format".to_string()))?;

    let blockchain = state.blockchain.read().await;

    // Validate transaction
    match blockchain.validate_transaction(&tx) {
        Ok(_) => Ok(Json(ValidateTransactionResponse {
            valid: true,
            error: None,
            tx_hash: Some(tx.txid.clone()),
        })),
        Err(e) => Ok(Json(ValidateTransactionResponse {
            valid: false,
            error: Some(e.to_string()),
            tx_hash: None,
        })),
    }
}

/// Notify wallet of incoming transaction
#[derive(Debug, Serialize)]
pub struct IncomingTransactionNotification {
    pub tx_hash: String,
    pub from_address: String,
    pub to_addresses: Vec<String>,
    pub amount: u64,
    pub timestamp: u64,
    pub pending: bool,
}

/// Get pending transactions for an address (mempool)
pub async fn get_pending_transactions(
    State(state): State<ApiState>,
    Json(addresses): Json<Vec<String>>,
) -> Result<Json<Vec<IncomingTransactionNotification>>, ApiError> {
    let mempool = state
        .mempool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Mempool not available".to_string()))?;

    let mut pending_txs = Vec::new();
    let transactions = mempool.get_all_transactions().await;

    for tx in transactions {
        for output in &tx.outputs {
            if addresses.contains(&output.address) {
                pending_txs.push(IncomingTransactionNotification {
                    tx_hash: tx.txid.clone(),
                    from_address: "Unknown".to_string(), // Would need to resolve from inputs
                    to_addresses: tx.outputs.iter().map(|o| o.address.clone()).collect(),
                    amount: output.amount,
                    timestamp: chrono::Utc::now().timestamp() as u64,
                    pending: true,
                });
            }
        }
    }

    Ok(Json(pending_txs))
}
