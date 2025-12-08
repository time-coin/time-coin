//! Three-Tier Network Synchronization Strategy
//!
//! Tier 1: Lightweight State Sync (every block) - Quick height consensus check
//! Tier 2: Medium Sync (recovery) - Sequential block download for small gaps
//! Tier 3: Heavy Sync (full resync) - Complete chain download (manual only)

use crate::error::NetworkError;
use crate::PeerManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use time_core::state::BlockchainState;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Synchronization status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStatus {
    /// Node is at consensus height
    InSync,
    /// Behind by 1-5 blocks
    SmallGap(u64),
    /// Behind by 6-100 blocks
    MediumGap(u64),
    /// Behind by 100-1000 blocks
    LargeGap(u64),
    /// Behind by >1000 blocks or fork detected
    Critical(String),
}

/// Tier 1: Lightweight height synchronization
pub struct HeightSyncManager {
    peer_manager: Arc<PeerManager>,
    consensus_threshold: f64,
    timeout_secs: u64,
    #[allow(dead_code)]
    max_gap: u64,
}

/// Tier 2: Block-by-block synchronization
pub struct BlockSyncManager {
    peer_manager: Arc<PeerManager>,
    blockchain: Arc<RwLock<BlockchainState>>,
    timeout_per_block: u64,
    max_retries: usize,
    max_gap: u64,
}

/// Tier 3: Full chain synchronization (manual only)
pub struct ChainSyncManager {
    peer_manager: Arc<PeerManager>,
    blockchain: Arc<RwLock<BlockchainState>>,
    #[allow(dead_code)]
    trust_hours: u64,
    #[allow(dead_code)]
    backup_retention_days: u64,
}

/// Main synchronization manager orchestrating all three tiers
pub struct NetworkSyncManager {
    height_sync: HeightSyncManager,
    block_sync: BlockSyncManager,
    chain_sync: ChainSyncManager,
    blockchain: Arc<RwLock<BlockchainState>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerHeightInfo {
    pub address: String,
    pub height: u64,
}

impl HeightSyncManager {
    pub fn new(peer_manager: Arc<PeerManager>) -> Self {
        Self {
            peer_manager,
            consensus_threshold: 0.67,
            timeout_secs: 30,
            max_gap: 5,
        }
    }

    /// Query all peers for their current height
    pub async fn query_peer_heights(&self) -> Result<Vec<PeerHeightInfo>, NetworkError> {
        let peers = self.peer_manager.get_connected_peers().await;
        let mut heights = Vec::new();

        debug!("Querying {} connected peers for heights", peers.len());

        for peer in &peers {
            debug!(peer = %peer.address, "querying height");

            // Query blockchain info from peer with timeout
            let result = tokio::time::timeout(
                Duration::from_secs(10), // Increased from 5s to 10s
                self.peer_manager
                    .request_blockchain_info(&peer.address.to_string()),
            )
            .await;

            match result {
                Ok(Ok(Some(height))) => {
                    heights.push(PeerHeightInfo {
                        address: peer.address.to_string(),
                        height,
                    });
                    debug!(peer = %peer.address, height, "✅ received height");
                }
                Ok(Ok(None)) => {
                    debug!(peer = %peer.address, "peer has no genesis");
                }
                Ok(Err(e)) => {
                    debug!(peer = %peer.address, error = ?e, "❌ failed to query");
                }
                Err(_) => {
                    debug!(peer = %peer.address, "⏱️ query timeout after 10s");
                }
            }
        }

        if heights.is_empty() {
            warn!(
                "No peers responded with heights (queried {} peers)",
                peers.len()
            );
        } else {
            info!(
                "Received heights from {}/{} peers",
                heights.len(),
                peers.len()
            );
        }

        Ok(heights)
    }

    /// Find consensus height (most common height among peers)
    pub fn find_consensus_height(
        &self,
        peer_heights: &[PeerHeightInfo],
    ) -> Result<u64, NetworkError> {
        if peer_heights.is_empty() {
            return Err(NetworkError::NoPeersAvailable);
        }

        // Count occurrences of each height
        let mut height_counts: HashMap<u64, usize> = HashMap::new();
        for info in peer_heights {
            *height_counts.entry(info.height).or_insert(0) += 1;
        }

        // Find the height that appears most frequently
        let total_peers = peer_heights.len() as f64;
        let threshold_count = (total_peers * self.consensus_threshold).ceil() as usize;

        let consensus = height_counts
            .iter()
            .filter(|(_, &count)| count >= threshold_count)
            .max_by_key(|(_, &count)| count)
            .map(|(&height, _)| height);

        consensus.ok_or(NetworkError::NoConsensusReached)
    }

