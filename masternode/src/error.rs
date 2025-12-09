//! Masternode error types
#![allow(missing_docs)]
//!
//! Unified error handling for the masternode module. All errors can be converted
//! to `MasternodeError` for consistent error handling across the codebase.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MasternodeError {
    // Core masternode errors
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

    // Storage and serialization errors
    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    // Violation-specific errors
    #[error("Violation detected: {0}")]
    ViolationDetected(String),

    #[error("Invalid evidence: {0}")]
    InvalidEvidence(String),

    #[error("Evidence too old: age {age_secs}s exceeds max {max_age_secs}s")]
    StaleEvidence { age_secs: u64, max_age_secs: u64 },

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

    // Network errors
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Invalid peer response: {0}")]
    InvalidPeerResponse(String),

    // Wallet and cryptographic errors
    #[error("Invalid xpub: {0}")]
    InvalidXpub(String),

    #[error("Address derivation error: {0}")]
    AddressDerivation(String),

    #[error("Key generation error: {0}")]
    KeyGeneration(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Decryption error: {0}")]
    Decryption(String),

    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),

    #[error("UTXO error: {0}")]
    UtxoError(String),

    // Configuration errors
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Invalid configuration parameter: {param}={value}")]
    InvalidConfigValue { param: String, value: String },

    #[error("Missing required configuration: {0}")]
    MissingConfig(String),

    // Wallet file errors
    #[error("Wallet file error: {0}")]
    WalletFileError(String),

    #[error("Wallet not found")]
    WalletNotFound,

    #[error("Invalid wallet format")]
    InvalidWalletFormat,

    // I/O and system errors
    #[error("IO error: {0}")]
    IoError(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    // Validation errors
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Out of range: {param} must be between {min} and {max}, got {value}")]
    OutOfRange {
        param: String,
        min: i64,
        max: i64,
        value: i64,
    },
}

// Conversion from standard library errors
impl From<std::io::Error> for MasternodeError {
    fn from(err: std::io::Error) -> Self {
        MasternodeError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for MasternodeError {
    fn from(err: serde_json::Error) -> Self {
        MasternodeError::SerializationError(err.to_string())
    }
}

impl From<bincode::Error> for MasternodeError {
    fn from(err: bincode::Error) -> Self {
        MasternodeError::SerializationError(err.to_string())
    }
}

// Conversion from config errors
impl From<crate::config::MasternodeConfigError> for MasternodeError {
    fn from(err: crate::config::MasternodeConfigError) -> Self {
        match err {
            crate::config::MasternodeConfigError::IoError(e) => {
                MasternodeError::IoError(e.to_string())
            }
            crate::config::MasternodeConfigError::ParseError { line, message } => {
                MasternodeError::ConfigError(format!("Parse error at line {}: {}", line, message))
            }
            crate::config::MasternodeConfigError::InvalidFormat(msg) => {
                MasternodeError::ConfigError(msg)
            }
            crate::config::MasternodeConfigError::DuplicateAlias(alias) => {
                MasternodeError::ConfigError(format!("Duplicate alias: {}", alias))
            }
            crate::config::MasternodeConfigError::MasternodeNotFound(id) => {
                MasternodeError::NotFound(id)
            }
        }
    }
}

// Conversion from wallet errors
impl From<crate::wallet_dat::WalletDatError> for MasternodeError {
    fn from(err: crate::wallet_dat::WalletDatError) -> Self {
        match err {
            crate::wallet_dat::WalletDatError::IoError(e) => {
                MasternodeError::IoError(e.to_string())
            }
            crate::wallet_dat::WalletDatError::SerializationError(msg) => {
                MasternodeError::SerializationError(msg)
            }
            crate::wallet_dat::WalletDatError::WalletNotFound => MasternodeError::WalletNotFound,
            crate::wallet_dat::WalletDatError::InvalidFormat => {
                MasternodeError::InvalidWalletFormat
            }
            crate::wallet_dat::WalletDatError::KeyGenerationError => {
                MasternodeError::KeyGeneration("Wallet key generation failed".to_string())
            }
            crate::wallet_dat::WalletDatError::KeypairError(e) => {
                MasternodeError::KeyGeneration(e.to_string())
            }
            crate::wallet_dat::WalletDatError::WalletError(e) => {
                MasternodeError::WalletFileError(e.to_string())
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, MasternodeError>;
