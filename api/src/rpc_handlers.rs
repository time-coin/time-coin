//! TIME Coin RPC-compatible handlers
//!
//! This module implements RPC endpoints tailored for TIME Coin's unique features:
//! - BFT consensus (no mining)
//! - 24-hour time blocks
//! - Masternode network with tiered collateral
//! - Treasury and governance system

#![allow(dead_code)] // RPC functions are part of full API spec

use crate::balance::calculate_mempool_balance;
use crate::{ApiError, ApiResult, ApiState};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

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
    pub mediantime: i64,
    pub verificationprogress: f64,
    pub initialblockdownload: bool,
    pub size_on_disk: u64,
    pub pruned: bool,
    pub consensus: String, // "BFT" for TIME Coin
}

pub async fn getblockchaininfo(State(state): State<ApiState>) -> ApiResult<Json<BlockchainInfo>> {
    let blockchain = state.blockchain.read().await;
    let height = blockchain.chain_tip_height();
    let best_hash = blockchain.chain_tip_hash().to_string();

    Ok(Json(BlockchainInfo {
        chain: state.network.clone(),
        blocks: height,
        headers: height,
        bestblockhash: best_hash,
        mediantime: chrono::Utc::now().timestamp(),
        verificationprogress: 1.0,
        initialblockdownload: false,
        size_on_disk: 0, // TODO: Calculate actual size
        pruned: false,
        consensus: "BFT".to_string(),
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
    pub tx: Vec<String>,
    pub previousblockhash: String,
    pub nextblockhash: Option<String>,
    pub is_timeblock: bool, // True if this is a 24-hour checkpoint block
}

#[derive(Deserialize)]
pub struct GetBlockParams {
    pub blockhash: String,
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
            let tx_ids: Vec<String> = block
                .transactions
                .iter()
                .map(|tx| tx.txid.clone())
                .collect();

            // Get next block hash if not at tip
            let next_hash = if height < tip_height {
                blockchain
                    .get_block_by_height(height + 1)
                    .map(|b| b.hash.clone())
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
                tx: tx_ids,
                previousblockhash: block.header.previous_hash.clone(),
                nextblockhash: next_hash,
                is_timeblock: height % 24 == 0, // Daily checkpoints
            }))
        }
        None => Err(ApiError::TransactionNotFound(format!(
            "Block not found: {}",
            params.blockhash
        ))),
    }
}

// ============================================================================
// TIME BLOCK METHODS (24-hour checkpoint system)
// ============================================================================

/// Response for gettimeblockinfo RPC
#[derive(Serialize)]
pub struct TimeBlockInfo {
    pub current_block: u64,
    pub next_timeblock: u64,
    pub seconds_until_next: i64,
    pub transactions_in_period: usize,
    pub total_fees_collected: u64,
}

pub async fn gettimeblockinfo(State(state): State<ApiState>) -> ApiResult<Json<TimeBlockInfo>> {
    let blockchain = state.blockchain.read().await;
    let current_height = blockchain.chain_tip_height();
    let next_timeblock = ((current_height / 24) + 1) * 24;

    // Calculate time until next 24-hour block
    let blocks_remaining = next_timeblock - current_height;
    let seconds_until = blocks_remaining as i64 * 3; // ~3 seconds per block for instant finality

    Ok(Json(TimeBlockInfo {
        current_block: current_height,
        next_timeblock,
        seconds_until_next: seconds_until,
        transactions_in_period: 0, // TODO: Count transactions since last timeblock
        total_fees_collected: 0,   // TODO: Sum fees
    }))
}

/// Response for gettimeblockrewards RPC
#[derive(Serialize)]
pub struct TimeBlockRewards {
    pub block_height: u64,
    pub total_reward: u64,
    pub masternode_rewards: u64,
    pub treasury_allocation: u64,
    pub timestamp: i64,
}

#[derive(Deserialize)]
pub struct TimeBlockParams {
    pub height: u64,
}

