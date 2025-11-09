//! Heartbeat and Network Synchronization Protocol
//!
//! Implements Phase 1 of the phased protocol: midnight synchronization
//! - Network-wide heartbeat exchange at midnight UTC
//! - Chain state verification
//! - Masternode set agreement
//! - Preparation for leader election

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use time_core::MasternodeTier;
use tokio::sync::RwLock;

/// Heartbeat message exchanged between masternodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    /// Node identifier (IP or ID)
    pub node_id: String,
    
    /// Timestamp of heartbeat
    pub timestamp: i64,
    
    /// Current block height
    pub block_height: u64,
    
    /// Current chain tip hash
    pub chain_tip_hash: String,
    
    /// Masternode tier
    pub tier: MasternodeTier,
    
    /// Software version
    pub version: String,
    
    /// Reputation score
    pub reputation_score: f32,
    
    /// Days since registration
    pub days_active: u64,
}

/// Synchronization status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStatus {
    /// Waiting for heartbeats
    WaitingForHeartbeats,
    
    /// Collecting heartbeats
    Collecting,
    
    /// Synchronized - all nodes agree
    Synchronized,
    
    /// Desynchronized - chain state mismatch
    Desynchronized,
    
    /// Timeout - not enough heartbeats
    Timeout,
}

/// Chain state agreement
#[derive(Debug, Clone)]
pub struct ChainAgreement {
    /// Most common block height
    pub consensus_height: u64,
    
    /// Most common chain tip hash
    pub consensus_hash: String,
    
    /// Number of nodes in agreement
    pub agreement_count: usize,
    
    /// Total nodes
    pub total_nodes: usize,
    
    /// Percentage agreement
    pub agreement_percentage: f32,
}

/// Heartbeat synchronization manager
pub struct HeartbeatManager {
    /// Received heartbeats
    heartbeats: Arc<RwLock<HashMap<String, Heartbeat>>>,
    
    /// Synchronization status
    status: Arc<RwLock<SyncStatus>>,
    
    /// Sync start time
    sync_start: Arc<RwLock<Option<DateTime<Utc>>>>,
    
    /// Timeout duration in seconds
    timeout_secs: u64,
}

