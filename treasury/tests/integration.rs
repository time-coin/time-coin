use treasury::*;

#[test]
fn test_treasury_basic_flow() {
    let mut pool = TreasuryPool::new();

    // Deposit block reward
    pool.deposit_block_reward(1, 1000).unwrap();
    assert_eq!(pool.balance(), TREASURY_BLOCK_REWARD);

    // Check balance in TIME
    assert_eq!(pool.balance_time(), 5.0);
}

#[test]
fn test_treasury_fee_distribution() {
    let mut pool = TreasuryPool::new();
    let total_fee = TIME_UNIT; // 1 TIME

    pool.deposit_transaction_fee("tx123".to_string(), total_fee, 1000)
        .unwrap();

    // Treasury should receive 50%
    assert_eq!(pool.balance(), TIME_UNIT / 2);
}
