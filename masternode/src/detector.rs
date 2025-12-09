//! Automated violation detection system
//!
//! This module provides automated detection of various violation types in the masternode network.
//! It tracks block signatures, heartbeats, and data requests to identify misbehavior.
#![allow(missing_docs)]

use crate::violations::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Configuration for violation detection thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorConfig {
    /// Maximum days offline before extended downtime violation
    pub max_downtime_days: u64,
    /// Number of consecutive data request failures to trigger violation
    pub max_consecutive_failures: u32,
    /// Number of coordinated nodes required to detect manipulation
    pub min_coordinated_nodes: u32,
    /// Window size for tracking data requests (in requests)
    pub data_request_window: usize,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            max_downtime_days: 90,
            max_consecutive_failures: 5,
            min_coordinated_nodes: 3,
            data_request_window: 100,
        }
    }
}

/// Block signature record for double-signing detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSignature {
    pub masternode_id: String,
    pub block_height: u64,
    pub block_hash: String,
    pub signature: String,
    pub timestamp: u64,
}

/// Data request tracking for withholding detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRequest {
    pub masternode_id: String,
    pub data_type: String,
    pub timestamp: u64,
    pub success: bool,
}

/// Vote record for manipulation detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRecord {
    pub masternode_id: String,
    pub proposal_id: String,
    pub vote: bool,
    pub timestamp: u64,
    pub block_height: u64,
}

/// Main violation detector
#[derive(Debug)]
pub struct ViolationDetector {
    config: DetectorConfig,
    /// Track block signatures by height
    block_signatures: HashMap<u64, Vec<BlockSignature>>,
    /// Track heartbeats by masternode
    heartbeats: HashMap<String, VecDeque<u64>>,
    /// Track data requests by masternode
    data_requests: HashMap<String, VecDeque<DataRequest>>,
    /// Track votes by proposal
    votes_by_proposal: HashMap<String, Vec<VoteRecord>>,
    /// Track detected violations
    detected_violations: Vec<Violation>,
}

/// Parameters for recording invalid block proposals
#[derive(Debug, Clone)]
pub struct InvalidBlockParams {
    pub masternode_id: String,
    pub block_height: u64,
    pub block_hash: String,
    pub reason: String,
    pub expected_merkle_root: Option<String>,
    pub actual_merkle_root: Option<String>,
    pub timestamp: u64,
}

impl ViolationDetector {
    /// Create a new violation detector with default config
    pub fn new() -> Self {
        Self::with_config(DetectorConfig::default())
    }

    /// Create a new violation detector with custom config
    pub fn with_config(config: DetectorConfig) -> Self {
        Self {
            config,
            block_signatures: HashMap::new(),
            heartbeats: HashMap::new(),
            data_requests: HashMap::new(),
            votes_by_proposal: HashMap::new(),
            detected_violations: Vec::new(),
        }
    }

    /// Record a block signature and detect double-signing
    pub fn record_block_signature(
        &mut self,
        masternode_id: String,
        block_height: u64,
        block_hash: String,
        signature: String,
        timestamp: u64,
    ) -> Option<Violation> {
        let sig = BlockSignature {
            masternode_id: masternode_id.clone(),
            block_height,
            block_hash: block_hash.clone(),
            signature: signature.clone(),
            timestamp,
        };

        // Get existing signatures at this height
        let signatures = self.block_signatures.entry(block_height).or_default();

        // Check if this masternode already signed a different block at this height
        for existing_sig in signatures.iter() {
            if existing_sig.masternode_id == masternode_id && existing_sig.block_hash != block_hash
            {
                // Double-signing detected!
                let ds = DoubleSigning {
                    block_height,
                    block_hash_1: existing_sig.block_hash.clone(),
                    block_hash_2: block_hash.clone(),
                    signature_1: existing_sig.signature.clone(),
                    signature_2: signature.clone(),
                };

                let evidence_data = serde_json::to_string(&ds).unwrap_or_default();
                let evidence =
                    Evidence::new("double_signing".to_string(), evidence_data, timestamp);

                let violation = Violation::new(
                    masternode_id.clone(),
                    ViolationType::DoubleSigning(ds),
                    evidence,
                    timestamp,
                    block_height,
                );

                self.detected_violations.push(violation.clone());
                return Some(violation);
            }
        }

        // No double-signing detected, record this signature
        signatures.push(sig);
        None
    }

