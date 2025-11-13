use time_masternode::slashing::{Violation, calculate_slash_amount};
use time_masternode::slashing_executor::SlashingExecutor;
use time_masternode::{Masternode, MasternodeNetwork, COIN};
use wallet::Address;
use chrono::Utc;

#[test]
fn test_complete_slashing_workflow() {
    let mut network = MasternodeNetwork::new();
    let mut executor = SlashingExecutor::new();

    // Create and register a masternode
    let node = Masternode::new(
        Address::from_public_key(&[1u8; 32], wallet::NetworkType::Mainnet).unwrap(),
        "collateral-tx-1".to_string(),
        10_000 * COIN, // Verified tier
    )
    .unwrap();

    let address = node.address.clone();
    let initial_collateral = node.collateral_amount;
    network.register(node).unwrap();

    // Create a violation (invalid block)
    let violation = Violation::InvalidBlock {
        block_height: 1000,
        reason: "Invalid transaction in block".to_string(),
    };

    let timestamp = Utc::now().timestamp();
    let block_height = 1000;

    // Execute slashing through the network
    let record = network
        .slash_masternode(&address, violation, timestamp, block_height)
        .unwrap();

    // Verify collateral was deducted (5% for invalid block)
    let expected_slash = (initial_collateral as f64 * 0.05) as u64;
    assert_eq!(record.amount, expected_slash);
    assert_eq!(record.remaining_collateral, initial_collateral - expected_slash);

    // Verify node was updated
    let node = network.get_node(&address).unwrap();
    assert_eq!(node.collateral_amount, initial_collateral - expected_slash);
    assert_eq!(node.slashing_history.len(), 1);

    // Execute the slashing through executor (treasury transfer)
    let event = executor.execute_slashing(record, timestamp as u64).unwrap();
    
    assert!(event.treasury_transfer_success);
    assert!(event.treasury_tx_id.is_some());
    assert_eq!(executor.total_slashed(), expected_slash);
    assert_eq!(executor.total_transferred_to_treasury(), expected_slash);
}

#[test]
fn test_double_signing_slashing() {
    let mut network = MasternodeNetwork::new();

    let node = Masternode::new(
        Address::from_public_key(&[2u8; 32], wallet::NetworkType::Mainnet).unwrap(),
        "collateral-tx-2".to_string(),
        100_000 * COIN, // Professional tier
    )
    .unwrap();

    let address = node.address.clone();
    let initial_collateral = node.collateral_amount;
    network.register(node).unwrap();

    // Double signing violation (50% slash)
    let violation = Violation::DoubleSigning {
        block_height: 2000,
        evidence: "signature-proof".to_string(),
    };

    let timestamp = Utc::now().timestamp();
    let record = network
        .slash_masternode(&address, violation, timestamp, 2000)
        .unwrap();

    // Should slash 50%
    let expected_slash = initial_collateral / 2;
    assert_eq!(record.amount, expected_slash);
    assert_eq!(record.remaining_collateral, initial_collateral - expected_slash);

    // Verify node still has enough collateral for Professional tier
    let node = network.get_node(&address).unwrap();
    assert_eq!(node.collateral_amount, initial_collateral - expected_slash);
    assert!(!node.is_slashed); // Still above minimum tier requirement
}

#[test]
fn test_network_attack_full_slashing() {
    let mut network = MasternodeNetwork::new();

    let node = Masternode::new(
        Address::from_public_key(&[3u8; 32], wallet::NetworkType::Mainnet).unwrap(),
        "collateral-tx-3".to_string(),
        10_000 * COIN, // Verified tier
    )
    .unwrap();

    let address = node.address.clone();
    let initial_collateral = node.collateral_amount;
    network.register(node).unwrap();

    // Network attack (100% slash)
    let violation = Violation::NetworkAttack {
        attack_type: "DDoS".to_string(),
        evidence: "attack-proof".to_string(),
    };

    let timestamp = Utc::now().timestamp();
    let record = network
        .slash_masternode(&address, violation, timestamp, 3000)
        .unwrap();

    // Should slash 100%
    assert_eq!(record.amount, initial_collateral);
    assert_eq!(record.remaining_collateral, 0);

    // Verify node is marked as slashed
    let node = network.get_node(&address).unwrap();
    assert_eq!(node.collateral_amount, 0);
    assert!(node.is_slashed); // Below minimum requirement
}

