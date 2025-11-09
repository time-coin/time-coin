//! Bitcoin RPC-compatible handlers for Time Coin
//! 
//! This module implements Bitcoin RPC-style endpoints to provide
//! familiar interfaces for developers and tools.

use crate::{ApiError, ApiResult, ApiState};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// BLOCKCHAIN RPC METHODS
// ============================================================================

/// Response for getblockchaininfo RPC
#[derive(Serialize)]
pub struct BlockchainInfo {
    pub chain: String,
    pub blocks: u64,
    pub headers: u64,
    pub bestblockhash: String,
    pub difficulty: f64,
    pub mediantime: i64,
    pub verificationprogress: f64,
    pub initialblockdownload: bool,
    pub chainwork: String,
    pub size_on_disk: u64,
    pub pruned: bool,
}

pub async fn getblockchaininfo(
    State(state): State<ApiState>,
) -> ApiResult<Json<BlockchainInfo>> {
    let blockchain = state.blockchain.read().await;
    let height = blockchain.chain_tip_height();
    let best_hash = blockchain.chain_tip_hash().to_string();
    
    Ok(Json(BlockchainInfo {
        chain: state.network.clone(),
        blocks: height,
        headers: height,
        bestblockhash: best_hash,
        difficulty: 1.0, // TIME uses BFT, not PoW difficulty
        mediantime: chrono::Utc::now().timestamp(),
        verificationprogress: 1.0,
        initialblockdownload: false,
        chainwork: format!("{:064x}", height), // Simplified chainwork
        size_on_disk: 0, // TODO: Calculate actual size
        pruned: false,
    }))
}

/// Response for getblockcount RPC
#[derive(Serialize)]
pub struct BlockCount {
    pub result: u64,
}

pub async fn getblockcount(State(state): State<ApiState>) -> ApiResult<Json<BlockCount>> {
    let blockchain = state.blockchain.read().await;
    Ok(Json(BlockCount {
        result: blockchain.chain_tip_height(),
    }))
}

/// Response for getblockhash RPC
#[derive(Serialize)]
pub struct BlockHash {
    pub result: String,
}

#[derive(Deserialize)]
pub struct BlockHeightParam {
    pub height: u64,
}

pub async fn getblockhash(
    State(state): State<ApiState>,
    Json(params): Json<BlockHeightParam>,
) -> ApiResult<Json<BlockHash>> {
    let blockchain = state.blockchain.read().await;
    
    match blockchain.get_block_by_height(params.height) {
        Some(block) => Ok(Json(BlockHash {
            result: block.hash.clone(),
        })),
        None => Err(ApiError::TransactionNotFound(format!(
            "Block not found at height {}",
            params.height
        ))),
    }
}

/// Response for getblock RPC
#[derive(Serialize)]
pub struct GetBlockResponse {
    pub hash: String,
    pub confirmations: u64,
    pub height: u64,
    pub version: u32,
    pub merkleroot: String,
    pub time: i64,
    pub mediantime: i64,
    pub nonce: u64,
    pub bits: String,
    pub difficulty: f64,
    pub chainwork: String,
    pub tx: Vec<String>,
    pub previousblockhash: String,
    pub nextblockhash: Option<String>,
}

#[derive(Deserialize)]
pub struct GetBlockParams {
    pub blockhash: String,
    #[serde(default)]
    pub verbosity: u32, // 0=hex, 1=json, 2=json with tx details
}

pub async fn getblock(
    State(state): State<ApiState>,
    Json(params): Json<GetBlockParams>,
) -> ApiResult<Json<GetBlockResponse>> {
    let blockchain = state.blockchain.read().await;
    let tip_height = blockchain.chain_tip_height();
    
    // Find block by hash
    let mut found_block = None;
    for height in 0..=tip_height {
        if let Some(block) = blockchain.get_block_by_height(height) {
            if block.hash == params.blockhash {
                found_block = Some((block.clone(), height));
                break;
            }
        }
    }
    
    match found_block {
        Some((block, height)) => {
            let confirmations = tip_height - height + 1;
            let tx_ids: Vec<String> = block.transactions.iter().map(|tx| tx.txid.clone()).collect();
            
            // Get next block hash if not at tip
            let next_hash = if height < tip_height {
                blockchain.get_block_by_height(height + 1).map(|b| b.hash.clone())
            } else {
                None
            };
            
            Ok(Json(GetBlockResponse {
                hash: block.hash.clone(),
                confirmations,
                height,
                version: 1,
                merkleroot: block.header.merkle_root.clone(),
                time: block.header.timestamp.timestamp(),
                mediantime: block.header.timestamp.timestamp(),
                nonce: 0, // TIME uses BFT, not PoW nonce
                bits: "00000000".to_string(), // No PoW bits in BFT
                difficulty: 1.0,
                chainwork: format!("{:064x}", height),
                tx: tx_ids,
                previousblockhash: block.header.previous_hash.clone(),
                nextblockhash: next_hash,
            }))
        }
        None => Err(ApiError::TransactionNotFound(format!(
            "Block not found: {}",
            params.blockhash
        ))),
    }
}

