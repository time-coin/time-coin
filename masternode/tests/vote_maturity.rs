//! Integration test for vote maturity enforcement
//! 
//! This test demonstrates how to properly check vote maturity before allowing
//! a masternode to participate in consensus voting.

use time_masternode::{CollateralTier, status::{MasternodeStatus, SyncStatus, VoteMaturityConfig}};

#[test]
fn test_vote_maturity_enforcement_community_tier() {
    // Create a Community tier masternode registered at block 100
    let mut mn_status = MasternodeStatus::new(
        "pubkey_community".to_string(),
        "192.168.1.10".to_string(),
        9000,
        100, // registered at block 100
    );
    
    // Make it active and synced
    mn_status.is_active = true;
    mn_status.sync_status = SyncStatus::Synced;
    
    let tier = CollateralTier::Community;
    
    // At block 100 (registration block), cannot vote yet
    assert!(!mn_status.can_vote_at_height(100, &tier), 
        "Community tier should not be able to vote at registration block");
    
    // At block 101 (1 block after registration), CAN vote
    assert!(mn_status.can_vote_at_height(101, &tier),
        "Community tier should be able to vote 1 block after registration");
    
    // At block 105, definitely can vote
    assert!(mn_status.can_vote_at_height(105, &tier),
        "Community tier should be able to vote at block 105");
}

#[test]
fn test_vote_maturity_enforcement_verified_tier() {
    // Create a Verified tier masternode registered at block 100
    let mut mn_status = MasternodeStatus::new(
        "pubkey_verified".to_string(),
        "192.168.1.11".to_string(),
        9000,
        100, // registered at block 100
    );
    
    // Make it active and synced
    mn_status.is_active = true;
    mn_status.sync_status = SyncStatus::Synced;
    
    let tier = CollateralTier::Verified;
    
    // At block 100, cannot vote
    assert!(!mn_status.can_vote_at_height(100, &tier),
        "Verified tier should not be able to vote at registration block");
    
    // At block 102 (2 blocks after), still cannot vote (needs 3)
    assert!(!mn_status.can_vote_at_height(102, &tier),
        "Verified tier should not be able to vote at block 102 (only 2 blocks)");
    
    // At block 103 (3 blocks after registration), CAN vote
    assert!(mn_status.can_vote_at_height(103, &tier),
        "Verified tier should be able to vote 3 blocks after registration");
}

#[test]
fn test_vote_maturity_enforcement_professional_tier() {
    // Create a Professional tier masternode registered at block 100
    let mut mn_status = MasternodeStatus::new(
        "pubkey_professional".to_string(),
        "192.168.1.12".to_string(),
        9000,
        100, // registered at block 100
    );
    
    // Make it active and synced
    mn_status.is_active = true;
    mn_status.sync_status = SyncStatus::Synced;
    
    let tier = CollateralTier::Professional;
    
    // At block 100, cannot vote
    assert!(!mn_status.can_vote_at_height(100, &tier),
        "Professional tier should not be able to vote at registration block");
    
    // At block 109 (9 blocks after), still cannot vote (needs 10)
    assert!(!mn_status.can_vote_at_height(109, &tier),
        "Professional tier should not be able to vote at block 109 (only 9 blocks)");
    
    // At block 110 (10 blocks after registration), CAN vote
    assert!(mn_status.can_vote_at_height(110, &tier),
        "Professional tier should be able to vote 10 blocks after registration");
}