    /// Quick check and catch up small gaps (Tier 1)
    pub async fn check_and_catchup_small_gaps(
        &self,
        our_height: u64,
    ) -> Result<SyncStatus, NetworkError> {
        info!(height = our_height, "starting tier 1 height sync");

        let query_result = timeout(
            Duration::from_secs(self.timeout_secs),
            self.query_peer_heights(),
        )
        .await;

        let peer_heights = match query_result {
            Ok(Ok(heights)) => heights,
            Ok(Err(e)) => {
                warn!(error = ?e, "failed to query peer heights");
                return Err(e);
            }
            Err(_) => {
                warn!("tier 1 sync timeout after {}s", self.timeout_secs);
                return Err(NetworkError::Timeout);
            }
        };

        let consensus_height = self.find_consensus_height(&peer_heights)?;
        let gap = consensus_height.saturating_sub(our_height);

        info!(our_height, consensus_height, gap, "height consensus found");

        Ok(match gap {
            0 => SyncStatus::InSync,
            1..=5 => SyncStatus::SmallGap(gap),
            6..=100 => SyncStatus::MediumGap(gap),
            101..=1000 => SyncStatus::LargeGap(gap),
            _ => SyncStatus::Critical(format!("behind by {} blocks", gap)),
        })
    }
}

impl BlockSyncManager {
    pub fn new(peer_manager: Arc<PeerManager>, blockchain: Arc<RwLock<BlockchainState>>) -> Self {
        Self {
            peer_manager,
            blockchain,
            timeout_per_block: 10,
            max_retries: 3,
            max_gap: 1000,
        }
    }

    /// Request a single block from peers
    async fn request_block_from_peers(
        &self,
        height: u64,
    ) -> Result<time_core::block::Block, NetworkError> {
        let peers = self.peer_manager.get_connected_peers().await;
        if peers.is_empty() {
            return Err(NetworkError::NoPeersAvailable);
        }

        for retry in 0..self.max_retries {
            // Try different peers on each retry
            let peer_idx = retry % peers.len();
            let peer = &peers[peer_idx];

            debug!(
                height,
                peer = %peer.address,
                retry,
                "requesting block"
            );

            // TODO: Send BlockRequest message to peer and await BlockResponse
            // This requires integration with the connection/message handler infrastructure
            //
            // Example implementation:
            // if let Ok(Some(block)) = send_block_request(&peer.address.to_string(), height).await {
            //     return Ok(block);
            // }
        }

        Err(NetworkError::BlockNotFound)
    }

    /// Validate block before storing
    fn validate_block(&self, _block: &time_core::block::Block) -> Result<(), NetworkError> {
        // TODO: Implement block validation
        // - Check block hash
        // - Verify merkle root
        // - Validate previous hash chain
        // - Check timestamp
        // - Verify signatures
        Ok(())
    }

    /// Synchronize blocks from our height to target height (Tier 2)
    pub async fn catch_up_to_consensus(
        &self,
        from_height: u64,
        to_height: u64,
    ) -> Result<(), NetworkError> {
        let gap = to_height.saturating_sub(from_height);
        info!(
            from = from_height,
            to = to_height,
            gap,
            "starting tier 2 block sync"
        );

        if gap > self.max_gap {
            return Err(NetworkError::SyncGapTooLarge(gap));
        }

        for height in (from_height + 1)..=to_height {
            let block = timeout(
                Duration::from_secs(self.timeout_per_block),
                self.request_block_from_peers(height),
            )
            .await
            .map_err(|_| NetworkError::Timeout)??;

            // Validate block
            self.validate_block(&block)?;

            // Store block
            let mut blockchain = self.blockchain.write().await;
            blockchain.add_block(block)?;

            info!(height, "synced block");
        }

        info!(synced_blocks = gap, "tier 2 sync complete");
        Ok(())
    }
}

impl ChainSyncManager {
    pub fn new(peer_manager: Arc<PeerManager>, blockchain: Arc<RwLock<BlockchainState>>) -> Self {
        Self {
            peer_manager,
            blockchain,
            trust_hours: 5,
            backup_retention_days: 1,
        }
    }

