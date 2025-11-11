//! Integration tests for foolproof block creation system

use time_consensus::foolproof_block::{
    BlockCreationStrategy, FoolproofBlockManager, FoolproofConfig,
};

#[tokio::test]
async fn test_full_strategy_progression() {
    // Simulate a complete failure scenario where all strategies are tried
    let config = FoolproofConfig::default();
    let manager = FoolproofBlockManager::new(config);
    
    manager.start_round().await;
    
    // Simulate failures through all strategies
    let strategies = vec![
        BlockCreationStrategy::NormalBFT,
        BlockCreationStrategy::LeaderRotation,
        BlockCreationStrategy::ReducedThreshold,
        BlockCreationStrategy::RewardOnly,
        BlockCreationStrategy::Emergency,
    ];
    
    for (i, expected_strategy) in strategies.iter().enumerate() {
        let current = manager.current_strategy().await;
        assert_eq!(current, *expected_strategy, "Strategy mismatch at iteration {}", i);
        
        // Record a failure for all but the last
        let succeeded = i == strategies.len() - 1; // Emergency always succeeds
        manager.record_attempt(
            current,
            format!("node{}", i),
            if succeeded { 2 } else { 1 },
            4,
            succeeded,
            if succeeded { None } else { Some("Timeout".to_string()) },
        ).await;
        
        if i < strategies.len() - 1 {
            // Advance to next strategy
            manager.advance_strategy().await;
        }
    }
    
    // Verify final state
    assert_eq!(manager.attempt_count().await, 5);
    assert!(manager.has_success().await);
    
    // Check that all attempts were recorded
    let attempts = manager.get_attempts().await;
    assert_eq!(attempts.len(), 5);
    
    // Verify last attempt succeeded
    assert!(attempts.last().unwrap().succeeded);
}

#[tokio::test]
async fn test_early_success() {
    // Simulate success on first attempt
    let config = FoolproofConfig::default();
    let manager = FoolproofBlockManager::new(config);
    
    manager.start_round().await;
    
    // First attempt succeeds
    manager.record_attempt(
        BlockCreationStrategy::NormalBFT,
        "node1".to_string(),
        4, // 4 out of 6 votes
        6,
        true,
        None,
    ).await;
    
    assert_eq!(manager.attempt_count().await, 1);
    assert!(manager.has_success().await);
}

#[tokio::test]
async fn test_consensus_thresholds() {
    let config = FoolproofConfig::default();
    let manager = FoolproofBlockManager::new(config);
    
    manager.start_round().await;
    
    // Test Normal BFT: need 2/3+ of 6 nodes = 4 votes
    assert!(manager.check_consensus_with_strategy(4, 6).await);
    assert!(!manager.check_consensus_with_strategy(3, 6).await);
    
    // Advance to LeaderRotation (still 2/3)
    manager.advance_strategy().await;
    assert!(manager.check_consensus_with_strategy(4, 6).await);
    assert!(!manager.check_consensus_with_strategy(3, 6).await);
    
    // Advance to ReducedThreshold (1/2+)
    manager.advance_strategy().await;
    assert!(manager.check_consensus_with_strategy(4, 6).await);
    assert!(manager.check_consensus_with_strategy(3, 6).await); // Now 3 is enough
    assert!(!manager.check_consensus_with_strategy(2, 6).await);
    
    // Advance to RewardOnly (1/3+)
    manager.advance_strategy().await;
    assert!(manager.check_consensus_with_strategy(2, 6).await);
    assert!(!manager.check_consensus_with_strategy(1, 6).await);
    
    // Advance to Emergency (any vote, 10%+)
    manager.advance_strategy().await;
    assert!(manager.check_consensus_with_strategy(1, 6).await); // Even 1 vote is enough
}

#[tokio::test]
async fn test_timeout_tracking() {
    let config = FoolproofConfig::default();
    let manager = FoolproofBlockManager::new(config);
    
    manager.start_round().await;
    
    // Verify elapsed time tracking works
    assert_eq!(manager.elapsed_time_secs().await, 0);
    
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    assert!(manager.elapsed_time_secs().await >= 1);
}

#[tokio::test]
async fn test_strategy_properties() {
    // Verify strategy properties are correct
    
    // NormalBFT and LeaderRotation include mempool
    assert!(BlockCreationStrategy::NormalBFT.includes_mempool_txs());
    assert!(BlockCreationStrategy::LeaderRotation.includes_mempool_txs());
    assert!(BlockCreationStrategy::ReducedThreshold.includes_mempool_txs());
    
    // RewardOnly and Emergency do not
    assert!(!BlockCreationStrategy::RewardOnly.includes_mempool_txs());
    assert!(!BlockCreationStrategy::Emergency.includes_mempool_txs());
    
    // Verify timeouts decrease (except Emergency which is 0)
    assert!(BlockCreationStrategy::NormalBFT.timeout_secs() > 
            BlockCreationStrategy::LeaderRotation.timeout_secs());
    assert!(BlockCreationStrategy::LeaderRotation.timeout_secs() > 
            BlockCreationStrategy::ReducedThreshold.timeout_secs());
    assert_eq!(BlockCreationStrategy::Emergency.timeout_secs(), 0);
}

#[tokio::test]
async fn test_large_network() {
    // Test with a larger network (100 nodes)
    let config = FoolproofConfig::default();
    let manager = FoolproofBlockManager::new(config);
    
    manager.start_round().await;
    
    // Normal BFT: need 2/3+ of 100 = 67 votes
    assert!(manager.check_consensus_with_strategy(67, 100).await);
    assert!(!manager.check_consensus_with_strategy(66, 100).await);
    
    // Advance to ReducedThreshold (1/2+)
    manager.advance_strategy().await;
    manager.advance_strategy().await;
    
    // Need 1/2+ of 100 = 51 votes
    assert!(manager.check_consensus_with_strategy(51, 100).await);
    assert!(!manager.check_consensus_with_strategy(50, 100).await);
}

#[tokio::test]
async fn test_minimum_network() {
    // Test with minimum BFT network (3 nodes)
    let config = FoolproofConfig::default();
    let manager = FoolproofBlockManager::new(config);
    
    manager.start_round().await;
    
    // Normal BFT: need 2/3+ of 3 = 2 votes
    assert!(manager.check_consensus_with_strategy(2, 3).await);
    assert!(!manager.check_consensus_with_strategy(1, 3).await);
    
    // Advance to ReducedThreshold (1/2+)
    manager.advance_strategy().await;
    manager.advance_strategy().await;
    
    // Need 1/2+ of 3 = 2 votes (still the same)
    assert!(manager.check_consensus_with_strategy(2, 3).await);
    assert!(!manager.check_consensus_with_strategy(1, 3).await);
}
