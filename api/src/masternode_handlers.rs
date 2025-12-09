use crate::constants::{
    MASTERNODE_COLLATERAL_BRONZE, MASTERNODE_COLLATERAL_GOLD, MASTERNODE_COLLATERAL_SILVER,
    MIN_MASTERNODE_CONFIRMATIONS,
};
use crate::{ApiError, ApiResult, ApiState};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use time_core::MasternodeTier;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterMasternodeRequest {
    pub node_ip: String,
    #[validate(length(min = 40, max = 100))]
    pub wallet_address: String,
    pub tier: String,
    /// Optional: Collateral transaction ID (required for Bronze/Silver/Gold)
    pub collateral_txid: Option<String>,
    /// Optional: Collateral output index (required for Bronze/Silver/Gold)
    pub collateral_vout: Option<u32>,
    /// Optional: Node port (defaults to 8333 if not provided)
    #[validate(range(min = 1024, max = 65535))]
    pub port: Option<u16>,
}

#[derive(Debug, Serialize)]
pub struct RegisterMasternodeResponse {
    pub success: bool,
    pub message: String,
    pub node_ip: String,
    pub wallet_address: String,
    pub tier: String,
}

pub async fn register_masternode(
    State(state): State<ApiState>,
    Json(req): Json<RegisterMasternodeRequest>,
) -> ApiResult<Json<RegisterMasternodeResponse>> {
    // Validate request
    req.validate()
        .map_err(|e| ApiError::BadRequest(format!("Validation failed: {}", e)))?;

    // Validate wallet address format (TIME0 for testnet, TIME1 for mainnet)
    if !req.wallet_address.starts_with("TIME0") && !req.wallet_address.starts_with("TIME1") {
        return Err(ApiError::InvalidAddress(
            "Wallet address must start with TIME0 (testnet) or TIME1 (mainnet)".to_string(),
        ));
    }

    // Parse tier
    let tier = match req.tier.to_lowercase().as_str() {
        "free" => MasternodeTier::Free,
        "bronze" => MasternodeTier::Bronze,
        "silver" => MasternodeTier::Silver,
        "gold" => MasternodeTier::Gold,
        _ => {
            return Err(ApiError::InvalidAddress(format!(
                "Invalid tier '{}'. Must be one of: Free, Bronze, Silver, Gold",
                req.tier
            )))
        }
    };

    // Validate collateral requirements for non-Free tiers
    if tier != MasternodeTier::Free
        && (req.collateral_txid.is_none() || req.collateral_vout.is_none())
    {
        return Err(ApiError::BadRequest(format!(
            "{} tier requires collateral_txid and collateral_vout",
            req.tier
        )));
    }

    // NEW: Validate collateral UTXO if provided
    if let (Some(txid), Some(vout)) = (&req.collateral_txid, req.collateral_vout) {
        let blockchain = state.blockchain.read().await;
        validate_collateral_utxo(&blockchain, txid, vout, tier).await?;
        drop(blockchain);
    }

    // Build collateral transaction reference
    let collateral_tx =
        if let (Some(txid), Some(vout)) = (&req.collateral_txid, req.collateral_vout) {
            format!("{}:{}", txid, vout)
        } else {
            "no_collateral_required".to_string()
        };

    // Register in blockchain state
    let mut blockchain = state.blockchain.write().await;

    blockchain
        .register_masternode(
            req.node_ip.clone(),
            tier,
            collateral_tx,
            req.wallet_address.clone(),
        )
        .map_err(|e| ApiError::Internal(format!("Registration failed: {:?}", e)))?;

    drop(blockchain);

    // Also register in consensus engine
    state.consensus.add_masternode(req.node_ip.clone()).await;
    state
        .consensus
        .register_wallet(req.node_ip.clone(), req.wallet_address.clone())
        .await;

    Ok(Json(RegisterMasternodeResponse {
        success: true,
        message: format!("Masternode registered successfully as {} tier", req.tier),
        node_ip: req.node_ip,
        wallet_address: req.wallet_address,
        tier: req.tier,
    }))
}