pub async fn gettimeblockrewards(
    State(state): State<ApiState>,
    Json(params): Json<TimeBlockParams>,
) -> ApiResult<Json<TimeBlockRewards>> {
    let blockchain = state.blockchain.read().await;

    match blockchain.get_block_by_height(params.height) {
        Some(block) => {
            let total_reward = 50_000_000_000u64; // 500 TIME per block
            let treasury_allocation = total_reward / 10; // 10% to treasury
            let masternode_rewards = total_reward - treasury_allocation;

            Ok(Json(TimeBlockRewards {
                block_height: params.height,
                total_reward,
                masternode_rewards,
                treasury_allocation,
                timestamp: block.header.timestamp.timestamp(),
            }))
        }
        None => Err(ApiError::TransactionNotFound(format!(
            "Time block not found at height {}",
            params.height
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
    pub finalized: bool, // TIME-specific: instant finality
}

#[derive(Serialize)]
pub struct TxInputInfo {
    pub txid: String,
    pub vout: u32,
    #[serde(rename = "scriptSig")]
    pub script_sig: ScriptSig,
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
    #[serde(rename = "scriptPubKey")]
    pub script_pub_key: ScriptPubKey,
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
                    script_sig: ScriptSig {
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
                    script_pub_key: ScriptPubKey {
                        asm: format!(
                            "OP_DUP OP_HASH160 {} OP_EQUALVERIFY OP_CHECKSIG",
                            output.address
                        ),
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
                finalized: confirmations.unwrap_or(0) > 0, // Instant finality in TIME
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
                            script_sig: ScriptSig {
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
                            script_pub_key: ScriptPubKey {
                                asm: format!(
                                    "OP_DUP OP_HASH160 {} OP_EQUALVERIFY OP_CHECKSIG",
                                    output.address
                                ),
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
                        finalized: state.mempool.as_ref().unwrap().is_finalized(&tx.txid).await,
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
// MASTERNODE RPC METHODS
// ============================================================================

/// Response for getmasternodeinfo RPC
#[derive(Serialize)]
pub struct GetMasternodeInfoResponse {
    pub success: bool,
    pub address: String,
    pub wallet_address: String,
    pub tier: String,
    pub is_active: bool,
    pub registered_height: u64,
}

/// Parameters for getmasternodeinfo RPC
#[derive(Deserialize)]
pub struct GetMasternodeInfoParams {
    pub address: String,
}

/// RPC handler to get masternode information by address
pub async fn getmasternodeinfo(
    State(state): State<ApiState>,
    Json(params): Json<GetMasternodeInfoParams>,
) -> ApiResult<Json<GetMasternodeInfoResponse>> {
    // Validate address format
    if params.address.is_empty() {
        return Err(ApiError::InvalidAddress(
            "Address cannot be empty".to_string(),
        ));
    }

    // Get blockchain state
    let blockchain = state.blockchain.read().await;

    // Look up the masternode by address
    let masternode = blockchain.get_masternode(&params.address).ok_or_else(|| {
        ApiError::InvalidAddress(format!(
            "Masternode not found with address: {}",
            params.address
        ))
    })?;

    // Return masternode information
    Ok(Json(GetMasternodeInfoResponse {
        success: true,
        address: masternode.address.clone(),
        wallet_address: masternode.wallet_address.clone(),
        tier: format!("{:?}", masternode.tier),
        is_active: masternode.is_active,
        registered_height: masternode.registered_height,
    }))
}

/// Response for listmasternodes RPC
#[derive(Serialize)]
pub struct MasternodeListItem {
    pub address: String,
    pub wallet_address: String,
    pub tier: String,
    pub is_active: bool,
    pub registered_height: u64,
}

pub async fn listmasternodes(
    State(state): State<ApiState>,
) -> ApiResult<Json<Vec<MasternodeListItem>>> {
    let blockchain = state.blockchain.read().await;

    let masternodes: Vec<MasternodeListItem> = blockchain
        .get_all_masternodes()
        .iter()
        .map(|mn| MasternodeListItem {
            address: mn.address.clone(),
            wallet_address: mn.wallet_address.clone(),
            tier: format!("{:?}", mn.tier),
            is_active: mn.is_active,
            registered_height: mn.registered_height,
        })
        .collect();

    Ok(Json(masternodes))
}

/// Response for getmasternodecount RPC
#[derive(Serialize)]
pub struct MasternodeCount {
    pub total: usize,
    pub free: usize,
    pub bronze: usize,
    pub silver: usize,
    pub gold: usize,
    pub active: usize,
}

pub async fn getmasternodecount(State(state): State<ApiState>) -> ApiResult<Json<MasternodeCount>> {
    let blockchain = state.blockchain.read().await;
    let stats = blockchain.get_stats();
    let all_masternodes = blockchain.get_all_masternodes();

    Ok(Json(MasternodeCount {
        total: all_masternodes.len(),
        free: stats.free_masternodes as usize,
        bronze: stats.bronze_masternodes as usize,
        silver: stats.silver_masternodes as usize,
        gold: stats.gold_masternodes as usize,
        active: stats.active_masternodes as usize,
    }))
}

// ============================================================================
// CONSENSUS RPC METHODS
// ============================================================================

/// Response for getconsensusstatus RPC
#[derive(Serialize)]
pub struct ConsensusStatus {
    pub consensus_type: String,
    pub active_validators: usize,
    pub bft_threshold: f64, // 67% for BFT
    pub instant_finality: bool,
    pub consensus_mode: String,
}

pub async fn getconsensusstatus(State(state): State<ApiState>) -> ApiResult<Json<ConsensusStatus>> {
    let masternode_count = state.consensus.masternode_count().await;
    let mode = state.consensus.consensus_mode().await;
    let mode_str = match mode {
        time_consensus::ConsensusMode::Development => "Development",
        time_consensus::ConsensusMode::BootstrapNoQuorum => "Bootstrap (No Quorum)",
        time_consensus::ConsensusMode::BFT => "BFT",
    };

    Ok(Json(ConsensusStatus {
        consensus_type: "BFT".to_string(),
        active_validators: masternode_count,
        bft_threshold: 0.67,
        instant_finality: true,
        consensus_mode: mode_str.to_string(),
    }))
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
    pub txcount: usize,
    pub keypoolsize: usize,
}

pub async fn getwalletinfo(State(state): State<ApiState>) -> ApiResult<Json<WalletInfo>> {
    // Calculate balance from UTXO set instead of the balances HashMap
    // This ensures the balance always reflects the actual spendable UTXOs
    let blockchain = state.blockchain.read().await;
    let wallet_balance = blockchain.utxo_set().get_balance(&state.wallet_address);

    // Calculate unconfirmed balance from mempool
    let unconfirmed_balance = if let Some(mempool) = &state.mempool {
        calculate_mempool_balance(&state.wallet_address, &blockchain, mempool).await
    } else {
        0
    };

    Ok(Json(WalletInfo {
        walletname: "time-wallet".to_string(),
        walletversion: 1,
        balance: wallet_balance as f64 / 100_000_000.0,
        unconfirmed_balance: unconfirmed_balance as f64 / 100_000_000.0,
        txcount: 0,
        keypoolsize: 100,
    }))
}

/// Response for getbalance RPC
#[derive(Serialize)]
pub struct GetBalanceResponse {
    pub result: f64,              // Confirmed balance in TIME coins
    pub unconfirmed_balance: f64, // Unconfirmed balance in TIME coins
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
    let address = params
        .address
        .unwrap_or_else(|| state.wallet_address.clone());

    // Calculate balance from UTXO set instead of the balances HashMap
    // This ensures the balance always reflects the actual spendable UTXOs
    // and properly handles multiple UTXOs for the same address
    let blockchain = state.blockchain.read().await;
    let balance = blockchain.utxo_set().get_balance(&address);

    // Calculate unconfirmed balance from mempool
    let unconfirmed_balance = if let Some(mempool) = &state.mempool {
        calculate_mempool_balance(&address, &blockchain, mempool).await
    } else {
        0
    };

    Ok(Json(GetBalanceResponse {
        result: balance as f64 / 100_000_000.0,
        unconfirmed_balance: unconfirmed_balance as f64 / 100_000_000.0,
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

    Ok(Json(NewAddress { result: address }))
}

/// Response for validateaddress RPC
#[derive(Serialize)]
pub struct ValidateAddressResponse {
    pub isvalid: bool,
    pub address: Option<String>,
    #[serde(rename = "scriptPubKey")]
    pub script_pub_key: Option<String>,
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
        script_pub_key: if is_valid {
            Some(hex::encode(&params.address))
        } else {
            None
        },
    }))
}

/// Response for listunspent RPC
#[derive(Serialize)]
pub struct UnspentOutput {
    pub txid: String,
    pub vout: u32,
    pub address: String,
    pub account: String,
    #[serde(rename = "scriptPubKey")]
    pub script_pub_key: String,
    pub amount: f64,
    pub confirmations: u64,
    pub spendable: bool,
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
            let confirmations = tip_height;

            if confirmations >= params.minconf && confirmations <= params.maxconf {
                unspent.push(UnspentOutput {
                    txid: outpoint.txid.clone(),
                    vout: outpoint.vout,
                    address: output.address.clone(),
                    account: "".to_string(),
                    script_pub_key: hex::encode(&output.address),
                    amount: output.amount as f64 / 100_000_000.0,
                    confirmations,
                    spendable: true,
                });
            }
        }
    }

    Ok(Json(unspent))
}

// ============================================================================
// NETWORK RPC METHODS
// ============================================================================

/// Response for getpeerinfo RPC
#[derive(Serialize)]
pub struct PeerInfoResponse {
    pub id: usize,
    pub addr: String,
    pub version: String,
    pub subver: String,
    pub inbound: bool,
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
            version: peer.version.clone(),
            subver: format!("/TIME:{}/", peer.version),
            inbound: false,
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
    pub connections: usize,
    pub networks: Vec<NetworkDetails>,
}

#[derive(Serialize)]
pub struct NetworkDetails {
    pub name: String,
    pub limited: bool,
    pub reachable: bool,
}

pub async fn getnetworkinfo(State(state): State<ApiState>) -> ApiResult<Json<NetworkInfo>> {
    let peers = state.peer_manager.get_connected_peers().await;

    Ok(Json(NetworkInfo {
        version: 1000000, // 1.0.0
        subversion: "/TIME:1.0.0/".to_string(),
        protocolversion: 1,
        connections: peers.len(),
        networks: vec![NetworkDetails {
            name: "ipv4".to_string(),
            limited: false,
            reachable: true,
        }],
    }))
}

// ============================================================================
// TREASURY & GOVERNANCE RPC METHODS
// ============================================================================

/// Response for gettreasury RPC
#[derive(Serialize)]
pub struct TreasuryInfo {
    pub balance: f64,
    pub total_allocated: f64,
    pub pending_proposals: usize,
    pub monthly_budget: f64,
}

pub async fn gettreasury(State(state): State<ApiState>) -> ApiResult<Json<TreasuryInfo>> {
    let blockchain = state.blockchain.read().await;
    let treasury_address = "TIME1treasury00000000000000000000000000";
    let treasury_balance = blockchain.get_balance(treasury_address);

    Ok(Json(TreasuryInfo {
        balance: treasury_balance as f64 / 100_000_000.0,
        total_allocated: 0.0,    // TODO: Track allocated funds
        pending_proposals: 0,    // TODO: Count pending proposals
        monthly_budget: 50000.0, // TODO: Calculate monthly budget
    }))
}

/// Response for listproposals RPC
#[derive(Serialize)]
pub struct ProposalListItem {
    pub id: String,
    pub title: String,
    pub amount: f64,
    pub votes_yes: u32,
    pub votes_no: u32,
    pub status: String,
}

pub async fn listproposals(
    State(_state): State<ApiState>,
) -> ApiResult<Json<Vec<ProposalListItem>>> {
    // TODO: Implement actual proposal listing from governance system
    Ok(Json(vec![]))
}
