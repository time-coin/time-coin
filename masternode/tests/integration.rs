#![cfg(any())]
use time_core::COIN;
use time_masternode::*;

#[test]
fn test_masternode_lifecycle() {
    let mut registry = MasternodeRegistry::new();

    // Fund treasury first
    registry
        .treasury_mut()
        .deposit_block_reward(1, 1000)
        .unwrap();
    registry
        .treasury_mut()
        .deposit_block_reward(2, 2000)
        .unwrap();
    registry
        .treasury_mut()
        .deposit_block_reward(3, 3000)
        .unwrap();

    // Register Verified tier masternode
    let id = registry
        .register(
            "owner123".to_string(),
            10_000 * COIN, // Verified tier
            NetworkInfo {
                ip_address: "127.0.0.1".to_string(),
                port: 9000,
                protocol_version: 1,
                public_key: "pubkey123".to_string(),
            },
            1000,
            1000, // timestamp
        )
        .unwrap();

    // Activate masternode
    registry.activate(&id).unwrap();

    // Check status
    let mn = registry.get(&id).unwrap();
    assert!(mn.is_active());
    assert_eq!(mn.tier, CollateralTier::Verified);
    assert_eq!(mn.voting_power(), 10);
}

#[test]
fn test_tier_benefits() {
    let benefits = TierBenefits::all();
    assert_eq!(benefits.len(), 3);

    // Check Community tier
    let community = &benefits[0];
    assert_eq!(community.collateral_required, 1_000 * COIN);
    assert_eq!(community.apy, 18.0);
    assert_eq!(community.voting_power, 1);

    // Check Verified tier
    let verified = &benefits[1];
    assert_eq!(verified.collateral_required, 10_000 * COIN);
    assert_eq!(verified.apy, 24.0);
    assert_eq!(verified.voting_power, 10);

    // Check Professional tier
    let professional = &benefits[2];
    assert_eq!(professional.collateral_required, 100_000 * COIN);
    assert_eq!(professional.apy, 30.0);
    assert_eq!(professional.voting_power, 50);
}

#[test]
fn test_reward_calculation() {
    let reward = RewardCalculator::calculate_reward("mn123".to_string(), 1.0);

    assert_eq!(reward.base_reward, 95 * COIN);
    assert_eq!(reward.tier_multiplier, 1.0);
    assert_eq!(reward.total_reward, 95 * COIN);
}

#[test]
fn test_voting_power_scaling() {
    // Community: 1x
    assert_eq!(CollateralTier::Community.voting_multiplier(), 1);

    // Verified: 10x
    assert_eq!(CollateralTier::Verified.voting_multiplier(), 10);

    // Professional: 50x
    assert_eq!(CollateralTier::Professional.voting_multiplier(), 50);
}

#[test]
fn test_tier_upgrade() {
    let mut registry = MasternodeRegistry::new();

    // Fund treasury first
    registry
        .treasury_mut()
        .deposit_block_reward(1, 1000)
        .unwrap();
    registry
        .treasury_mut()
        .deposit_block_reward(2, 2000)
        .unwrap();

    // Start with Community tier
    let id = registry
        .register(
            "owner".to_string(),
            1_000 * COIN,
            NetworkInfo {
                ip_address: "127.0.0.1".to_string(),
                port: 9000,
                protocol_version: 1,
                public_key: "pubkey".to_string(),
            },
            100,
            1000, // timestamp
        )
        .unwrap();

    let mn = registry.get(&id).unwrap();
    assert_eq!(mn.tier, CollateralTier::Community);

    // Can upgrade by adding more collateral (future feature)
    // This would require additional implementation
}
