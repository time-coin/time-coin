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

/// Proposal response structure
#[derive(Debug, Serialize)]
pub struct ProposalResponse {
    pub id: String,
    pub title: String,
    pub description: String,
    pub recipient: String,
    pub amount: u64,
    pub amount_time: f64,
    pub submitter: String,
    pub submission_time: u64,
    pub voting_deadline: u64,
    pub execution_deadline: u64,
    pub status: String,
    pub yes_votes: u64,
    pub no_votes: u64,
    pub total_votes: u64,
    pub approval_percentage: f64,
}

/// Create proposal request
#[derive(Debug, Deserialize)]
pub struct CreateProposalRequest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub recipient: String,
    pub amount: u64,
    pub submitter: String,
    pub voting_period_days: u64,
}

/// Vote on proposal request
#[derive(Debug, Deserialize)]
pub struct VoteOnProposalRequest {
    pub proposal_id: String,
    pub masternode_id: String,
    pub vote_choice: String, // "yes", "no", or "abstain"
    pub voting_power: u64,
}

/// Get all treasury proposals
pub async fn list_treasury_proposals(
    State(state): State<ApiState>,
) -> ApiResult<Json<Vec<ProposalResponse>>> {
    let blockchain = state.blockchain.read().await;
    let proposals = blockchain.get_treasury_proposals();
    
    let response: Vec<ProposalResponse> = proposals
        .iter()
        .map(|p| {
            let (yes_votes, no_votes, total_votes) = calculate_votes(p);
            let approval_percentage = if total_votes > 0 {
                (yes_votes as f64 / total_votes as f64) * 100.0
            } else {
                0.0
            };
            
            ProposalResponse {
                id: p.id.clone(),
                title: p.title.clone(),
                description: p.description.clone(),
                recipient: p.recipient.clone(),
                amount: p.amount,
                amount_time: p.amount as f64 / 100_000_000.0,
                submitter: p.submitter.clone(),
                submission_time: p.submission_time,
                voting_deadline: p.voting_deadline,
                execution_deadline: p.execution_deadline,
                status: format!("{:?}", p.status),
                yes_votes,
                no_votes,
                total_votes,
                approval_percentage,
            }
        })
        .collect();
    
    Ok(Json(response))
}

/// Get a specific treasury proposal
pub async fn get_treasury_proposal(
    State(state): State<ApiState>,
    axum::extract::Path(proposal_id): axum::extract::Path<String>,
) -> ApiResult<Json<ProposalResponse>> {
    let blockchain = state.blockchain.read().await;
    
    let proposal = blockchain
        .get_treasury_proposal(&proposal_id)
        .ok_or_else(|| ApiError::BadRequest(format!("Proposal {} not found", proposal_id)))?;
    
    let (yes_votes, no_votes, total_votes) = calculate_votes(&proposal);
    let approval_percentage = if total_votes > 0 {
        (yes_votes as f64 / total_votes as f64) * 100.0
    } else {
        0.0
    };
    
    Ok(Json(ProposalResponse {
        id: proposal.id,
        title: proposal.title,
        description: proposal.description,
        recipient: proposal.recipient,
        amount: proposal.amount,
        amount_time: proposal.amount as f64 / 100_000_000.0,
        submitter: proposal.submitter,
        submission_time: proposal.submission_time,
        voting_deadline: proposal.voting_deadline,
        execution_deadline: proposal.execution_deadline,
        status: format!("{:?}", proposal.status),
        yes_votes,
        no_votes,
        total_votes,
        approval_percentage,
    }))
}

/// Create a new treasury proposal
pub async fn create_treasury_proposal(
    State(state): State<ApiState>,
    Json(request): Json<CreateProposalRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut blockchain = state.blockchain.write().await;
    
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    blockchain
        .create_treasury_proposal(time_core::CreateProposalParams {
            id: request.id.clone(),
            title: request.title,
            description: request.description,
            recipient: request.recipient,
            amount: request.amount,
            submitter: request.submitter,
            submission_time: current_time,
            voting_period_days: request.voting_period_days,
        })
        .map_err(|e| ApiError::BadRequest(format!("Failed to create proposal: {}", e)))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "proposal_id": request.id,
        "message": "Proposal created successfully"
    })))
}

/// Vote on a treasury proposal
pub async fn vote_on_treasury_proposal(
    State(state): State<ApiState>,
    Json(request): Json<VoteOnProposalRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut blockchain = state.blockchain.write().await;
    
    let vote_choice = match request.vote_choice.to_lowercase().as_str() {
        "yes" => time_core::VoteChoice::Yes,
        "no" => time_core::VoteChoice::No,
        "abstain" => time_core::VoteChoice::Abstain,
        _ => return Err(ApiError::BadRequest(
            "Invalid vote choice. Must be 'yes', 'no', or 'abstain'".to_string()
        )),
    };
    
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    blockchain
        .vote_on_treasury_proposal(
            &request.proposal_id,
            request.masternode_id.clone(),
            vote_choice,
            request.voting_power,
            current_time,
        )
        .map_err(|e| ApiError::BadRequest(format!("Failed to vote: {}", e)))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "proposal_id": request.proposal_id,
        "masternode_id": request.masternode_id,
        "vote": request.vote_choice,
        "message": "Vote recorded successfully"
    })))
}

/// Helper function to calculate vote totals
fn calculate_votes(proposal: &time_core::TreasuryProposal) -> (u64, u64, u64) {
    let mut yes_votes = 0u64;
    let mut no_votes = 0u64;
    let mut total_votes = 0u64;
    
    for vote in proposal.votes.values() {
        total_votes += vote.voting_power;
        match vote.vote_choice {
            time_core::VoteChoice::Yes => yes_votes += vote.voting_power,
            time_core::VoteChoice::No => no_votes += vote.voting_power,
            time_core::VoteChoice::Abstain => {},
        }
    }
    
    (yes_votes, no_votes, total_votes)
}
