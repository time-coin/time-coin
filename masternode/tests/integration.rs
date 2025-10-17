use time_masternode::*;
use time_core::COIN;

#[test]
fn test_masternode_lifecycle() {
    let mut registry = MasternodeRegistry::new();
    
    // Register masternode
    let id = registry.register(
        "owner123".to_string(),
        10_000 * COIN, // Gold tier
        NetworkInfo {
            ip_address: "127.0.0.1".to_string(),
            port: 9000,
            protocol_version: 1,
            public_key: "pubkey123".to_string(),
        },
        1000,
    ).unwrap();
    
    // Activate masternode
    registry.activate(&id).unwrap();
    
    // Check status
    let mn = registry.get(&id).unwrap();
    assert!(mn.is_active());
    assert_eq!(mn.tier, CollateralTier::Gold);
    assert_eq!(mn.voting_power(), 10);
}

#[test]
fn test_tier_benefits() {
    let benefits = TierBenefits::all();
    assert_eq!(benefits.len(), 5);
    
    // Check Bronze tier
    let bronze = &benefits[0];
    assert_eq!(bronze.collateral_required, 1_000 * COIN);
    assert_eq!(bronze.apy, 18.0);
    assert_eq!(bronze.voting_power, 1);
    
    // Check Diamond tier
    let diamond = &benefits[4];
    assert_eq!(diamond.collateral_required, 100_000 * COIN);
    assert_eq!(diamond.apy, 30.0);
    assert_eq!(diamond.voting_power, 100);
}

#[test]
fn test_reward_calculation() {
    let reward = RewardCalculator::calculate_reward(
        "mn123".to_string(),
        1.0,
    );
    
    assert_eq!(reward.base_reward, 95 * COIN);
    assert_eq!(reward.tier_multiplier, 1.0);
    assert_eq!(reward.total_reward, 95 * COIN);
}

#[test]
fn test_voting_power_scaling() {
    // Bronze: 1x
    assert_eq!(CollateralTier::Bronze.voting_multiplier(), 1);
    
    // Silver: 5x
    assert_eq!(CollateralTier::Silver.voting_multiplier(), 5);
    
    // Gold: 10x
    assert_eq!(CollateralTier::Gold.voting_multiplier(), 10);
    
    // Platinum: 50x
    assert_eq!(CollateralTier::Platinum.voting_multiplier(), 50);
    
    // Diamond: 100x
    assert_eq!(CollateralTier::Diamond.voting_multiplier(), 100);
}