// ============================================================================
// TRANSACTION RPC METHODS
// ============================================================================

/// Response for getrawtransaction RPC
#[derive(Serialize)]
pub struct RawTransaction {
    pub hex: String,
    pub txid: String,
    pub hash: String,
    pub size: usize,
    pub vsize: usize,
    pub version: u32,
    pub locktime: u32,
    pub vin: Vec<TxInputInfo>,
    pub vout: Vec<TxOutputInfo>,
    pub blockhash: Option<String>,
    pub confirmations: Option<u64>,
    pub time: Option<i64>,
    pub blocktime: Option<i64>,
}

#[derive(Serialize)]
pub struct TxInputInfo {
    pub txid: String,
    pub vout: u32,
    pub scriptSig: ScriptSig,
    pub sequence: u32,
}

#[derive(Serialize)]
pub struct ScriptSig {
    pub asm: String,
    pub hex: String,
}

#[derive(Serialize)]
pub struct TxOutputInfo {
    pub value: f64, // In TIME coins
    pub n: usize,
    pub scriptPubKey: ScriptPubKey,
}

#[derive(Serialize)]
pub struct ScriptPubKey {
    pub asm: String,
    pub hex: String,
    #[serde(rename = "type")]
    pub script_type: String,
    pub address: String,
}

#[derive(Deserialize)]
pub struct GetRawTransactionParams {
    pub txid: String,
    #[serde(default)]
    pub verbose: bool,
}

