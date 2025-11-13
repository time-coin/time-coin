//! Comprehensive tests for violation detection system
//!
//! These tests simulate known attack vectors and verify that the automated
//! violation detection system correctly identifies and penalizes violations.

use time_core::constants::COIN;
use time_masternode::collateral::CollateralTier;
use time_masternode::detector::{BlockSignature, DetectorConfig, ViolationDetector};
use time_masternode::node::{Masternode, MasternodeStatus};
use time_masternode::reputation::Reputation;
use time_masternode::violations::{ViolationSeverity, ViolationType};

/// Helper to create a test masternode
fn create_test_masternode(id: &str, last_heartbeat: u64, tier: CollateralTier) -> Masternode {
    Masternode {
        id: id.to_string(),
        public_key: format!("pubkey_{}", id),
        tier,
        status: MasternodeStatus::Active,
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
fn test_double_signing_attack_detection() {
    let mut detector = ViolationDetector::default();

    // Simulate an attacker trying to create conflicting blocks
    let attacker_id = "malicious_mn1";

    // First block at height 1000
    let sig1 = BlockSignature {
        block_height: 1000,
        block_hash: "block_a_hash".to_string(),
        signature: "signature_a".to_string(),
        masternode_id: attacker_id.to_string(),
        timestamp: 10000,
    };

    // Attacker signs the first block - should be OK
    let result = detector.check_double_signing(sig1, 10000).unwrap();
    assert!(
        result.is_none(),
        "First signature should not trigger violation"
    );

    // Attacker tries to sign a different block at the same height - attack!
    let sig2 = BlockSignature {
        block_height: 1000,
        block_hash: "block_b_hash".to_string(),
        signature: "signature_b".to_string(),
        masternode_id: attacker_id.to_string(),
        timestamp: 10001,
    };

    let result = detector.check_double_signing(sig2, 10001).unwrap();
    assert!(result.is_some(), "Double-signing should be detected");

    let violation = result.unwrap();
    assert_eq!(violation.violation_type, ViolationType::DoubleSigning);
    assert_eq!(violation.masternode_id, attacker_id);
    assert_eq!(violation.severity(), ViolationSeverity::Critical);
    assert!(
        violation.should_auto_ban(),
        "Double-signing should result in auto-ban"
    );

    // Verify the violation was recorded
    let violations = detector.get_violations_for_masternode(attacker_id);
    assert_eq!(violations.len(), 1);
}

#[test]
fn test_multiple_masternodes_same_height_no_violation() {
    let mut detector = ViolationDetector::default();

    // Multiple masternodes can sign different blocks at the same height
    // This is normal in a distributed consensus system
    for i in 0..5 {
        let sig = BlockSignature {
            block_height: 1000,
            block_hash: format!("block_hash_{}", i),
            signature: format!("signature_{}", i),
            masternode_id: format!("mn{}", i),
            timestamp: 10000 + i,
        };

        let result = detector.check_double_signing(sig, 10000 + i).unwrap();
        assert!(
            result.is_none(),
            "Different masternodes signing different blocks should be OK"
        );
    }

    // No violations should be detected
    assert_eq!(detector.get_violations().len(), 0);
}

#[test]
fn test_invalid_block_creation_attack() {
    let mut detector = ViolationDetector::default();

    // Simulate masternode creating invalid blocks
    let attacker_id = "malicious_mn2";

    // Attempt 1: Invalid merkle root
    let violation1 = detector
        .check_invalid_block(
            attacker_id.to_string(),
            100,
            "invalid_block_1".to_string(),
            "Invalid merkle root: calculated hash does not match header".to_string(),
            1000,
        )
        .unwrap();

    assert_eq!(violation1.violation_type, ViolationType::InvalidBlock);
    assert_eq!(violation1.severity(), ViolationSeverity::High);
    assert!(violation1.evidence.contains("Invalid merkle root"));

    // Attempt 2: Invalid transaction
    let violation2 = detector
        .check_invalid_block(
            attacker_id.to_string(),
            101,
            "invalid_block_2".to_string(),
            "Transaction validation failed: double spend detected".to_string(),
            1001,
        )
        .unwrap();

    assert_eq!(violation2.violation_type, ViolationType::InvalidBlock);

    // Multiple invalid blocks should be recorded
    let violations = detector.get_violations_for_masternode(attacker_id);
    assert_eq!(violations.len(), 2);
}

#[test]
fn test_extended_downtime_abandonment() {
    let mut detector = ViolationDetector::default();

    // Masternode that has been offline for 100 days (exceeds 90-day threshold)
    let abandoned_mn_id = "abandoned_mn";
    let last_seen = 1000;
    let current_time = last_seen + (100 * 24 * 60 * 60); // 100 days later

    let masternode = create_test_masternode(abandoned_mn_id, last_seen, CollateralTier::Community);

    let result = detector
        .check_extended_downtime(&masternode, current_time)
        .unwrap();

    assert!(result.is_some(), "Extended downtime should be detected");

    let violation = result.unwrap();
    assert_eq!(violation.violation_type, ViolationType::ExtendedDowntime);
    assert_eq!(violation.masternode_id, abandoned_mn_id);
    assert_eq!(violation.severity(), ViolationSeverity::Medium);
    assert!(violation.evidence.contains("100 days"));
}

#[test]
fn test_downtime_within_threshold_no_violation() {
    let mut detector = ViolationDetector::default();

    // Masternode offline for 85 days (under 90-day threshold)
    let mn_id = "temporary_offline_mn";
    let last_seen = 1000;
    let current_time = last_seen + (85 * 24 * 60 * 60); // 85 days

    let masternode = create_test_masternode(mn_id, last_seen, CollateralTier::Verified);

    let result = detector
        .check_extended_downtime(&masternode, current_time)
        .unwrap();

    assert!(
        result.is_none(),
        "Downtime under threshold should not trigger violation"
    );
}

#[test]
fn test_custom_downtime_threshold() {
    // Test with custom threshold of 30 days
    let config = DetectorConfig {
        max_downtime_seconds: 30 * 24 * 60 * 60,
        ..Default::default()
    };

    let mut detector = ViolationDetector::new(config);

    let mn_id = "short_downtime_mn";
    let last_seen = 1000;
    let current_time = last_seen + (35 * 24 * 60 * 60); // 35 days

    let masternode = create_test_masternode(mn_id, last_seen, CollateralTier::Professional);

    let result = detector
        .check_extended_downtime(&masternode, current_time)
        .unwrap();

    assert!(result.is_some(), "Custom threshold should be respected");
}

#[test]
fn test_data_withholding_censorship_attack() {
    let mut detector = ViolationDetector::default();

    let attacker_id = "censoring_mn";
    let request_type = "block_data";

    // Simulate masternode repeatedly failing to respond to data requests
    for i in 0..15 {
        detector.record_data_request(
            attacker_id.to_string(),
            request_type.to_string(),
            false, // Failed response
            1000 + i,
        );
    }

    // Check for violation
    let result = detector.check_data_withholding(attacker_id, 2000).unwrap();

    assert!(result.is_some(), "Data withholding should be detected");

    let violation = result.unwrap();
    assert_eq!(violation.violation_type, ViolationType::DataWithholding);
    assert_eq!(violation.severity(), ViolationSeverity::High);
    assert!(violation.evidence.contains("15 failed"));
}

#[test]
fn test_data_withholding_intermittent_failures_no_violation() {
    let mut detector = ViolationDetector::default();

    let mn_id = "intermittent_mn";
    let request_type = "transaction_data";

    // Mix of successful and failed requests (under threshold)
    for i in 0..20 {
        let success = i % 3 != 0; // Fail every 3rd request
        detector.record_data_request(
            mn_id.to_string(),
            request_type.to_string(),
            success,
            1000 + i,
        );
    }

    // Should not trigger violation (only ~7 failures out of 20)
    let result = detector.check_data_withholding(mn_id, 2000).unwrap();
    assert!(
        result.is_none(),
        "Intermittent failures under threshold should be OK"
    );
}

#[test]
fn test_network_manipulation_coordinated_attack() {
    let mut detector = ViolationDetector::default();

    let attacker_id = "coordinated_attacker";

    // Detect coordinated voting manipulation
    let violation = detector
        .check_network_manipulation(
            attacker_id.to_string(),
            "coordinated_voting".to_string(),
            "Detected pattern of coordinated votes with other masternodes to manipulate governance outcomes".to_string(),
            1000,
        )
        .unwrap();

    assert_eq!(violation.violation_type, ViolationType::NetworkManipulation);
    assert_eq!(violation.severity(), ViolationSeverity::Critical);
    assert!(
        violation.should_auto_ban(),
        "Network manipulation should result in auto-ban"
    );
    assert!(violation.evidence.contains("coordinated votes"));
}

#[test]
fn test_sybil_attack_detection() {
    let mut detector = ViolationDetector::default();

    let attacker_id = "sybil_attacker";

    // Detect Sybil attack attempt
    let violation = detector
        .check_network_manipulation(
            attacker_id.to_string(),
            "sybil_attack".to_string(),
            "Multiple masternodes controlled by single entity detected through IP analysis"
                .to_string(),
            1000,
        )
        .unwrap();

    assert_eq!(violation.violation_type, ViolationType::NetworkManipulation);
    assert_eq!(violation.severity(), ViolationSeverity::Critical);
}

#[test]
fn test_penalty_application() {
    let mut detector = ViolationDetector::default();

    // Create violation
    let mut violation = detector
        .check_invalid_block(
            "mn1".to_string(),
            100,
            "bad_block".to_string(),
            "Invalid block".to_string(),
            1000,
        )
        .unwrap();

    // Apply penalty
    let collateral = 10_000 * COIN; // 10,000 TIME
    violation.apply_penalty(collateral).unwrap();

    assert!(violation.penalty_applied);
    assert_eq!(violation.reputation_penalty, -500);

    // 10% slash for invalid block
    let expected_slash = (collateral as f64 * 0.10) as u64;
    assert_eq!(violation.collateral_slashed, expected_slash);
}

#[test]
fn test_critical_violations_max_penalty() {
    let mut detector = ViolationDetector::default();

    // Double-signing violation
    let sig1 = BlockSignature {
        block_height: 1000,
        block_hash: "hash1".to_string(),
        signature: "sig1".to_string(),
        masternode_id: "mn1".to_string(),
        timestamp: 1000,
    };
    detector.check_double_signing(sig1, 1000).unwrap();

    let sig2 = BlockSignature {
        block_height: 1000,
        block_hash: "hash2".to_string(),
        signature: "sig2".to_string(),
        masternode_id: "mn1".to_string(),
        timestamp: 1001,
    };
    let mut violation = detector.check_double_signing(sig2, 1001).unwrap().unwrap();

    // Apply penalty
    let collateral = 100_000 * COIN; // 100,000 TIME
    violation.apply_penalty(collateral).unwrap();

    // Critical violations should slash 100% of collateral
    assert_eq!(violation.reputation_penalty, -1000); // Max reputation penalty
    assert_eq!(violation.collateral_slashed, collateral); // Full slash
}

#[test]
fn test_detector_statistics() {
    let mut detector = ViolationDetector::default();

    // Create various violations
    detector
        .check_invalid_block(
            "mn1".to_string(),
            100,
            "hash1".to_string(),
            "reason1".to_string(),
            1000,
        )
        .unwrap();

    detector
        .check_invalid_block(
            "mn2".to_string(),
            101,
            "hash2".to_string(),
            "reason2".to_string(),
            1001,
        )
        .unwrap();

    detector
        .check_network_manipulation(
            "mn3".to_string(),
            "type1".to_string(),
            "details1".to_string(),
            1002,
        )
        .unwrap();

    let stats = detector.get_stats();
    assert_eq!(stats.total, 3);
    assert_eq!(stats.invalid_blocks, 2);
    assert_eq!(stats.network_manipulation, 1);
    assert_eq!(stats.double_signing, 0);
}

#[test]
fn test_signature_cleanup() {
    let mut detector = ViolationDetector::default();

    // Add signatures at many different heights
    for height in 1..=1000 {
        let sig = BlockSignature {
            block_height: height,
            block_hash: format!("hash_{}", height),
            signature: format!("sig_{}", height),
            masternode_id: "mn1".to_string(),
            timestamp: 1000 + height,
        };
        detector.check_double_signing(sig, 1000 + height).unwrap();
    }

    // Verify all signatures are stored
    assert_eq!(detector.signature_count(), 1000);

    // Cleanup old signatures, keeping only last 100 blocks
    detector.cleanup_old_signatures(1000, 100);

    // Should have cleaned up old signatures
    assert!(detector.signature_count() <= 100);

    // Recent signatures should still be present
    assert!(detector.has_signature_at_height(1000));
}

#[test]
fn test_detector_configuration() {
    let config = DetectorConfig {
        max_downtime_seconds: 60 * 24 * 60 * 60, // 60 days
        max_failed_requests: 5,
        enable_double_sign_detection: true,
        enable_invalid_block_detection: false,
        enable_downtime_detection: true,
        enable_data_withholding_detection: true,
        enable_network_manipulation_detection: true,
    };

    let mut detector = ViolationDetector::new(config.clone());

    // Double-signing should work
    let sig1 = BlockSignature {
        block_height: 100,
        block_hash: "hash1".to_string(),
        signature: "sig1".to_string(),
        masternode_id: "mn1".to_string(),
        timestamp: 1000,
    };
    detector.check_double_signing(sig1, 1000).unwrap();

    // Invalid block detection should be disabled
    let result = detector.check_invalid_block(
        "mn1".to_string(),
        100,
        "hash1".to_string(),
        "reason".to_string(),
        1000,
    );
    assert!(result.is_err(), "Disabled detection should return error");
}

#[test]
fn test_violation_type_display() {
    assert_eq!(
        format!("{}", ViolationType::DoubleSigning),
        "Double Signing"
    );
    assert_eq!(format!("{}", ViolationType::InvalidBlock), "Invalid Block");
    assert_eq!(
        format!("{}", ViolationType::ExtendedDowntime),
        "Extended Downtime"
    );
    assert_eq!(
        format!("{}", ViolationType::DataWithholding),
        "Data Withholding"
    );
    assert_eq!(
        format!("{}", ViolationType::NetworkManipulation),
        "Network Manipulation"
    );
}

#[test]
fn test_severity_ordering() {
    assert!(ViolationSeverity::Critical > ViolationSeverity::High);
    assert!(ViolationSeverity::High > ViolationSeverity::Medium);
    assert!(ViolationSeverity::Medium > ViolationSeverity::Low);
}