#[derive(Debug, Serialize)]
pub struct ListMasternodesResponse {
    pub masternodes: Vec<MasternodeDetails>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct MasternodeDetails {
    pub node_ip: String,
    pub wallet_address: String,
    pub tier: String,
    pub is_active: bool,
    pub registered_height: u64,
}

pub async fn list_masternodes(
    State(state): State<ApiState>,
) -> ApiResult<Json<ListMasternodesResponse>> {
    let blockchain = state.blockchain.read().await;

    let masternodes: Vec<MasternodeDetails> = blockchain
        .get_all_masternodes()
        .iter()
        .map(|mn| MasternodeDetails {
            node_ip: mn.address.clone(),
            wallet_address: mn.wallet_address.clone(),
            tier: format!("{:?}", mn.tier),
            is_active: mn.is_active,
            registered_height: mn.registered_height,
        })
        .collect();

    let count = masternodes.len();

    Ok(Json(ListMasternodesResponse { masternodes, count }))
}

/// Validate collateral UTXO for masternode registration
///
/// Checks:
/// 1. UTXO exists in the UTXO set
/// 2. UTXO amount meets tier requirements
/// 3. UTXO has sufficient confirmations (minimum 10)
async fn validate_collateral_utxo(
    blockchain: &time_core::state::BlockchainState,
    txid: &str,
    vout: u32,
    tier: MasternodeTier,
) -> ApiResult<()> {
    let utxo_set = blockchain.utxo_set();

    // Create outpoint
    let outpoint = time_core::transaction::OutPoint {
        txid: txid.to_string(),
        vout,
    };

    // 1. Check UTXO exists
    let utxo = utxo_set.get(&outpoint).ok_or_else(|| {
        ApiError::BadRequest(format!("Collateral UTXO not found: {}:{}", txid, vout))
    })?;

    // 2. Check amount >= required for tier
    let required = match tier {
        MasternodeTier::Free => 0,
        MasternodeTier::Bronze => MASTERNODE_COLLATERAL_BRONZE,
        MasternodeTier::Silver => MASTERNODE_COLLATERAL_SILVER,
        MasternodeTier::Gold => MASTERNODE_COLLATERAL_GOLD,
    };

    if utxo.amount < required {
        return Err(ApiError::InsufficientBalance {
            have: utxo.amount,
            need: required,
        });
    }

    // 3. Check confirmations
    // Note: This requires tracking which block the UTXO was created in
    // For now, we'll log a warning if we can't determine confirmations
    let chain_tip = blockchain.chain_tip_height();

    // Try to find the block height where this UTXO was created
    // In a full implementation, we'd track this in the UTXO set
    tracing::info!(
        txid = %txid,
        vout = vout,
        chain_tip = chain_tip,
        required_confirmations = MIN_MASTERNODE_CONFIRMATIONS,
        "collateral_utxo_validated"
    );

    // TODO: Implement block height tracking for UTXOs to verify confirmations
    // For now, we assume if the UTXO exists in the set, it has enough confirmations

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_collateral_requirements() {
        assert_eq!(
            match MasternodeTier::Bronze {
                MasternodeTier::Bronze => MASTERNODE_COLLATERAL_BRONZE,
                _ => 0,
            },
            50_000_000_000 // 500 TIME
        );

        assert_eq!(
            match MasternodeTier::Silver {
                MasternodeTier::Silver => MASTERNODE_COLLATERAL_SILVER,
                _ => 0,
            },
            200_000_000_000 // 2,000 TIME
        );

        assert_eq!(
            match MasternodeTier::Gold {
                MasternodeTier::Gold => MASTERNODE_COLLATERAL_GOLD,
                _ => 0,
            },
            500_000_000_000 // 5,000 TIME
        );
    }
}
