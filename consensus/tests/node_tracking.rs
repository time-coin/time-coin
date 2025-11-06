use time_consensus::block_consensus::BlockConsensusManager;

#[tokio::test]
async fn test_sync_with_connected_peers_removes_disconnected_nodes() {
    let manager = BlockConsensusManager::new();

    // Initially set 3 masternodes
    let initial_nodes = vec![
        "192.168.1.1".to_string(),
        "192.168.1.2".to_string(),
        "192.168.1.3".to_string(),
    ];
    manager.set_masternodes(initial_nodes.clone()).await;

    // Initialize health tracking for all nodes
    for node in &initial_nodes {
        manager.init_masternode_health(node.clone()).await;
    }

    // Verify initial count
    assert_eq!(manager.active_masternode_count().await, 3);

    // Now simulate that only 2 nodes are still connected
    let connected_nodes = vec!["192.168.1.1".to_string(), "192.168.1.2".to_string()];

    // Sync with connected peers
    manager.sync_with_connected_peers(connected_nodes).await;

    // Verify that active count is now 2 (disconnected node should be excluded)
    assert_eq!(manager.active_masternode_count().await, 2);
}

#[tokio::test]
async fn test_sync_with_connected_peers_marks_disconnected_as_offline() {
    let manager = BlockConsensusManager::new();

    // Set up initial masternodes
    let initial_nodes = vec![
        "10.0.0.1".to_string(),
        "10.0.0.2".to_string(),
        "10.0.0.3".to_string(),
    ];
    manager.set_masternodes(initial_nodes.clone()).await;

    // Initialize health tracking
    for node in &initial_nodes {
        manager.init_masternode_health(node.clone()).await;
    }

    // Node 3 disconnects
    let connected_nodes = vec!["10.0.0.1".to_string(), "10.0.0.2".to_string()];

    manager.sync_with_connected_peers(connected_nodes).await;

    // Verify count reflects only connected nodes
    assert_eq!(manager.active_masternode_count().await, 2);
}

#[tokio::test]
async fn test_sync_with_connected_peers_handles_reconnection() {
    let manager = BlockConsensusManager::new();

    // Start with 3 nodes
    let initial_nodes = vec![
        "172.16.0.1".to_string(),
        "172.16.0.2".to_string(),
        "172.16.0.3".to_string(),
    ];
    manager.set_masternodes(initial_nodes.clone()).await;

    for node in &initial_nodes {
        manager.init_masternode_health(node.clone()).await;
    }

    assert_eq!(manager.active_masternode_count().await, 3);

    // Node 3 disconnects
    let connected_without_3 = vec!["172.16.0.1".to_string(), "172.16.0.2".to_string()];
    manager.sync_with_connected_peers(connected_without_3).await;
    assert_eq!(manager.active_masternode_count().await, 2);

    // Node 3 reconnects
    let all_connected = vec![
        "172.16.0.1".to_string(),
        "172.16.0.2".to_string(),
        "172.16.0.3".to_string(),
    ];
    manager.sync_with_connected_peers(all_connected).await;

    // Count should be back to 3
    assert_eq!(manager.active_masternode_count().await, 3);
}

#[tokio::test]
async fn test_sync_with_connected_peers_immediate_reflection() {
    let manager = BlockConsensusManager::new();

    // Start with 5 nodes
    let nodes = vec![
        "192.168.10.1".to_string(),
        "192.168.10.2".to_string(),
        "192.168.10.3".to_string(),
        "192.168.10.4".to_string(),
        "192.168.10.5".to_string(),
    ];
    manager.set_masternodes(nodes.clone()).await;

    for node in &nodes {
        manager.init_masternode_health(node.clone()).await;
    }

    assert_eq!(manager.active_masternode_count().await, 5);

    // Immediately after sync with only 3 connected, count should reflect this
    let connected = vec![
        "192.168.10.1".to_string(),
        "192.168.10.2".to_string(),
        "192.168.10.3".to_string(),
    ];
    manager.sync_with_connected_peers(connected).await;

    // The very next call to active_masternode_count should show 3, not 5
    assert_eq!(manager.active_masternode_count().await, 3);
}

#[tokio::test]
async fn test_sync_with_empty_peer_list() {
    let manager = BlockConsensusManager::new();

    // Start with some nodes
    let initial_nodes = vec!["10.1.1.1".to_string(), "10.1.1.2".to_string()];
    manager.set_masternodes(initial_nodes.clone()).await;

    for node in &initial_nodes {
        manager.init_masternode_health(node.clone()).await;
    }

    assert_eq!(manager.active_masternode_count().await, 2);

    // All nodes disconnect
    manager.sync_with_connected_peers(vec![]).await;

    // Count should be 0
    assert_eq!(manager.active_masternode_count().await, 0);
}
