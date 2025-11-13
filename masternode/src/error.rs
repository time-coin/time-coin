//! Masternode error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MasternodeError {
    #[error("Insufficient collateral: required {required}, provided {provided}")]
    InsufficientCollateral { required: u64, provided: u64 },

    #[error("Masternode not found: {0}")]
    NotFound(String),

    #[error("Masternode already registered: {0}")]
    AlreadyRegistered(String),

    #[error("Invalid masternode status: {0}")]
    InvalidStatus(String),

    #[error("Reputation too low: {score}")]
    LowReputation { score: i32 },

    #[error("Heartbeat timeout: last seen {last_seen} seconds ago")]
    HeartbeatTimeout { last_seen: u64 },

    #[error("Invalid tier: {0}")]
    InvalidTier(String),

    #[error("Slashed masternode cannot participate")]
    Slashed,

    #[error("Not eligible for rewards")]
    NotEligible,

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Violation detected: {0}")]
    ViolationDetected(String),
}

pub type Result<T> = std::result::Result<T, MasternodeError>;
