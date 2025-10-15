//! TIME Coin Governance Module
//!
//! Implements the masternode voting system for treasury proposals
//! and protocol parameter adjustments.

pub mod proposal;
pub mod voting;
pub mod masternode;
pub mod error;

pub use proposal::{Proposal, ProposalType, ProposalStatus, Milestone};
pub use voting::{Vote, VoteChoice, VotingPower, VotingResult};
pub use masternode::{MasternodeTier, Masternode};
pub use error::{GovernanceError, Result};

/// Governance configuration constants
pub mod config {
    /// Discussion period before voting starts (7 days)
    pub const DISCUSSION_PERIOD_DAYS: u64 = 7;
    
    /// Standard voting period (14 days)
    pub const VOTING_PERIOD_DAYS: u64 = 14;
    
    /// Emergency voting period (5 days)
    pub const EMERGENCY_VOTING_DAYS: u64 = 5;
    
    /// Required approval percentage (60%)
    pub const APPROVAL_THRESHOLD: u64 = 60;
    
    /// Required quorum percentage (60%)
    pub const QUORUM_THRESHOLD: u64 = 60;
    
    /// Emergency approval threshold (75%)
    pub const EMERGENCY_APPROVAL_THRESHOLD: u64 = 75;
    
    /// Proposal submission deposit (100 TIME)
    pub const SUBMISSION_DEPOSIT: u64 = 100 * crate::TIME_UNIT;
    
    /// Emergency proposal deposit (500 TIME)
    pub const EMERGENCY_DEPOSIT: u64 = 500 * crate::TIME_UNIT;
    
    /// Voting participation bonus (5%)
    pub const VOTING_BONUS_PERCENT: u64 = 5;
}

/// TIME token unit (8 decimal places)
pub const TIME_UNIT: u64 = 100_000_000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_governance_constants() {
        assert_eq!(config::APPROVAL_THRESHOLD, 60);
        assert_eq!(config::VOTING_PERIOD_DAYS, 14);
    }
}
