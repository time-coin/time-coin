//! Violation types and their associated penalties
//!
//! This module defines all types of violations that can be detected in the masternode network,
//! along with their severity, reputation penalties, and slashing percentages.
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Severity level of a violation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationSeverity {
    /// Minor violation - warning level
    Minor,
    /// Moderate violation - requires attention
    Moderate,
    /// Severe violation - significant penalty
    Severe,
    /// Critical violation - immediate ban
    Critical,
}

/// Evidence type for violations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Type of evidence
    pub evidence_type: String,
    /// Raw evidence data (serialized)
    pub data: String,
    /// Cryptographic proof (hash of evidence)
    pub proof: String,
    /// Timestamp when evidence was collected
    pub timestamp: u64,
}

impl Evidence {
    /// Create new evidence with cryptographic proof
    pub fn new(evidence_type: String, data: String, timestamp: u64) -> Self {
        let proof = Self::generate_proof(&data);
        Self {
            evidence_type,
            data,
            proof,
            timestamp,
        }
    }

    /// Generate cryptographic proof (SHA-256 hash) of evidence
    fn generate_proof(data: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Verify evidence integrity
    pub fn verify(&self) -> bool {
        let computed_proof = Self::generate_proof(&self.data);
        computed_proof == self.proof
    }
}

/// Double-signing violation - signing two conflicting blocks at the same height
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoubleSigning {
    /// Block height where double-signing occurred
    pub block_height: u64,
    /// Hash of first block
    pub block_hash_1: String,
    /// Hash of second block
    pub block_hash_2: String,
    /// Signature on first block
    pub signature_1: String,
    /// Signature on second block
    pub signature_2: String,
}

/// Invalid block creation violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidBlock {
    /// Block height
    pub block_height: u64,
    /// Hash of invalid block
    pub block_hash: String,
    /// Reason for invalidity
    pub reason: String,
    /// Expected merkle root
    pub expected_merkle_root: Option<String>,
    /// Actual merkle root
    pub actual_merkle_root: Option<String>,
}

/// Extended downtime violation (>90 days offline)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedDowntime {
    /// Number of days offline
    pub days_offline: u64,
    /// Last seen timestamp
    pub last_seen: u64,
    /// Detection timestamp
    pub detected_at: u64,
}

/// Data withholding violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataWithholding {
    /// Number of consecutive failed requests
    pub consecutive_failures: u32,
    /// Type of data withheld
    pub data_type: String,
    /// List of request timestamps that failed
    pub failed_requests: Vec<u64>,
}

/// Network manipulation violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkManipulation {
    /// Type of manipulation
    pub manipulation_type: String,
    /// Number of coordinated nodes involved
    pub coordinated_nodes: u32,
    /// Description of the attack
    pub description: String,
}

/// Comprehensive violation type with all supported violations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationType {
    /// Double-signing at same height
    DoubleSigning(DoubleSigning),
    /// Invalid block creation
    InvalidBlock(InvalidBlock),
    /// Extended downtime (>90 days)
    ExtendedDowntime(ExtendedDowntime),
    /// Data withholding
    DataWithholding(DataWithholding),
    /// Network manipulation
    NetworkManipulation(NetworkManipulation),
}

impl ViolationType {
    /// Get severity of this violation type
    pub fn severity(&self) -> ViolationSeverity {
        match self {
            ViolationType::DoubleSigning(_) => ViolationSeverity::Critical,
            ViolationType::InvalidBlock(_) => ViolationSeverity::Moderate,
            ViolationType::ExtendedDowntime(_) => ViolationSeverity::Minor,
            ViolationType::DataWithholding(_) => ViolationSeverity::Moderate,
            ViolationType::NetworkManipulation(_) => ViolationSeverity::Critical,
        }
    }

    /// Get reputation penalty for this violation
    pub fn reputation_penalty(&self) -> i32 {
        match self {
            ViolationType::DoubleSigning(_) => -1000,
            ViolationType::InvalidBlock(_) => -200,
            ViolationType::ExtendedDowntime(_) => -200,
            ViolationType::DataWithholding(_) => -300,
            ViolationType::NetworkManipulation(_) => -1000,
        }
    }

