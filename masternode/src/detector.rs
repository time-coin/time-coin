//! Automatic violation detection for masternodes
//!
//! This module implements automated detection of various masternode violations
//! including double-signing, invalid blocks, extended downtime, data withholding,
//! and network manipulation.

use crate::error::{MasternodeError, Result};
use crate::node::Masternode;
use crate::violations::{
    DataWithholdingEvidence, DoubleSignEvidence, DowntimeEvidence, InvalidBlockEvidence,
    NetworkManipulationEvidence, Violation, ViolationType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for violation detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorConfig {
    /// Maximum downtime in seconds before triggering extended downtime violation (default: 90 days)
    pub max_downtime_seconds: u64,
    /// Number of failed data requests before triggering data withholding violation
    pub max_failed_requests: u32,
    /// Enable double-signing detection
    pub enable_double_sign_detection: bool,
    /// Enable invalid block detection
    pub enable_invalid_block_detection: bool,
    /// Enable downtime detection
    pub enable_downtime_detection: bool,
    /// Enable data withholding detection
    pub enable_data_withholding_detection: bool,
    /// Enable network manipulation detection
    pub enable_network_manipulation_detection: bool,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            max_downtime_seconds: 90 * 24 * 60 * 60, // 90 days
            max_failed_requests: 10,
            enable_double_sign_detection: true,
            enable_invalid_block_detection: true,
            enable_downtime_detection: true,
            enable_data_withholding_detection: true,
            enable_network_manipulation_detection: true,
        }
    }
}

/// Block signature record for double-signing detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSignature {
    pub block_height: u64,
    pub block_hash: String,
    pub signature: String,
    pub masternode_id: String,
    pub timestamp: u64,
}

/// Data request tracking for data withholding detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRequestTracker {
    pub masternode_id: String,
    pub request_type: String,
    pub failed_count: u32,
    pub total_count: u32,
    pub last_failure: u64,
}

/// Automated violation detector
#[derive(Debug)]
pub struct ViolationDetector {
    config: DetectorConfig,
    /// Track block signatures by height and masternode
    block_signatures: HashMap<u64, Vec<BlockSignature>>,
    /// Track data requests by masternode
    data_requests: HashMap<String, DataRequestTracker>,
    /// Detected violations
    violations: Vec<Violation>,
}

impl ViolationDetector {
    pub fn new(config: DetectorConfig) -> Self {
        Self {
            config,
            block_signatures: HashMap::new(),
            data_requests: HashMap::new(),
            violations: Vec::new(),
        }
    }

    /// Check for double-signing violation
    ///
    /// Double-signing occurs when a masternode signs two different blocks at the same height.
    /// This is a critical Byzantine fault that undermines consensus safety.
    pub fn check_double_signing(
        &mut self,
        signature: BlockSignature,
        current_time: u64,
    ) -> Result<Option<Violation>> {
        if !self.config.enable_double_sign_detection {
            return Ok(None);
        }

        let height = signature.block_height;
        let masternode_id = &signature.masternode_id;

        // Get or create list of signatures at this height
        let signatures_at_height = self.block_signatures.entry(height).or_insert_with(Vec::new);

        // Check if this masternode has already signed a different block at this height
        for existing_sig in signatures_at_height.iter() {
            if existing_sig.masternode_id == *masternode_id
                && existing_sig.block_hash != signature.block_hash
            {
                // Double-signing detected!
                let evidence = DoubleSignEvidence {
                    block_height: height,
                    first_block_hash: existing_sig.block_hash.clone(),
                    first_signature: existing_sig.signature.clone(),
                    second_block_hash: signature.block_hash.clone(),
                    second_signature: signature.signature.clone(),
                    timestamp: current_time,
                };

                let violation = Violation::new(
                    masternode_id.clone(),
                    ViolationType::DoubleSigning,
                    current_time,
                    evidence.to_string_evidence(),
                );

                self.violations.push(violation.clone());
                return Ok(Some(violation));
            }
        }

        // Record this signature
        signatures_at_height.push(signature);

        Ok(None)
    }

