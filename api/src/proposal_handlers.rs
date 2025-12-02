//! API handlers for treasury grant proposals

use crate::{ApiError, ApiResult, ApiState};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing as log;

#[derive(Debug, Deserialize)]
pub struct CreateProposalRequest {
    pub recipient: String,
    pub amount: u64,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct CreateProposalResponse {
    pub success: bool,
    pub id: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct VoteProposalRequest {
    pub proposal_id: String,
    pub approve: bool,
}

#[derive(Debug, Serialize)]
pub struct VoteProposalResponse {
    pub success: bool,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct ListProposalsQuery {
    pub pending: Option<bool>,
}

/// Create a new treasury grant proposal
pub async fn create_proposal(
    State(state): State<ApiState>,
    Json(request): Json<CreateProposalRequest>,
) -> ApiResult<Json<CreateProposalResponse>> {
    // Get node ID (proposer)
    let node_id = std::env::var("NODE_PUBLIC_IP").unwrap_or_else(|_| "unknown".to_string());

    // Validate inputs
    if request.recipient.is_empty() {
        return Err(ApiError::BadRequest(
            "Recipient address cannot be empty".to_string(),
        ));
    }

    if request.amount == 0 {
        return Err(ApiError::BadRequest(
            "Amount must be greater than 0".to_string(),
        ));
    }

    if request.reason.is_empty() {
        return Err(ApiError::BadRequest("Reason cannot be empty".to_string()));
    }

    // Get proposal manager from consensus
    let consensus = &state.consensus;

    let proposal_manager = consensus
        .proposal_manager()
        .ok_or_else(|| ApiError::Internal("Proposal manager not available".to_string()))?;

    // Create the proposal
    let proposal = proposal_manager
        .create_proposal(node_id, request.recipient, request.amount, request.reason)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create proposal: {}", e)))?;

    log::info!(
        proposal_id = %proposal.id,
        proposer = %proposal.proposer,
        recipient = %proposal.recipient,
        amount = proposal.amount,
        reason = %proposal.reason,
        "proposal_created"
    );

    Ok(Json(CreateProposalResponse {
        success: true,
        id: proposal.id,
        message: "Proposal created successfully. Masternodes can now vote.".to_string(),
    }))
}

/// Vote on a proposal
pub async fn vote_proposal(
    State(state): State<ApiState>,
    Json(request): Json<VoteProposalRequest>,
) -> ApiResult<Json<VoteProposalResponse>> {
    // Get node ID (voter)
    let node_id = std::env::var("NODE_PUBLIC_IP").unwrap_or_else(|_| "unknown".to_string());

    log::debug!(node_id = %node_id, "vote_request_received");

    // Get proposal manager from consensus
    let consensus = &state.consensus;

    let proposal_manager = consensus
        .proposal_manager()
        .ok_or_else(|| ApiError::Internal("Proposal manager not available".to_string()))?;

    // Check if voter is a masternode
    let masternodes = consensus.get_masternodes().await;
    log::debug!(masternodes = ?masternodes, "registered_masternodes");

    let is_masternode = consensus.is_masternode(&node_id).await;
    log::debug!(node_id = %node_id, is_masternode = is_masternode, "masternode_check");

    if !is_masternode {
        return Err(ApiError::BadRequest(
            format!("Only masternodes can vote on proposals. Your node_id '{}' is not in the masternode list: {:?}", node_id, masternodes),
        ));
    }

    // Record the vote
    proposal_manager
        .vote(&request.proposal_id, node_id.clone(), request.approve)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to vote: {}", e)))?;

    // Update proposal statuses
    let masternode_count = consensus.masternode_count().await;
    proposal_manager.update_statuses(masternode_count).await;

    // Get updated proposal
    let proposal = proposal_manager
        .get(&request.proposal_id)
        .await
        .ok_or_else(|| ApiError::NotFound("Proposal not found".to_string()))?;

    log::info!(
        proposal_id = %request.proposal_id,
        voter = %node_id,
        vote = if request.approve { "APPROVE" } else { "REJECT" },
        status = ?proposal.status,
        "vote_recorded"
    );

    Ok(Json(VoteProposalResponse {
        success: true,
        status: format!("{:?}", proposal.status),
        message: format!(
            "Vote recorded. Votes: {} for, {} against",
            proposal.votes_for.len(),
            proposal.votes_against.len()
        ),
    }))
}

/// List all proposals
pub async fn list_proposals(
    State(state): State<ApiState>,
    Query(query): Query<ListProposalsQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    // Get proposal manager from consensus
    let consensus = &state.consensus;

    let proposal_manager = consensus
        .proposal_manager()
        .ok_or_else(|| ApiError::Internal("Proposal manager not available".to_string()))?;

    // Get proposals
    let mut proposals = proposal_manager.get_all().await;

    // Filter if requested
    if query.pending.unwrap_or(false) {
        proposals.retain(|p| p.status == time_consensus::proposals::ProposalStatus::Pending);
    }

    // Sort by created_at descending (newest first)
    proposals.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(Json(serde_json::json!({
        "success": true,
        "count": proposals.len(),
        "proposals": proposals,
    })))
}

/// Get a specific proposal
pub async fn get_proposal(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    // Get proposal manager from consensus
    let consensus = &state.consensus;

    let proposal_manager = consensus
        .proposal_manager()
        .ok_or_else(|| ApiError::Internal("Proposal manager not available".to_string()))?;

    // Get proposal
    let proposal = proposal_manager
        .get(&id)
        .await
        .ok_or_else(|| ApiError::NotFound("Proposal not found".to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "proposal": proposal,
    })))
}
