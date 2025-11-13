//! Masternode violation types and detection
//!
//! This module defines the types of violations that masternodes can commit
//! and the penalties associated with each violation type.

use crate::error::{MasternodeError, Result};
use serde::{Deserialize, Serialize};

/// Types of violations that can be detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationType {
    /// Masternode signed two different blocks at the same height
    DoubleSigning,
    /// Masternode created an invalid block (bad merkle root, invalid txs, etc.)
    InvalidBlock,
    /// Masternode has been offline for more than 90 days
    ExtendedDowntime,
    /// Masternode withheld data or refused to respond to valid requests
    DataWithholding,
    /// Masternode attempted to manipulate consensus (e.g., vote manipulation)
    NetworkManipulation,
}

impl ViolationType {
    /// Get the severity level of this violation
    pub fn severity(&self) -> ViolationSeverity {
        match self {
            ViolationType::DoubleSigning => ViolationSeverity::Critical,
            ViolationType::InvalidBlock => ViolationSeverity::High,
            ViolationType::ExtendedDowntime => ViolationSeverity::Medium,
            ViolationType::DataWithholding => ViolationSeverity::High,
            ViolationType::NetworkManipulation => ViolationSeverity::Critical,
        }
    }

    /// Get the reputation penalty for this violation
    pub fn reputation_penalty(&self) -> i32 {
        match self {
            ViolationType::DoubleSigning => -1000, // Maximum penalty
            ViolationType::InvalidBlock => -500,
            ViolationType::ExtendedDowntime => -200,
            ViolationType::DataWithholding => -400,
            ViolationType::NetworkManipulation => -1000, // Maximum penalty
        }
    }

    /// Get the collateral slash percentage (0.0 to 1.0)
    pub fn slash_percentage(&self) -> f64 {
        match self {
            ViolationType::DoubleSigning => 1.0, // 100% slash
            ViolationType::InvalidBlock => 0.10, // 10% slash
            ViolationType::ExtendedDowntime => 0.05, // 5% slash
            ViolationType::DataWithholding => 0.20, // 20% slash
            ViolationType::NetworkManipulation => 1.0, // 100% slash
        }
    }

    /// Whether this violation should result in immediate ban
    pub fn auto_ban(&self) -> bool {
        matches!(
            self,
            ViolationType::DoubleSigning | ViolationType::NetworkManipulation
        )
    }
}

impl std::fmt::Display for ViolationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViolationType::DoubleSigning => write!(f, "Double Signing"),
            ViolationType::InvalidBlock => write!(f, "Invalid Block"),
            ViolationType::ExtendedDowntime => write!(f, "Extended Downtime"),
            ViolationType::DataWithholding => write!(f, "Data Withholding"),
            ViolationType::NetworkManipulation => write!(f, "Network Manipulation"),
        }
    }
}

/// Severity levels for violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for ViolationSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViolationSeverity::Low => write!(f, "Low"),
            ViolationSeverity::Medium => write!(f, "Medium"),
            ViolationSeverity::High => write!(f, "High"),
            ViolationSeverity::Critical => write!(f, "Critical"),
        }
    }
}

/// A recorded violation with evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    /// Unique identifier for this violation
    pub id: String,
    /// Masternode that committed the violation
    pub masternode_id: String,
    /// Type of violation
    pub violation_type: ViolationType,
    /// When the violation was detected
    pub detected_at: u64,
    /// Evidence/details about the violation
    pub evidence: String,
    /// Whether penalty has been applied
    pub penalty_applied: bool,
    /// Reputation penalty applied
    pub reputation_penalty: i32,
    /// Amount of collateral slashed (in satoshis)
    pub collateral_slashed: u64,
}

impl Violation {
    /// Create a new violation record
    pub fn new(
        masternode_id: String,
        violation_type: ViolationType,
        timestamp: u64,
        evidence: String,
    ) -> Self {
        let id = format!("{}-{}-{}", masternode_id, violation_type as u8, timestamp);
        Self {
            id,
            masternode_id,
            violation_type,
            detected_at: timestamp,
            evidence,
            penalty_applied: false,
            reputation_penalty: 0,
            collateral_slashed: 0,
        }
    }

    /// Apply penalty to this violation record
    pub fn apply_penalty(&mut self, collateral_amount: u64) -> Result<()> {
        if self.penalty_applied {
            return Err(MasternodeError::InvalidOperation(
                "Penalty already applied".to_string(),
            ));
        }

        self.reputation_penalty = self.violation_type.reputation_penalty();
        self.collateral_slashed =
            (collateral_amount as f64 * self.violation_type.slash_percentage()) as u64;
        self.penalty_applied = true;

        Ok(())
    }

    /// Get severity of this violation
    pub fn severity(&self) -> ViolationSeverity {
        self.violation_type.severity()
    }

