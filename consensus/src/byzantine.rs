//! Byzantine behavior detection for TIME Coin consensus
//!
//! Identifies and tracks malicious node behavior including:
//! - Double voting (voting for conflicting blocks at same height)
//! - Invalid proposals (proposing blocks that violate consensus rules)
//! - Contradictory votes (approving conflicting transactions)

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Types of Byzantine violations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ByzantineViolation {
    /// Node voted for two different blocks at the same height
    DoubleVote {
        height: u64,
        hash1: String,
        hash2: String,
    },

    /// Node proposed an invalid block
    InvalidProposal { height: u64, reason: String },

    /// Node voted inconsistently on related items
    ContradictoryVote {
        item1: String,
        item2: String,
        reason: String,
    },

    /// Node is consistently offline or non-responsive
    Unavailable { consecutive_failures: usize },
}

/// Severity of Byzantine violation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViolationSeverity {
    Minor,    // Single failure, could be network issue
    Moderate, // Repeated failures
    Severe,   // Clear malicious intent (double-voting)
    Critical, // Coordinated attack pattern
}

impl ByzantineViolation {
    pub fn severity(&self) -> ViolationSeverity {
        match self {
            ByzantineViolation::DoubleVote { .. } => ViolationSeverity::Severe,
            ByzantineViolation::InvalidProposal { .. } => ViolationSeverity::Moderate,
            ByzantineViolation::ContradictoryVote { .. } => ViolationSeverity::Moderate,
            ByzantineViolation::Unavailable {
                consecutive_failures,
            } => {
                if *consecutive_failures > 10 {
                    ViolationSeverity::Moderate
                } else {
                    ViolationSeverity::Minor
                }
            }
        }
    }
}

/// Record of a node's Byzantine behavior
#[derive(Debug, Clone)]
pub struct ViolationRecord {
    pub node_id: String,
    pub violation: ByzantineViolation,
    pub timestamp: i64,
    pub evidence: Vec<String>,
}

/// Byzantine behavior detector
pub struct ByzantineDetector {
    /// Votes recorded per node per height: (node_id, height) -> block_hashes
    vote_history: Arc<DashMap<(String, u64), HashSet<String>>>,

    /// Violations recorded per node
    violations: Arc<RwLock<Vec<ViolationRecord>>>,

    /// Failure count per node
    failure_counts: Arc<DashMap<String, usize>>,

    /// Threshold for considering a node Byzantine
    violation_threshold: usize,
}

impl ByzantineDetector {
    pub fn new(violation_threshold: usize) -> Self {
        Self {
            vote_history: Arc::new(DashMap::new()),
            violations: Arc::new(RwLock::new(Vec::new())),
            failure_counts: Arc::new(DashMap::new()),
            violation_threshold,
        }
    }

    /// Record a vote and check for double-voting
    pub async fn record_vote(
        &self,
        node_id: &str,
        height: u64,
        block_hash: &str,
    ) -> Result<(), ByzantineViolation> {
        let key = (node_id.to_string(), height);

        let mut hashes = self.vote_history.entry(key).or_default();

        // Check for double vote
        if !hashes.is_empty() && !hashes.contains(block_hash) {
            let existing_hash = hashes.iter().next().unwrap().clone();
            let violation = ByzantineViolation::DoubleVote {
                height,
                hash1: existing_hash,
                hash2: block_hash.to_string(),
            };

            self.record_violation(ViolationRecord {
                node_id: node_id.to_string(),
                violation: violation.clone(),
                timestamp: chrono::Utc::now().timestamp(),
                evidence: vec![
                    format!("Vote 1: {}", hashes.iter().next().unwrap()),
                    format!("Vote 2: {}", block_hash),
                ],
            })
            .await;

            return Err(violation);
        }

        hashes.insert(block_hash.to_string());
        Ok(())
    }

    /// Record an invalid proposal
    pub async fn record_invalid_proposal(&self, node_id: &str, height: u64, reason: String) {
        let violation = ByzantineViolation::InvalidProposal {
            height,
            reason: reason.clone(),
        };

        self.record_violation(ViolationRecord {
            node_id: node_id.to_string(),
            violation,
            timestamp: chrono::Utc::now().timestamp(),
            evidence: vec![reason],
        })
        .await;
    }

    /// Record a node failure
    pub async fn record_failure(&self, node_id: &str) {
        let mut count = self.failure_counts.entry(node_id.to_string()).or_insert(0);
        *count += 1;

        // Report unavailability if failures exceed threshold
        if *count >= 5 {
            let violation = ByzantineViolation::Unavailable {
                consecutive_failures: *count,
            };

            self.record_violation(ViolationRecord {
                node_id: node_id.to_string(),
                violation,
                timestamp: chrono::Utc::now().timestamp(),
                evidence: vec![format!("{} consecutive failures", *count)],
            })
            .await;
        }
    }