    /// Check for invalid block violation
    ///
    /// An invalid block is one that fails validation rules (e.g., invalid merkle root,
    /// invalid transactions, timestamp issues, etc.)
    pub fn check_invalid_block(
        &mut self,
        masternode_id: String,
        block_height: u64,
        block_hash: String,
        reason: String,
        timestamp: u64,
    ) -> Result<Violation> {
        if !self.config.enable_invalid_block_detection {
            return Err(MasternodeError::InvalidOperation(
                "Invalid block detection is disabled".to_string(),
            ));
        }

        let evidence = InvalidBlockEvidence {
            block_height,
            block_hash,
            reason,
            timestamp,
        };

        let violation = Violation::new(
            masternode_id,
            ViolationType::InvalidBlock,
            timestamp,
            evidence.to_string_evidence(),
        );

        self.violations.push(violation.clone());
        Ok(violation)
    }

    /// Check for extended downtime violation
    ///
    /// Extended downtime is when a masternode has been offline for more than the configured
    /// threshold (default: 90 days). This indicates abandonment of the masternode.
    pub fn check_extended_downtime(
        &mut self,
        masternode: &Masternode,
        current_time: u64,
    ) -> Result<Option<Violation>> {
        if !self.config.enable_downtime_detection {
            return Ok(None);
        }

        let time_offline = current_time - masternode.last_heartbeat;

        if time_offline > self.config.max_downtime_seconds {
            let days_offline = time_offline / (24 * 60 * 60);

            let evidence = DowntimeEvidence {
                last_seen: masternode.last_heartbeat,
                detected_at: current_time,
                days_offline,
            };

            let violation = Violation::new(
                masternode.id.clone(),
                ViolationType::ExtendedDowntime,
                current_time,
                evidence.to_string_evidence(),
            );

            self.violations.push(violation.clone());
            return Ok(Some(violation));
        }

        Ok(None)
    }

    /// Record a data request result
    pub fn record_data_request(
        &mut self,
        masternode_id: String,
        request_type: String,
        success: bool,
        timestamp: u64,
    ) {
        let tracker = self
            .data_requests
            .entry(masternode_id.clone())
            .or_insert_with(|| DataRequestTracker {
                masternode_id: masternode_id.clone(),
                request_type: request_type.clone(),
                failed_count: 0,
                total_count: 0,
                last_failure: 0,
            });

        tracker.total_count += 1;
        if !success {
            tracker.failed_count += 1;
            tracker.last_failure = timestamp;
        }
    }

    /// Check for data withholding violation
    ///
    /// Data withholding occurs when a masternode repeatedly fails to respond to valid
    /// data requests. This can be a form of censorship or availability attack.
    pub fn check_data_withholding(
        &mut self,
        masternode_id: &str,
        current_time: u64,
    ) -> Result<Option<Violation>> {
        if !self.config.enable_data_withholding_detection {
            return Ok(None);
        }

        if let Some(tracker) = self.data_requests.get(masternode_id) {
            if tracker.failed_count >= self.config.max_failed_requests {
                let evidence = DataWithholdingEvidence {
                    request_type: tracker.request_type.clone(),
                    failed_responses: tracker.failed_count,
                    timestamp: current_time,
                };

                let violation = Violation::new(
                    masternode_id.to_string(),
                    ViolationType::DataWithholding,
                    current_time,
                    evidence.to_string_evidence(),
                );

                self.violations.push(violation.clone());

                // Reset counter after detection
                if let Some(tracker) = self.data_requests.get_mut(masternode_id) {
                    tracker.failed_count = 0;
                }

                return Ok(Some(violation));
            }
        }

        Ok(None)
    }

    /// Check for network manipulation violation
    ///
    /// Network manipulation includes attempts to subvert consensus through coordinated
    /// voting manipulation, Sybil attacks, or other malicious coordination.
    pub fn check_network_manipulation(
        &mut self,
        masternode_id: String,
        manipulation_type: String,
        details: String,
        timestamp: u64,
    ) -> Result<Violation> {
        if !self.config.enable_network_manipulation_detection {
            return Err(MasternodeError::InvalidOperation(
                "Network manipulation detection is disabled".to_string(),
            ));
        }

        let evidence = NetworkManipulationEvidence {
            manipulation_type,
            details,
            timestamp,
        };

        let violation = Violation::new(
            masternode_id,
            ViolationType::NetworkManipulation,
            timestamp,
            evidence.to_string_evidence(),
        );

        self.violations.push(violation.clone());
        Ok(violation)
    }

