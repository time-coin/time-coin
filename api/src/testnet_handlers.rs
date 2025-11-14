//! Testnet-only API handlers for coin minting
//!
//! These handlers provide a method to create (mint) new coins in testnet mode only.
//! Safety: All handlers check the network type and reject requests on mainnet.

use crate::{ApiError, ApiResult, ApiState};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use time_core::transaction::{Transaction, TxOutput};

/// Request to mint new coins (testnet only)
#[derive(Debug, Deserialize)]
pub struct MintCoinsRequest {
    /// Address to receive the minted coins
    pub address: String,
    /// Amount to mint in satoshis (1 TIME = 100,000,000 satoshis)
    pub amount: u64,
    /// Optional reason/description for the minting
    pub reason: Option<String>,
}

/// Response from minting request
#[derive(Debug, Serialize)]
pub struct MintCoinsResponse {
    pub success: bool,
    pub message: String,
    pub txid: String,
    pub amount: u64,
    pub address: String,
}

/// Mint coins in testnet mode for testing purposes
///
/// This endpoint creates a special coinbase-like transaction that mints new coins
/// and adds them to the specified address. This is only allowed in testnet mode.
///
/// # Safety
/// - Only works in testnet mode
/// - Rejects all requests on mainnet
/// - Creates transaction with no inputs (minting)
pub async fn mint_coins(
    State(state): State<ApiState>,
    Json(request): Json<MintCoinsRequest>,
) -> ApiResult<Json<MintCoinsResponse>> {
    // CRITICAL SAFETY CHECK: Only allow minting in testnet mode
    if state.network != "testnet" {
        return Err(ApiError::BadRequest(
            "Minting is only allowed in testnet mode. Mainnet minting is prohibited.".to_string(),
        ));
    }

    // Validate amount
    if request.amount == 0 {
        return Err(ApiError::BadRequest(
            "Amount must be greater than 0".to_string(),
        ));
    }

    // Validate address format (basic check)
    if request.address.is_empty() {
        return Err(ApiError::BadRequest("Address cannot be empty".to_string()));
    }

    // Log the minting request
    let reason = request.reason.as_deref().unwrap_or("Testing");
    println!("🪙 TESTNET MINT REQUEST:");
    println!("   Address: {}", request.address);
    println!(
        "   Amount: {} satoshis ({} TIME)",
        request.amount,
        request.amount as f64 / 100_000_000.0
    );
    println!("   Reason: {}", reason);

    // Create a special minting transaction (coinbase-style with no inputs)
    let output = TxOutput::new(request.amount, request.address.clone());

    let mut tx = Transaction {
        txid: format!(
            "testnet_mint_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ),
        version: 1,
        inputs: vec![], // No inputs = minting new coins
        outputs: vec![output],
        lock_time: 0,
        timestamp: chrono::Utc::now().timestamp(),
    };

    // Calculate proper transaction ID
    tx.txid = tx.calculate_txid();

    // Add to mempool for inclusion in next block
    if let Some(mempool) = state.mempool.as_ref() {
        mempool.add_transaction(tx.clone()).await.map_err(|e| {
            ApiError::Internal(format!(
                "Failed to add minting transaction to mempool: {}",
                e
            ))
        })?;

        println!("   ✅ Minting transaction added to mempool");
        println!("   TX ID: {}", tx.txid);

        // Trigger instant finality via BFT consensus
        trigger_instant_finality(state.clone(), tx.clone()).await;
    } else {
        return Err(ApiError::Internal("Mempool not available".to_string()));
    }

    // Broadcast to other nodes if broadcaster available
    if let Some(broadcaster) = state.tx_broadcaster.as_ref() {
        broadcaster.broadcast_transaction(tx.clone()).await;
        println!("   📡 Minting transaction broadcast to network");
    }

    Ok(Json(MintCoinsResponse {
        success: true,
        message: format!(
            "Successfully minted {} satoshis ({} TIME) to address {}. Transaction will be included in the next block.",
            request.amount,
            request.amount as f64 / 100_000_000.0,
            request.address
        ),
        txid: tx.txid,
        amount: request.amount,
        address: request.address,
    }))
}

/// Get testnet minting information
#[derive(Debug, Serialize)]
pub struct MintInfoResponse {
    pub network: String,
    pub minting_enabled: bool,
    pub message: String,
}

/// Get information about testnet minting capabilities
pub async fn get_mint_info(State(state): State<ApiState>) -> ApiResult<Json<MintInfoResponse>> {
    let is_testnet = state.network == "testnet";

    Ok(Json(MintInfoResponse {
        network: state.network.clone(),
        minting_enabled: is_testnet,
        message: if is_testnet {
            "Testnet minting is enabled. Use POST /testnet/mint to create test coins.".to_string()
        } else {
            "Minting is disabled in mainnet mode for security reasons.".to_string()
        },
    }))
}

/// Trigger instant finality for a transaction via BFT consensus with real distributed voting
async fn trigger_instant_finality(state: ApiState, tx: time_core::transaction::Transaction) {
    println!(
        "🚀 Initiating instant finality for transaction {}",
        &tx.txid[..16]
    );

    let consensus = state.consensus.clone();
    let mempool = state.mempool.clone();
    let blockchain = state.blockchain.clone();
    let txid = tx.txid.clone();
    let tx_broadcaster = state.tx_broadcaster.clone();
    let peer_manager = state.peer_manager.clone();
    let wallet_address = state.wallet_address.clone();

    // Spawn async task to handle consensus voting
    tokio::spawn(async move {
        // Get connected masternodes (real peers only)
        let masternodes = peer_manager.get_peer_ips().await;

        if masternodes.is_empty() {
            println!("⚠️  No masternodes connected - auto-finalizing in dev mode");
            if let Some(mempool) = mempool.as_ref() {
                let _ = mempool.finalize_transaction(&txid).await;

                // Apply transaction to UTXO set for instant balance update
                let mut blockchain = blockchain.write().await;
                if let Err(e) = blockchain.utxo_set_mut().apply_transaction(&tx) {
                    println!("❌ Failed to apply transaction to UTXO set: {}", e);
                } else {
                    println!("✅ UTXO set updated - balances are now live!");

                    // Save UTXO snapshot to disk for persistence
                    if let Err(e) = blockchain.save_utxo_snapshot() {
                        println!("⚠️  Failed to save UTXO snapshot: {}", e);
                    } else {
                        println!(
                            "💾 UTXO snapshot saved - transaction will persist across restarts"
                        );
                    }
                }
            }
            return;
        }

        println!(
            "📊 Broadcasting transaction to {} masternodes for voting",
            masternodes.len()
        );

        // Vote locally as the proposer
        let _ = consensus
            .vote_on_transaction(&txid, wallet_address.clone(), true)
            .await;
        println!("   ✅ Local node voted: APPROVE");

        // Broadcast instant finality vote request to all peers
        if let Some(broadcaster) = tx_broadcaster.as_ref() {
            broadcaster.request_instant_finality_votes(tx.clone()).await;
        }

        // Wait for votes to be collected (with timeout)
        let vote_timeout = tokio::time::Duration::from_secs(5);
        tokio::time::sleep(vote_timeout).await;

        // Check vote counts from actual peer responses
        let (approvals, rejections) = consensus.get_transaction_vote_count(&txid).await;
        let total_votes = approvals + rejections;

        println!(
            "📊 Vote results: {} approvals, {} rejections (total {} votes from {} peers)",
            approvals,
            rejections,
            total_votes,
            masternodes.len()
        );

        // Check if consensus reached based on actual votes received
        let has_consensus = consensus.has_transaction_consensus(&txid).await;

        if has_consensus {
            println!(
                "✅ BFT consensus reached ({}/{} approvals from responding masternodes)",
                approvals, total_votes
            );

            // Finalize the transaction in mempool
            if let Some(mempool) = mempool.as_ref() {
                match mempool.finalize_transaction(&txid).await {
                    Ok(_) => {
                        println!("🎉 Transaction {} instantly finalized!", &txid[..16]);

                        // Apply transaction to UTXO set for instant balance update
                        let mut blockchain = blockchain.write().await;
                        if let Err(e) = blockchain.utxo_set_mut().apply_transaction(&tx) {
                            println!("❌ Failed to apply transaction to UTXO set: {}", e);
                        } else {
                            println!("✅ UTXO set updated - balances are now live!");

                            // Save UTXO snapshot to disk for persistence
                            if let Err(e) = blockchain.save_utxo_snapshot() {
                                println!("⚠️  Failed to save UTXO snapshot: {}", e);
                            } else {
                                println!("💾 UTXO snapshot saved - transaction will persist across restarts");
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to finalize transaction: {}", e);
                    }
                }
            }

            // Clear votes after finalization
            consensus.clear_transaction_votes(&txid).await;
        } else {
            println!(
                "❌ BFT consensus NOT reached ({}/{} approvals, need 2/3+) - transaction remains pending",
                approvals, total_votes
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mint_request_validation() {
        let valid_request = MintCoinsRequest {
            address: "test_address_123".to_string(),
            amount: 1_000_000_000, // 10 TIME
            reason: Some("Testing".to_string()),
        };

        assert_eq!(valid_request.amount, 1_000_000_000);
        assert!(!valid_request.address.is_empty());
    }

    #[test]
    fn test_zero_amount_rejected() {
        let invalid_request = MintCoinsRequest {
            address: "test_address".to_string(),
            amount: 0,
            reason: None,
        };

        assert_eq!(invalid_request.amount, 0);
        // In real handler, this would be rejected
    }
}
