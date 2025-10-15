//! Voting system implementation

use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum VoteChoice {
    Yes,
    No,
    Abstain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub voter: String,
    pub proposal_id: String,
    pub choice: VoteChoice,
    pub voting_power: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingPower {
    pub masternode_id: String,
    pub tier: super::masternode::MasternodeTier,
    pub power: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingResult {
    pub proposal_id: String,
    pub total_power: u64,
    pub yes_power: u64,
    pub no_power: u64,
    pub abstain_power: u64,
    pub votes: Vec<Vote>,
}

impl VotingResult {
    pub fn new(proposal_id: String) -> Self {
        Self {
            proposal_id,
            total_power: 0,
            yes_power: 0,
            no_power: 0,
            abstain_power: 0,
            votes: Vec::new(),
        }
    }

    pub fn add_vote(&mut self, vote: Vote) {
        match vote.choice {
            VoteChoice::Yes => self.yes_power += vote.voting_power,
            VoteChoice::No => self.no_power += vote.voting_power,
            VoteChoice::Abstain => self.abstain_power += vote.voting_power,
        }
        self.total_power += vote.voting_power;
        self.votes.push(vote);
    }

    pub fn approval_percentage(&self) -> u64 {
        if self.total_power == 0 {
            return 0;
        }
        (self.yes_power * 100) / self.total_power
    }

    pub fn participation_rate(&self, total_possible: u64) -> u64 {
        if total_possible == 0 {
            return 0;
        }
        (self.total_power * 100) / total_possible
    }
}
