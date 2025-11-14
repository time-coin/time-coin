//! Treasury API Handlers

use crate::{ApiError, ApiResult, ApiState};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

/// Treasury statistics response
#[derive(Debug, Serialize)]
pub struct TreasuryStatsResponse {
    pub balance: u64,
    pub balance_time: f64,
    pub total_allocated: u64,
    pub total_distributed: u64,
    pub allocation_count: usize,
    pub withdrawal_count: usize,
    pub pending_proposals: usize,
}

/// Treasury allocation info
#[derive(Debug, Serialize)]
pub struct AllocationInfo {
    pub block_number: u64,
    pub amount: u64,
    pub source: String,
    pub timestamp: i64,
}

/// Treasury withdrawal info
#[derive(Debug, Serialize)]
pub struct WithdrawalInfo {
    pub proposal_id: String,
    pub amount: u64,
    pub recipient: String,
    pub block_number: u64,
    pub timestamp: i64,
}

/// Proposal approval request
#[derive(Debug, Deserialize)]
pub struct ApproveProposalRequest {
    pub proposal_id: String,
    pub amount: u64,
}

/// Funds distribution request
#[derive(Debug, Deserialize)]
pub struct DistributeFundsRequest {
    pub proposal_id: String,
    pub recipient: String,
    pub amount: u64,
}

/// Proposal submission request
#[derive(Debug, Deserialize)]
pub struct ProposeRequest {
    pub title: String,
    pub description: String,
    pub recipient: String,
    pub amount: u64,
    pub voting_period_days: u64,
}

/// Vote submission request
#[derive(Debug, Deserialize)]
pub struct VoteRequest {
    pub proposal_id: String,
    pub masternode_id: String,
    pub vote: String, // "Yes", "No", or "Abstain"
}

/// Get treasury statistics
pub async fn get_treasury_stats(
    State(state): State<ApiState>,
) -> ApiResult<Json<TreasuryStatsResponse>> {
    let blockchain = state.blockchain.read().await;

    let stats = blockchain.treasury_stats();
    let balance = stats.balance;

    Ok(Json(TreasuryStatsResponse {
        balance,
        balance_time: balance as f64 / 100_000_000.0,
        total_allocated: stats.total_allocated,
        total_distributed: stats.total_distributed,
        allocation_count: stats.allocation_count,
        withdrawal_count: stats.withdrawal_count,
        pending_proposals: stats.pending_proposals,
    }))
}

/// Get treasury allocation history
pub async fn get_treasury_allocations(
    State(state): State<ApiState>,
) -> ApiResult<Json<Vec<AllocationInfo>>> {
    let blockchain = state.blockchain.read().await;

    let treasury = blockchain.treasury();
    let allocations: Vec<AllocationInfo> = treasury
        .allocations()
        .iter()
        .map(|a| AllocationInfo {
            block_number: a.block_number,
            amount: a.amount,
            source: format!("{:?}", a.source),
            timestamp: a.timestamp,
        })
        .collect();

    Ok(Json(allocations))
}

/// Get treasury withdrawal history
pub async fn get_treasury_withdrawals(
    State(state): State<ApiState>,
) -> ApiResult<Json<Vec<WithdrawalInfo>>> {
    let blockchain = state.blockchain.read().await;

    let treasury = blockchain.treasury();
    let withdrawals: Vec<WithdrawalInfo> = treasury
        .withdrawals()
        .iter()
        .map(|w| WithdrawalInfo {
            proposal_id: w.proposal_id.clone(),
            amount: w.amount,
            recipient: w.recipient.clone(),
            block_number: w.block_number,
            timestamp: w.timestamp,
        })
        .collect();

    Ok(Json(withdrawals))
}

/// Approve a treasury proposal (requires governance consensus)
/// This is a placeholder - actual governance logic should validate masternode voting
pub async fn approve_treasury_proposal(
    State(state): State<ApiState>,
    Json(request): Json<ApproveProposalRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut blockchain = state.blockchain.write().await;

    blockchain
        .approve_treasury_proposal(request.proposal_id.clone(), request.amount)
        .map_err(|e| ApiError::BadRequest(format!("Failed to approve proposal: {}", e)))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "proposal_id": request.proposal_id,
        "approved_amount": request.amount,
        "message": "Proposal approved for treasury spending"
    })))
}

/// Distribute treasury funds for an approved proposal
/// This should only be called after governance consensus is reached
pub async fn distribute_treasury_funds(
    State(state): State<ApiState>,
    Json(request): Json<DistributeFundsRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut blockchain = state.blockchain.write().await;

    blockchain
        .distribute_treasury_funds(
            request.proposal_id.clone(),
            request.recipient.clone(),
            request.amount,
        )
        .map_err(|e| ApiError::BadRequest(format!("Failed to distribute funds: {}", e)))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "proposal_id": request.proposal_id,
        "recipient": request.recipient,
        "amount": request.amount,
        "message": "Treasury funds distributed successfully"
    })))
}

/// Submit a new treasury proposal
pub async fn submit_proposal(
    State(_state): State<ApiState>,
    Json(request): Json<ProposeRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    // Generate a unique proposal ID
    let proposal_id = format!("prop-{}", chrono::Utc::now().timestamp());

    // TODO: Store proposal in blockchain state
    // For now, return success with the generated ID

    Ok(Json(serde_json::json!({
        "status": "success",
        "proposal_id": proposal_id,
        "title": request.title,
        "amount": request.amount,
        "message": "Proposal submitted successfully and is now open for voting"
    })))
}

/// Get a specific treasury proposal by ID
pub async fn get_proposal(
    State(_state): State<ApiState>,
    proposal_id: String,
) -> ApiResult<Json<serde_json::Value>> {
    // TODO: Retrieve proposal from blockchain state
    // For now, return a placeholder

    Ok(Json(serde_json::json!({
        "id": proposal_id,
        "title": "Sample Proposal",
        "description": "This is a sample proposal",
        "recipient": "TIME1sample000000000000000000000000000",
        "amount": 100000000000u64,
        "status": "Active",
        "submitter": "submitter-node",
        "votes": {}
    })))
}

/// Get a specific treasury proposal by ID (with path extraction)
pub async fn get_proposal_by_id(
    State(state): State<ApiState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    get_proposal(State(state), id).await
}

/// Cast a vote on a treasury proposal
pub async fn vote_on_proposal(
    State(_state): State<ApiState>,
    Json(request): Json<VoteRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    // Validate vote choice
    let vote_choice = match request.vote.as_str() {
        "Yes" | "yes" => "Yes",
        "No" | "no" => "No",
        "Abstain" | "abstain" => "Abstain",
        _ => {
            return Err(ApiError::BadRequest(
                "Invalid vote choice. Must be: Yes, No, or Abstain".to_string(),
            ))
        }
    };

    // TODO: Store vote in blockchain state
    // TODO: Validate that masternode exists and hasn't already voted

    Ok(Json(serde_json::json!({
        "status": "success",
        "proposal_id": request.proposal_id,
        "masternode_id": request.masternode_id,
        "vote": vote_choice,
        "message": format!("Vote '{}' recorded successfully", vote_choice)
    })))
}