    /// Reset failure count for a node (after successful operation)
    pub fn reset_failure_count(&self, node_id: &str) {
        self.failure_counts.remove(node_id);
    }

    /// Record a violation
    async fn record_violation(&self, record: ViolationRecord) {
        println!(
            "⚠️ Byzantine violation detected: {} - {:?}",
            record.node_id, record.violation
        );

        let mut violations = self.violations.write().await;
        violations.push(record);
    }

    /// Check if a node should be considered Byzantine
    pub async fn is_byzantine(&self, node_id: &str) -> bool {
        let violations = self.violations.read().await;
        let node_violations: Vec<_> = violations.iter().filter(|v| v.node_id == node_id).collect();

        // Count severe violations
        let severe_count = node_violations
            .iter()
            .filter(|v| v.violation.severity() >= ViolationSeverity::Severe)
            .count();

        // Any severe violation = Byzantine
        if severe_count > 0 {
            return true;
        }

        // Multiple moderate violations = Byzantine
        let moderate_count = node_violations
            .iter()
            .filter(|v| v.violation.severity() >= ViolationSeverity::Moderate)
            .count();

        moderate_count >= self.violation_threshold
    }

    /// Get violation history for a node
    pub async fn get_violations(&self, node_id: &str) -> Vec<ViolationRecord> {
        let violations = self.violations.read().await;
        violations
            .iter()
            .filter(|v| v.node_id == node_id)
            .cloned()
            .collect()
    }

    /// Get all Byzantine nodes
    pub async fn get_byzantine_nodes(&self) -> Vec<String> {
        let violations = self.violations.read().await;
        let mut byzantine_nodes = HashSet::new();

        for violation in violations.iter() {
            if violation.violation.severity() >= ViolationSeverity::Severe {
                byzantine_nodes.insert(violation.node_id.clone());
            }
        }

        byzantine_nodes.into_iter().collect()
    }

    /// Cleanup old violations (keep last N days)
    pub async fn cleanup_old(&self, keep_days: i64) {
        let cutoff = chrono::Utc::now().timestamp() - (keep_days * 86400);
        let mut violations = self.violations.write().await;
        violations.retain(|v| v.timestamp >= cutoff);
    }

    /// Clear vote history for heights older than specified
    pub fn cleanup_vote_history(&self, min_height: u64) {
        self.vote_history
            .retain(|(_, height), _| *height >= min_height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_double_vote_detection() {
        let detector = ByzantineDetector::new(3);

        // First vote should succeed
        assert!(detector.record_vote("node1", 100, "hash1").await.is_ok());

        // Same vote again should succeed (idempotent)
        assert!(detector.record_vote("node1", 100, "hash1").await.is_ok());

        // Different hash at same height should fail
        let result = detector.record_vote("node1", 100, "hash2").await;
        assert!(result.is_err());

        if let Err(ByzantineViolation::DoubleVote { height, .. }) = result {
            assert_eq!(height, 100);
        } else {
            panic!("Expected DoubleVote violation");
        }

        // Node should be marked Byzantine
        assert!(detector.is_byzantine("node1").await);
    }

    #[tokio::test]
    async fn test_failure_tracking() {
        let detector = ByzantineDetector::new(3);

        // Record multiple failures
        for _ in 0..5 {
            detector.record_failure("node1").await;
        }

        let violations = detector.get_violations("node1").await;
        assert!(!violations.is_empty());

        // Check that unavailability was recorded
        assert!(violations
            .iter()
            .any(|v| matches!(v.violation, ByzantineViolation::Unavailable { .. })));
    }

    #[tokio::test]
    async fn test_failure_reset() {
        let detector = ByzantineDetector::new(3);

        detector.record_failure("node1").await;
        detector.record_failure("node1").await;

        // Reset should clear failure count
        detector.reset_failure_count("node1");

        // New failures should start from 0
        for _ in 0..4 {
            detector.record_failure("node1").await;
        }

        // Should not be enough to trigger unavailability violation
        let violations = detector.get_violations("node1").await;
        assert!(violations.is_empty());
    }

    #[tokio::test]
    async fn test_moderate_violations_threshold() {
        let detector = ByzantineDetector::new(3);

        // Record multiple moderate violations
        for i in 0..3 {
            detector
                .record_invalid_proposal("node1", i, "Invalid merkle root".to_string())
                .await;
        }

        // Should be Byzantine after threshold
        assert!(detector.is_byzantine("node1").await);
    }
}
