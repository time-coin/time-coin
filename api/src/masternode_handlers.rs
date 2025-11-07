use crate::{ApiError, ApiResult, ApiState};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use time_core::MasternodeTier;

#[derive(Debug, Deserialize)]
pub struct RegisterMasternodeRequest {
    pub node_ip: String,
    pub wallet_address: String,
    pub tier: String,
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
    // Validate wallet address format
    if !req.wallet_address.starts_with("TIME1") {
        return Err(ApiError::InvalidAddress(
            "Wallet address must start with TIME1".to_string(),
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

    // Register in blockchain state
    let mut blockchain = state.blockchain.write().await;

    blockchain
        .register_masternode(
            req.node_ip.clone(),
            tier,
            "peer_connection".to_string(),
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

    println!(
        "✅ Masternode registered: {} -> {} ({})",
        req.node_ip, req.wallet_address, req.tier
    );

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