#[test]
fn test_vote_maturity_with_config_override() {
    let mut config = VoteMaturityConfig::new();
    
    // Default configuration
    assert_eq!(config.get_maturity_blocks(&CollateralTier::Community), 1);
    assert_eq!(config.get_maturity_blocks(&CollateralTier::Verified), 3);
    assert_eq!(config.get_maturity_blocks(&CollateralTier::Professional), 10);
    
    // Admin adjusts Professional tier to require 20 blocks
    config.set_professional_maturity(20);
    assert_eq!(config.get_maturity_blocks(&CollateralTier::Professional), 20);
    
    // Emergency: disable all maturity checks
    config.emergency_disable_maturity();
    assert_eq!(config.get_maturity_blocks(&CollateralTier::Community), 0);
    assert_eq!(config.get_maturity_blocks(&CollateralTier::Verified), 0);
    assert_eq!(config.get_maturity_blocks(&CollateralTier::Professional), 0);
    
    // Emergency: set all tiers to same maturity
    config.emergency_set_all_maturity(5);
    assert_eq!(config.get_maturity_blocks(&CollateralTier::Community), 5);
    assert_eq!(config.get_maturity_blocks(&CollateralTier::Verified), 5);
    assert_eq!(config.get_maturity_blocks(&CollateralTier::Professional), 5);
}

#[test]
fn test_inactive_masternode_cannot_vote_even_after_maturity() {
    let mut mn_status = MasternodeStatus::new(
        "pubkey_inactive".to_string(),
        "192.168.1.13".to_string(),
        9000,
        100,
    );
    
    // Synced but NOT active
    mn_status.is_active = false;
    mn_status.sync_status = SyncStatus::Synced;
    
    let tier = CollateralTier::Community;
    
    // Even after maturity period, cannot vote because not active
    assert!(!mn_status.can_vote_at_height(110, &tier),
        "Inactive masternode should not be able to vote even after maturity period");
}

#[test]
fn test_not_synced_masternode_cannot_vote_even_after_maturity() {
    let mut mn_status = MasternodeStatus::new(
        "pubkey_notsynced".to_string(),
        "192.168.1.14".to_string(),
        9000,
        100,
    );
    
    // Active but NOT synced
    mn_status.is_active = true;
    mn_status.sync_status = SyncStatus::NotSynced;
    
    let tier = CollateralTier::Community;
    
    // Even after maturity period, cannot vote because not synced
    assert!(!mn_status.can_vote_at_height(110, &tier),
        "Not synced masternode should not be able to vote even after maturity period");
}

#[test]
fn test_coordinated_attack_prevention_scenario() {
    // Scenario: An attacker tries to register multiple masternodes
    // and immediately vote to take control of consensus
    
    let registration_block = 1000;
    let current_block = 1000; // Attack happens immediately
    
    // Create 5 attacking masternodes (different tiers)
    let attacker_nodes = vec![
        (CollateralTier::Community, "attacker1"),
        (CollateralTier::Community, "attacker2"),
        (CollateralTier::Verified, "attacker3"),
        (CollateralTier::Verified, "attacker4"),
        (CollateralTier::Professional, "attacker5"),
    ];
    
    let mut nodes_that_can_vote = 0;
    
    for (tier, name) in &attacker_nodes {
        let mut status = MasternodeStatus::new(
            format!("pubkey_{}", name),
            format!("10.0.0.{}", nodes_that_can_vote + 1),
            9000,
            registration_block,
        );
        status.is_active = true;
        status.sync_status = SyncStatus::Synced;
        
        if status.can_vote_at_height(current_block, tier) {
            nodes_that_can_vote += 1;
        }
    }
    
    // NONE of the newly registered nodes can vote immediately
    assert_eq!(nodes_that_can_vote, 0,
        "Newly registered masternodes should not be able to vote immediately, preventing instant takeover");
    
    // Check that they can vote after maturity
    let future_block = registration_block + 10; // 10 blocks later
    nodes_that_can_vote = 0;
    
    for (tier, name) in &attacker_nodes {
        let mut status = MasternodeStatus::new(
            format!("pubkey_{}", name),
            format!("10.0.0.{}", nodes_that_can_vote + 1),
            9000,
            registration_block,
        );
        status.is_active = true;
        status.sync_status = SyncStatus::Synced;
        
        if status.can_vote_at_height(future_block, tier) {
            nodes_that_can_vote += 1;
        }
    }
    
    // After 10 blocks, all nodes should be able to vote (Professional needs exactly 10)
    assert_eq!(nodes_that_can_vote, 5,
        "After maturity period, all masternodes should be able to vote");
}
