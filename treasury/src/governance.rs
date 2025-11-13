//! Treasury Governance Integration
//!
//! Integrates treasury operations with masternode governance for proposal
//! approval and fund distribution with 2/3+ masternode consensus.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::{Result, TreasuryError};

/// Treasury proposal for spending funds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryProposal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub recipient: String,
    pub amount: u64,
    pub submitter: String,
    pub submission_time: u64,
    pub voting_deadline: u64,
    pub execution_deadline: u64,
    pub status: ProposalStatus,
    pub votes: HashMap<String, Vote>,
    pub total_voting_power: u64,
}

/// Status of a treasury proposal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalStatus {
    /// Proposal is active and accepting votes
    Active,
    /// Proposal has been approved by consensus
    Approved,
    /// Proposal was rejected
    Rejected,
    /// Proposal has been executed (funds distributed)
    Executed,
    /// Proposal expired without reaching consensus
    Expired,
}

/// A vote on a treasury proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub masternode_id: String,
    pub vote_choice: VoteChoice,
    pub voting_power: u64,
    pub timestamp: u64,
}

/// Vote choice
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum VoteChoice {
    Yes,
    No,
    Abstain,
}

/// Parameters for creating a new treasury proposal
#[derive(Debug, Clone)]
pub struct ProposalParams {
    pub id: String,
    pub title: String,
    pub description: String,
    pub recipient: String,
    pub amount: u64,
    pub submitter: String,
    pub submission_time: u64,
    pub voting_period_days: u64,
}

impl TreasuryProposal {
    /// Create a new treasury proposal
    pub fn new(params: ProposalParams) -> Self {
        let voting_deadline = params.submission_time + (params.voting_period_days * 86400);
        let execution_deadline = voting_deadline + (30 * 86400); // 30 days to execute

        Self {
            id: params.id,
            title: params.title,
            description: params.description,
            recipient: params.recipient,
            amount: params.amount,
            submitter: params.submitter,
            submission_time: params.submission_time,
            voting_deadline,
            execution_deadline,
            status: ProposalStatus::Active,
            votes: HashMap::new(),
            total_voting_power: 0,
        }
    }

    /// Add a vote from a masternode
    pub fn add_vote(
        &mut self,
        masternode_id: String,
        vote_choice: VoteChoice,
        voting_power: u64,
        timestamp: u64,
    ) -> Result<()> {
        // Check if proposal is still active
        if self.status != ProposalStatus::Active {
            return Err(TreasuryError::InvalidAmount(format!(
                "Cannot vote on proposal with status {:?}",
                self.status
            )));
        }

        // Check if voting period has ended
        if timestamp > self.voting_deadline {
            return Err(TreasuryError::InvalidAmount(
                "Voting period has ended".to_string(),
            ));
        }

        // Check if masternode has already voted
        if self.votes.contains_key(&masternode_id) {
            return Err(TreasuryError::InvalidAmount(format!(
                "Masternode {} has already voted",
                masternode_id
            )));
        }

        // Add the vote
        let vote = Vote {
            masternode_id: masternode_id.clone(),
            vote_choice,
            voting_power,
            timestamp,
        };

        self.votes.insert(masternode_id, vote);
        Ok(())
    }

    /// Calculate voting results
    pub fn calculate_results(&self) -> VotingResults {
        let mut yes_power = 0;
        let mut no_power = 0;
        let mut abstain_power = 0;

        for vote in self.votes.values() {
            match vote.vote_choice {
                VoteChoice::Yes => yes_power += vote.voting_power,
                VoteChoice::No => no_power += vote.voting_power,
                VoteChoice::Abstain => abstain_power += vote.voting_power,
            }
        }

        let total_votes = yes_power + no_power + abstain_power;

        VotingResults {
            yes_power,
            no_power,
            abstain_power,
            total_votes,
            total_possible_power: self.total_voting_power,
        }
    }