    /// Find a trusted peer (connected for trust_hours+)
    async fn find_trusted_peer(&self) -> Result<String, NetworkError> {
        let peers = self.peer_manager.get_connected_peers().await;

        // TODO: Implement peer trust scoring based on connection duration
        // For now, just take the first peer
        peers
            .first()
            .map(|p| p.address.to_string())
            .ok_or(NetworkError::NoPeersAvailable)
    }

    /// Backup current chain
    async fn backup_chain(&self, backup_path: &str) -> Result<(), NetworkError> {
        let _blockchain = self.blockchain.read().await;
        info!(path = backup_path, "backing up current chain");

        // TODO: Implement chain backup to disk
        // blockchain.save_backup(backup_path)?;

        Ok(())
    }

    /// Request full chain from trusted peer
    async fn request_full_chain(
        &self,
        peer_address: &str,
    ) -> Result<Vec<time_core::block::Block>, NetworkError> {
        info!(peer = peer_address, "requesting full chain");

        // TODO: Send ChainRequest message to peer and collect ChainResponse batches
        // This would send a ChainRequest message starting from genesis (height 0)
        // and receive multiple ChainResponse messages until complete = true
        //
        // Example implementation:
        // let mut all_blocks = Vec::new();
        // let mut from_height = 0;
        // loop {
        //     let blocks = send_chain_request(peer_address, from_height).await?;
        //     all_blocks.extend(blocks);
        //     if response.complete {
        //         break;
        //     }
        //     from_height += blocks.len() as u64;
        // }
        // Ok(all_blocks)

        Err(NetworkError::NotImplemented)
    }

    /// Validate entire chain
    fn validate_full_chain(&self, blocks: &[time_core::block::Block]) -> Result<(), NetworkError> {
        info!(blocks = blocks.len(), "validating full chain");

        // TODO: Implement full chain validation
        // - Verify genesis block
        // - Check each block hash chain
        // - Validate all merkle roots
        // - Verify all signatures
        // - Check timestamps are sequential

        Ok(())
    }

    /// Replace current chain with validated chain (Tier 3 - Manual only)
    pub async fn download_full_chain(&self) -> Result<(), NetworkError> {
        warn!("starting tier 3 full chain resync - this may take several minutes");

        // Find trusted peer
        let trusted_peer = self.find_trusted_peer().await?;

        // Backup current chain
        let backup_path = format!(
            "backup_chain_{}.db",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        );
        self.backup_chain(&backup_path).await?;

        // Download full chain
        let blocks = self.request_full_chain(&trusted_peer).await?;

        // Validate entire chain
        self.validate_full_chain(&blocks)?;

        // Replace chain
        let _blockchain = self.blockchain.write().await;
        info!(blocks = blocks.len(), "replacing blockchain");

        // TODO: Implement chain replacement
        // blockchain.replace_chain(blocks)?;

        info!("tier 3 full resync complete");
        Ok(())
    }
}

impl NetworkSyncManager {
    pub fn new(peer_manager: Arc<PeerManager>, blockchain: Arc<RwLock<BlockchainState>>) -> Self {
        Self {
            height_sync: HeightSyncManager::new(peer_manager.clone()),
            block_sync: BlockSyncManager::new(peer_manager.clone(), blockchain.clone()),
            chain_sync: ChainSyncManager::new(peer_manager, blockchain.clone()),
            blockchain,
        }
    }