    /// Get all detected violations
    pub fn get_violations(&self) -> &[Violation] {
        &self.violations
    }

    /// Get violations for a specific masternode
    pub fn get_violations_for_masternode(&self, masternode_id: &str) -> Vec<&Violation> {
        self.violations
            .iter()
            .filter(|v| v.masternode_id == masternode_id)
            .collect()
    }

    /// Clear old block signatures to prevent memory bloat
    /// Keep only the last N block heights
    pub fn cleanup_old_signatures(&mut self, current_height: u64, keep_blocks: u64) {
        if current_height > keep_blocks {
            let cutoff_height = current_height - keep_blocks;
            self.block_signatures
                .retain(|&height, _| height > cutoff_height);
        }
    }

    /// Get detection statistics
    pub fn get_stats(&self) -> DetectorStats {
        let mut stats = DetectorStats::default();

        for violation in &self.violations {
            match violation.violation_type {
                ViolationType::DoubleSigning => stats.double_signing += 1,
                ViolationType::InvalidBlock => stats.invalid_blocks += 1,
                ViolationType::ExtendedDowntime => stats.extended_downtime += 1,
                ViolationType::DataWithholding => stats.data_withholding += 1,
                ViolationType::NetworkManipulation => stats.network_manipulation += 1,
            }
            stats.total += 1;
        }

        stats
    }

    /// Get count of tracked block signatures (for testing/monitoring)
    pub fn signature_count(&self) -> usize {
        self.block_signatures.len()
    }

    /// Check if a signature exists for a specific height (for testing)
    pub fn has_signature_at_height(&self, height: u64) -> bool {
        self.block_signatures.contains_key(&height)
    }
}

impl Default for ViolationDetector {
    fn default() -> Self {
        Self::new(DetectorConfig::default())
    }
}