    /// Check if proposal has reached 2/3+ approval
    pub fn has_approval(&self) -> bool {
        let results = self.calculate_results();

        // Require 2/3 (67%) of YES votes from participating masternodes
        if results.total_votes == 0 {
            return false;
        }

        // Calculate percentage of YES votes
        let yes_percentage = (results.yes_power * 100) / results.total_votes;
        
        // Require at least 67% YES votes
        yes_percentage >= 67
    }

    /// Update proposal status based on current time and votes
    pub fn update_status(&mut self, current_time: u64) {
        if self.status != ProposalStatus::Active {
            return;
        }

        // Check if voting period has ended
        if current_time > self.voting_deadline {
            // Check if proposal was approved
            if self.has_approval() {
                self.status = ProposalStatus::Approved;
            } else {
                self.status = ProposalStatus::Rejected;
            }
        }
    }

    /// Check if proposal has expired without execution
    pub fn is_expired(&self, current_time: u64) -> bool {
        self.status == ProposalStatus::Approved && current_time > self.execution_deadline
    }

    /// Mark proposal as executed
    pub fn mark_executed(&mut self) -> Result<()> {
        if self.status != ProposalStatus::Approved {
            return Err(TreasuryError::ProposalNotApproved(self.id.clone()));
        }
        self.status = ProposalStatus::Executed;
        Ok(())
    }

    /// Set the total voting power (total of all active masternodes)
    pub fn set_total_voting_power(&mut self, power: u64) {
        self.total_voting_power = power;
    }
}

/// Voting results summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingResults {
    pub yes_power: u64,
    pub no_power: u64,
    pub abstain_power: u64,
    pub total_votes: u64,
    pub total_possible_power: u64,
}

impl VotingResults {
    /// Get approval percentage (YES / total votes)
    pub fn approval_percentage(&self) -> u64 {
        if self.total_votes == 0 {
            return 0;
        }
        (self.yes_power * 100) / self.total_votes
    }