    /// Check if this violation should result in auto-ban
    pub fn should_auto_ban(&self) -> bool {
        self.violation_type.auto_ban()
    }
}

/// Evidence for double-signing violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoubleSignEvidence {
    pub block_height: u64,
    pub first_block_hash: String,
    pub first_signature: String,
    pub second_block_hash: String,
    pub second_signature: String,
    pub timestamp: u64,
}

impl DoubleSignEvidence {
    pub fn to_string_evidence(&self) -> String {
        format!(
            "Double signing at height {}: blocks {} and {}",
            self.block_height, self.first_block_hash, self.second_block_hash
        )
    }
}

/// Evidence for invalid block violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidBlockEvidence {
    pub block_height: u64,
    pub block_hash: String,
    pub reason: String,
    pub timestamp: u64,
}

impl InvalidBlockEvidence {
    pub fn to_string_evidence(&self) -> String {
        format!(
            "Invalid block {} at height {}: {}",
            self.block_hash, self.block_height, self.reason
        )
    }
}

/// Evidence for extended downtime violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DowntimeEvidence {
    pub last_seen: u64,
    pub detected_at: u64,
    pub days_offline: u64,
}

impl DowntimeEvidence {
    pub fn to_string_evidence(&self) -> String {
        format!(
            "Extended downtime: {} days offline (last seen: {}, detected: {})",
            self.days_offline, self.last_seen, self.detected_at
        )
    }
}

/// Evidence for data withholding violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataWithholdingEvidence {
    pub request_type: String,
    pub failed_responses: u32,
    pub timestamp: u64,
}

impl DataWithholdingEvidence {
    pub fn to_string_evidence(&self) -> String {
        format!(
            "Data withholding: {} failed responses to {} requests",
            self.failed_responses, self.request_type
        )
    }
}

/// Evidence for network manipulation violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkManipulationEvidence {
    pub manipulation_type: String,
    pub details: String,
    pub timestamp: u64,
}

impl NetworkManipulationEvidence {
    pub fn to_string_evidence(&self) -> String {
        format!(
            "Network manipulation ({}): {}",
            self.manipulation_type, self.details
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_violation_type_severity() {
        assert_eq!(
            ViolationType::DoubleSigning.severity(),
            ViolationSeverity::Critical
        );
        assert_eq!(
            ViolationType::InvalidBlock.severity(),
            ViolationSeverity::High
        );
        assert_eq!(
            ViolationType::ExtendedDowntime.severity(),
            ViolationSeverity::Medium
        );
    }

    #[test]
    fn test_violation_type_penalties() {
        // Double signing should have maximum penalty
        assert_eq!(ViolationType::DoubleSigning.reputation_penalty(), -1000);
        assert_eq!(ViolationType::DoubleSigning.slash_percentage(), 1.0);
        assert!(ViolationType::DoubleSigning.auto_ban());

        // Extended downtime should have lower penalty
        assert_eq!(ViolationType::ExtendedDowntime.reputation_penalty(), -200);
        assert_eq!(ViolationType::ExtendedDowntime.slash_percentage(), 0.05);
        assert!(!ViolationType::ExtendedDowntime.auto_ban());
    }

    #[test]
    fn test_violation_creation() {
        let violation = Violation::new(
            "mn1".to_string(),
            ViolationType::InvalidBlock,
            1000,
            "Test evidence".to_string(),
        );

        assert_eq!(violation.masternode_id, "mn1");
        assert_eq!(violation.violation_type, ViolationType::InvalidBlock);
        assert_eq!(violation.detected_at, 1000);
        assert!(!violation.penalty_applied);
    }

    #[test]
    fn test_apply_penalty() {
        let mut violation = Violation::new(
            "mn1".to_string(),
            ViolationType::InvalidBlock,
            1000,
            "Test evidence".to_string(),
        );

        let collateral = 10_000_000_000; // 100 TIME coins
        violation.apply_penalty(collateral).unwrap();

        assert!(violation.penalty_applied);
        assert_eq!(violation.reputation_penalty, -500);
        assert_eq!(
            violation.collateral_slashed,
            (collateral as f64 * 0.10) as u64
        );

        // Should fail on second attempt
        assert!(violation.apply_penalty(collateral).is_err());
    }

    #[test]
    fn test_double_sign_evidence() {
        let evidence = DoubleSignEvidence {
            block_height: 100,
            first_block_hash: "hash1".to_string(),
            first_signature: "sig1".to_string(),
            second_block_hash: "hash2".to_string(),
            second_signature: "sig2".to_string(),
            timestamp: 1000,
        };

        let evidence_str = evidence.to_string_evidence();
        assert!(evidence_str.contains("height 100"));
        assert!(evidence_str.contains("hash1"));
        assert!(evidence_str.contains("hash2"));
    }
}
