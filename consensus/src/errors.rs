//! Consensus error types
//!
//! Unified error handling for all consensus operations

use std::fmt;

/// Consensus errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsensusError {
    /// Not enough nodes to achieve consensus
    NotEnoughNodes { required: usize, available: usize },

    /// Consensus threshold not reached
    ConsensusNotReached { approvals: usize, required: usize },

    /// Invalid proposal received
    InvalidProposal(String),

    /// Vote error
    VoteError(String),

    /// Duplicate vote detected
    DuplicateVote { voter: String, height: u64 },

    /// Voter not authorized
    UnauthorizedVoter(String),

    /// Invalid leader for height
    InvalidLeader {
        expected: String,
        actual: String,
        height: u64,
    },

    /// Consensus timeout
    Timeout,

    /// Network partition detected
    NetworkPartition,

    /// Byzantine behavior detected
    ByzantineNode { node_id: String, reason: String },

    /// Configuration error
    ConfigError(String),

    /// Internal error
    Internal(String),
}

impl fmt::Display for ConsensusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotEnoughNodes {
                required,
                available,
            } => {
                write!(
                    f,
                    "Not enough nodes for consensus: required {}, available {}",
                    required, available
                )
            }
            Self::ConsensusNotReached {
                approvals,
                required,
            } => {
                write!(
                    f,
                    "Consensus not reached: {} approvals received, {} required",
                    approvals, required
                )
            }
            Self::InvalidProposal(reason) => {
                write!(f, "Invalid proposal: {}", reason)
            }
            Self::VoteError(reason) => {
                write!(f, "Vote error: {}", reason)
            }
            Self::DuplicateVote { voter, height } => {
                write!(f, "Duplicate vote from {} at height {}", voter, height)
            }
            Self::UnauthorizedVoter(voter) => {
                write!(f, "Unauthorized voter: {}", voter)
            }
            Self::InvalidLeader {
                expected,
                actual,
                height,
            } => {
                write!(
                    f,
                    "Invalid leader at height {}: expected {}, got {}",
                    height, expected, actual
                )
            }
            Self::Timeout => {
                write!(f, "Consensus timeout")
            }
            Self::NetworkPartition => {
                write!(f, "Network partition detected")
            }
            Self::ByzantineNode { node_id, reason } => {
                write!(f, "Byzantine node {}: {}", node_id, reason)
            }
            Self::ConfigError(msg) => {
                write!(f, "Configuration error: {}", msg)
            }
            Self::Internal(msg) => {
                write!(f, "Internal error: {}", msg)
            }
        }
    }
}

impl std::error::Error for ConsensusError {}

impl From<ConsensusError> for String {
    fn from(e: ConsensusError) -> String {
        e.to_string()
    }
}

/// Type alias for consensus results
pub type ConsensusResult<T> = Result<T, ConsensusError>;

/// Configuration validation errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    InvalidRotationCount,
    TimeoutCannotBeZero,
    VotingTimeoutExceedsLeaderTimeout,
    InvalidThreshold {
        numerator: usize,
        denominator: usize,
    },
    InvalidQuorumSize {
        size: usize,
        min_required: usize,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRotationCount => {
                write!(f, "Invalid rotation count: must be > 0")
            }
            Self::TimeoutCannotBeZero => {
                write!(f, "Timeout cannot be zero")
            }
            Self::VotingTimeoutExceedsLeaderTimeout => {
                write!(f, "Voting timeout cannot exceed leader timeout")
            }
            Self::InvalidThreshold {
                numerator,
                denominator,
            } => {
                write!(
                    f,
                    "Invalid threshold: {}/{} (numerator must be <= denominator)",
                    numerator, denominator
                )
            }
            Self::InvalidQuorumSize { size, min_required } => {
                write!(
                    f,
                    "Invalid quorum size: {} (minimum required: {})",
                    size, min_required
                )
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<ConfigError> for ConsensusError {
    fn from(e: ConfigError) -> Self {
        ConsensusError::ConfigError(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ConsensusError::NotEnoughNodes {
            required: 5,
            available: 3,
        };
        assert_eq!(
            err.to_string(),
            "Not enough nodes for consensus: required 5, available 3"
        );
    }

    #[test]
    fn test_error_conversion() {
        let err = ConsensusError::Timeout;
        let s: String = err.into();
        assert_eq!(s, "Consensus timeout");
    }
}