    /// Record heartbeat and detect extended downtime
    pub fn record_heartbeat(&mut self, masternode_id: String, timestamp: u64) {
        let heartbeats = self.heartbeats.entry(masternode_id).or_default();
        heartbeats.push_back(timestamp);

        // Keep only recent heartbeats (last 1000)
        while heartbeats.len() > 1000 {
            heartbeats.pop_front();
        }
    }

    /// Check for extended downtime violations
    pub fn check_downtime(
        &mut self,
        masternode_id: &str,
        current_timestamp: u64,
        current_block_height: u64,
    ) -> Option<Violation> {
        if let Some(heartbeats) = self.heartbeats.get(masternode_id) {
            if let Some(&last_seen) = heartbeats.back() {
                let seconds_offline = current_timestamp.saturating_sub(last_seen);
                let days_offline = seconds_offline / 86400; // seconds per day

                if days_offline > self.config.max_downtime_days {
                    let downtime = ExtendedDowntime {
                        days_offline,
                        last_seen,
                        detected_at: current_timestamp,
                    };

                    let evidence_data = serde_json::to_string(&downtime).unwrap_or_default();
                    let evidence = Evidence::new(
                        "extended_downtime".to_string(),
                        evidence_data,
                        current_timestamp,
                    );

                    let violation = Violation::new(
                        masternode_id.to_string(),
                        ViolationType::ExtendedDowntime(downtime),
                        evidence,
                        current_timestamp,
                        current_block_height,
                    );

                    self.detected_violations.push(violation.clone());
                    return Some(violation);
                }
            }
        }
        None
    }

    /// Record a data request and detect withholding
    pub fn record_data_request(
        &mut self,
        masternode_id: String,
        data_type: String,
        timestamp: u64,
        success: bool,
        current_block_height: u64,
    ) -> Option<Violation> {
        let request = DataRequest {
            masternode_id: masternode_id.clone(),
            data_type: data_type.clone(),
            timestamp,
            success,
        };

        let requests = self.data_requests.entry(masternode_id.clone()).or_default();
        requests.push_back(request);

        // Maintain window size
        while requests.len() > self.config.data_request_window {
            requests.pop_front();
        }

        // Check for consecutive failures
        let mut consecutive_failures = 0u32;
        let mut failed_requests = Vec::new();

        for req in requests.iter().rev() {
            if req.data_type == data_type {
                if !req.success {
                    consecutive_failures += 1;
                    failed_requests.push(req.timestamp);
                } else {
                    break; // Reset on success
                }
            }
        }

        if consecutive_failures >= self.config.max_consecutive_failures {
            let dw = DataWithholding {
                consecutive_failures,
                data_type: data_type.clone(),
                failed_requests: failed_requests.into_iter().rev().collect(),
            };

            let evidence_data = serde_json::to_string(&dw).unwrap_or_default();
            let evidence = Evidence::new("data_withholding".to_string(), evidence_data, timestamp);

            let violation = Violation::new(
                masternode_id.clone(),
                ViolationType::DataWithholding(dw),
                evidence,
                timestamp,
                current_block_height,
            );

            self.detected_violations.push(violation.clone());
            return Some(violation);
        }

        None
    }

    /// Record invalid block proposal
    pub fn record_invalid_block(&mut self, params: InvalidBlockParams) -> Violation {
        let invalid = InvalidBlock {
            block_height: params.block_height,
            block_hash: params.block_hash,
            reason: params.reason,
            expected_merkle_root: params.expected_merkle_root,
            actual_merkle_root: params.actual_merkle_root,
        };

        let evidence_data = serde_json::to_string(&invalid).unwrap_or_default();
        let evidence = Evidence::new("invalid_block".to_string(), evidence_data, params.timestamp);

        let violation = Violation::new(
            params.masternode_id,
            ViolationType::InvalidBlock(invalid),
            evidence,
            params.timestamp,
            params.block_height,
        );

        self.detected_violations.push(violation.clone());
        violation
    }