    /// Get collateral slash percentage (0.0 to 1.0)
    pub fn slash_percentage(&self) -> f64 {
        match self {
            ViolationType::DoubleSigning(_) => 1.0, // 100%
            ViolationType::InvalidBlock(invalid) => {
                // 10-20% based on severity
                if invalid.expected_merkle_root.is_some() {
                    0.20 // 20% for merkle root mismatch
                } else {
                    0.10 // 10% for other invalid blocks
                }
            }
            ViolationType::ExtendedDowntime(downtime) => {
                // 5% for >90 days, scales slightly with duration
                if downtime.days_offline > 365 {
                    0.10 // 10% for over a year
                } else {
                    0.05 // 5% for >90 days
                }
            }
            ViolationType::DataWithholding(withholding) => {
                // 10-20% based on consecutive failures
                if withholding.consecutive_failures >= 10 {
                    0.20 // 20% for 10+ consecutive failures
                } else {
                    0.10 // 10% for 5-9 consecutive failures
                }
            }
            ViolationType::NetworkManipulation(_) => 1.0, // 100%
        }
    }

    /// Whether this violation results in automatic ban
    pub fn auto_ban(&self) -> bool {
        match self {
            ViolationType::DoubleSigning(_) => true,
            ViolationType::InvalidBlock(_) => false,
            ViolationType::ExtendedDowntime(_) => false,
            ViolationType::DataWithholding(_) => false,
            ViolationType::NetworkManipulation(_) => true,
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> String {
        match self {
            ViolationType::DoubleSigning(ds) => {
                format!(
                    "Double-signing at block height {} (blocks: {} vs {})",
                    ds.block_height, ds.block_hash_1, ds.block_hash_2
                )
            }
            ViolationType::InvalidBlock(invalid) => {
                format!(
                    "Invalid block at height {}: {}",
                    invalid.block_height, invalid.reason
                )
            }
            ViolationType::ExtendedDowntime(downtime) => {
                format!("Extended downtime: {} days offline", downtime.days_offline)
            }
            ViolationType::DataWithholding(dw) => {
                format!(
                    "Data withholding: {} consecutive failures for {}",
                    dw.consecutive_failures, dw.data_type
                )
            }
            ViolationType::NetworkManipulation(nm) => {
                format!(
                    "Network manipulation: {} (involving {} nodes)",
                    nm.manipulation_type, nm.coordinated_nodes
                )
            }
        }
    }
}

/// A detected violation with evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    /// Unique identifier for this violation
    pub id: String,
    /// Masternode that committed the violation
    pub masternode_id: String,
    /// Type of violation
    pub violation_type: ViolationType,
    /// Evidence supporting the violation
    pub evidence: Evidence,
    /// Timestamp when violation was detected
    pub detected_at: u64,
    /// Block height when detected
    pub block_height: u64,
}

impl Violation {
    /// Create a new violation
    pub fn new(
        masternode_id: String,
        violation_type: ViolationType,
        evidence: Evidence,
        detected_at: u64,
        block_height: u64,
    ) -> Self {
        let id = format!("violation-{}-{}", masternode_id, detected_at);
        Self {
            id,
            masternode_id,
            violation_type,
            evidence,
            detected_at,
            block_height,
        }
    }

    /// Get severity of this violation
    pub fn severity(&self) -> ViolationSeverity {
        self.violation_type.severity()
    }

    /// Get reputation penalty
    pub fn reputation_penalty(&self) -> i32 {
        self.violation_type.reputation_penalty()
    }

    /// Get slash percentage
    pub fn slash_percentage(&self) -> f64 {
        self.violation_type.slash_percentage()
    }

    /// Whether this should result in automatic ban
    pub fn auto_ban(&self) -> bool {
        self.violation_type.auto_ban()
    }

    /// Get description
    pub fn description(&self) -> String {
        self.violation_type.description()
    }

