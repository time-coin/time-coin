//! Voting system for block consensus

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub block_hash: String,
    pub voter: String,
    pub vote_type: VoteType,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VoteType {
    Approve,
    Reject,
}

impl Vote {
    pub fn new(block_hash: String, voter: String, approve: bool) -> Self {
        Self {
            block_hash,
            voter,
            vote_type: if approve { VoteType::Approve } else { VoteType::Reject },
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    pub fn is_approval(&self) -> bool {
        matches!(self.vote_type, VoteType::Approve)
    }
}
