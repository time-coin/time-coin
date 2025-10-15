//! Governance error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GovernanceError {
    #[error("Invalid proposal: {0}")]
    InvalidProposal(String),

    #[error("Insufficient deposit: required {required}, provided {provided}")]
    InsufficientDeposit { required: u64, provided: u64 },

    #[error("Voting period not active: {0}")]
    VotingNotActive(String),

    #[error("Already voted: {0}")]
    AlreadyVoted(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Proposal not found: {0}")]
    ProposalNotFound(String),

    #[error("Quorum not reached: {current}% of {required}%")]
    QuorumNotReached { current: u64, required: u64 },

    #[error("Threshold not met: {yes}% approval, {required}% required")]
    ThresholdNotMet { yes: u64, required: u64 },
}

pub type Result<T> = std::result::Result<T, GovernanceError>;