impl HeartbeatManager {
    /// Create new heartbeat manager with timeout
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            heartbeats: Arc::new(RwLock::new(HashMap::new())),
            status: Arc::new(RwLock::new(SyncStatus::WaitingForHeartbeats)),
            sync_start: Arc::new(RwLock::new(None)),
            timeout_secs,
        }
    }
    
    /// Start synchronization phase
    pub async fn start_sync(&self) {
        let mut status = self.status.write().await;
        *status = SyncStatus::Collecting;
        
        let mut start = self.sync_start.write().await;
        *start = Some(Utc::now());
        
        // Clear previous heartbeats
        self.heartbeats.write().await.clear();
        
        println!("💓 Phase 1: Heartbeat synchronization started");
    }
    
    /// Register a heartbeat from a masternode
    pub async fn register_heartbeat(&self, heartbeat: Heartbeat) -> Result<(), String> {
        let status = self.status.read().await;
        if *status != SyncStatus::Collecting {
            return Err("Not in collecting state".to_string());
        }
        drop(status);
        
        let mut heartbeats = self.heartbeats.write().await;
        heartbeats.insert(heartbeat.node_id.clone(), heartbeat);
        
        Ok(())
    }
    
    /// Get all received heartbeats
    pub async fn get_heartbeats(&self) -> Vec<Heartbeat> {
        self.heartbeats.read().await.values().cloned().collect()
    }
    
    /// Check if synchronization timeout has been reached
    pub async fn check_timeout(&self) -> bool {
        let start = self.sync_start.read().await;
        if let Some(start_time) = *start {
            let elapsed = (Utc::now() - start_time).num_seconds();
            elapsed >= self.timeout_secs as i64
        } else {
            false
        }
    }
    
    /// Check chain state agreement among masternodes
    pub async fn check_chain_agreement(&self) -> ChainAgreement {
        let heartbeats = self.heartbeats.read().await;
        
        if heartbeats.is_empty() {
            return ChainAgreement {
                consensus_height: 0,
                consensus_hash: String::new(),
                agreement_count: 0,
                total_nodes: 0,
                agreement_percentage: 0.0,
            };
        }
        
        // Count occurrences of each (height, hash) pair
        let mut state_counts: HashMap<(u64, String), Vec<String>> = HashMap::new();
        
        for heartbeat in heartbeats.values() {
            let key = (heartbeat.block_height, heartbeat.chain_tip_hash.clone());
            state_counts.entry(key).or_insert_with(Vec::new).push(heartbeat.node_id.clone());
        }
        
        // Find most common state
        let total_nodes = heartbeats.len();
        let (consensus_state, agreeing_nodes) = state_counts
            .into_iter()
            .max_by_key(|(_, nodes)| nodes.len())
            .unwrap();
        
        let agreement_count = agreeing_nodes.len();
        let agreement_percentage = (agreement_count as f32 / total_nodes as f32) * 100.0;
        
        ChainAgreement {
            consensus_height: consensus_state.0,
            consensus_hash: consensus_state.1,
            agreement_count,
            total_nodes,
            agreement_percentage,
        }
    }
    
    /// Check if synchronization is complete
    /// Returns true if:
    /// - At least 2/3 of expected nodes responded
    /// - At least 2/3 of responding nodes agree on chain state
    pub async fn check_sync_complete(&self, expected_nodes: usize) -> bool {
        let heartbeats = self.heartbeats.read().await;
        let received_count = heartbeats.len();
        drop(heartbeats);
        
        // Need at least 2/3 of expected nodes
        let response_threshold = (expected_nodes * 2).div_ceil(3);
        if received_count < response_threshold {
            return false;
        }
        
        // Check chain agreement
        let agreement = self.check_chain_agreement().await;
        
        // Need at least 2/3 agreement on chain state
        let agreement_threshold = (agreement.total_nodes * 2).div_ceil(3);
        agreement.agreement_count >= agreement_threshold
    }
    
    /// Finalize synchronization and update status
    pub async fn finalize_sync(&self, expected_nodes: usize) -> SyncStatus {
        // Check if timeout occurred
        if self.check_timeout().await {
            let mut status = self.status.write().await;
            *status = SyncStatus::Timeout;
            println!("⏱️  Phase 1: Heartbeat timeout");
            return SyncStatus::Timeout;
        }
        
        // Check if synchronized
        if self.check_sync_complete(expected_nodes).await {
            let agreement = self.check_chain_agreement().await;
            let mut status = self.status.write().await;
            *status = SyncStatus::Synchronized;
            
            println!("✅ Phase 1: Network synchronized");
            println!("   - Height: {}", agreement.consensus_height);
            println!("   - Hash: {}...", &agreement.consensus_hash[..16]);
            println!("   - Agreement: {}/{} nodes ({:.1}%)",
                agreement.agreement_count,
                agreement.total_nodes,
                agreement.agreement_percentage
            );
            
            SyncStatus::Synchronized
        } else {
            let mut status = self.status.write().await;
            *status = SyncStatus::Desynchronized;
            println!("⚠️  Phase 1: Network desynchronized");
            SyncStatus::Desynchronized
        }
    }
    
    /// Get current synchronization status
    pub async fn get_status(&self) -> SyncStatus {
        self.status.read().await.clone()
    }
    
    /// Reset for next synchronization round
    pub async fn reset(&self) {
        let mut status = self.status.write().await;
        *status = SyncStatus::WaitingForHeartbeats;
        
        self.heartbeats.write().await.clear();
        self.sync_start.write().await.take();
    }
    
    /// Get list of synchronized nodes (those in agreement)
    pub async fn get_synchronized_nodes(&self) -> Vec<String> {
        let heartbeats = self.heartbeats.read().await;
        let agreement = self.check_chain_agreement().await;
        
        heartbeats
            .values()
            .filter(|hb| {
                hb.block_height == agreement.consensus_height
                    && hb.chain_tip_hash == agreement.consensus_hash
            })
            .map(|hb| hb.node_id.clone())
            .collect()
    }
    
    /// Get list of desynchronized nodes
    pub async fn get_desynchronized_nodes(&self) -> Vec<(String, u64, String)> {
        let heartbeats = self.heartbeats.read().await;
        let agreement = self.check_chain_agreement().await;
        
        heartbeats
            .values()
            .filter(|hb| {
                hb.block_height != agreement.consensus_height
                    || hb.chain_tip_hash != agreement.consensus_hash
            })
            .map(|hb| (hb.node_id.clone(), hb.block_height, hb.chain_tip_hash.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_heartbeat(node_id: &str, height: u64, hash: &str) -> Heartbeat {
        Heartbeat {
            node_id: node_id.to_string(),
            timestamp: Utc::now().timestamp(),
            block_height: height,
            chain_tip_hash: hash.to_string(),
            tier: MasternodeTier::Gold,
            version: "1.0.0".to_string(),
            reputation_score: 1.0,
            days_active: 30,
        }
    }
    
    #[tokio::test]
    async fn test_heartbeat_registration() {
        let manager = HeartbeatManager::new(30);
        manager.start_sync().await;
        
        let hb1 = create_heartbeat("node1", 100, "hash1");
        let hb2 = create_heartbeat("node2", 100, "hash1");
        
        assert!(manager.register_heartbeat(hb1).await.is_ok());
        assert!(manager.register_heartbeat(hb2).await.is_ok());
        
        let heartbeats = manager.get_heartbeats().await;
        assert_eq!(heartbeats.len(), 2);
    }
    
    #[tokio::test]
    async fn test_chain_agreement_unanimous() {
        let manager = HeartbeatManager::new(30);
        manager.start_sync().await;
        
        // All nodes agree on same state
        manager.register_heartbeat(create_heartbeat("node1", 100, "hash_abc")).await.unwrap();
        manager.register_heartbeat(create_heartbeat("node2", 100, "hash_abc")).await.unwrap();
        manager.register_heartbeat(create_heartbeat("node3", 100, "hash_abc")).await.unwrap();
        
        let agreement = manager.check_chain_agreement().await;
        assert_eq!(agreement.consensus_height, 100);
        assert_eq!(agreement.consensus_hash, "hash_abc");
        assert_eq!(agreement.agreement_count, 3);
        assert_eq!(agreement.agreement_percentage, 100.0);
    }
    
    #[tokio::test]
    async fn test_chain_agreement_majority() {
        let manager = HeartbeatManager::new(30);
        manager.start_sync().await;
        
        // 3 nodes agree, 1 disagrees
        manager.register_heartbeat(create_heartbeat("node1", 100, "hash_abc")).await.unwrap();
        manager.register_heartbeat(create_heartbeat("node2", 100, "hash_abc")).await.unwrap();
        manager.register_heartbeat(create_heartbeat("node3", 100, "hash_abc")).await.unwrap();
        manager.register_heartbeat(create_heartbeat("node4", 99, "hash_xyz")).await.unwrap();
        
        let agreement = manager.check_chain_agreement().await;
        assert_eq!(agreement.consensus_height, 100);
        assert_eq!(agreement.consensus_hash, "hash_abc");
        assert_eq!(agreement.agreement_count, 3);
        assert_eq!(agreement.total_nodes, 4);
        assert_eq!(agreement.agreement_percentage, 75.0);
    }
    
    #[tokio::test]
    async fn test_sync_complete_with_threshold() {
        let manager = HeartbeatManager::new(30);
        manager.start_sync().await;
        
        // 3 out of 4 expected nodes, all agree
        manager.register_heartbeat(create_heartbeat("node1", 100, "hash_abc")).await.unwrap();
        manager.register_heartbeat(create_heartbeat("node2", 100, "hash_abc")).await.unwrap();
        manager.register_heartbeat(create_heartbeat("node3", 100, "hash_abc")).await.unwrap();
        
        assert!(manager.check_sync_complete(4).await); // 3/4 >= 2/3
    }
    
    #[tokio::test]
    async fn test_sync_incomplete_below_threshold() {
        let manager = HeartbeatManager::new(30);
        manager.start_sync().await;
        
        // Only 2 out of 4 expected nodes
        manager.register_heartbeat(create_heartbeat("node1", 100, "hash_abc")).await.unwrap();
        manager.register_heartbeat(create_heartbeat("node2", 100, "hash_abc")).await.unwrap();
        
        assert!(!manager.check_sync_complete(4).await); // 2/4 < 2/3
    }
    
    #[tokio::test]
    async fn test_get_synchronized_nodes() {
        let manager = HeartbeatManager::new(30);
        manager.start_sync().await;
        
        manager.register_heartbeat(create_heartbeat("node1", 100, "hash_abc")).await.unwrap();
        manager.register_heartbeat(create_heartbeat("node2", 100, "hash_abc")).await.unwrap();
        manager.register_heartbeat(create_heartbeat("node3", 99, "hash_xyz")).await.unwrap();
        
        let synced = manager.get_synchronized_nodes().await;
        assert_eq!(synced.len(), 2);
        assert!(synced.contains(&"node1".to_string()));
        assert!(synced.contains(&"node2".to_string()));
    }
    
    #[tokio::test]
    async fn test_get_desynchronized_nodes() {
        let manager = HeartbeatManager::new(30);
        manager.start_sync().await;
        
        manager.register_heartbeat(create_heartbeat("node1", 100, "hash_abc")).await.unwrap();
        manager.register_heartbeat(create_heartbeat("node2", 100, "hash_abc")).await.unwrap();
        manager.register_heartbeat(create_heartbeat("node3", 99, "hash_xyz")).await.unwrap();
        
        let desynced = manager.get_desynchronized_nodes().await;
        assert_eq!(desynced.len(), 1);
        assert_eq!(desynced[0].0, "node3");
        assert_eq!(desynced[0].1, 99);
    }
}
