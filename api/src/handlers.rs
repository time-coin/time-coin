//! API Request Handlers

use crate::{
    error::{ApiError, ApiResult},
    models::*,
    state::{ApiState, TransactionData},
};
use serde_json::json;
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use time_crypto::KeyPair;

// ============================================
// Health Check
// ============================================

pub async fn health_check(State(state): State<ApiState>) -> ApiResult<Json<HealthResponse>> {
    Ok(Json(HealthResponse {
        status: "healthy".to_string(),
        version: "0.1.0".to_string(),
        uptime: state.start_time.elapsed().as_secs(),
        dev_mode: state.dev_mode,
    }))
}

// ============================================
// Transaction Handlers
// ============================================

pub async fn create_transaction(
    State(state): State<ApiState>,
    Json(req): Json<CreateTransactionRequest>,
) -> ApiResult<Json<CreateTransactionResponse>> {
    // Validate addresses
    if !req.from.starts_with("TIME1") || !req.to.starts_with("TIME1") {
        return Err(ApiError::InvalidAddress(
            "Address must start with TIME1".to_string(),
        ));
    }
    
    // SECURITY: Protected addresses require admin authorization
    let protected_addresses = [
        "TIME1treasury00000000000000000000000000",
        "TIME1development0000000000000000000000",
        "TIME1operations0000000000000000000000",
        "TIME1rewards000000000000000000000000000",
    ];
    
    if protected_addresses.contains(&req.from.as_str()) {
        // Check admin token authorization
        if let Some(token) = &state.admin_token {
            // TODO: Extract Authorization header from request
            // For now, only allow in dev_mode
            if !state.dev_mode {
                return Err(ApiError::Unauthorized(
                    "Protected address requires admin authorization token".to_string(),
                ));
            }
        } else if !state.dev_mode {
            return Err(ApiError::Unauthorized(
                "Protected address spending disabled (no admin token configured)".to_string(),
            ));
        }
    }

    // Check balance
    let balances = state.balances.read().await;
    let balance = balances.get(&req.from).copied().unwrap_or(0);
    let total_needed = req.amount + req.fee;

    if balance < total_needed {
        return Err(ApiError::InsufficientBalance {
            have: balance,
            need: total_needed,
        });
    }
    drop(balances);

    // Create transaction ID
    let txid = uuid::Uuid::new_v4().to_string();

    // In dev mode, auto-approve
    if state.dev_mode {
        // Update balances
        let mut balances = state.balances.write().await;
        *balances.entry(req.from.clone()).or_insert(0) -= total_needed;
        *balances.entry(req.to.clone()).or_insert(0) += req.amount;
        drop(balances);

        // Store transaction
        let mut txs = state.transactions.write().await;
        txs.insert(
            txid.clone(),
            TransactionData {
                txid: txid.clone(),
                from: req.from,
                to: req.to,
                amount: req.amount,
                fee: req.fee,
                timestamp: Utc::now().timestamp(),
                status: "confirmed".to_string(),
            },
        );

        Ok(Json(CreateTransactionResponse {
            txid,
            status: "confirmed".to_string(),
            message: "Transaction confirmed (dev mode)".to_string(),
        }))
    } else {
        // Broadcast to network for instant validation
        let tx_msg = time_network::TransactionMessage {
            txid: txid.clone(),
            from: req.from.clone(),
            to: req.to.clone(),
            amount: req.amount,
            fee: req.fee,
            timestamp: Utc::now().timestamp(),
            signature: req.private_key.clone(), // TODO: Proper signature
            nonce: 0, // TODO: Implement nonce tracking
        };
        
        match state.peer_manager.broadcast_transaction(tx_msg).await {
            Ok(peer_count) => {
                // For testnet with 2 nodes: accept immediately
                // TODO: Wait for 67% validation in production
                let mut balances = state.balances.write().await;
                *balances.entry(req.from.clone()).or_insert(0) -= total_needed;
                *balances.entry(req.to.clone()).or_insert(0) += req.amount;
                drop(balances);
                
                let mut txs = state.transactions.write().await;
                txs.insert(
                    txid.clone(),
                    TransactionData {
                        txid: txid.clone(),
                        from: req.from,
                        to: req.to,
                        amount: req.amount,
                        fee: req.fee,
                        timestamp: Utc::now().timestamp(),
                        status: "confirmed".to_string(),
                    },
                );
                
                Ok(Json(CreateTransactionResponse {
                    txid,
                    status: "confirmed".to_string(),
                    message: format!("Transaction confirmed instantly! Broadcast to {} peer(s)", peer_count),
                }))
            }
            Err(e) => Err(ApiError::Internal(format!("Broadcast failed: {}", e))),
        }
    }
}

