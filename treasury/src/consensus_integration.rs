//! Consensus Integration for Treasury Proposals
//!
//! This module integrates treasury proposal voting with the consensus engine
//! to ensure 2/3+ masternode approval for all treasury spending.

use crate::error::{Result, TreasuryError};
use crate::governance::{ProposalStatus, TreasuryProposal, VoteChoice, VotingResults};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Treasury consensus manager that handles proposal consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryConsensusManager {
    /// Active proposals indexed by ID
    proposals: HashMap<String, TreasuryProposal>,
    
    /// Total voting power of all active masternodes
    total_voting_power: u64,
    
    /// Active masternodes with their voting power
    masternode_power: HashMap<String, u64>,
}

impl TreasuryConsensusManager {
    /// Create a new treasury consensus manager
    pub fn new() -> Self {
        Self {
            proposals: HashMap::new(),
            total_voting_power: 0,
            masternode_power: HashMap::new(),
        }
    }
    
    /// Register a masternode with its voting power
    pub fn register_masternode(&mut self, masternode_id: String, voting_power: u64) {
        let old_power = self.masternode_power.insert(masternode_id, voting_power);
        if let Some(old) = old_power {
            self.total_voting_power -= old;
        }
        self.total_voting_power += voting_power;
    }
    
    /// Unregister a masternode
    pub fn unregister_masternode(&mut self, masternode_id: &str) {
        if let Some(power) = self.masternode_power.remove(masternode_id) {
            self.total_voting_power -= power;
        }
    }
    
    /// Get total voting power
    pub fn get_total_voting_power(&self) -> u64 {
        self.total_voting_power
    }
    
    /// Add a new proposal to the consensus system
    pub fn add_proposal(&mut self, mut proposal: TreasuryProposal) -> Result<()> {
        // Check if proposal already exists
        if self.proposals.contains_key(&proposal.id) {
            return Err(TreasuryError::InvalidAmount(format!(
                "Proposal {} already exists",
                proposal.id
            )));
        }
        
        // Set the total voting power for this proposal
        proposal.set_total_voting_power(self.total_voting_power);
        
        self.proposals.insert(proposal.id.clone(), proposal);
        Ok(())
    }
    
    /// Cast a vote on a proposal
    pub fn vote_on_proposal(
        &mut self,
        proposal_id: &str,
        masternode_id: String,
        vote_choice: VoteChoice,
        timestamp: u64,
    ) -> Result<()> {
        // Get the proposal
        let proposal = self
            .proposals
            .get_mut(proposal_id)
            .ok_or_else(|| TreasuryError::InvalidAmount(format!("Proposal {} not found", proposal_id)))?;
        
        // Get masternode voting power
        let voting_power = *self
            .masternode_power
            .get(&masternode_id)
            .ok_or_else(|| TreasuryError::InvalidAmount(format!(
                "Masternode {} not registered",
                masternode_id
            )))?;
        
        // Add the vote
        proposal.add_vote(masternode_id, vote_choice, voting_power, timestamp)?;
        
        Ok(())
    }
    
    /// Check if a proposal has reached consensus (2/3+ approval)
    pub fn has_consensus(&self, proposal_id: &str) -> Result<bool> {
        let proposal = self
            .proposals
            .get(proposal_id)
            .ok_or_else(|| TreasuryError::InvalidAmount(format!("Proposal {} not found", proposal_id)))?;
        
        Ok(proposal.has_approval())
    }
    
    /// Update proposal statuses based on current time
    pub fn update_proposal_statuses(&mut self, current_time: u64) {
        for proposal in self.proposals.values_mut() {
            proposal.update_status(current_time);
        }
    }
    
    /// Get proposal by ID
    pub fn get_proposal(&self, proposal_id: &str) -> Option<&TreasuryProposal> {
        self.proposals.get(proposal_id)
    }
    
    /// Get mutable proposal by ID
    pub fn get_proposal_mut(&mut self, proposal_id: &str) -> Option<&mut TreasuryProposal> {
        self.proposals.get_mut(proposal_id)
    }
    
    /// Get all proposals
    pub fn get_all_proposals(&self) -> Vec<&TreasuryProposal> {
        self.proposals.values().collect()
    }
    
    /// Get proposals by status
    pub fn get_proposals_by_status(&self, status: &ProposalStatus) -> Vec<&TreasuryProposal> {
        self.proposals
            .values()
            .filter(|p| &p.status == status)
            .collect()
    }
    
    /// Mark proposal as executed
    pub fn mark_proposal_executed(&mut self, proposal_id: &str) -> Result<()> {
        let proposal = self
            .proposals
            .get_mut(proposal_id)
            .ok_or_else(|| TreasuryError::InvalidAmount(format!("Proposal {} not found", proposal_id)))?;
        
        proposal.mark_executed()
    }
    