pub async fn getrawtransaction(
    State(state): State<ApiState>,
    Json(params): Json<GetRawTransactionParams>,
) -> ApiResult<Json<RawTransaction>> {
    let blockchain = state.blockchain.read().await;
    let tip_height = blockchain.chain_tip_height();
    
    // Search for transaction in blockchain
    let mut found_tx = None;
    let mut found_block_hash = None;
    let mut found_height = 0u64;
    
    for height in 0..=tip_height {
        if let Some(block) = blockchain.get_block_by_height(height) {
            for tx in &block.transactions {
                if tx.txid == params.txid {
                    found_tx = Some(tx.clone());
                    found_block_hash = Some(block.hash.clone());
                    found_height = height;
                    break;
                }
            }
            if found_tx.is_some() {
                break;
            }
        }
    }
    
    match found_tx {
        Some(tx) => {
            // Convert transaction to hex (simplified)
            let tx_json = serde_json::to_string(&tx).unwrap_or_default();
            let tx_hex = hex::encode(tx_json.as_bytes());
            
            let vin: Vec<TxInputInfo> = tx
                .inputs
                .iter()
                .map(|input| TxInputInfo {
                    txid: input.previous_output.txid.clone(),
                    vout: input.previous_output.vout,
                    scriptSig: ScriptSig {
                        asm: hex::encode(&input.signature),
                        hex: hex::encode(&input.signature),
                    },
                    sequence: input.sequence,
                })
                .collect();
            
            let vout: Vec<TxOutputInfo> = tx
                .outputs
                .iter()
                .enumerate()
                .map(|(n, output)| TxOutputInfo {
                    value: output.amount as f64 / 100_000_000.0, // Convert to TIME coins
                    n,
                    scriptPubKey: ScriptPubKey {
                        asm: format!("OP_DUP OP_HASH160 {} OP_EQUALVERIFY OP_CHECKSIG", output.address),
                        hex: hex::encode(&output.address),
                        script_type: "pubkeyhash".to_string(),
                        address: output.address.clone(),
                    },
                })
                .collect();
            
            let confirmations = if found_block_hash.is_some() {
                Some(tip_height - found_height + 1)
            } else {
                None
            };
            
            Ok(Json(RawTransaction {
                hex: tx_hex.clone(),
                txid: tx.txid.clone(),
                hash: tx_hex,
                size: tx_json.len(),
                vsize: tx_json.len(),
                version: tx.version,
                locktime: tx.lock_time,
                vin,
                vout,
                blockhash: found_block_hash,
                confirmations,
                time: Some(tx.timestamp),
                blocktime: Some(tx.timestamp),
            }))
        }
        None => {
            // Check mempool
            if let Some(mempool) = state.mempool.as_ref() {
                let mempool_txs = mempool.get_all_transactions().await;
                if let Some(tx) = mempool_txs.iter().find(|t| t.txid == params.txid) {
                    let tx_json = serde_json::to_string(&tx).unwrap_or_default();
                    let tx_hex = hex::encode(tx_json.as_bytes());
                    
                    let vin: Vec<TxInputInfo> = tx
                        .inputs
                        .iter()
                        .map(|input| TxInputInfo {
                            txid: input.previous_output.txid.clone(),
                            vout: input.previous_output.vout,
                            scriptSig: ScriptSig {
                                asm: hex::encode(&input.signature),
                                hex: hex::encode(&input.signature),
                            },
                            sequence: input.sequence,
                        })
                        .collect();
                    
                    let vout: Vec<TxOutputInfo> = tx
                        .outputs
                        .iter()
                        .enumerate()
                        .map(|(n, output)| TxOutputInfo {
                            value: output.amount as f64 / 100_000_000.0,
                            n,
                            scriptPubKey: ScriptPubKey {
                                asm: format!("OP_DUP OP_HASH160 {} OP_EQUALVERIFY OP_CHECKSIG", output.address),
                                hex: hex::encode(&output.address),
                                script_type: "pubkeyhash".to_string(),
                                address: output.address.clone(),
                            },
                        })
                        .collect();
                    
                    return Ok(Json(RawTransaction {
                        hex: tx_hex.clone(),
                        txid: tx.txid.clone(),
                        hash: tx_hex,
                        size: tx_json.len(),
                        vsize: tx_json.len(),
                        version: tx.version,
                        locktime: tx.lock_time,
                        vin,
                        vout,
                        blockhash: None,
                        confirmations: None,
                        time: Some(tx.timestamp),
                        blocktime: Some(tx.timestamp),
                    }));
                }
            }
            
            Err(ApiError::TransactionNotFound(format!(
                "Transaction not found: {}",
                params.txid
            )))
        }
    }
}

/// Response for sendrawtransaction RPC
#[derive(Serialize)]
pub struct SendRawTransactionResponse {
    pub result: String, // Transaction ID
}

#[derive(Deserialize)]
pub struct SendRawTransactionParams {
    pub hexstring: String,
    #[serde(default)]
    pub maxfeerate: f64,
}

pub async fn sendrawtransaction(
    State(state): State<ApiState>,
    Json(params): Json<SendRawTransactionParams>,
) -> ApiResult<Json<SendRawTransactionResponse>> {
    // Decode hex to transaction
    let tx_bytes = hex::decode(&params.hexstring)
        .map_err(|e| ApiError::BadRequest(format!("Invalid hex: {}", e)))?;
    
    let tx_json = String::from_utf8(tx_bytes)
        .map_err(|e| ApiError::BadRequest(format!("Invalid transaction encoding: {}", e)))?;
    
    let tx: time_core::Transaction = serde_json::from_str(&tx_json)
        .map_err(|e| ApiError::BadRequest(format!("Invalid transaction format: {}", e)))?;
    
    let txid = tx.txid.clone();
    
    // Add to mempool
    if let Some(mempool) = state.mempool.as_ref() {
        mempool
            .add_transaction(tx.clone())
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to add transaction: {}", e)))?;
        
        // Broadcast to peers
        if let Some(broadcaster) = state.tx_broadcaster.as_ref() {
            broadcaster.broadcast_transaction(tx).await;
        }
    }
    
    Ok(Json(SendRawTransactionResponse { result: txid }))
}

// ============================================================================
// WALLET RPC METHODS
// ============================================================================

