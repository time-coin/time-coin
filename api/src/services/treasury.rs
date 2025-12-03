//! Treasury service - encapsulates treasury and proposal operations

use crate::{ApiError, ApiResult};
use std::sync::Arc;
use time_consensus::ConsensusEngine;

/// Service for treasury operations
#[derive(Clone)]
pub struct TreasuryService {
    consensus: Arc<ConsensusEngine>,
}

impl TreasuryService {
    /// Create a new treasury service
    pub fn new(consensus: Arc<ConsensusEngine>) -> Self {
        Self { consensus }
    }

    /// Create a new proposal
    pub async fn create_proposal(
        &self,
        proposer: String,
        recipient: String,
        amount: u64,
        reason: String,
    ) -> ApiResult<ProposalInfo> {
        let proposal_manager = self
            .consensus
            .proposal_manager()
            .ok_or_else(|| ApiError::Internal("Proposal manager not available".to_string()))?;

        let proposal = proposal_manager
            .create_proposal(proposer, recipient, amount, reason)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to create proposal: {}", e)))?;

        Ok(ProposalInfo {
            id: proposal.id,
            proposer: proposal.proposer,
            recipient: proposal.recipient,
            amount: proposal.amount,
            reason: proposal.reason,
            status: format!("{:?}", proposal.status),
        })
    }

    /// Vote on a proposal
    pub async fn vote_proposal(
        &self,
        proposal_id: &str,
        voter: String,
        approve: bool,
    ) -> ApiResult<VoteResult> {
        // Check if voter is a masternode
        let is_masternode = self.consensus.is_masternode(&voter).await;
        if !is_masternode {
            let masternodes = self.consensus.get_masternodes().await;
            return Err(ApiError::BadRequest(format!(
                "Only masternodes can vote. Node '{}' not in masternode list: {:?}",
                voter, masternodes
            )));
        }

        let proposal_manager = self
            .consensus
            .proposal_manager()
            .ok_or_else(|| ApiError::Internal("Proposal manager not available".to_string()))?;

        // Record the vote
        proposal_manager
            .vote(proposal_id, voter.clone(), approve)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to vote: {}", e)))?;

        // Update proposal statuses
        let masternode_count = self.consensus.masternode_count().await;
        proposal_manager.update_statuses(masternode_count).await;

        // Get updated proposal
        let proposal = proposal_manager
            .get(proposal_id)
            .await
            .ok_or_else(|| ApiError::NotFound("Proposal not found".to_string()))?;

        Ok(VoteResult {
            proposal_id: proposal_id.to_string(),
            voter,
            approved: approve,
            status: format!("{:?}", proposal.status),
        })
    }

    /// Get proposal by ID
    pub async fn get_proposal(&self, proposal_id: &str) -> ApiResult<ProposalInfo> {
        let proposal_manager = self
            .consensus
            .proposal_manager()
            .ok_or_else(|| ApiError::Internal("Proposal manager not available".to_string()))?;

        let proposal = proposal_manager
            .get(proposal_id)
            .await
            .ok_or_else(|| ApiError::NotFound("Proposal not found".to_string()))?;

        Ok(ProposalInfo {
            id: proposal.id,
            proposer: proposal.proposer,
            recipient: proposal.recipient,
            amount: proposal.amount,
            reason: proposal.reason,
            status: format!("{:?}", proposal.status),
        })
    }

    /// List proposals (optionally filter by pending status)
    pub async fn list_proposals(&self, pending_only: bool) -> ApiResult<Vec<ProposalInfo>> {
        let proposal_manager = self
            .consensus
            .proposal_manager()
            .ok_or_else(|| ApiError::Internal("Proposal manager not available".to_string()))?;

        let mut proposals = proposal_manager.get_all().await;

        // Filter if requested
        if pending_only {
            proposals.retain(|p| p.status == time_consensus::proposals::ProposalStatus::Pending);
        }

        let result: Vec<ProposalInfo> = proposals
            .into_iter()
            .map(|p| ProposalInfo {
                id: p.id,
                proposer: p.proposer,
                recipient: p.recipient,
                amount: p.amount,
                reason: p.reason,
                status: format!("{:?}", p.status),
            })
            .collect();

        Ok(result)
    }

    /// Get masternode count
    pub async fn get_masternode_count(&self) -> usize {
        self.consensus.masternode_count().await
    }
}

/// Proposal information
#[derive(Debug, Clone)]
pub struct ProposalInfo {
    pub id: String,
    pub proposer: String,
    pub recipient: String,
    pub amount: u64,
    pub reason: String,
    pub status: String,
}

/// Vote result
#[derive(Debug, Clone)]
pub struct VoteResult {
    pub proposal_id: String,
    pub voter: String,
    pub approved: bool,
    pub status: String,
}