    /// Record a vote
    pub fn record_vote(
        &mut self,
        masternode_id: String,
        proposal_id: String,
        vote: bool,
        timestamp: u64,
        block_height: u64,
    ) {
        let vote_record = VoteRecord {
            masternode_id,
            proposal_id: proposal_id.clone(),
            vote,
            timestamp,
            block_height,
        };

        self.votes_by_proposal
            .entry(proposal_id)
            .or_default()
            .push(vote_record);
    }

    /// Detect coordinated vote manipulation
    /// This checks for suspicious voting patterns indicating coordinated attacks
    pub fn detect_vote_manipulation(
        &mut self,
        proposal_id: &str,
        current_timestamp: u64,
        current_block_height: u64,
    ) -> Option<Violation> {
        if let Some(votes) = self.votes_by_proposal.get(proposal_id) {
            // Check for coordinated voting within a short time window (e.g., 60 seconds)
            let time_window = 60u64;
            let mut time_clusters: HashMap<u64, Vec<&VoteRecord>> = HashMap::new();

            for vote in votes {
                let time_bucket = vote.timestamp / time_window;
                time_clusters.entry(time_bucket).or_default().push(vote);
            }

            // Look for clusters with many votes in the same direction
            for (_, cluster) in time_clusters.iter() {
                if cluster.len() >= self.config.min_coordinated_nodes as usize {
                    // Check if all votes are in the same direction
                    let all_same = cluster.iter().all(|v| v.vote == cluster[0].vote);

                    if all_same {
                        // Potential coordinated attack detected
                        let coordinated_nodes: Vec<String> =
                            cluster.iter().map(|v| v.masternode_id.clone()).collect();

                        // For now, we'll report the first node in the cluster
                        // In production, this would trigger investigation of all nodes
                        let masternode_id = coordinated_nodes[0].clone();

                        let nm = NetworkManipulation {
                            manipulation_type: "coordinated_voting".to_string(),
                            coordinated_nodes: coordinated_nodes.len() as u32,
                            description: format!(
                                "Detected {} nodes voting identically within {} second window on proposal {}",
                                coordinated_nodes.len(),
                                time_window,
                                proposal_id
                            ),
                        };

                        let evidence_data = serde_json::to_string(&nm).unwrap_or_default();
                        let evidence = Evidence::new(
                            "network_manipulation".to_string(),
                            evidence_data,
                            current_timestamp,
                        );

                        let violation = Violation::new(
                            masternode_id,
                            ViolationType::NetworkManipulation(nm),
                            evidence,
                            current_timestamp,
                            current_block_height,
                        );

                        self.detected_violations.push(violation.clone());
                        return Some(violation);
                    }
                }
            }
        }
        None
    }

    /// Get all detected violations
    pub fn get_violations(&self) -> &[Violation] {
        &self.detected_violations
    }

    /// Get violations for a specific masternode
    pub fn get_violations_for_masternode(&self, masternode_id: &str) -> Vec<&Violation> {
        self.detected_violations
            .iter()
            .filter(|v| v.masternode_id == masternode_id)
            .collect()
    }

    /// Clear old block signatures to save memory (keep only recent blocks)
    pub fn cleanup_old_signatures(&mut self, current_block_height: u64, keep_blocks: u64) {
        let cutoff = current_block_height.saturating_sub(keep_blocks);
        self.block_signatures.retain(|&height, _| height >= cutoff);
    }

    /// Get configuration
    pub fn config(&self) -> &DetectorConfig {
        &self.config
    }
}

impl Default for ViolationDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_double_signing_detection() {
        let mut detector = ViolationDetector::new();

