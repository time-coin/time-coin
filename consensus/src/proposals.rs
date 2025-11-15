//! Treasury grant proposal system
//!
//! Allows masternodes to propose and vote on treasury grants.
//! Approved grants are automatically funded in the next block.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Status of a proposal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalStatus {
    Pending,
    Approved,
    Rejected,
    Executed,
}

/// A treasury grant proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub proposer: String,  // Masternode IP that proposed it
    pub recipient: String, // Wallet address to receive funds
    pub amount: u64,       // Amount in satoshis
    pub reason: String,
    pub created_at: i64,
    pub status: ProposalStatus,
    pub votes_for: Vec<String>, // List of masternode IPs that voted yes
    pub votes_against: Vec<String>, // List of masternode IPs that voted no
    pub executed_at: Option<i64>,
    pub tx_id: Option<String>, // Transaction ID when executed
}

impl Proposal {
    pub fn new(proposer: String, recipient: String, amount: u64, reason: String) -> Self {
        let id = format!(
            "proposal_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );

        Proposal {
            id,
            proposer,
            recipient,
            amount,
            reason,
            created_at: chrono::Utc::now().timestamp(),
            status: ProposalStatus::Pending,
            votes_for: Vec::new(),
            votes_against: Vec::new(),
            executed_at: None,
            tx_id: None,
        }
    }

    /// Check if proposal has reached consensus (2/3+ approval)
    pub fn has_consensus(&self, total_masternodes: usize) -> bool {
        if total_masternodes < 3 {
            return false; // Need at least 3 masternodes for BFT
        }

        let total_votes = self.votes_for.len() + self.votes_against.len();

        // Need at least 2/3 of masternodes to have voted
        if total_votes < (total_masternodes * 2 + 2) / 3 {
            return false;
        }

        // Need 2/3+ approval from those who voted
        let required = (total_votes * 2 + 2) / 3;
        self.votes_for.len() >= required
    }

    /// Check if proposal is rejected (1/3+ rejection)
    pub fn is_rejected(&self, total_masternodes: usize) -> bool {
        if total_masternodes < 3 {
            return false;
        }

        // If more than 1/3 reject, proposal is dead
        let rejection_threshold = (total_masternodes + 2) / 3;
        self.votes_against.len() > rejection_threshold
    }
}

/// Manages treasury grant proposals
pub struct ProposalManager {
    proposals: Arc<RwLock<HashMap<String, Proposal>>>,
    data_dir: String,
}

impl ProposalManager {
    pub fn new(data_dir: String) -> Self {
        ProposalManager {
            proposals: Arc::new(RwLock::new(HashMap::new())),
            data_dir,
        }
    }

    /// Load proposals from disk
    pub async fn load(&self) -> Result<(), String> {
        let path = format!("{}/proposals.json", self.data_dir);

        if !std::path::Path::new(&path).exists() {
            // No proposals file yet, start fresh
            return Ok(());
        }

        let data = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| format!("Failed to read proposals: {}", e))?;

        let proposals: Vec<Proposal> =
            serde_json::from_str(&data).map_err(|e| format!("Failed to parse proposals: {}", e))?;

        let mut map = self.proposals.write().await;
        for proposal in proposals {
            map.insert(proposal.id.clone(), proposal);
        }

        Ok(())
    }

    /// Save proposals to disk
    pub async fn save(&self) -> Result<(), String> {
        let path = format!("{}/proposals.json", self.data_dir);

        let proposals: Vec<Proposal> = self.proposals.read().await.values().cloned().collect();

        let json = serde_json::to_string_pretty(&proposals)
            .map_err(|e| format!("Failed to serialize proposals: {}", e))?;

        tokio::fs::write(&path, json)
            .await
            .map_err(|e| format!("Failed to write proposals: {}", e))?;

        Ok(())
    }

    /// Create a new proposal
    pub async fn create_proposal(
        &self,
        proposer: String,
        recipient: String,
        amount: u64,
        reason: String,
    ) -> Result<Proposal, String> {
        let proposal = Proposal::new(proposer, recipient, amount, reason);

        self.proposals
            .write()
            .await
            .insert(proposal.id.clone(), proposal.clone());
        self.save().await?;

        Ok(proposal)
    }

    /// Vote on a proposal
    pub async fn vote(
        &self,
        proposal_id: &str,
        voter: String,
        approve: bool,
    ) -> Result<(), String> {
        let mut proposals = self.proposals.write().await;

        let proposal = proposals
            .get_mut(proposal_id)
            .ok_or_else(|| "Proposal not found".to_string())?;

        // Can't vote on executed proposals
        if proposal.status == ProposalStatus::Executed {
            return Err("Cannot vote on executed proposal".to_string());
        }

        // Remove any existing vote from this voter
        proposal.votes_for.retain(|v| v != &voter);
        proposal.votes_against.retain(|v| v != &voter);

        // Add new vote
        if approve {
            proposal.votes_for.push(voter);
        } else {
            proposal.votes_against.push(voter);
        }

        drop(proposals);
        self.save().await?;

        Ok(())
    }

    /// Get all proposals
    pub async fn get_all(&self) -> Vec<Proposal> {
        self.proposals.read().await.values().cloned().collect()
    }

    /// Get a specific proposal
    pub async fn get(&self, id: &str) -> Option<Proposal> {
        self.proposals.read().await.get(id).cloned()
    }

    /// Get approved proposals that haven't been executed yet
    pub async fn get_approved_pending(&self, total_masternodes: usize) -> Vec<Proposal> {
        self.proposals
            .read()
            .await
            .values()
            .filter(|p| p.status == ProposalStatus::Pending && p.has_consensus(total_masternodes))
            .cloned()
            .collect()
    }

    /// Mark a proposal as executed
    pub async fn mark_executed(&self, proposal_id: &str, tx_id: String) -> Result<(), String> {
        let mut proposals = self.proposals.write().await;

        let proposal = proposals
            .get_mut(proposal_id)
            .ok_or_else(|| "Proposal not found".to_string())?;

        proposal.status = ProposalStatus::Executed;
        proposal.executed_at = Some(chrono::Utc::now().timestamp());
        proposal.tx_id = Some(tx_id);

        drop(proposals);
        self.save().await?;

        Ok(())
    }

    /// Update proposal statuses based on votes
    pub async fn update_statuses(&self, total_masternodes: usize) {
        let mut proposals = self.proposals.write().await;

        for proposal in proposals.values_mut() {
            if proposal.status != ProposalStatus::Pending {
                continue;
            }

            if proposal.has_consensus(total_masternodes) {
                proposal.status = ProposalStatus::Approved;
            } else if proposal.is_rejected(total_masternodes) {
                proposal.status = ProposalStatus::Rejected;
            }
        }
    }
}
