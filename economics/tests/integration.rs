use economics::*;

#[test]
fn test_supply_management() {
    let mut supply = SupplyManager::new();
    
    supply.mint(1000);
    assert_eq!(supply.stats().total_minted, 1000);
    assert_eq!(supply.stats().circulating_supply, 1000);
    
    supply.burn(100);
    assert_eq!(supply.stats().total_burned, 100);
    assert_eq!(supply.stats().circulating_supply, 900);
    assert_eq!(supply.net_supply(), 900);
}

#[test]
fn test_reward_calculation() {
    let reward = RewardCalculator::calculate_block_reward(1);
    
    // Whitepaper: 100 TIME per block (24 hours)
    // 5 TIME to treasury, 95 TIME to masternodes
    assert_eq!(reward.treasury_reward, 5 * constants::TIME_UNIT);
    assert_eq!(reward.masternode_reward, 95 * constants::TIME_UNIT);
    assert_eq!(reward.total_reward, 100 * constants::TIME_UNIT);
}

#[test]
fn test_fee_split() {
    let fee = 1000;
    let (treasury, masternode) = RewardCalculator::split_transaction_fee(fee);
    
    // 50/50 split
    assert_eq!(treasury, 500);
    assert_eq!(masternode, 500);
}

#[test]
fn test_masternode_apy() {
    // Test with 24-hour block model
    // 
    // Assumptions for this test:
    // - 100 masternodes in network
    // - Each gets proportional share of 95 TIME per day
    // - Collateral: 10,000 TIME
    // - Daily reward: 95/100 = 0.95 TIME
    // - Annual reward: 0.95 * 365 = 346.75 TIME
    // - APY: (346.75 / 10,000) * 100 = 3.47%
    
    let collateral = 10_000 * constants::TIME_UNIT;
    
    // Assume this masternode gets 1% of network rewards
    // (realistic if there are ~100 masternodes of similar size)
    let daily_reward_share = 0.01; // 1% of network
    let daily_masternode_pool = 95 * constants::TIME_UNIT;
    let daily = (daily_masternode_pool as f64 * daily_reward_share) as u64;
    
    let apy = RewardCalculator::calculate_masternode_apy(collateral, daily);
    
    // With 1% share: 0.95 TIME/day = 346.75 TIME/year
    // APY = (346.75 / 10,000) * 100 = ~3.47%
    assert!(apy > 3.0 && apy < 4.0, "APY was {}, expected 3-4%", apy);
}

#[test]
fn test_masternode_apy_realistic_scenarios() {
    // Scenario 1: Bronze tier in large network (low share)
    let bronze_collateral = 1_000 * constants::TIME_UNIT;
    let bronze_daily = (95 * constants::TIME_UNIT) / 1000; // 0.1% share
    let bronze_apy = RewardCalculator::calculate_masternode_apy(bronze_collateral, bronze_daily);
    
    // 0.095 TIME/day = 34.675 TIME/year
    // APY = (34.675 / 1,000) * 100 = ~3.47%
    assert!(bronze_apy > 3.0 && bronze_apy < 4.0, "Bronze APY: {}", bronze_apy);
    
    // Scenario 2: Gold tier with significant share
    let gold_collateral = 100_000 * constants::TIME_UNIT;
    let gold_daily = (95 * constants::TIME_UNIT) / 10; // 10% share (dominant node)
    let gold_apy = RewardCalculator::calculate_masternode_apy(gold_collateral, gold_daily);
    
    // 9.5 TIME/day = 3,467.5 TIME/year
    // APY = (3,467.5 / 100,000) * 100 = ~3.47%
    assert!(gold_apy > 3.0 && gold_apy < 4.0, "Gold APY: {}", gold_apy);
    
    // Note: With proportional distribution, APY is similar across tiers
    // The difference comes from longevity multiplier (not tested here)
}

#[test]
fn test_annual_block_rewards() {
    // Verify annual issuance from block rewards
    let blocks_per_year = 365;
    let reward_per_block = 100 * constants::TIME_UNIT;
    
    let annual_issuance = blocks_per_year * reward_per_block;
    let expected = 36_500 * constants::TIME_UNIT; // 36,500 TIME per year
    
    assert_eq!(annual_issuance, expected);
    
    // Treasury gets 5 TIME per block
    let annual_treasury = blocks_per_year * 5 * constants::TIME_UNIT;
    assert_eq!(annual_treasury, 1_825 * constants::TIME_UNIT); // 1,825 TIME/year
    
    // Masternodes get 95 TIME per block
    let annual_masternode = blocks_per_year * 95 * constants::TIME_UNIT;
    assert_eq!(annual_masternode, 34_675 * constants::TIME_UNIT); // 34,675 TIME/year
}

#[test]
fn test_block_economics() {
    // Whitepaper verification
    assert_eq!(constants::BLOCK_TIME, 86400); // 24 hours
    assert_eq!(constants::BLOCKS_PER_DAY, 1); // 1 block per day
    assert_eq!(constants::BLOCK_REWARD, 100 * constants::TIME_UNIT);
    assert_eq!(constants::TREASURY_REWARD, 5 * constants::TIME_UNIT);
    assert_eq!(constants::MASTERNODE_REWARD, 95 * constants::TIME_UNIT);
}
