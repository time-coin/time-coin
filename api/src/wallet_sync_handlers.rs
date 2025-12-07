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
        // Scan UTXO set for this address
        let utxo_entries = blockchain.utxo_set().get_utxos_for_address(address);

        let mut address_utxos = Vec::new();
        for (outpoint, output) in utxo_entries {
            // Find the block containing this transaction to get confirmations
            let mut block_height = 0u64;
            let mut confirmations = 0u64;

            // Search for transaction in recent blocks (optimization: could cache this)
            for height in (current_height.saturating_sub(1000))..=current_height {
                if let Some(block) = blockchain.get_block_by_height(height) {
                    if block.transactions.iter().any(|tx| tx.txid == outpoint.txid) {
                        block_height = height;
                        confirmations = current_height.saturating_sub(height);
                        break;
                    }
                }
            }

            address_utxos.push(UtxoInfo {
                tx_hash: outpoint.txid.clone(),
                output_index: outpoint.vout,
                amount: output.amount,
                address: address.clone(),
                block_height,
                confirmations,
            });

            total_balance += output.amount;
        }

        // Get balance for validation
        let _balance = blockchain.get_balance(address);

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

    // Save wallet balances to database for persistence
    for address in &request.addresses {
        let balance = blockchain.get_balance(address);
        if let Err(e) = blockchain.db().save_wallet_balance(address, balance) {
            eprintln!("⚠️  Failed to save wallet balance for {}: {}", address, e);
        }
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

/// Simple balance query by xpub
#[derive(Debug, Serialize)]
pub struct BalanceResponse {
    pub confirmed: u64,
    pub pending: u64,
    pub total: u64,
}

/// Get balance for an xpub (thin client endpoint)
pub async fn get_xpub_balance(
    State(state): State<ApiState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Json<BalanceResponse>, ApiError> {
    let xpub = params
        .get("xpub")
        .ok_or_else(|| ApiError::BadRequest("Missing xpub parameter".to_string()))?;

    // Use existing sync logic to get balance
    let sync_request = WalletSyncXpubRequest {
        xpub: xpub.clone(),
        start_index: 0,
    };

    let sync_response = sync_wallet_xpub(State(state.clone()), Json(sync_request)).await?;

    // Calculate pending balance from mempool
    let mempool = state.mempool.as_ref();
    let pending = if let Some(mempool) = mempool {
        let transactions = mempool.get_all_transactions().await;
        transactions
            .iter()
            .flat_map(|tx| &tx.outputs)
            .filter(|output| {
                // Check if output address belongs to any address from xpub
                sync_response.utxos.contains_key(&output.address)
            })
            .map(|output| output.amount)
            .sum()
    } else {
        0u64
    };

    Ok(Json(BalanceResponse {
        confirmed: sync_response.total_balance,
        pending,
        total: sync_response.total_balance + pending,
    }))
}

/// Get transaction list for xpub (thin client endpoint)
pub async fn get_xpub_transactions(
    State(state): State<ApiState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Json<Vec<TransactionNotification>>, ApiError> {
    let xpub = params
        .get("xpub")
        .ok_or_else(|| ApiError::BadRequest("Missing xpub parameter".to_string()))?;

    let limit: usize = params
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    // Use existing sync logic to get transactions
    let sync_request = WalletSyncXpubRequest {
        xpub: xpub.clone(),
        start_index: 0,
    };

    let sync_response = sync_wallet_xpub(State(state), Json(sync_request)).await?;

    // Return transactions limited by the requested amount
    let mut transactions = sync_response.0.recent_transactions.clone();
    transactions.truncate(limit);

    Ok(Json(transactions))
}

/// Get UTXOs for xpub (thin client endpoint)  
pub async fn get_xpub_utxos(
    State(state): State<ApiState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Json<Vec<UtxoInfo>>, ApiError> {
    let xpub = params
        .get("xpub")
        .ok_or_else(|| ApiError::BadRequest("Missing xpub parameter".to_string()))?;

    // Use existing sync logic to get UTXOs
    let sync_request = WalletSyncXpubRequest {
        xpub: xpub.clone(),
        start_index: 0,
    };

    let sync_response = sync_wallet_xpub(State(state), Json(sync_request)).await?;

    // Flatten all UTXOs into a single list
    let all_utxos: Vec<UtxoInfo> = sync_response
        .0
        .utxos
        .clone()
        .into_values()
        .flatten()
        .collect();

    Ok(Json(all_utxos))
}

/// Broadcast transaction request (thin client endpoint)
#[derive(Debug, Deserialize)]
pub struct BroadcastTransactionRequest {
    pub tx: String, // Hex-encoded transaction
}

/// Broadcast transaction response
#[derive(Debug, Serialize)]
pub struct BroadcastTransactionResponse {
    pub txid: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Broadcast a signed transaction to the network
pub async fn broadcast_transaction(
    State(state): State<ApiState>,
    Json(request): Json<BroadcastTransactionRequest>,
) -> Result<Json<BroadcastTransactionResponse>, ApiError> {
    // Decode transaction
    let tx_bytes = hex::decode(&request.tx)
        .map_err(|_| ApiError::BadRequest("Invalid hex encoding".to_string()))?;

    let tx: CoreTransaction = serde_json::from_slice(&tx_bytes)
        .map_err(|_| ApiError::BadRequest("Invalid transaction format".to_string()))?;

    let txid = tx.txid.clone();

    // Validate transaction
    let blockchain = state.blockchain.read().await;
    if let Err(e) = blockchain.validate_transaction(&tx) {
        return Ok(Json(BroadcastTransactionResponse {
            txid: txid.clone(),
            success: false,
            error: Some(format!("Validation failed: {}", e)),
        }));
    }
    drop(blockchain);

    // Add to mempool
    if let Some(mempool) = &state.mempool {
        mempool
            .add_transaction(tx)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to add to mempool: {}", e)))?;

        Ok(Json(BroadcastTransactionResponse {
            txid,
            success: true,
            error: None,
        }))
    } else {
        Ok(Json(BroadcastTransactionResponse {
            txid,
            success: false,
            error: Some("Mempool not available".to_string()),
        }))
    }
}

/// Register xpub for real-time transaction monitoring
#[derive(Debug, Deserialize)]
pub struct RegisterXpubRequest {
    pub xpub: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterXpubResponse {
    pub success: bool,
    pub message: String,
}

/// Register an xpub with the masternode for real-time UTXO tracking
/// Note: This endpoint is optional and only works when address monitor is configured
pub async fn register_xpub(
    State(_state): State<ApiState>,
    Json(request): Json<RegisterXpubRequest>,
) -> Result<Json<RegisterXpubResponse>, ApiError> {
    tracing::info!(
        "📝 Xpub registration request: {}...",
        &request.xpub[..std::cmp::min(20, request.xpub.len())]
    );

    // TODO: Integrate with address monitor when running as masternode
    // For now, just acknowledge receipt
    // The actual monitoring will be set up through the masternode's
    // UTXO integration layer

    Ok(Json(RegisterXpubResponse {
        success: true,
        message: "Xpub registration acknowledged".to_string(),
    }))
}

/// Request to sync wallet using xpub (deterministic address discovery)
#[derive(Debug, Deserialize)]
pub struct WalletSyncXpubRequest {
    /// Extended public key for deriving addresses
    pub xpub: String,
    /// Optional: starting index (default 0)
    #[serde(default)]
    pub start_index: u32,
}

/// Sync wallet using extended public key (xpub)
/// Automatically discovers all used addresses using gap limit
pub async fn sync_wallet_xpub(
    State(state): State<ApiState>,
    Json(request): Json<WalletSyncXpubRequest>,
) -> Result<Json<WalletSyncResponse>, ApiError> {
    const GAP_LIMIT: u32 = 20; // BIP-44 standard gap limit
    const MAX_ADDRESSES: u32 = 1000; // Safety limit to prevent infinite loops

    let blockchain = state.blockchain.read().await;
    let current_height = blockchain.chain_tip_height();

    let mut utxos_by_address: HashMap<String, Vec<UtxoInfo>> = HashMap::new();
    let mut total_balance = 0u64;
    let mut recent_transactions = Vec::new();
    let mut gap_count = 0u32;
    let mut index = request.start_index;

    tracing::info!("Starting xpub scan from index {}", index);

    // Derive addresses and scan until gap limit reached
    while gap_count < GAP_LIMIT && index < MAX_ADDRESSES {
        // Derive address at this index
        // change=0 for receiving addresses
        // Determine network type from state
        let network = if state.network.to_lowercase() == "testnet" {
            wallet::NetworkType::Testnet
        } else {
            wallet::NetworkType::Mainnet
        };

        let address = match wallet::xpub_to_address(&request.xpub, 0, index, network) {
            Ok(addr) => addr,
            Err(e) => {
                tracing::error!("Failed to derive address at index {}: {}", index, e);
                return Err(ApiError::BadRequest(format!(
                    "Failed to derive address: {}",
                    e
                )));
            }
        };

        tracing::debug!("Scanning address {} at index {}", address, index);

        // Check if this address has any transactions
        let balance = blockchain.get_balance(&address);
        let has_activity = balance > 0;

        if has_activity {
            tracing::info!(
                "Found activity at index {}: {} with balance {}",
                index,
                address,
                balance
            );
            gap_count = 0; // Reset gap counter

            // Get UTXOs for this address
            let utxo_entries = blockchain.utxo_set().get_utxos_for_address(&address);
            let mut address_utxos = Vec::new();

            for (outpoint, output) in utxo_entries {
                // Find block height for this UTXO
                let mut block_height = 0u64;
                let mut confirmations = 0u64;

                for height in (current_height.saturating_sub(1000))..=current_height {
                    if let Some(block) = blockchain.get_block_by_height(height) {
                        if block.transactions.iter().any(|tx| tx.txid == outpoint.txid) {
                            block_height = height;
                            confirmations = current_height.saturating_sub(height);
                            break;
                        }
                    }
                }

                address_utxos.push(UtxoInfo {
                    tx_hash: outpoint.txid.clone(),
                    output_index: outpoint.vout,
                    amount: output.amount,
                    address: address.clone(),
                    block_height,
                    confirmations,
                });

                total_balance += output.amount;
            }

            // Scan recent transactions for this address
            let start_height = current_height.saturating_sub(100);
            for height in start_height..=current_height {
                if let Some(block) = blockchain.get_block_by_height(height) {
                    for tx in &block.transactions {
                        let mut involves_address = false;
                        let from_address = String::new();
                        let mut to_address = String::new();
                        let mut amount = 0u64;

                        // Check outputs for this address
                        for output in &tx.outputs {
                            if output.address == address {
                                involves_address = true;
                                to_address = address.clone();
                                amount = output.amount;
                            }
                        }

                        if involves_address {
                            let confirmations = current_height.saturating_sub(height);
                            recent_transactions.push(TransactionNotification {
                                tx_hash: tx.txid.clone(),
                                from_address: from_address.clone(),
                                to_address: to_address.clone(),
                                amount,
                                block_height: height,
                                timestamp: block.header.timestamp.timestamp() as u64,
                                confirmations,
                            });
                        }
                    }
                }
            }

            utxos_by_address.insert(address, address_utxos);
        } else {
            gap_count += 1;
            tracing::debug!("No activity at index {}, gap count: {}", index, gap_count);
        }

        index += 1;
    }

    tracing::info!(
        "xpub scan complete: checked {} addresses, found {} with activity",
        index - request.start_index,
        utxos_by_address.len()
    );

    // Sort transactions by block height (most recent first)
    recent_transactions.sort_by(|a, b| b.block_height.cmp(&a.block_height));
    recent_transactions.truncate(50);

    Ok(Json(WalletSyncResponse {
        utxos: utxos_by_address,
        total_balance,
        recent_transactions,
        current_height,
    }))
}
