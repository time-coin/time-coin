use crate::{ApiError, ApiResult, ApiState};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use time_core::transaction::{Transaction, TxInput, TxOutput};
use wallet::Wallet;

#[derive(Debug, Deserialize)]
pub struct WalletSendRequest {
    pub to: String,
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
    // Load the node's wallet to get keypair
    let wallet_path = std::env::var("WALLET_PATH")
        .unwrap_or_else(|_| "/var/lib/time-coin/wallets/node.json".to_string());
    
    let wallet = Wallet::load_from_file(&wallet_path)
        .map_err(|e| ApiError::Internal(format!("Failed to load wallet: {}", e)))?;

    let from_address = wallet.address_string();

    // Validate destination address format
    if !req.to.starts_with("TIME0") && !req.to.starts_with("TIME") {
        return Err(ApiError::BadRequest(
            "Invalid destination address format".to_string(),
        ));
    }

    // Check if amount is valid
    if req.amount == 0 {
        return Err(ApiError::BadRequest("Amount must be greater than 0".to_string()));
    }

    // Get blockchain state to find UTXOs
    let blockchain = state.blockchain.read().await;
    let utxo_set = blockchain.utxo_set();

    // Find UTXOs for the sender
    let sender_utxos: Vec<_> = utxo_set
        .get_utxos_by_address(&from_address)
        .into_iter()
        .collect();

    if sender_utxos.is_empty() {
        return Err(ApiError::BadRequest(format!(
            "No UTXOs found for address {}",
            from_address
        )));
    }

    // Calculate total available balance
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

    // Sign each input using the wallet's keypair
    // Use the transaction ID as the signing hash
    let tx_hash = hex::decode(&txid)
        .map_err(|e| ApiError::Internal(format!("Failed to decode txid: {}", e)))?;

    // Sign with keypair
    let keypair = wallet.keypair();
    let signature = keypair.sign(&tx_hash);
    let public_key = keypair.public_key_bytes().to_vec();

    // Apply signature to all inputs
    for input in &mut tx.inputs {
        input.signature = signature.clone();
        input.public_key = public_key.clone();
    }

    // Recalculate TXID after signing
    tx.txid = tx.calculate_txid();
    let final_txid = tx.txid.clone();

    // Add to mempool
    if let Some(mempool) = state.mempool.as_ref() {
        mempool
            .add_transaction(tx.clone())
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to add to mempool: {}", e)))?;

        println!("📤 Transaction created and added to mempool:");
        println!("   From:   {}", from_address);
        println!("   To:     {}", req.to);
        println!("   Amount: {} TIME", req.amount as f64 / 100_000_000.0);
        println!("   TxID:   {}", &final_txid[..16]);

        // Broadcast to network
        if let Some(broadcaster) = state.tx_broadcaster.as_ref() {
            broadcaster.broadcast_transaction(tx).await;
            println!("   📡 Broadcasting to network...");
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