/// Response for getwalletinfo RPC
#[derive(Serialize)]
pub struct WalletInfo {
    pub walletname: String,
    pub walletversion: u32,
    pub balance: f64,
    pub unconfirmed_balance: f64,
    pub immature_balance: f64,
    pub txcount: usize,
    pub keypoololdest: i64,
    pub keypoolsize: usize,
    pub paytxfee: f64,
    pub hdseedid: Option<String>,
    pub private_keys_enabled: bool,
}

pub async fn getwalletinfo(State(state): State<ApiState>) -> ApiResult<Json<WalletInfo>> {
    let balances = state.balances.read().await;
    let wallet_balance = balances.get(&state.wallet_address).copied().unwrap_or(0);
    
    Ok(Json(WalletInfo {
        walletname: "time-wallet".to_string(),
        walletversion: 1,
        balance: wallet_balance as f64 / 100_000_000.0,
        unconfirmed_balance: 0.0,
        immature_balance: 0.0,
        txcount: 0,
        keypoololdest: chrono::Utc::now().timestamp(),
        keypoolsize: 100,
        paytxfee: 0.00001, // 0.00001 TIME
        hdseedid: None,
        private_keys_enabled: true,
    }))
}

/// Response for getbalance RPC
#[derive(Serialize)]
pub struct GetBalanceResponse {
    pub result: f64, // Balance in TIME coins
}

#[derive(Deserialize)]
pub struct GetBalanceParams {
    #[serde(default)]
    pub address: Option<String>,
}

pub async fn getbalance(
    State(state): State<ApiState>,
    Json(params): Json<GetBalanceParams>,
) -> ApiResult<Json<GetBalanceResponse>> {
    let address = params.address.unwrap_or_else(|| state.wallet_address.clone());
    let balances = state.balances.read().await;
    let balance = balances.get(&address).copied().unwrap_or(0);
    
    Ok(Json(GetBalanceResponse {
        result: balance as f64 / 100_000_000.0,
    }))
}

/// Response for getnewaddress RPC
#[derive(Serialize)]
pub struct NewAddress {
    pub result: String,
}

pub async fn getnewaddress(State(_state): State<ApiState>) -> ApiResult<Json<NewAddress>> {
    // Generate new address
    let keypair = time_crypto::KeyPair::generate();
    let public_key_hex = keypair.public_key_hex();
    let address = time_crypto::public_key_to_address(&public_key_hex);
    
    Ok(Json(NewAddress {
        result: address,
    }))
}

/// Response for validateaddress RPC
#[derive(Serialize)]
pub struct ValidateAddressResponse {
    pub isvalid: bool,
    pub address: Option<String>,
    pub scriptPubKey: Option<String>,
    pub isscript: bool,
    pub iswitness: bool,
}

#[derive(Deserialize)]
pub struct ValidateAddressParams {
    pub address: String,
}

pub async fn validateaddress(
    State(_state): State<ApiState>,
    Json(params): Json<ValidateAddressParams>,
) -> ApiResult<Json<ValidateAddressResponse>> {
    // Basic validation - check if address starts with TIME1
    let is_valid = params.address.starts_with("TIME1") && params.address.len() > 10;
    
    Ok(Json(ValidateAddressResponse {
        isvalid: is_valid,
        address: if is_valid {
            Some(params.address.clone())
        } else {
            None
        },
        scriptPubKey: if is_valid {
            Some(hex::encode(&params.address))
        } else {
            None
        },
        isscript: false,
        iswitness: false,
    }))
}

/// Response for listunspent RPC
#[derive(Serialize)]
pub struct UnspentOutput {
    pub txid: String,
    pub vout: u32,
    pub address: String,
    pub account: String,
    pub scriptPubKey: String,
    pub amount: f64,
    pub confirmations: u64,
    pub spendable: bool,
    pub solvable: bool,
}

#[derive(Deserialize)]
pub struct ListUnspentParams {
    #[serde(default)]
    pub minconf: u64,
    #[serde(default = "default_maxconf")]
    pub maxconf: u64,
    #[serde(default)]
    pub addresses: Vec<String>,
}

fn default_maxconf() -> u64 {
    9999999
}

