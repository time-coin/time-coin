//! Slashing mechanism for punishing misbehaving masternodes

use serde::{Deserialize, Serialize};

/// Types of violations that can result in slashing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Violation {
    /// Masternode signed two conflicting blocks at the same height
    DoubleSigning { block_height: u64, evidence: String },

    /// Masternode offline for extended period
    LongTermAbandonment { days_offline: u64 },

    /// Masternode withheld required data
    DataWithholding { evidence: String },

    /// Masternode participated in network attack
    NetworkAttack {
        attack_type: String,
        evidence: String,
    },

    /// Masternode attempted to manipulate consensus
    ConsensusManipulation {
        manipulation_type: String,
        evidence: String,
    },

    /// Invalid block validation (e.g., invalid transactions)
    InvalidBlock { block_height: u64, reason: String },
}

impl Violation {
    /// Get human-readable description of the violation
    pub fn description(&self) -> String {
        match self {
            Violation::DoubleSigning { block_height, .. } => {
                format!("Double-signing at block height {}", block_height)
            }
            Violation::LongTermAbandonment { days_offline } => {
                format!("Offline for {} days", days_offline)
            }
            Violation::DataWithholding { .. } => "Data withholding".to_string(),
            Violation::NetworkAttack { attack_type, .. } => {
                format!("Network attack: {}", attack_type)
            }
            Violation::ConsensusManipulation {
                manipulation_type, ..
            } => {
                format!("Consensus manipulation: {}", manipulation_type)
            }
            Violation::InvalidBlock {
                block_height,
                reason,
            } => {
                format!("Invalid block at height {}: {}", block_height, reason)
            }
        }
    }

    /// Get evidence for the violation
    pub fn evidence(&self) -> Option<String> {
        match self {
            Violation::DoubleSigning { evidence, .. }
            | Violation::DataWithholding { evidence }
            | Violation::NetworkAttack { evidence, .. }
            | Violation::ConsensusManipulation { evidence, .. } => Some(evidence.clone()),
            _ => None,
        }
    }
}

/// Record of a slashing event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingRecord {
    /// Unique identifier for this slashing event
    pub id: String,

    /// ID of the slashed masternode
    pub masternode_id: String,

    /// Type of violation
    pub violation: Violation,

    /// Amount slashed (in smallest units)
    pub amount: u64,

    /// Remaining collateral after slashing
    pub remaining_collateral: u64,

    /// Timestamp of the slashing event
    pub timestamp: u64,

    /// Block height at which slashing occurred
    pub block_height: u64,
}

impl SlashingRecord {
    pub fn new(
        id: String,
        masternode_id: String,
        violation: Violation,
        amount: u64,
        remaining_collateral: u64,
        timestamp: u64,
        block_height: u64,
    ) -> Self {
        Self {
            id,
            masternode_id,
            violation,
            amount,
            remaining_collateral,
            timestamp,
            block_height,
        }
    }
}

/// Calculate the amount to slash based on violation type and collateral
pub fn calculate_slash_amount(violation: &Violation, collateral: u64) -> u64 {
    let percentage = match violation {
        Violation::DoubleSigning { .. } => 0.5, // 50%
        Violation::LongTermAbandonment { days_offline } => {
            if *days_offline > 90 {
                0.2 // 20%
            } else if *days_offline > 60 {
                0.15 // 15%
            } else {
                0.1 // 10%
            }
        }
        Violation::DataWithholding { .. } => 0.25, // 25%
        Violation::NetworkAttack { .. } => 1.0,    // 100%
        Violation::ConsensusManipulation { .. } => 0.7, // 70%
        Violation::InvalidBlock { .. } => 0.05,    // 5%
    };

    (collateral as f64 * percentage) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_slash_amount_double_signing() {
        let violation = Violation::DoubleSigning {
            block_height: 1000,
            evidence: "proof".to_string(),
        };
        let collateral = 10_000_000_000; // 100 TIME in satoshis
        let slash_amount = calculate_slash_amount(&violation, collateral);

        // Should slash 50%
        assert_eq!(slash_amount, 5_000_000_000);
    }

    #[test]
    fn test_calculate_slash_amount_invalid_block() {
        let violation = Violation::InvalidBlock {
            block_height: 1000,
            reason: "invalid transaction".to_string(),
        };
        let collateral = 10_000_000_000; // 100 TIME in satoshis
        let slash_amount = calculate_slash_amount(&violation, collateral);

        // Should slash 5%
        assert_eq!(slash_amount, 500_000_000);
    }

    #[test]
    fn test_calculate_slash_amount_abandonment() {
        let collateral = 10_000_000_000; // 100 TIME

        // 50 days offline - 10%
        let violation = Violation::LongTermAbandonment { days_offline: 50 };
        assert_eq!(
            calculate_slash_amount(&violation, collateral),
            1_000_000_000
        );

        // 70 days offline - 15%
        let violation = Violation::LongTermAbandonment { days_offline: 70 };
        assert_eq!(
            calculate_slash_amount(&violation, collateral),
            1_500_000_000
        );

        // 100 days offline - 20%
        let violation = Violation::LongTermAbandonment { days_offline: 100 };
        assert_eq!(
            calculate_slash_amount(&violation, collateral),
            2_000_000_000
        );
    }

    #[test]
    fn test_calculate_slash_amount_network_attack() {
        let violation = Violation::NetworkAttack {
            attack_type: "DDoS".to_string(),
            evidence: "proof".to_string(),
        };
        let collateral = 10_000_000_000; // 100 TIME
        let slash_amount = calculate_slash_amount(&violation, collateral);

        // Should slash 100%
        assert_eq!(slash_amount, collateral);
    }

    #[test]
    fn test_violation_description() {
        let violation = Violation::DoubleSigning {
            block_height: 1000,
            evidence: "proof".to_string(),
        };
        assert_eq!(
            violation.description(),
            "Double-signing at block height 1000"
        );

        let violation = Violation::InvalidBlock {
            block_height: 500,
            reason: "invalid tx".to_string(),
        };
        assert_eq!(
            violation.description(),
            "Invalid block at height 500: invalid tx"
        );
    }

    #[test]
    fn test_violation_evidence() {
        let violation = Violation::DoubleSigning {
            block_height: 1000,
            evidence: "proof123".to_string(),
        };
        assert_eq!(violation.evidence(), Some("proof123".to_string()));

        let violation = Violation::LongTermAbandonment { days_offline: 50 };
        assert_eq!(violation.evidence(), None);
    }
}
