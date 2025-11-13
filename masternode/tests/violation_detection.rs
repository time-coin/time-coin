//! Integration tests for violation detection system
//!
//! Tests various attack scenarios and violation detection mechanisms

use time_masternode::{
    detector::{DetectorConfig, ViolationDetector},
    violations::*,
};

#[test]
fn test_double_signing_attack_scenario() {
    let mut detector = ViolationDetector::new();

    // Attacker signs two conflicting blocks at the same height
    let violation = detector.record_block_signature(
        "malicious_node".to_string(),
        1000,
        "block_hash_a".to_string(),
        "signature_a".to_string(),
        1000,
    );
    assert!(violation.is_none(), "First signature should be accepted");

    // Try to sign another block at the same height
    let violation = detector.record_block_signature(
        "malicious_node".to_string(),
        1000,
        "block_hash_b".to_string(),
        "signature_b".to_string(),
        1001,
    );

    assert!(violation.is_some(), "Double-signing should be detected");
    let v = violation.unwrap();

    // Verify critical penalty
    assert_eq!(v.severity(), ViolationSeverity::Critical);
    assert_eq!(v.slash_percentage(), 1.0, "Should slash 100% collateral");
    assert!(v.auto_ban(), "Should result in automatic ban");
    assert_eq!(v.reputation_penalty(), -1000);

    // Verify evidence is cryptographically sound
    assert!(v.verify_evidence());
}

#[test]
fn test_invalid_merkle_root_attack() {
    let mut detector = ViolationDetector::new();

    // Node proposes a block with invalid merkle root
    let violation = detector.record_invalid_block(
        "bad_node".to_string(),
        5000,
        "invalid_block_hash".to_string(),
        "Merkle root mismatch".to_string(),
        Some("expected_merkle_root".to_string()),
        Some("actual_merkle_root".to_string()),
        10000,
    );

    assert_eq!(violation.severity(), ViolationSeverity::Moderate);
    assert_eq!(
        violation.slash_percentage(),
        0.20,
        "Should slash 20% for merkle mismatch"
    );
    assert!(!violation.auto_ban(), "Should not auto-ban");
    assert_eq!(violation.reputation_penalty(), -200);
    assert!(violation.verify_evidence());
}

#[test]
fn test_100_day_downtime_scenario() {
    let mut detector = ViolationDetector::new();

    // Record initial heartbeat
    detector.record_heartbeat("inactive_node".to_string(), 1000);

    // Simulate 100 days of downtime (90 day threshold)
    let days_100 = 100 * 86400;
    let current_time = 1000 + days_100;

    let violation = detector.check_downtime("inactive_node", current_time, 50000);

    assert!(violation.is_some());
    let v = violation.unwrap();

    assert_eq!(v.severity(), ViolationSeverity::Minor);
    assert_eq!(v.slash_percentage(), 0.05, "Should slash 5% for >90 days");
    assert!(!v.auto_ban());
    assert_eq!(v.reputation_penalty(), -200);
}

#[test]
fn test_120_day_downtime_scenario() {
    let mut detector = ViolationDetector::new();

    detector.record_heartbeat("very_inactive_node".to_string(), 1000);

    // 120 days of downtime
    let days_120 = 120 * 86400;
    let current_time = 1000 + days_120;

    let violation = detector.check_downtime("very_inactive_node", current_time, 50000);

    assert!(violation.is_some());
    let v = violation.unwrap();

    // Should still be 5% for < 365 days
    assert_eq!(v.slash_percentage(), 0.05);
}

#[test]
fn test_data_withholding_5_consecutive_failures() {
    let mut detector = ViolationDetector::new();

    // Record 5 consecutive data request failures
    for i in 0..5 {
        let result = detector.record_data_request(
            "withholding_node".to_string(),
            "block_data".to_string(),
            1000 + i,
            false, // failure
            1000,
        );

        if i < 4 {
            assert!(result.is_none(), "Should not trigger until 5th failure");
        } else {
            assert!(
                result.is_some(),
                "Should trigger on 5th consecutive failure"
            );
            let v = result.unwrap();
            assert_eq!(v.severity(), ViolationSeverity::Moderate);
            assert_eq!(
                v.slash_percentage(),
                0.10,
                "Should slash 10% for 5 failures"
            );
            assert_eq!(v.reputation_penalty(), -300);
        }
    }
}