pub async fn listunspent(
    State(state): State<ApiState>,
    Json(params): Json<ListUnspentParams>,
) -> ApiResult<Json<Vec<UnspentOutput>>> {
    let blockchain = state.blockchain.read().await;
    let utxo_set = blockchain.utxo_set();
    let tip_height = blockchain.chain_tip_height();
    
    let addresses = if params.addresses.is_empty() {
        vec![state.wallet_address.clone()]
    } else {
        params.addresses
    };
    
    let mut unspent = Vec::new();
    
    for address in addresses {
        let utxos = utxo_set.get_utxos_by_address(&address);
        
        for (outpoint, output) in utxos {
            // Calculate confirmations (simplified - would need block height of tx)
            let confirmations = tip_height;
            
            if confirmations >= params.minconf && confirmations <= params.maxconf {
                unspent.push(UnspentOutput {
                    txid: outpoint.txid.clone(),
                    vout: outpoint.vout,
                    address: output.address.clone(),
                    account: "".to_string(),
                    scriptPubKey: hex::encode(&output.address),
                    amount: output.amount as f64 / 100_000_000.0,
                    confirmations,
                    spendable: true,
                    solvable: true,
                });
            }
        }
    }
    
    Ok(Json(unspent))
}

/// Response for listtransactions RPC
#[derive(Serialize)]
pub struct TransactionListItem {
    pub address: String,
    pub category: String,
    pub amount: f64,
    pub vout: usize,
    pub confirmations: u64,
    pub blockhash: String,
    pub blockheight: u64,
    pub blocktime: i64,
    pub txid: String,
    pub time: i64,
    pub timereceived: i64,
}

#[derive(Deserialize)]
pub struct ListTransactionsParams {
    #[serde(default)]
    pub count: usize,
    #[serde(default)]
    pub skip: usize,
}

pub async fn listtransactions(
    State(state): State<ApiState>,
    Json(params): Json<ListTransactionsParams>,
) -> ApiResult<Json<Vec<TransactionListItem>>> {
    let blockchain = state.blockchain.read().await;
    let tip_height = blockchain.chain_tip_height();
    let wallet_addr = state.wallet_address.clone();
    
    let count = if params.count == 0 { 10 } else { params.count };
    let mut transactions = Vec::new();
    
    // Scan recent blocks for transactions involving this wallet
    let start_height = if tip_height > 100 { tip_height - 100 } else { 0 };
    
    for height in (start_height..=tip_height).rev() {
        if let Some(block) = blockchain.get_block_by_height(height) {
            for tx in &block.transactions {
                // Check if transaction involves our wallet
                let has_input = tx.inputs.iter().any(|_| false); // Would need to check UTXO ownership
                let has_output = tx.outputs.iter().any(|out| out.address == wallet_addr);
                
                if has_output {
                    for (vout, output) in tx.outputs.iter().enumerate() {
                        if output.address == wallet_addr {
                            transactions.push(TransactionListItem {
                                address: output.address.clone(),
                                category: if has_input { "send".to_string() } else { "receive".to_string() },
                                amount: output.amount as f64 / 100_000_000.0,
                                vout,
                                confirmations: tip_height - height + 1,
                                blockhash: block.hash.clone(),
                                blockheight: height,
                                blocktime: block.header.timestamp.timestamp(),
                                txid: tx.txid.clone(),
                                time: tx.timestamp,
                                timereceived: tx.timestamp,
                            });
                        }
                    }
                }
                
                if transactions.len() >= count + params.skip {
                    break;
                }
            }
            if transactions.len() >= count + params.skip {
                break;
            }
        }
    }
    
    // Apply skip and limit
    let result: Vec<TransactionListItem> = transactions
        .into_iter()
        .skip(params.skip)
        .take(count)
        .collect();
    
    Ok(Json(result))
}

// ============================================================================
// NETWORK RPC METHODS
// ============================================================================

/// Response for getpeerinfo RPC
#[derive(Serialize)]
pub struct PeerInfoResponse {
    pub id: usize,
    pub addr: String,
    pub addrlocal: Option<String>,
    pub services: String,
    pub relaytxes: bool,
    pub lastsend: i64,
    pub lastrecv: i64,
    pub bytessent: u64,
    pub bytesrecv: u64,
    pub conntime: i64,
    pub timeoffset: i64,
    pub pingtime: f64,
    pub version: String,
    pub subver: String,
    pub inbound: bool,
    pub startingheight: u64,
    pub banscore: u32,
    pub synced_headers: u64,
    pub synced_blocks: u64,
}