    /// Verify evidence integrity
    pub fn verify_evidence(&self) -> bool {
        self.evidence.verify()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_creation_and_verification() {
        let evidence = Evidence::new(
            "double_signing".to_string(),
            "block1:hash1,block2:hash2".to_string(),
            1000,
        );
        assert!(evidence.verify());
        assert!(!evidence.proof.is_empty());
    }

    #[test]
    fn test_evidence_tamper_detection() {
        let mut evidence = Evidence::new("test".to_string(), "original data".to_string(), 1000);
        // Tamper with data
        evidence.data = "tampered data".to_string();
        assert!(!evidence.verify());
    }

    #[test]
    fn test_double_signing_violation() {
        let ds = DoubleSigning {
            block_height: 1000,
            block_hash_1: "hash1".to_string(),
            block_hash_2: "hash2".to_string(),
            signature_1: "sig1".to_string(),
            signature_2: "sig2".to_string(),
        };
        let vtype = ViolationType::DoubleSigning(ds);

        assert_eq!(vtype.severity(), ViolationSeverity::Critical);
        assert_eq!(vtype.reputation_penalty(), -1000);
        assert_eq!(vtype.slash_percentage(), 1.0);
        assert!(vtype.auto_ban());
    }

    #[test]
    fn test_invalid_block_violation() {
        let invalid = InvalidBlock {
            block_height: 500,
            block_hash: "invalid_hash".to_string(),
            reason: "Invalid merkle root".to_string(),
            expected_merkle_root: Some("expected".to_string()),
            actual_merkle_root: Some("actual".to_string()),
        };
        let vtype = ViolationType::InvalidBlock(invalid);

        assert_eq!(vtype.severity(), ViolationSeverity::Moderate);
        assert_eq!(vtype.reputation_penalty(), -200);
        assert_eq!(vtype.slash_percentage(), 0.20); // 20% for merkle root mismatch
        assert!(!vtype.auto_ban());
    }

    #[test]
    fn test_extended_downtime_violation() {
        let downtime = ExtendedDowntime {
            days_offline: 100,
            last_seen: 1000,
            detected_at: 1000 + (100 * 86400),
        };
        let vtype = ViolationType::ExtendedDowntime(downtime);

        assert_eq!(vtype.severity(), ViolationSeverity::Minor);
        assert_eq!(vtype.reputation_penalty(), -200);
        assert_eq!(vtype.slash_percentage(), 0.05); // 5%
        assert!(!vtype.auto_ban());
    }

    #[test]
    fn test_data_withholding_violation() {
        let dw = DataWithholding {
            consecutive_failures: 5,
            data_type: "block_data".to_string(),
            failed_requests: vec![1000, 1001, 1002, 1003, 1004],
        };
        let vtype = ViolationType::DataWithholding(dw);

        assert_eq!(vtype.severity(), ViolationSeverity::Moderate);
        assert_eq!(vtype.reputation_penalty(), -300);
        assert_eq!(vtype.slash_percentage(), 0.10); // 10% for 5 failures
        assert!(!vtype.auto_ban());
    }

    #[test]
    fn test_network_manipulation_violation() {
        let nm = NetworkManipulation {
            manipulation_type: "coordinated_voting_attack".to_string(),
            coordinated_nodes: 5,
            description: "Coordinated vote manipulation detected".to_string(),
        };
        let vtype = ViolationType::NetworkManipulation(nm);

        assert_eq!(vtype.severity(), ViolationSeverity::Critical);
        assert_eq!(vtype.reputation_penalty(), -1000);
        assert_eq!(vtype.slash_percentage(), 1.0); // 100%
        assert!(vtype.auto_ban());
    }

    #[test]
    fn test_violation_creation() {
        let evidence = Evidence::new(
            "double_signing".to_string(),
            "evidence_data".to_string(),
            1000,
        );
        let ds = DoubleSigning {
            block_height: 1000,
            block_hash_1: "hash1".to_string(),
            block_hash_2: "hash2".to_string(),
            signature_1: "sig1".to_string(),
            signature_2: "sig2".to_string(),
        };

        let violation = Violation::new(
            "masternode1".to_string(),
            ViolationType::DoubleSigning(ds),
            evidence,
            1000,
            1000,
        );

        assert_eq!(violation.masternode_id, "masternode1");
        assert_eq!(violation.severity(), ViolationSeverity::Critical);
        assert!(violation.verify_evidence());
    }

    #[test]
    fn test_data_withholding_scaling() {
        // Test 5 consecutive failures - 10%
        let dw5 = DataWithholding {
            consecutive_failures: 5,
            data_type: "test".to_string(),
            failed_requests: vec![],
        };
        let vtype5 = ViolationType::DataWithholding(dw5);
        assert_eq!(vtype5.slash_percentage(), 0.10);

        // Test 10 consecutive failures - 20%
        let dw10 = DataWithholding {
            consecutive_failures: 10,
            data_type: "test".to_string(),
            failed_requests: vec![],
        };
        let vtype10 = ViolationType::DataWithholding(dw10);
        assert_eq!(vtype10.slash_percentage(), 0.20);
    }

    #[test]
    fn test_extended_downtime_scaling() {
        // Test 100 days - 5%
        let downtime100 = ExtendedDowntime {
            days_offline: 100,
            last_seen: 1000,
            detected_at: 2000,
        };
        let vtype100 = ViolationType::ExtendedDowntime(downtime100);
        assert_eq!(vtype100.slash_percentage(), 0.05);

        // Test 400 days - 10%
        let downtime400 = ExtendedDowntime {
            days_offline: 400,
            last_seen: 1000,
            detected_at: 2000,
        };
        let vtype400 = ViolationType::ExtendedDowntime(downtime400);
        assert_eq!(vtype400.slash_percentage(), 0.10);
    }
}
