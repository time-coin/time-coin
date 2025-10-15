//! Proposal types and management

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalType {
    DevelopmentGrant,
    MarketingInitiative,
    SecurityAudit,
    Infrastructure,
    Research,
    CommunityProgram,
    EmergencyAction,
    ParameterChange,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalStatus {
    Draft,
    Submitted,
    Discussion,
    Voting,
    Approved,
    Rejected,
    Executed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub description: String,
    pub amount: u64,
    pub due_date: u64,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub title: String,
    pub proposal_type: ProposalType,
    pub status: ProposalStatus,
    pub submitter: String,
    pub requested_amount: u64,
    pub deposit: u64,
    pub submission_time: u64,
    pub discussion_end: u64,
    pub voting_end: u64,
    pub description: String,
    pub milestones: Vec<Milestone>,
}

impl Proposal {
    pub fn new(
        id: String,
        title: String,
        proposal_type: ProposalType,
        submitter: String,
        requested_amount: u64,
        description: String,
    ) -> Self {
        let now = current_timestamp();
        
        Self {
            id,
            title,
            proposal_type,
            status: ProposalStatus::Draft,
            submitter,
            requested_amount,
            deposit: 0,
            submission_time: now,
            discussion_end: now + (7 * 86400), // 7 days
            voting_end: now + (21 * 86400), // 7 + 14 days
            description,
            milestones: Vec::new(),
        }
    }
    
    pub fn add_milestone(&mut self, milestone: Milestone) {
        self.milestones.push(milestone);
    }
}

fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