#[test]
fn test_data_withholding_10_consecutive_failures() {
    let mut detector = ViolationDetector::new();

    // Record 10 consecutive failures (more severe)
    for i in 0..10 {
        let result = detector.record_data_request(
            "severe_withholding_node".to_string(),
            "block_data".to_string(),
            1000 + i,
            false,
            1000,
        );

        if i == 4 {
            // First violation at 5 failures
            assert!(result.is_some());
        }
    }

    // Check the last violation
    let violations = detector.get_violations_for_masternode("severe_withholding_node");
    let last_violation = violations.last().unwrap();

    assert_eq!(
        last_violation.slash_percentage(),
        0.20,
        "Should slash 20% for 10+ failures"
    );
}

#[test]
fn test_coordinated_vote_manipulation_3_nodes() {
    let mut detector = ViolationDetector::with_config(DetectorConfig {
        min_coordinated_nodes: 3,
        ..Default::default()
    });

    // 3 nodes vote identically within 60 second window (suspicious)
    for i in 0..3 {
        detector.record_vote(
            format!("coordinated_node_{}", i),
            "proposal_xyz".to_string(),
            true,     // all vote the same way
            1000 + i, // within same time bucket
            100,
        );
    }

    let violation = detector.detect_vote_manipulation("proposal_xyz", 1100, 150);

    assert!(violation.is_some());
    let v = violation.unwrap();

    assert_eq!(v.severity(), ViolationSeverity::Critical);
    assert_eq!(v.slash_percentage(), 1.0, "Should slash 100%");
    assert!(v.auto_ban(), "Should auto-ban for network manipulation");
    assert_eq!(v.reputation_penalty(), -1000);
}

#[test]
fn test_coordinated_vote_manipulation_5_nodes() {
    let mut detector = ViolationDetector::with_config(DetectorConfig {
        min_coordinated_nodes: 3,
        ..Default::default()
    });

    // 5 nodes coordinating (more severe attack)
    for i in 0..5 {
        detector.record_vote(
            format!("attacker_{}", i),
            "proposal_123".to_string(),
            false,          // all vote no
            2000 + (i * 2), // spread within 10 seconds
            200,
        );
    }

    let violation = detector.detect_vote_manipulation("proposal_123", 2100, 250);
    assert!(violation.is_some());

    let v = violation.unwrap();
    assert_eq!(v.severity(), ViolationSeverity::Critical);
}

#[test]
fn test_no_false_positive_legitimate_voting() {
    let mut detector = ViolationDetector::with_config(DetectorConfig {
        min_coordinated_nodes: 3,
        ..Default::default()
    });

    // Nodes voting at different times (legitimate)
    for i in 0..3 {
        detector.record_vote(
            format!("legitimate_node_{}", i),
            "proposal_abc".to_string(),
            true,
            1000 + (i * 100), // spread out over time
            100,
        );
    }

    let violation = detector.detect_vote_manipulation("proposal_abc", 1500, 150);
    assert!(
        violation.is_none(),
        "Should not flag legitimate voting patterns"
    );
}

#[test]
fn test_evidence_integrity_verification() {
    let evidence = Evidence::new("test_type".to_string(), "original_data".to_string(), 1000);

    assert!(evidence.verify(), "Original evidence should verify");

    // Simulate tampering
    let mut tampered = evidence.clone();
    tampered.data = "tampered_data".to_string();

    assert!(!tampered.verify(), "Tampered evidence should not verify");
}

#[test]
fn test_multiple_violations_same_node() {
    let mut detector = ViolationDetector::new();

    // Node commits multiple violations

    // 1. Invalid block
    detector.record_invalid_block(
        "bad_actor".to_string(),
        1000,
        "hash1".to_string(),
        "Invalid transaction".to_string(),
        None,
        None,
        5000,
    );

    // 2. Data withholding (triggers at 5th failure)
    for i in 0..5 {
        detector.record_data_request(
            "bad_actor".to_string(),
            "block_data".to_string(),
            5010 + i,
            false,
            1000,
        );
    }

    // 3. Another invalid block
    detector.record_invalid_block(
        "bad_actor".to_string(),
        1100,
        "hash2".to_string(),
        "Invalid signature".to_string(),
        None,
        None,
        5100,
    );

    let violations = detector.get_violations_for_masternode("bad_actor");
    // Should have: 1 invalid block + 1 data withholding + 1 invalid block = 3 violations
    assert_eq!(violations.len(), 3, "Should track all violations");
}