pub async fn get_transaction(
    State(state): State<ApiState>,
    Path(txid): Path<String>,
) -> ApiResult<Json<TransactionStatusResponse>> {
    let txs = state.transactions.read().await;

    if let Some(tx) = txs.get(&txid) {
        Ok(Json(TransactionStatusResponse {
            txid: tx.txid.clone(),
            status: tx.status.clone(),
            from: tx.from.clone(),
            to: tx.to.clone(),
            amount: tx.amount,
            fee: tx.fee,
            timestamp: tx.timestamp,
            confirmations: if tx.status == "confirmed" { 1 } else { 0 },
        }))
    } else {
        Err(ApiError::TransactionNotFound(txid))
    }
}

// ============================================
// Balance Handlers
// ============================================

pub async fn get_balance(
    State(state): State<ApiState>,
    Path(address): Path<String>,
) -> ApiResult<Json<BalanceResponse>> {
    if !address.starts_with("TIME1") {
        return Err(ApiError::InvalidAddress(
            "Address must start with TIME1".to_string(),
        ));
    }

    let balances = state.balances.read().await;
    let balance = balances.get(&address).copied().unwrap_or(0);

    Ok(Json(BalanceResponse {
        address,
        balance,
        balance_time: format!("{:.2} TIME", balance as f64 / 100_000_000.0),
        pending: 0,
    }))
}

// ============================================
// Blockchain Handlers
// ============================================

pub async fn get_blockchain_info(
    State(state): State<ApiState>,
) -> ApiResult<Json<BlockchainInfoResponse>> {
    let balances = state.balances.read().await;
    let total_supply: u64 = balances.values().sum();

    Ok(Json(BlockchainInfoResponse {
        network: state.network.clone(),
        height: 0, // Genesis block
        best_block_hash: "00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048"
            .to_string(),
        total_supply,
        timestamp: Utc::now().timestamp(),
    }))
}

// ============================================
// Keypair Handlers
// ============================================

pub async fn generate_keypair() -> ApiResult<Json<GenerateKeypairResponse>> {
    let keypair = KeyPair::generate();
    let address = time_crypto::public_key_to_address(&keypair.public_key_hex());

    Ok(Json(GenerateKeypairResponse {
        address,
        public_key: keypair.public_key_hex(),
        private_key: keypair.private_key_hex(),
        warning: "⚠️  NEVER share your private key! Store it securely!".to_string(),
    }))
}




/// Get connected peers info (similar to bitcoin-cli getpeerinfo)
pub async fn get_peers(State(state): State<ApiState>) -> ApiResult<Json<serde_json::Value>> {
    let peers = state.peer_manager.get_connected_peers().await;
    
    // Deduplicate by address, keeping the most recent
    let mut unique_peers: std::collections::HashMap<String, time_network::discovery::PeerInfo> = std::collections::HashMap::new();
    for peer in peers {
        let addr = peer.address.to_string();
        unique_peers.entry(addr.clone())
            .and_modify(|existing: &mut _| {
                if peer.last_seen > existing.last_seen {
                    *existing = peer.clone();
                }
            })
            .or_insert(peer);
    }
    
    let peer_info: Vec<serde_json::Value> = unique_peers
        .values()
        .map(|peer| {
            json!({
                "addr": peer.address.to_string(),
                "ip": peer.address.ip().to_string(),
                "port": peer.address.port(),
                "version": peer.version,
                "network": format!("{:?}", peer.network),
                "lastseen": peer.last_seen,
                "connected": true
            })
        })
        .collect();
    
    Ok(Json(json!({
        "peers": peer_info,
        "count": peer_info.len()
    })))
}
