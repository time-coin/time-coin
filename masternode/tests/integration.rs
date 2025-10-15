use masternode::*;

#[test]
fn test_collateral_tiers() {
    assert_eq!(CollateralTier::Bronze.required_collateral_time(), 1_000);
    assert_eq!(CollateralTier::Diamond.voting_power(), 100);
}

#[test]
fn test_masternode_creation() {
    let mn = Masternode::new(
        "test1".to_string(),
        "pubkey".to_string(),
        CollateralTier::Gold,
        "127.0.0.1".to_string(),
        9999,
        1000,
    );

    assert_eq!(mn.tier, CollateralTier::Gold);
    assert_eq!(mn.status, MasternodeStatus::Pending);
}

#[test]
fn test_reputation_tracking() {
    let mut rep = Reputation::new("mn1".to_string(), 1000);
    
    rep.record_block_validated(1001);
    assert_eq!(rep.blocks_validated, 1);
    assert_eq!(rep.score, 1);
    
    rep.record_block_missed(1002);
    assert_eq!(rep.blocks_missed, 1);
    assert!(rep.score < 1);
}