    /// Get participation rate (total votes / total possible)
    pub fn participation_rate(&self) -> u64 {
        if self.total_possible_power == 0 {
            return 0;
        }
        (self.total_votes * 100) / self.total_possible_power
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_proposal() {
        let proposal = TreasuryProposal::new(ProposalParams {
            id: "prop-1".to_string(),
            title: "Test Proposal".to_string(),
            description: "A test proposal".to_string(),
            recipient: "recipient123".to_string(),
            amount: 1000000,
            submitter: "submitter123".to_string(),
            submission_time: 1000,
            voting_period_days: 14, // 14 days voting period
        });

        assert_eq!(proposal.status, ProposalStatus::Active);
        assert_eq!(proposal.votes.len(), 0);
        assert_eq!(proposal.voting_deadline, 1000 + (14 * 86400));
    }

    #[test]
    fn test_add_vote() {
        let mut proposal = TreasuryProposal::new(ProposalParams {
            id: "prop-1".to_string(),
            title: "Test".to_string(),
            description: "Desc".to_string(),
            recipient: "recipient".to_string(),
            amount: 1000000,
            submitter: "submitter".to_string(),
            submission_time: 1000,
            voting_period_days: 14,
        });

        // Add a vote
        let result = proposal.add_vote("mn1".to_string(), VoteChoice::Yes, 100, 2000);
        assert!(result.is_ok());
        assert_eq!(proposal.votes.len(), 1);

        // Try to vote again - should fail
        let result = proposal.add_vote("mn1".to_string(), VoteChoice::No, 100, 2000);
        assert!(result.is_err());
    }

    #[test]
    fn test_voting_results() {
        let mut proposal = TreasuryProposal::new(ProposalParams {
            id: "prop-1".to_string(),
            title: "Test".to_string(),
            description: "Desc".to_string(),
            recipient: "recipient".to_string(),
            amount: 1000000,
            submitter: "submitter".to_string(),
            submission_time: 1000,
            voting_period_days: 14,
        });

        proposal.set_total_voting_power(300);

        // Add votes
        proposal
            .add_vote("mn1".to_string(), VoteChoice::Yes, 100, 2000)
            .unwrap();
        proposal
            .add_vote("mn2".to_string(), VoteChoice::Yes, 100, 2000)
            .unwrap();
        proposal
            .add_vote("mn3".to_string(), VoteChoice::No, 50, 2000)
            .unwrap();

        let results = proposal.calculate_results();
        assert_eq!(results.yes_power, 200);
        assert_eq!(results.no_power, 50);
        assert_eq!(results.total_votes, 250);

        // 200/250 = 80% YES - should be approved
        assert!(proposal.has_approval());
    }

    #[test]
    fn test_approval_threshold() {
        let mut proposal = TreasuryProposal::new(ProposalParams {
            id: "prop-1".to_string(),
            title: "Test".to_string(),
            description: "Desc".to_string(),
            recipient: "recipient".to_string(),
            amount: 1000000,
            submitter: "submitter".to_string(),
            submission_time: 1000,
            voting_period_days: 14,
        });

        proposal.set_total_voting_power(300);

        // 66% YES - should NOT be approved (need 67%)
        proposal
            .add_vote("mn1".to_string(), VoteChoice::Yes, 66, 2000)
            .unwrap();
        proposal
            .add_vote("mn2".to_string(), VoteChoice::No, 34, 2000)
            .unwrap();

        assert!(!proposal.has_approval());

        // Add more YES votes to reach 67%
        proposal
            .add_vote("mn3".to_string(), VoteChoice::Yes, 2, 2000)
            .unwrap();

        // Now 68/102 = 66.67% - still not quite 67%
        assert!(!proposal.has_approval());

        // Add one more YES vote
        proposal
            .add_vote("mn4".to_string(), VoteChoice::Yes, 1, 2000)
            .unwrap();

        // Now 69/103 = 66.99% - still not 67%
        // Let's use better numbers: 67 YES out of 100 total
        let mut proposal2 = TreasuryProposal::new(ProposalParams {
            id: "prop-2".to_string(),
            title: "Test".to_string(),
            description: "Desc".to_string(),
            recipient: "recipient".to_string(),
            amount: 1000000,
            submitter: "submitter".to_string(),
            submission_time: 1000,
            voting_period_days: 14,
        });

        // 67 YES, 33 NO = exactly 67%
        proposal2
            .add_vote("mn1".to_string(), VoteChoice::Yes, 67, 2000)
            .unwrap();
        proposal2
            .add_vote("mn2".to_string(), VoteChoice::No, 33, 2000)
            .unwrap();

        assert!(proposal2.has_approval());
    }

    #[test]
    fn test_status_update() {
        let mut proposal = TreasuryProposal::new(ProposalParams {
            id: "prop-1".to_string(),
            title: "Test".to_string(),
            description: "Desc".to_string(),
            recipient: "recipient".to_string(),
            amount: 1000000,
            submitter: "submitter".to_string(),
            submission_time: 1000,
            voting_period_days: 14,
        });

        // Add enough votes for approval
        proposal
            .add_vote("mn1".to_string(), VoteChoice::Yes, 70, 2000)
            .unwrap();
        proposal
            .add_vote("mn2".to_string(), VoteChoice::No, 30, 2000)
            .unwrap();

        assert!(proposal.has_approval());

        // Update status after voting deadline
        let after_deadline = proposal.voting_deadline + 1;
        proposal.update_status(after_deadline);

        assert_eq!(proposal.status, ProposalStatus::Approved);
    }

    #[test]
    fn test_proposal_expiration() {
        let mut proposal = TreasuryProposal::new(ProposalParams {
            id: "prop-1".to_string(),
            title: "Test".to_string(),
            description: "Desc".to_string(),
            recipient: "recipient".to_string(),
            amount: 1000000,
            submitter: "submitter".to_string(),
            submission_time: 1000,
            voting_period_days: 14,
        });

        proposal.status = ProposalStatus::Approved;

        // Not expired yet
        assert!(!proposal.is_expired(proposal.execution_deadline));

        // Expired
        assert!(proposal.is_expired(proposal.execution_deadline + 1));
    }
}
