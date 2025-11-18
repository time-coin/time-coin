use crate::{ApiError, ApiResult, ApiState};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use time_core::transaction::{Transaction, TxInput, TxOutput, OutPoint};

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
    // Get the node's wallet address
    let wallet_address = if let Some(from) = req.from {
        from
    } else {
        state.wallet_address.clone()
    };

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
        .get_utxos_by_address(&wallet_address)
        .into_iter()
        .collect();

    if sender_utxos.is_empty() {
        return Err(ApiError::BadRequest(format!(
            "No UTXOs found for address {}",
            wallet_address
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
            signature: Vec::new(), // Will be signed later
            public_key: Vec::new(),
            sequence: 0xffffffff, // Default sequence
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
        outputs.push(TxOutput::new(change, wallet_address.clone()));
    }

    // Create transaction
    let mut tx = Transaction {
        txid: String::new(), // Will be calculated
        version: 1,
        inputs,
        outputs,
        lock_time: 0,
        timestamp: chrono::Utc::now().timestamp(),
    };

    // Calculate transaction ID
    tx.calculate_txid();
    let txid = tx.txid.clone();

    drop(blockchain);

    // Add to mempool
    if let Some(mempool) = state.mempool.as_ref() {
        mempool
            .add_transaction(tx.clone())
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to add to mempool: {}", e)))?;

        println!("📤 Transaction created and added to mempool:");
        println!("   From:   {}", wallet_address);
        println!("   To:     {}", req.to);
        println!("   Amount: {} TIME", req.amount);
        println!("   TxID:   {}", &txid[..16]);

        // Broadcast to network
        if let Some(broadcaster) = state.tx_broadcaster.as_ref() {
            broadcaster.broadcast_transaction(tx).await;
            println!("   📡 Broadcasting to network...");
        }

        Ok(Json(WalletSendResponse {
            success: true,
            txid,
            message: "Transaction created and broadcast to network".to_string(),
        }))
    } else {
        Err(ApiError::Internal("Mempool not initialized".to_string()))
    }
}