pub async fn getpeerinfo(State(state): State<ApiState>) -> ApiResult<Json<Vec<PeerInfoResponse>>> {
    let peers = state.peer_manager.get_connected_peers().await;
    
    let peer_info: Vec<PeerInfoResponse> = peers
        .iter()
        .enumerate()
        .map(|(id, peer)| PeerInfoResponse {
            id,
            addr: peer.address.to_string(),
            addrlocal: None,
            services: "0000000000000001".to_string(),
            relaytxes: true,
            lastsend: chrono::Utc::now().timestamp(),
            lastrecv: chrono::Utc::now().timestamp(),
            bytessent: 0,
            bytesrecv: 0,
            conntime: chrono::Utc::now().timestamp(),
            timeoffset: 0,
            pingtime: 0.05,
            version: peer.version.clone(),
            subver: format!("/TIME:{}/", peer.version),
            inbound: false,
            startingheight: 0,
            banscore: 0,
            synced_headers: 0,
            synced_blocks: 0,
        })
        .collect();
    
    Ok(Json(peer_info))
}

/// Response for getnetworkinfo RPC
#[derive(Serialize)]
pub struct NetworkInfo {
    pub version: u32,
    pub subversion: String,
    pub protocolversion: u32,
    pub localservices: String,
    pub localrelay: bool,
    pub timeoffset: i64,
    pub networkactive: bool,
    pub connections: usize,
    pub networks: Vec<NetworkDetails>,
    pub relayfee: f64,
    pub incrementalfee: f64,
    pub localaddresses: Vec<HashMap<String, serde_json::Value>>,
    pub warnings: String,
}

#[derive(Serialize)]
pub struct NetworkDetails {
    pub name: String,
    pub limited: bool,
    pub reachable: bool,
    pub proxy: String,
    pub proxy_randomize_credentials: bool,
}

pub async fn getnetworkinfo(State(state): State<ApiState>) -> ApiResult<Json<NetworkInfo>> {
    let peers = state.peer_manager.get_connected_peers().await;
    
    Ok(Json(NetworkInfo {
        version: 1000000, // 1.0.0
        subversion: "/TIME:1.0.0/".to_string(),
        protocolversion: 1,
        localservices: "0000000000000001".to_string(),
        localrelay: true,
        timeoffset: 0,
        networkactive: true,
        connections: peers.len(),
        networks: vec![NetworkDetails {
            name: "ipv4".to_string(),
            limited: false,
            reachable: true,
            proxy: "".to_string(),
            proxy_randomize_credentials: false,
        }],
        relayfee: 0.00001,
        incrementalfee: 0.00001,
        localaddresses: vec![],
        warnings: "".to_string(),
    }))
}

// ============================================================================
// MINING/CONSENSUS RPC METHODS
// ============================================================================

/// Response for getmininginfo RPC
#[derive(Serialize)]
pub struct MiningInfo {
    pub blocks: u64,
    pub currentblockweight: u64,
    pub currentblocktx: usize,
    pub difficulty: f64,
    pub networkhashps: f64,
    pub pooledtx: usize,
    pub chain: String,
    pub warnings: String,
}

pub async fn getmininginfo(State(state): State<ApiState>) -> ApiResult<Json<MiningInfo>> {
    let blockchain = state.blockchain.read().await;
    let mempool_size = if let Some(mempool) = state.mempool.as_ref() {
        mempool.get_all_transactions().await.len()
    } else {
        0
    };
    
    Ok(Json(MiningInfo {
        blocks: blockchain.chain_tip_height(),
        currentblockweight: 0,
        currentblocktx: 0,
        difficulty: 1.0, // TIME uses BFT, not PoW
        networkhashps: 0.0, // Not applicable for BFT
        pooledtx: mempool_size,
        chain: state.network.clone(),
        warnings: "TIME Coin uses BFT consensus, not Proof-of-Work mining".to_string(),
    }))
}

/// Response for estimatefee RPC
#[derive(Serialize)]
pub struct EstimateFeeResponse {
    pub feerate: f64, // Fee in TIME per KB
    pub blocks: u64,
}

#[derive(Deserialize)]
pub struct EstimateFeeParams {
    pub conf_target: u64,
}

pub async fn estimatefee(
    State(_state): State<ApiState>,
    Json(_params): Json<EstimateFeeParams>,
) -> ApiResult<Json<EstimateFeeResponse>> {
    // TIME has fast finality, so fees are relatively constant
    Ok(Json(EstimateFeeResponse {
        feerate: 0.00001, // 0.00001 TIME per KB
        blocks: 1, // Instant finality
    }))
}