/// Statistics for violation detection
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DetectorStats {
    pub total: u32,
    pub double_signing: u32,
    pub invalid_blocks: u32,
    pub extended_downtime: u32,
    pub data_withholding: u32,
    pub network_manipulation: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collateral::CollateralTier;
    use crate::reputation::Reputation;

    fn create_test_masternode(id: &str, last_heartbeat: u64) -> Masternode {
        Masternode {
            id: id.to_string(),
            public_key: "test_key".to_string(),
            tier: CollateralTier::Professional,
            status: crate::node::MasternodeStatus::Active,
            reputation: Reputation::new(id.to_string(), last_heartbeat),
            registered_at: last_heartbeat,
            last_heartbeat,
            ip_address: "127.0.0.1".to_string(),
            port: 9999,
            blocks_validated: 0,
            total_rewards: 0,
        }
    }

    #[test]
    fn test_double_signing_detection() {
        let mut detector = ViolationDetector::default();

        // First signature at height 100
        let sig1 = BlockSignature {
            block_height: 100,
            block_hash: "hash1".to_string(),
            signature: "sig1".to_string(),
            masternode_id: "mn1".to_string(),
            timestamp: 1000,
        };

        let result = detector.check_double_signing(sig1, 1000).unwrap();
        assert!(result.is_none()); // No violation yet

        // Second signature at same height but different block - violation!
        let sig2 = BlockSignature {
            block_height: 100,
            block_hash: "hash2".to_string(),
            signature: "sig2".to_string(),
            masternode_id: "mn1".to_string(),
            timestamp: 1001,
        };

        let result = detector.check_double_signing(sig2, 1001).unwrap();
        assert!(result.is_some());

        let violation = result.unwrap();
        assert_eq!(violation.violation_type, ViolationType::DoubleSigning);
        assert_eq!(violation.masternode_id, "mn1");
    }

    #[test]
    fn test_double_signing_different_masternodes() {
        let mut detector = ViolationDetector::default();

        // MN1 signs block at height 100
        let sig1 = BlockSignature {
            block_height: 100,
            block_hash: "hash1".to_string(),
            signature: "sig1".to_string(),
            masternode_id: "mn1".to_string(),
            timestamp: 1000,
        };

        detector.check_double_signing(sig1, 1000).unwrap();

        // MN2 signs different block at same height - this is OK
        let sig2 = BlockSignature {
            block_height: 100,
            block_hash: "hash2".to_string(),
            signature: "sig2".to_string(),
            masternode_id: "mn2".to_string(),
            timestamp: 1001,
        };

        let result = detector.check_double_signing(sig2, 1001).unwrap();
        assert!(result.is_none()); // No violation - different masternodes
    }

    #[test]
    fn test_invalid_block_detection() {
        let mut detector = ViolationDetector::default();

        let violation = detector
            .check_invalid_block(
                "mn1".to_string(),
                100,
                "bad_hash".to_string(),
                "Invalid merkle root".to_string(),
                1000,
            )
            .unwrap();

        assert_eq!(violation.violation_type, ViolationType::InvalidBlock);
        assert_eq!(violation.masternode_id, "mn1");
        assert!(violation.evidence.contains("Invalid merkle root"));
    }

    #[test]
    fn test_extended_downtime_detection() {
        let mut detector = ViolationDetector::default();

        // Masternode last seen 100 days ago
        let last_seen = 1000;
        let current_time = last_seen + (100 * 24 * 60 * 60);

        let masternode = create_test_masternode("mn1", last_seen);

        let result = detector
            .check_extended_downtime(&masternode, current_time)
            .unwrap();

        assert!(result.is_some());
        let violation = result.unwrap();
        assert_eq!(violation.violation_type, ViolationType::ExtendedDowntime);
        assert_eq!(violation.masternode_id, "mn1");
    }

    #[test]
    fn test_downtime_under_threshold() {
        let mut detector = ViolationDetector::default();

        // Masternode last seen 80 days ago (under 90 day threshold)
        let last_seen = 1000;
        let current_time = last_seen + (80 * 24 * 60 * 60);

        let masternode = create_test_masternode("mn1", last_seen);

        let result = detector
            .check_extended_downtime(&masternode, current_time)
            .unwrap();

        assert!(result.is_none()); // No violation - under threshold
    }

    #[test]
    fn test_data_withholding_detection() {
        let mut detector = ViolationDetector::default();

        // Record multiple failed requests
        for i in 0..15 {
            detector.record_data_request(
                "mn1".to_string(),
                "block_data".to_string(),
                false,
                1000 + i,
            );
        }

        let result = detector.check_data_withholding("mn1", 2000).unwrap();
        assert!(result.is_some());

        let violation = result.unwrap();
        assert_eq!(violation.violation_type, ViolationType::DataWithholding);
        assert_eq!(violation.masternode_id, "mn1");
    }

    #[test]
    fn test_network_manipulation_detection() {
        let mut detector = ViolationDetector::default();

        let violation = detector
            .check_network_manipulation(
                "mn1".to_string(),
                "vote_manipulation".to_string(),
                "Coordinated voting detected".to_string(),
                1000,
            )
            .unwrap();

        assert_eq!(
            violation.violation_type,
            ViolationType::NetworkManipulation
        );
        assert_eq!(violation.masternode_id, "mn1");
    }

    #[test]
    fn test_cleanup_old_signatures() {
        let mut detector = ViolationDetector::default();

        // Add signatures at various heights
        for height in 1..=100 {
            let sig = BlockSignature {
                block_height: height,
                block_hash: format!("hash_{}", height),
                signature: format!("sig_{}", height),
                masternode_id: "mn1".to_string(),
                timestamp: 1000 + height,
            };
            detector.check_double_signing(sig, 1000 + height).unwrap();
        }

        assert_eq!(detector.block_signatures.len(), 100);

        // Cleanup - keep only last 10 blocks
        detector.cleanup_old_signatures(100, 10);

        assert!(detector.block_signatures.len() <= 10);
    }

    #[test]
    fn test_detector_stats() {
        let mut detector = ViolationDetector::default();

        // Add various violations
        detector
            .check_invalid_block(
                "mn1".to_string(),
                100,
                "hash1".to_string(),
                "reason".to_string(),
                1000,
            )
            .unwrap();

        detector
            .check_network_manipulation(
                "mn2".to_string(),
                "type".to_string(),
                "details".to_string(),
                1001,
            )
            .unwrap();

        let stats = detector.get_stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.invalid_blocks, 1);
        assert_eq!(stats.network_manipulation, 1);
    }
}