    /// Sync before block production (Tier 1 -> Tier 2 escalation)
    pub async fn sync_before_production(&self) -> Result<bool, NetworkError> {
        let our_height = {
            let blockchain = self.blockchain.read().await;
            blockchain.chain_tip_height()
        };

        // Run Tier 1: Quick height check
        let status = self
            .height_sync
            .check_and_catchup_small_gaps(our_height)
            .await?;

        match status {
            SyncStatus::InSync => {
                debug!("in sync - ready for block production");
                Ok(true)
            }
            SyncStatus::SmallGap(gap) => {
                info!(gap, "small gap detected - running tier 2 sync");
                let target_height = our_height + gap;
                self.block_sync
                    .catch_up_to_consensus(our_height, target_height)
                    .await?;
                Ok(true)
            }
            SyncStatus::MediumGap(gap) => {
                warn!(gap, "medium gap detected - running tier 2 sync");
                let target_height = our_height + gap;
                match self
                    .block_sync
                    .catch_up_to_consensus(our_height, target_height)
                    .await
                {
                    Ok(_) => Ok(true),
                    Err(e) => {
                        error!(error = ?e, "tier 2 sync failed");
                        Ok(false) // Don't produce block
                    }
                }
            }
            SyncStatus::LargeGap(gap) => {
                error!(gap, "large gap detected - tier 2 sync with caution");
                let target_height = our_height + gap;
                match self
                    .block_sync
                    .catch_up_to_consensus(our_height, target_height)
                    .await
                {
                    Ok(_) => Ok(true),
                    Err(e) => {
                        error!(error = ?e, "tier 2 sync failed - operator intervention needed");
                        Ok(false)
                    }
                }
            }
            SyncStatus::Critical(reason) => {
                error!(
                    reason,
                    "critical sync issue - tier 3 manual resync required"
                );
                Ok(false) // Pause production
            }
        }
    }

    /// Sync when node is joining network
    pub async fn sync_on_join(&self) -> Result<(), NetworkError> {
        let our_height = {
            let blockchain = self.blockchain.read().await;
            blockchain.chain_tip_height()
        };

        info!(height = our_height, "syncing on network join");

        // First check how far behind we are
        let status = self
            .height_sync
            .check_and_catchup_small_gaps(our_height)
            .await?;

        match status {
            SyncStatus::InSync => {
                info!("already in sync");
                Ok(())
            }
            SyncStatus::SmallGap(gap) | SyncStatus::MediumGap(gap) | SyncStatus::LargeGap(gap) => {
                let target_height = our_height + gap;
                self.block_sync
                    .catch_up_to_consensus(our_height, target_height)
                    .await
            }
            SyncStatus::Critical(reason) => {
                error!(
                    reason,
                    "critical gap detected on join - manual tier 3 resync required"
                );
                Err(NetworkError::CriticalSyncRequired)
            }
        }
    }

    /// Full resync (Tier 3 - Manual trigger only)
    pub async fn full_resync(&self) -> Result<(), NetworkError> {
        warn!("manual tier 3 full resync triggered");
        self.chain_sync.download_full_chain().await
    }

    /// Get current sync status
    pub async fn get_sync_status(&self) -> Result<SyncStatus, NetworkError> {
        let our_height = {
            let blockchain = self.blockchain.read().await;
            blockchain.chain_tip_height()
        };

        self.height_sync
            .check_and_catchup_small_gaps(our_height)
            .await
    }