    /// Expire old approved proposals that weren't executed in time
    pub fn expire_old_proposals(&mut self, current_time: u64) -> Vec<String> {
        let mut expired_ids = Vec::new();
        
        for (id, proposal) in self.proposals.iter_mut() {
            if proposal.is_expired(current_time) {
                proposal.status = ProposalStatus::Expired;
                expired_ids.push(id.clone());
            }
        }
        
        expired_ids
    }
    
    /// Get voting results for a proposal
    pub fn get_voting_results(&self, proposal_id: &str) -> Result<VotingResults> {
        let proposal = self
            .proposals
            .get(proposal_id)
            .ok_or_else(|| TreasuryError::InvalidAmount(format!("Proposal {} not found", proposal_id)))?;
        
        Ok(proposal.calculate_results())
    }
    
    /// Check if voting deadline has passed for a proposal
    pub fn is_voting_ended(&self, proposal_id: &str, current_time: u64) -> Result<bool> {
        let proposal = self
            .proposals
            .get(proposal_id)
            .ok_or_else(|| TreasuryError::InvalidAmount(format!("Proposal {} not found", proposal_id)))?;
        
        Ok(current_time > proposal.voting_deadline)
    }
    
    /// Remove old proposals to free memory (keep only recent ones)
    pub fn cleanup_old_proposals(&mut self, current_time: u64, keep_days: u64) {
        let cutoff_time = current_time.saturating_sub(keep_days * 86400);
        
        self.proposals.retain(|_, proposal| {
            // Keep if:
            // - Still active or approved (not executed/rejected/expired yet)
            // - Or was recently updated
            match proposal.status {
                ProposalStatus::Active | ProposalStatus::Approved => true,
                _ => proposal.submission_time > cutoff_time,
            }
        });
    }
}