#[test]
fn test_long_term_abandonment_slashing() {
    let mut network = MasternodeNetwork::new();

    let node = Masternode::new(
        Address::from_public_key(&[4u8; 32], wallet::NetworkType::Mainnet).unwrap(),
        "collateral-tx-4".to_string(),
        10_000 * COIN,
    )
    .unwrap();

    let address = node.address.clone();
    let initial_collateral = node.collateral_amount;
    network.register(node).unwrap();

    // Test different abandonment periods
    
    // 50 days offline - 10% slash
    let violation = Violation::LongTermAbandonment { days_offline: 50 };
    let calculated_slash = calculate_slash_amount(&violation, initial_collateral);
    assert_eq!(calculated_slash, (initial_collateral as f64 * 0.1) as u64);

    // 70 days offline - 15% slash
    let violation = Violation::LongTermAbandonment { days_offline: 70 };
    let calculated_slash = calculate_slash_amount(&violation, initial_collateral);
    assert_eq!(calculated_slash, (initial_collateral as f64 * 0.15) as u64);

    // 100 days offline - 20% slash
    let violation = Violation::LongTermAbandonment { days_offline: 100 };
    let timestamp = Utc::now().timestamp();
    let record = network
        .slash_masternode(&address, violation, timestamp, 4000)
        .unwrap();

    let expected_slash = (initial_collateral as f64 * 0.2) as u64;
    assert_eq!(record.amount, expected_slash);
    assert_eq!(record.remaining_collateral, initial_collateral - expected_slash);
}

#[test]
fn test_multiple_slashings_same_node() {
    let mut network = MasternodeNetwork::new();

    let node = Masternode::new(
        Address::from_public_key(&[5u8; 32], wallet::NetworkType::Mainnet).unwrap(),
        "collateral-tx-5".to_string(),
        100_000 * COIN, // Professional tier
    )
    .unwrap();

    let address = node.address.clone();
    let initial_collateral = node.collateral_amount;
    network.register(node).unwrap();

    // First slashing: Invalid block (5%)
    let violation1 = Violation::InvalidBlock {
        block_height: 1000,
        reason: "Invalid tx".to_string(),
    };
    let timestamp1 = Utc::now().timestamp();
    let record1 = network
        .slash_masternode(&address, violation1, timestamp1, 1000)
        .unwrap();

    let after_first_slash = initial_collateral - record1.amount;

    // Second slashing: Data withholding (25% of remaining)
    let violation2 = Violation::DataWithholding {
        evidence: "proof".to_string(),
    };
    let timestamp2 = timestamp1 + 100;
    let record2 = network
        .slash_masternode(&address, violation2, timestamp2, 2000)
        .unwrap();

    // Verify both slashings are recorded
    let records = network.get_slashing_records(&address);
    assert_eq!(records.len(), 2);

    // Verify cumulative collateral deduction
    let node = network.get_node(&address).unwrap();
    assert_eq!(node.collateral_amount, after_first_slash - record2.amount);
    assert_eq!(node.slashing_history.len(), 2);
}