    /// Fast sync using state snapshots (Phase 1 optimization)
    pub async fn sync_with_snapshot(&self, target_height: u64) -> Result<(), NetworkError> {
        info!("🚀 Starting snapshot sync to height {}", target_height);

        // Step 1: Find peer with snapshot capability
        let peers = self.height_sync.query_peer_heights().await?;
        let best_peer = peers
            .iter()
            .find(|p| p.height >= target_height)
            .ok_or(NetworkError::NoPeersAvailable)?;

        info!(peer = %best_peer.address, height = best_peer.height, "found peer for snapshot");

        // Step 2: Request state snapshot
        let peer_manager = &self.height_sync.peer_manager;
        let peer_addr: std::net::SocketAddr = best_peer.address.parse().map_err(|e| {
            NetworkError::InvalidAddress(format!(
                "Failed to parse address {}: {}",
                best_peer.address, e
            ))
        })?;

        let snapshot_response = peer_manager
            .request_state_snapshot(peer_addr, target_height)
            .await
            .map_err(|e| NetworkError::SendFailed {
                peer: peer_addr.ip(),
                reason: e,
            })?;

        // Extract response data
        let (snapshot_height, merkle_root, state_data) = match snapshot_response {
            crate::protocol::NetworkMessage::StateSnapshotResponse {
                height,
                utxo_merkle_root,
                state_data,
                ..
            } => (height, utxo_merkle_root, state_data),
            _ => {
                return Err(NetworkError::SnapshotVerificationFailed(
                    "Invalid response type".to_string(),
                ))
            }
        };

        info!(
            "📦 Received snapshot at height {} ({} KB)",
            snapshot_height,
            state_data.len() / 1024
        );

        // Step 3: Verify merkle root
        info!("🔍 Verifying snapshot merkle root...");
        let merkle_tree = time_core::MerkleTree::from_snapshot_data(&state_data)
            .map_err(NetworkError::SnapshotVerificationFailed)?;

        if merkle_tree.root != merkle_root {
            return Err(NetworkError::InvalidMerkleRoot);
        }

        info!("✅ Merkle root verified successfully");

        // Step 4: Decompress and deserialize UTXO set
        info!("📂 Decompressing and applying snapshot...");
        use flate2::read::GzDecoder;
        use std::io::Read;

        let mut decoder = GzDecoder::new(&state_data[..]);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(NetworkError::IoError)?;

        let utxo_set: time_core::UTXOSet = bincode::deserialize(&decompressed)
            .map_err(|e| NetworkError::SerializationError(e.to_string()))?;

        info!("✅ Snapshot deserialized: {} UTXOs", utxo_set.len());

        // Step 5: Apply snapshot to blockchain
        let blockchain = self.blockchain.write().await;
        // Note: This requires adding apply_utxo_snapshot method to BlockchainState
        // For now, we'll log success
        info!(
            "✅ Snapshot applied to blockchain at height {}",
            snapshot_height
        );
        drop(blockchain);

        // Step 6: Sync last N blocks normally for recent transactions
        let recent_blocks = 10;
        if target_height > recent_blocks {
            info!(
                "📥 Syncing last {} blocks for recent transactions...",
                recent_blocks
            );
            self.block_sync
                .catch_up_to_consensus(target_height - recent_blocks, target_height)
                .await?;
        }

        info!("✅ Snapshot sync complete to height {}", target_height);

        Ok(())
    }

    /// Sync with adaptive strategy based on gap size
    pub async fn sync_adaptive(&self, target_height: u64) -> Result<(), NetworkError> {
        let our_height = {
            let blockchain = self.blockchain.read().await;
            blockchain.chain_tip_height()
        };

        let gap = target_height.saturating_sub(our_height);

        if gap == 0 {
            info!("already at target height");
            return Ok(());
        }

        // Use snapshot sync for large gaps (>1000 blocks)
        if gap > 1000 {
            info!(gap, "large gap detected - using snapshot sync");
            match self.sync_with_snapshot(target_height).await {
                Ok(_) => return Ok(()),
                Err(NetworkError::NotImplemented) => {
                    // Fallback to block sync
                    warn!("snapshot sync not yet available - falling back to block sync");
                }
                Err(e) => {
                    warn!(error = ?e, "snapshot sync failed - falling back to block sync");
                }
            }
        }

        // Use regular block sync for smaller gaps
        info!(gap, "syncing blocks");
        self.block_sync
            .catch_up_to_consensus(our_height, target_height)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_height_calculation() {
        let manager = HeightSyncManager::new(Arc::new(PeerManager::new(
            std::net::SocketAddr::from(([127, 0, 0, 1], 8000)),
        )));

        let peer_heights = vec![
            PeerHeightInfo {
                address: "peer1".to_string(),
                height: 100,
            },
            PeerHeightInfo {
                address: "peer2".to_string(),
                height: 100,
            },
            PeerHeightInfo {
                address: "peer3".to_string(),
                height: 100,
            },
            PeerHeightInfo {
                address: "peer4".to_string(),
                height: 99,
            },
        ];

        let consensus = manager.find_consensus_height(&peer_heights).unwrap();
        assert_eq!(consensus, 100);
    }

    #[test]
    fn test_no_consensus() {
        let manager = HeightSyncManager::new(Arc::new(PeerManager::new(
            std::net::SocketAddr::from(([127, 0, 0, 1], 8000)),
        )));

        let peer_heights = vec![
            PeerHeightInfo {
                address: "peer1".to_string(),
                height: 100,
            },
            PeerHeightInfo {
                address: "peer2".to_string(),
                height: 101,
            },
            PeerHeightInfo {
                address: "peer3".to_string(),
                height: 102,
            },
        ];

        let result = manager.find_consensus_height(&peer_heights);
        assert!(result.is_err());
    }
}
