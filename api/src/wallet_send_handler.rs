use crate::{ApiError, ApiResult, ApiState};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use time_core::transaction::{Transaction, TxInput, TxOutput};
use time_core::UTXOState;
use tracing as log;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct WalletSendRequest {
    #[validate(length(min = 1, message = "Recipient address cannot be empty"))]
    pub to: String,
    #[validate(range(min = 1, message = "Amount must be greater than 0"))]
    pub amount: u64,
    #[serde(default)]
    pub from: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WalletSendResponse {
    pub success: bool,
    pub txid: String,
    pub message: String,
}

/// Send TIME coins from the node's wallet
pub async fn wallet_send(
    State(state): State<ApiState>,
    Json(req): Json<WalletSendRequest>,
) -> ApiResult<Json<WalletSendResponse>> {
    // Validate request
    req.validate()
        .map_err(|e| ApiError::BadRequest(format!("Validation failed: {}", e)))?;

    // Validate destination address format
    if !req.to.starts_with("TIME0") && !req.to.starts_with("TIME") {
        return Err(ApiError::BadRequest(
            "Invalid destination address format".to_string(),
        ));
    }

    // Get the node's wallet address from config
    let from_address = state.wallet_address.clone();

    // Get blockchain state to find UTXOs
    let blockchain = state.blockchain.read().await;
    let utxo_set = blockchain.utxo_set();

    // Find UTXOs for the sender
    let all_utxos: Vec<_> = utxo_set
        .get_utxos_by_address(&from_address)
        .into_iter()
        .collect();

    if all_utxos.is_empty() {
        return Err(ApiError::BadRequest(format!(
            "No UTXOs found for address {}",
            from_address
        )));
    }

    // ⚡ INSTANT FINALITY: Filter out locked/spent UTXOs
    let utxo_manager = blockchain.utxo_state_manager();
    let mut sender_utxos = Vec::new();

    for (outpoint, output) in all_utxos {
        let utxo_state = utxo_manager.get_state(&outpoint).await;
        let is_available = match utxo_state {
            Some(UTXOState::Unspent) | None => true, // Unspent or no tracking
            Some(_) => false,                        // Locked, Spent, etc.
        };

        if is_available {
            sender_utxos.push((outpoint, output));
        } else if let Some(state) = utxo_state {
            log::debug!(
                txid = %outpoint.txid,
                vout = outpoint.vout,
                state = ?state,
                "skipping_locked_or_spent_utxo"
            );
        }
    }

    if sender_utxos.is_empty() {
        return Err(ApiError::BadRequest(format!(
            "No unspent UTXOs available for address {} (all UTXOs are locked or spent)",
            from_address
        )));
    }

    // Calculate total available balance (only unspent UTXOs)
    let total_balance: u64 = sender_utxos.iter().map(|(_, output)| output.amount).sum();

    if total_balance < req.amount {
        return Err(ApiError::BadRequest(format!(
            "Insufficient balance. Available: {}, Required: {}",
            total_balance, req.amount
        )));
    }

    // Select UTXOs to cover the amount (simple greedy algorithm)
    let mut inputs = Vec::new();
    let mut input_total = 0u64;

    for (outpoint, output) in sender_utxos {
        inputs.push(TxInput {
            previous_output: outpoint.clone(),
            signature: Vec::new(), // Will be signed
            public_key: Vec::new(),
            sequence: 0xffffffff,
        });
        input_total += output.amount;

        if input_total >= req.amount {
            break;
        }
    }

    // Create outputs
    let mut outputs = vec![TxOutput::new(req.amount, req.to.clone())];

    // Add change output if necessary
    if input_total > req.amount {
        let change = input_total - req.amount;
        outputs.push(TxOutput::new(change, from_address.clone()));
    }

    // Create transaction with calculated TXID
    let txid = {
        let tx_temp = Transaction {
            txid: String::new(),
            version: 1,
            inputs: inputs.clone(),
            outputs: outputs.clone(),
            lock_time: 0,
            timestamp: chrono::Utc::now().timestamp(),
        };
        tx_temp.calculate_txid()
    };

    let mut tx = Transaction {
        txid: txid.clone(),
        version: 1,
        inputs,
        outputs,
        lock_time: 0,
        timestamp: chrono::Utc::now().timestamp(),
    };

    drop(blockchain);

    // Load the node's wallet to get the keypair for signing
    let wallet_path = format!("{}/wallets/node.json", state.data_dir);
    let wallet = wallet::Wallet::load_from_file(&wallet_path)
        .map_err(|e| ApiError::Internal(format!("Failed to load wallet: {}", e)))?;

    // Get keypair to use for signing
    let keypair = wallet.keypair();
    let public_key = keypair.public_key_bytes().to_vec();

    log::debug!(
        public_key_len = public_key.len(),
        public_key_hex = %hex::encode(&public_key),
        "signing_transaction"
    );

    // Set public keys on all inputs BEFORE calculating signing hash
    for input in &mut tx.inputs {
        input.public_key = public_key.clone();
    }

    // Recalculate TXID after setting public keys (TXID includes public keys)
    tx.txid = tx.calculate_txid();
    log::debug!(txid = %&tx.txid[..16], "updated_txid");

    // Now calculate the signing hash with public_keys set
    // This must match how mempool calculates it
    let tx_hash = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();

        // Hash transaction fields (excluding signatures)
        hasher.update(tx.txid.as_bytes());
        hasher.update(tx.version.to_le_bytes());

        for input in &tx.inputs {
            hasher.update(input.previous_output.txid.as_bytes());
            hasher.update(input.previous_output.vout.to_le_bytes());
            hasher.update(&input.public_key); // Now has the real public key!
            hasher.update(input.sequence.to_le_bytes());
        }

        for output in &tx.outputs {
            hasher.update(output.address.as_bytes());
            hasher.update(output.amount.to_le_bytes());
        }

        hasher.update(tx.lock_time.to_le_bytes());
        hasher.update(tx.timestamp.to_le_bytes());

        hasher.finalize().to_vec()
    };

    log::debug!(signing_hash = %hex::encode(&tx_hash), "calculated_signing_hash");

    // Sign the hash
    let signature = keypair.sign(&tx_hash);
    log::debug!(
        signature_len = signature.len(),
        signature_hex = %hex::encode(&signature[..16]),
        "transaction_signed"
    );

    // Apply signatures to all inputs
    for input in &mut tx.inputs {
        input.signature = signature.clone();
    }

    // DO NOT recalculate TXID after signing - TXID should not include signatures
    let final_txid = tx.txid.clone();

    // Add to mempool
    if let Some(mempool) = state.mempool.as_ref() {
        log::debug!("adding_transaction_to_mempool");
        match mempool.add_transaction(tx.clone()).await {
            Ok(_) => log::info!(txid = %final_txid, "transaction_added_to_mempool"),
            Err(e) => {
                log::error!(txid = %final_txid, error = %e, "failed_to_add_to_mempool");
                return Err(ApiError::Internal(format!(
                    "Failed to add to mempool: {}",
                    e
                )));
            }
        }

        log::info!(
            from = %from_address,
            to = %req.to,
            amount = req.amount,
            txid = %&final_txid[..16],
            "transaction_created_and_added_to_mempool"
        );

        // Trigger instant finality via BFT consensus
        log::debug!("triggering_instant_finality");
        crate::routes::mempool::trigger_instant_finality_for_received_tx(state.clone(), tx.clone())
            .await;

        // Broadcast to network
        if let Some(broadcaster) = state.tx_broadcaster.as_ref() {
            broadcaster.broadcast_transaction(tx).await;
            log::debug!("transaction_broadcast_to_network");
        }

        Ok(Json(WalletSendResponse {
            success: true,
            txid: final_txid,
            message: "Transaction created and broadcast to network".to_string(),
        }))
    } else {
        Err(ApiError::Internal("Mempool not initialized".to_string()))
    }
}