        // Record first signature
        let result1 = detector.record_block_signature(
            "mn1".to_string(),
            1000,
            "hash1".to_string(),
            "sig1".to_string(),
            1000,
        );
        assert!(result1.is_none());

        // Record second signature at same height with different hash
        let result2 = detector.record_block_signature(
            "mn1".to_string(),
            1000,
            "hash2".to_string(),
            "sig2".to_string(),
            1001,
        );
        assert!(result2.is_some());

        let violation = result2.unwrap();
        assert_eq!(violation.masternode_id, "mn1");
        assert_eq!(violation.severity(), ViolationSeverity::Critical);
        assert!(violation.auto_ban());
    }

    #[test]
    fn test_no_double_signing_same_hash() {
        let mut detector = ViolationDetector::new();

        // Record same signature twice (idempotent, not double-signing)
        let result1 = detector.record_block_signature(
            "mn1".to_string(),
            1000,
            "hash1".to_string(),
            "sig1".to_string(),
            1000,
        );
        assert!(result1.is_none());

        let result2 = detector.record_block_signature(
            "mn1".to_string(),
            1000,
            "hash1".to_string(),
            "sig1".to_string(),
            1001,
        );
        assert!(result2.is_none());
    }

    #[test]
    fn test_extended_downtime_detection() {
        let mut detector = ViolationDetector::new();

        // Record heartbeat at timestamp 1000
        detector.record_heartbeat("mn1".to_string(), 1000);

        // Check after 100 days (90 days is threshold)
        let current_time = 1000 + (100 * 86400);
        let violation = detector.check_downtime("mn1", current_time, 5000);

        assert!(violation.is_some());
        let v = violation.unwrap();
        assert_eq!(v.masternode_id, "mn1");
        assert_eq!(v.severity(), ViolationSeverity::Minor);
    }

    #[test]
    fn test_no_downtime_within_threshold() {
        let mut detector = ViolationDetector::new();

        // Record heartbeat
        detector.record_heartbeat("mn1".to_string(), 1000);

        // Check after 80 days (below 90 day threshold)
        let current_time = 1000 + (80 * 86400);
        let violation = detector.check_downtime("mn1", current_time, 5000);

        assert!(violation.is_none());
    }

    #[test]
    fn test_data_withholding_detection() {
        let mut detector = ViolationDetector::new();

        // Record 5 consecutive failures
        for i in 0..5 {
            let result = detector.record_data_request(
                "mn1".to_string(),
                "block_data".to_string(),
                1000 + i,
                false, // failure
                1000,
            );

            if i < 4 {
                assert!(result.is_none());
            } else {
                assert!(result.is_some());
                let v = result.unwrap();
                assert_eq!(v.masternode_id, "mn1");
                assert_eq!(v.severity(), ViolationSeverity::Moderate);
            }
        }
    }

    #[test]
    fn test_data_withholding_reset_on_success() {
        let mut detector = ViolationDetector::new();

        // Record 4 failures
        for i in 0..4 {
            let result = detector.record_data_request(
                "mn1".to_string(),
                "block_data".to_string(),
                1000 + i,
                false,
                1000,
            );
            assert!(result.is_none());
        }

        // Record success - should reset counter
        let result = detector.record_data_request(
            "mn1".to_string(),
            "block_data".to_string(),
            1004,
            true,
            1000,
        );
        assert!(result.is_none());

        // Record 4 more failures - should not trigger yet
        for i in 5..9 {
            let result = detector.record_data_request(
                "mn1".to_string(),
                "block_data".to_string(),
                1000 + i,
                false,
                1000,
            );
            assert!(result.is_none());
        }
    }

    #[test]
    fn test_invalid_block_recording() {
        let mut detector = ViolationDetector::new();

        let violation = detector.record_invalid_block(InvalidBlockParams {
            masternode_id: "mn1".to_string(),
            block_height: 1000,
            block_hash: "invalid_hash".to_string(),
            reason: "Invalid merkle root".to_string(),
            expected_merkle_root: Some("expected".to_string()),
            actual_merkle_root: Some("actual".to_string()),
            timestamp: 5000,
        });

        assert_eq!(violation.masternode_id, "mn1");
        assert_eq!(violation.severity(), ViolationSeverity::Moderate);
        assert_eq!(violation.slash_percentage(), 0.20); // 20% for merkle root issue
    }

    #[test]
    fn test_vote_manipulation_detection() {
        let mut detector = ViolationDetector::with_config(DetectorConfig {
            min_coordinated_nodes: 3,
            ..Default::default()
        });

        // Record 3 coordinated votes within 60 seconds
        for i in 0..3 {
            detector.record_vote(
                format!("mn{}", i),
                "proposal1".to_string(),
                true,     // all voting the same way
                1000 + i, // within same time window
                100,
            );
        }

        let violation = detector.detect_vote_manipulation("proposal1", 1100, 150);
        assert!(violation.is_some());

        let v = violation.unwrap();
        assert_eq!(v.severity(), ViolationSeverity::Critical);
        assert!(v.auto_ban());
    }

    #[test]
    fn test_no_vote_manipulation_different_times() {
        let mut detector = ViolationDetector::with_config(DetectorConfig {
            min_coordinated_nodes: 3,
            ..Default::default()
        });

        // Record votes spread out over time (different time buckets)
        for i in 0..3 {
            detector.record_vote(
                format!("mn{}", i),
                "proposal1".to_string(),
                true,
                1000 + (i * 100), // spread out
                100,
            );
        }

        let violation = detector.detect_vote_manipulation("proposal1", 1500, 150);
        assert!(violation.is_none());
    }

    #[test]
    fn test_get_violations_for_masternode() {
        let mut detector = ViolationDetector::new();

        // Create violations for different masternodes
        detector.record_invalid_block(InvalidBlockParams {
            masternode_id: "mn1".to_string(),
            block_height: 1000,
            block_hash: "hash1".to_string(),
            reason: "reason1".to_string(),
            expected_merkle_root: None,
            actual_merkle_root: None,
            timestamp: 5000,
        });

        detector.record_invalid_block(InvalidBlockParams {
            masternode_id: "mn2".to_string(),
            block_height: 1001,
            block_hash: "hash2".to_string(),
            reason: "reason2".to_string(),
            expected_merkle_root: None,
            actual_merkle_root: None,
            timestamp: 5001,
        });

        detector.record_invalid_block(InvalidBlockParams {
            masternode_id: "mn1".to_string(),
            block_height: 1002,
            block_hash: "hash3".to_string(),
            reason: "reason3".to_string(),
            expected_merkle_root: None,
            actual_merkle_root: None,
            timestamp: 5002,
        });

        let mn1_violations = detector.get_violations_for_masternode("mn1");
        assert_eq!(mn1_violations.len(), 2);

        let mn2_violations = detector.get_violations_for_masternode("mn2");
        assert_eq!(mn2_violations.len(), 1);
    }

    #[test]
    fn test_cleanup_old_signatures() {
        let mut detector = ViolationDetector::new();

        // Record signatures at various heights
        for height in 1000..1100 {
            detector.record_block_signature(
                "mn1".to_string(),
                height,
                format!("hash{}", height),
                format!("sig{}", height),
                height,
            );
        }

        assert_eq!(detector.block_signatures.len(), 100);

        // Clean up old signatures (keep only last 10 blocks from height 1099)
        // cutoff = 1099 - 10 = 1089, so keeps 1089-1099 (11 blocks)
        detector.cleanup_old_signatures(1099, 10);

        // Should have kept 11 blocks (1089-1099 inclusive)
        assert_eq!(detector.block_signatures.len(), 11);

        // Verify we kept the right range
        for height in 1089..1100 {
            assert!(detector.block_signatures.contains_key(&height));
        }

        // Verify old blocks were removed
        assert!(!detector.block_signatures.contains_key(&1088));
        assert!(!detector.block_signatures.contains_key(&1000));
    }
}