#[test]
fn test_violation_penalties_accumulate() {
    let mut detector = ViolationDetector::new();

    // Multiple minor violations
    for i in 0..3 {
        detector.record_invalid_block(
            "serial_offender".to_string(),
            1000 + i,
            format!("hash{}", i),
            "Minor issue".to_string(),
            None,
            None,
            5000 + i,
        );
    }

    let violations = detector.get_violations_for_masternode("serial_offender");

    // Calculate total penalties
    let total_reputation_penalty: i32 = violations.iter().map(|v| v.reputation_penalty()).sum();

    assert_eq!(
        total_reputation_penalty, -600,
        "Penalties should accumulate"
    );
}

#[test]
fn test_no_downtime_violation_within_threshold() {
    let mut detector = ViolationDetector::new();

    detector.record_heartbeat("healthy_node".to_string(), 1000);

    // Check after 80 days (below 90 day threshold)
    let days_80 = 80 * 86400;
    let current_time = 1000 + days_80;

    let violation = detector.check_downtime("healthy_node", current_time, 50000);

    assert!(
        violation.is_none(),
        "Should not flag downtime below threshold"
    );
}

#[test]
fn test_data_withholding_resets_on_success() {
    let mut detector = ViolationDetector::new();

    // 4 failures
    for i in 0..4 {
        let result = detector.record_data_request(
            "recovering_node".to_string(),
            "block_data".to_string(),
            1000 + i,
            false,
            1000,
        );
        assert!(result.is_none());
    }

    // Success - should reset counter
    let result = detector.record_data_request(
        "recovering_node".to_string(),
        "block_data".to_string(),
        1004,
        true, // success
        1000,
    );
    assert!(result.is_none());

    // 4 more failures - should not trigger yet
    for i in 5..9 {
        let result = detector.record_data_request(
            "recovering_node".to_string(),
            "block_data".to_string(),
            1000 + i,
            false,
            1000,
        );
        assert!(
            result.is_none(),
            "Counter should have been reset by success"
        );
    }
}

#[test]
fn test_different_nodes_same_block_height() {
    let mut detector = ViolationDetector::new();

    // Different nodes signing the same block height is OK
    let v1 = detector.record_block_signature(
        "node1".to_string(),
        1000,
        "block_hash_x".to_string(),
        "sig1".to_string(),
        1000,
    );
    assert!(v1.is_none());

    let v2 = detector.record_block_signature(
        "node2".to_string(),
        1000,
        "block_hash_x".to_string(),
        "sig2".to_string(),
        1000,
    );
    assert!(
        v2.is_none(),
        "Different nodes signing same block is legitimate"
    );
}

#[test]
fn test_reputation_penalties_match_spec() {
    // Verify all penalties match requirements
    let ds = ViolationType::DoubleSigning(DoubleSigning {
        block_height: 1,
        block_hash_1: "a".to_string(),
        block_hash_2: "b".to_string(),
        signature_1: "s1".to_string(),
        signature_2: "s2".to_string(),
    });
    assert_eq!(ds.reputation_penalty(), -1000);

    let invalid = ViolationType::InvalidBlock(InvalidBlock {
        block_height: 1,
        block_hash: "h".to_string(),
        reason: "r".to_string(),
        expected_merkle_root: None,
        actual_merkle_root: None,
    });
    assert_eq!(invalid.reputation_penalty(), -200);

    let downtime = ViolationType::ExtendedDowntime(ExtendedDowntime {
        days_offline: 100,
        last_seen: 1000,
        detected_at: 2000,
    });
    assert_eq!(downtime.reputation_penalty(), -200);

    let withholding = ViolationType::DataWithholding(DataWithholding {
        consecutive_failures: 5,
        data_type: "data".to_string(),
        failed_requests: vec![],
    });
    assert_eq!(withholding.reputation_penalty(), -300);

    let manipulation = ViolationType::NetworkManipulation(NetworkManipulation {
        manipulation_type: "vote".to_string(),
        coordinated_nodes: 3,
        description: "desc".to_string(),
    });
    assert_eq!(manipulation.reputation_penalty(), -1000);
}
