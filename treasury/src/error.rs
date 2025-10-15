//! Treasury error types

use thiserror::Error;

/// Treasury pool errors
#[derive(Error, Debug)]
pub enum TreasuryError {
    #[error("Insufficient treasury balance: requested {requested}, available {available}")]
    InsufficientBalance { requested: u64, available: u64 },
    
    #[error("Unauthorized withdrawal attempt")]
    UnauthorizedWithdrawal,
    
    #[error("Invalid amount: {0}")]
    InvalidAmount(String),
    
    #[error("Proposal not found: {0}")]
    ProposalNotFound(String),
    
    #[error("Proposal not approved: {0}")]
    ProposalNotApproved(String),
    
    #[error("Milestone not reached: {0}")]
    MilestoneNotReached(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub type Result<T> = std::result::Result<T, TreasuryError>;