#[test]
fn test_slashing_record_details() {
    let mut network = MasternodeNetwork::new();

    let node = Masternode::new(
        Address::from_public_key(&[6u8; 32], wallet::NetworkType::Mainnet).unwrap(),
        "collateral-tx-6".to_string(),
        10_000 * COIN,
    )
    .unwrap();

    let address = node.address.clone();
    network.register(node).unwrap();

    let violation = Violation::InvalidBlock {
        block_height: 5000,
        reason: "Invalid transaction".to_string(),
    };

    let timestamp = Utc::now().timestamp();
    let block_height = 5000;
    let record = network
        .slash_masternode(&address, violation.clone(), timestamp, block_height)
        .unwrap();

    // Verify record details
    assert_eq!(record.masternode_id, address.to_string());
    assert_eq!(record.block_height, block_height);
    assert!(record.id.starts_with("slash-"));
    
    match &record.violation {
        Violation::InvalidBlock { block_height, reason } => {
            assert_eq!(*block_height, 5000);
            assert_eq!(reason, "Invalid transaction");
        }
        _ => panic!("Expected InvalidBlock violation"),
    }
}

#[test]
fn test_prevent_double_slashing_same_node() {
    let mut node = Masternode::new(
        Address::from_public_key(&[7u8; 32], wallet::NetworkType::Mainnet).unwrap(),
        "collateral-tx-7".to_string(),
        1_000 * COIN, // Community tier - minimum collateral
    )
    .unwrap();

    // First slashing that brings collateral below minimum
    let violation = Violation::InvalidBlock {
        block_height: 1000,
        reason: "Invalid tx".to_string(),
    };

    let timestamp = Utc::now().timestamp();
    let _record = node.execute_slash(violation.clone(), timestamp, 1000).unwrap();
    
    // Node should be marked as slashed
    assert!(node.is_slashed);

    // Try to slash again - should fail
    let result = node.execute_slash(violation, timestamp + 100, 1001);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Masternode is already slashed");
}

#[test]
fn test_slashing_executor_integration() {
    let mut executor = SlashingExecutor::new();

    // Create multiple slashing records
    let violations = vec![
        Violation::InvalidBlock {
            block_height: 1000,
            reason: "Invalid tx".to_string(),
        },
        Violation::DataWithholding {
            evidence: "proof1".to_string(),
        },
        Violation::DoubleSigning {
            block_height: 2000,
            evidence: "proof2".to_string(),
        },
    ];

    let mut total_expected = 0u64;

    for (i, violation) in violations.into_iter().enumerate() {
        let collateral = 10_000 * COIN;
        let slash_amount = calculate_slash_amount(&violation, collateral);
        total_expected += slash_amount;

        let record = time_masternode::slashing::SlashingRecord::new(
            format!("slash-{}", i),
            format!("node-{}", i),
            violation,
            slash_amount,
            collateral - slash_amount,
            (1234567890 + i) as u64,
            (1000 + i) as u64,
        );

        let event = executor.execute_slashing(record, 1234567890).unwrap();
        assert!(event.treasury_transfer_success);
    }

    // Verify totals
    assert_eq!(executor.total_slashed(), total_expected);
    assert_eq!(executor.total_transferred_to_treasury(), total_expected);
    assert_eq!(executor.get_events().len(), 3);
}

#[test]
fn test_get_all_slashing_records() {
    let mut network = MasternodeNetwork::new();

    // Create three masternodes
    for i in 0..3 {
        let node = Masternode::new(
            Address::from_public_key(&[i as u8; 32], wallet::NetworkType::Mainnet).unwrap(),
            format!("collateral-tx-{}", i),
            10_000 * COIN,
        )
        .unwrap();
        network.register(node).unwrap();
    }

    // Slash each one
    let timestamp = Utc::now().timestamp();
    for i in 0..3 {
        let address = Address::from_public_key(&[i as u8; 32], wallet::NetworkType::Mainnet).unwrap();
        let violation = Violation::InvalidBlock {
            block_height: 1000 + i as u64,
            reason: format!("Violation {}", i),
        };
        network.slash_masternode(&address, violation, timestamp, 1000 + i as u64).unwrap();
    }

    // Get all records
    let all_records = network.get_all_slashing_records();
    assert_eq!(all_records.len(), 3);

    // Verify each node has one record
    for i in 0..3 {
        let address = Address::from_public_key(&[i as u8; 32], wallet::NetworkType::Mainnet).unwrap();
        let records = network.get_slashing_records(&address);
        assert_eq!(records.len(), 1);
    }
}
