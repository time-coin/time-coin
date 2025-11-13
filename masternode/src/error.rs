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

    // Violation-specific errors
    #[error("Violation detected: {0}")]
    ViolationDetected(String),

    #[error("Invalid evidence: {0}")]
    InvalidEvidence(String),

    #[error("Detector error: {0}")]
    DetectorError(String),

    #[error("Double-signing detected at block {block_height}")]
    DoubleSigning { block_height: u64 },

    #[error("Extended downtime: {days} days offline")]
    ExtendedDowntime { days: u64 },

    #[error("Data withholding: {consecutive_failures} consecutive failures")]
    DataWithholding { consecutive_failures: u32 },

    #[error("Network manipulation detected")]
    NetworkManipulation,
}

pub type Result<T> = std::result::Result<T, MasternodeError>;