impl Default for TreasuryConsensusManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::ProposalParams;

    #[test]
    fn test_register_masternodes() {
        let mut manager = TreasuryConsensusManager::new();
        
        manager.register_masternode("mn1".to_string(), 100);
        manager.register_masternode("mn2".to_string(), 50);
        manager.register_masternode("mn3".to_string(), 25);
        
        assert_eq!(manager.get_total_voting_power(), 175);
    }
    
    #[test]
    fn test_add_proposal() {
        let mut manager = TreasuryConsensusManager::new();
        
        manager.register_masternode("mn1".to_string(), 100);
        
        let proposal = TreasuryProposal::new(ProposalParams {
            id: "prop-1".to_string(),
            title: "Test Proposal".to_string(),
            description: "A test proposal".to_string(),
            recipient: "recipient123".to_string(),
            amount: 1000000,
            submitter: "submitter123".to_string(),
            submission_time: 1000,
            voting_period_days: 14,
        });
        
        let result = manager.add_proposal(proposal);
        assert!(result.is_ok());
        
        let stored = manager.get_proposal("prop-1").unwrap();
        assert_eq!(stored.total_voting_power, 100);
    }
    
    #[test]
    fn test_voting_consensus() {
        let mut manager = TreasuryConsensusManager::new();
        
        // Register masternodes with voting power
        manager.register_masternode("mn1".to_string(), 100);
        manager.register_masternode("mn2".to_string(), 100);
        manager.register_masternode("mn3".to_string(), 100);
        
        // Add proposal
        let proposal = TreasuryProposal::new(ProposalParams {
            id: "prop-1".to_string(),
            title: "Test".to_string(),
            description: "Desc".to_string(),
            recipient: "recipient".to_string(),
            amount: 1000000,
            submitter: "submitter".to_string(),
            submission_time: 1000,
            voting_period_days: 14,
        });
        manager.add_proposal(proposal).unwrap();
        
        // Vote: 2 YES, 1 NO = 66.67% - should NOT reach consensus (need 67%)
        manager.vote_on_proposal("prop-1", "mn1".to_string(), VoteChoice::Yes, 2000).unwrap();
        manager.vote_on_proposal("prop-1", "mn2".to_string(), VoteChoice::Yes, 2000).unwrap();
        manager.vote_on_proposal("prop-1", "mn3".to_string(), VoteChoice::No, 2000).unwrap();
        
        // 200 YES out of 300 total = 66.67% - not enough
        assert!(!manager.has_consensus("prop-1").unwrap());
        
        // Now test with 3 masternodes where 2 vote YES and 1 abstains
        let mut manager2 = TreasuryConsensusManager::new();
        manager2.register_masternode("mn1".to_string(), 100);
        manager2.register_masternode("mn2".to_string(), 100);
        manager2.register_masternode("mn3".to_string(), 50);
        
        let proposal2 = TreasuryProposal::new(ProposalParams {
            id: "prop-2".to_string(),
            title: "Test".to_string(),
            description: "Desc".to_string(),
            recipient: "recipient".to_string(),
            amount: 1000000,
            submitter: "submitter".to_string(),
            submission_time: 1000,
            voting_period_days: 14,
        });
        manager2.add_proposal(proposal2).unwrap();
        
        // Vote: 200 YES, 50 NO = 80% - should reach consensus
        manager2.vote_on_proposal("prop-2", "mn1".to_string(), VoteChoice::Yes, 2000).unwrap();
        manager2.vote_on_proposal("prop-2", "mn2".to_string(), VoteChoice::Yes, 2000).unwrap();
        manager2.vote_on_proposal("prop-2", "mn3".to_string(), VoteChoice::No, 2000).unwrap();
        
        assert!(manager2.has_consensus("prop-2").unwrap());
    }
    
    #[test]
    fn test_proposal_status_update() {
        let mut manager = TreasuryConsensusManager::new();
        
        manager.register_masternode("mn1".to_string(), 100);
        manager.register_masternode("mn2".to_string(), 50);
        
        let proposal = TreasuryProposal::new(ProposalParams {
            id: "prop-1".to_string(),
            title: "Test".to_string(),
            description: "Desc".to_string(),
            recipient: "recipient".to_string(),
            amount: 1000000,
            submitter: "submitter".to_string(),
            submission_time: 1000,
            voting_period_days: 14,
        });
        let deadline = proposal.voting_deadline;
        manager.add_proposal(proposal).unwrap();
        
        // Vote with enough approval
        manager.vote_on_proposal("prop-1", "mn1".to_string(), VoteChoice::Yes, 2000).unwrap();
        manager.vote_on_proposal("prop-1", "mn2".to_string(), VoteChoice::Yes, 2000).unwrap();
        
        // Update status after deadline
        manager.update_proposal_statuses(deadline + 1);
        
        let proposal = manager.get_proposal("prop-1").unwrap();
        assert_eq!(proposal.status, ProposalStatus::Approved);
    }
    
    #[test]
    fn test_proposal_expiration() {
        let mut manager = TreasuryConsensusManager::new();
        
        manager.register_masternode("mn1".to_string(), 100);
        
        let proposal = TreasuryProposal::new(ProposalParams {
            id: "prop-1".to_string(),
            title: "Test".to_string(),
            description: "Desc".to_string(),
            recipient: "recipient".to_string(),
            amount: 1000000,
            submitter: "submitter".to_string(),
            submission_time: 1000,
            voting_period_days: 14,
        });
        let execution_deadline = proposal.execution_deadline;
        manager.add_proposal(proposal).unwrap();
        
        // Manually set to approved
        manager.get_proposal_mut("prop-1").unwrap().status = ProposalStatus::Approved;
        
        // Expire proposals after execution deadline
        let expired = manager.expire_old_proposals(execution_deadline + 1);
        
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], "prop-1");
        
        let proposal = manager.get_proposal("prop-1").unwrap();
        assert_eq!(proposal.status, ProposalStatus::Expired);
    }
    
    #[test]
    fn test_multiple_proposals() {
        let mut manager = TreasuryConsensusManager::new();
        
        manager.register_masternode("mn1".to_string(), 100);
        manager.register_masternode("mn2".to_string(), 100);
        manager.register_masternode("mn3".to_string(), 100);
        
        // Add multiple proposals
        for i in 1..=5 {
            let proposal = TreasuryProposal::new(ProposalParams {
                id: format!("prop-{}", i),
                title: format!("Proposal {}", i),
                description: "Test".to_string(),
                recipient: "recipient".to_string(),
                amount: 1000000 * i as u64,
                submitter: "submitter".to_string(),
                submission_time: 1000,
                voting_period_days: 14,
            });
            manager.add_proposal(proposal).unwrap();
        }
        
        assert_eq!(manager.get_all_proposals().len(), 5);
        
        // Vote on some proposals
        manager.vote_on_proposal("prop-1", "mn1".to_string(), VoteChoice::Yes, 2000).unwrap();
        manager.vote_on_proposal("prop-1", "mn2".to_string(), VoteChoice::Yes, 2000).unwrap();
        
        manager.vote_on_proposal("prop-2", "mn1".to_string(), VoteChoice::No, 2000).unwrap();
        manager.vote_on_proposal("prop-2", "mn2".to_string(), VoteChoice::No, 2000).unwrap();
        
        // Check results
        let results1 = manager.get_voting_results("prop-1").unwrap();
        assert_eq!(results1.yes_power, 200);
        
        let results2 = manager.get_voting_results("prop-2").unwrap();
        assert_eq!(results2.no_power, 200);
    }
    
    #[test]
    fn test_masternode_power_update() {
        let mut manager = TreasuryConsensusManager::new();
        
        manager.register_masternode("mn1".to_string(), 100);
        assert_eq!(manager.get_total_voting_power(), 100);
        
        // Update masternode power
        manager.register_masternode("mn1".to_string(), 200);
        assert_eq!(manager.get_total_voting_power(), 200);
        
        // Unregister masternode
        manager.unregister_masternode("mn1");
        assert_eq!(manager.get_total_voting_power(), 0);
    }
}
