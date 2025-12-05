//! Simplified Blockchain Synchronization
//!
//! Two sync methods:
//! 1. Batch Sync (fast) - Parallel download of all missing blocks
//! 2. Sequential Sync (fallback) - One block at a time for reliability

use std::sync::Arc;
use time_core::block::Block;
use time_core::state::BlockchainState;
use time_network::PeerManager;
use time_network::PeerQuarantine;
use tokio::sync::RwLock;

const BATCH_SIZE: u64 = 50; // Download 50 blocks at a time
const BLOCK_TIMEOUT_SECS: u64 = 5; // Timeout per block
const BATCH_TIMEOUT_SECS: u64 = 30; // Timeout per batch

pub struct SimpleSync {
    blockchain: Arc<RwLock<BlockchainState>>,
    peer_manager: Arc<PeerManager>,
    quarantine: Arc<PeerQuarantine>,
}

impl SimpleSync {
    pub fn new(
        blockchain: Arc<RwLock<BlockchainState>>,
        peer_manager: Arc<PeerManager>,
        quarantine: Arc<PeerQuarantine>,
    ) -> Self {
        Self {
            blockchain,
            peer_manager,
            quarantine,
        }
    }

    /// Main sync entry point - tries batch sync first, falls back to sequential
    pub async fn sync(&self) -> Result<u64, String> {
        println!("🔄 Starting blockchain sync...");

        let our_height = self.get_local_height().await;
        let (network_height, best_peer) = self.get_network_consensus().await?;

        if our_height >= network_height {
            println!("   ✓ Blockchain is up to date (height: {})", our_height);
            return Ok(0);
        }

        let blocks_behind = network_height - our_height;
        println!(
            "   📊 Local: {}, Network: {}, Behind: {} blocks",
            our_height, network_height, blocks_behind
        );

        // Try batch sync first
        println!("   ⚡ Attempting batch sync...");
        match self
            .batch_sync(&best_peer, our_height + 1, network_height)
            .await
        {
            Ok(count) => {
                println!("   ✅ Batch sync complete: {} blocks", count);
                Ok(count)
            }
            Err(e) => {
                println!("   ⚠️  Batch sync failed: {}", e);
                println!("   🔄 Falling back to sequential sync...");

                // Fall back to sequential sync
                self.sequential_sync(&best_peer, our_height + 1, network_height)
                    .await
            }
        }
    }

    /// Batch sync: Download all blocks in parallel batches
    async fn batch_sync(
        &self,
        peer: &str,
        start_height: u64,
        end_height: u64,
    ) -> Result<u64, String> {
        let mut current_height = start_height;
        let mut total_synced = 0;

        while current_height <= end_height {
            let batch_end = (current_height + BATCH_SIZE - 1).min(end_height);

            // Download batch in parallel
            let blocks = self
                .download_batch_parallel(peer, current_height, batch_end)
                .await?;

            // Import blocks sequentially (must be in order)
            for block in blocks {
                self.import_block(block).await?;
                total_synced += 1;
            }

            current_height = batch_end + 1;

            // Progress update
            let progress = ((current_height - start_height) as f64
                / (end_height - start_height + 1) as f64)
                * 100.0;
            println!(
                "      📊 Progress: {:.0}% ({}/{})",
                progress,
                total_synced,
                end_height - start_height + 1
            );
        }

        Ok(total_synced)
    }

    /// Sequential sync: Download and import one block at a time (most reliable)
    async fn sequential_sync(
        &self,
        peer: &str,
        start_height: u64,
        end_height: u64,
    ) -> Result<u64, String> {
        let mut synced = 0;

        for height in start_height..=end_height {
            // Download single block
            let block = self
                .download_block(peer, height)
                .await
                .map_err(|e| format!("Failed to download block {}: {}", height, e))?;

            // Import immediately
            self.import_block(block).await?;
            synced += 1;

            // Progress every 10 blocks
            if synced % 10 == 0 {
                let progress = (synced as f64 / (end_height - start_height + 1) as f64) * 100.0;
                println!(
                    "      📊 Progress: {:.0}% ({}/{})",
                    progress,
                    synced,
                    end_height - start_height + 1
                );
            }
        }

        println!("   ✅ Sequential sync complete: {} blocks", synced);
        Ok(synced)
    }

    /// Download a batch of blocks in parallel
    async fn download_batch_parallel(
        &self,
        peer: &str,
        start_height: u64,
        end_height: u64,
    ) -> Result<Vec<Block>, String> {
        let mut tasks = Vec::new();

        // Create download tasks
        for height in start_height..=end_height {
            let peer_clone = peer.to_string();
            let peer_manager = self.peer_manager.clone();

            tasks.push(tokio::spawn(async move {
                let p2p_port = match peer_manager.network {
                    time_network::discovery::NetworkType::Mainnet => 24000,
                    time_network::discovery::NetworkType::Testnet => 24100,
                };
                let peer_addr = format!("{}:{}", peer_clone, p2p_port);

                // Timeout per block
                let result = tokio::time::timeout(
                    tokio::time::Duration::from_secs(BLOCK_TIMEOUT_SECS),
                    peer_manager.request_block_by_height(&peer_addr, height),
                )
                .await;

                let block_result = match result {
                    Ok(Ok(block)) => Ok(block),
                    Ok(Err(e)) => Err(e),
                    Err(_) => Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        format!(
                            "Block {} download timeout after {}s",
                            height, BLOCK_TIMEOUT_SECS
                        ),
                    )) as Box<dyn std::error::Error + Send>),
                };

                (height, block_result)
            }));
        }

        // Wait for all downloads with batch timeout
        let results = match tokio::time::timeout(
            tokio::time::Duration::from_secs(BATCH_TIMEOUT_SECS),
            futures::future::join_all(tasks),
        )
        .await
        {
            Ok(results) => results,
            Err(_) => {
                return Err(format!(
                    "Batch download timed out after {}s",
                    BATCH_TIMEOUT_SECS
                ));
            }
        };

        // Collect blocks and handle failures
        let mut blocks = Vec::new();
        let mut failed_heights = Vec::new();

        for result in results {
            match result {
                Ok((height, Ok(block))) => blocks.push((height, block)),
                Ok((height, Err(e))) => {
                    eprintln!("      ⚠️  Failed to download block {}: {}", height, e);
                    failed_heights.push(height);
                }
                Err(e) => {
                    eprintln!("      ⚠️  Task panicked: {}", e);
                }
            }
        }

        // Retry failed blocks once
        if !failed_heights.is_empty() {
            eprintln!(
                "      🔄 Retrying {} failed blocks...",
                failed_heights.len()
            );
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            for height in failed_heights {
                match self.download_block(peer, height).await {
                    Ok(block) => blocks.push((height, block)),
                    Err(e) => {
                        return Err(format!("Retry failed for block {}: {}", height, e));
                    }
                }
            }
        }

        // Sort blocks by height to maintain order
        blocks.sort_by_key(|(height, _)| *height);

        // Verify we have all blocks in sequence
        for (i, (height, _)) in blocks.iter().enumerate() {
            let expected_height = start_height + i as u64;
            if *height != expected_height {
                return Err(format!(
                    "Missing block {} in sequence (got {})",
                    expected_height, height
                ));
            }
        }

        Ok(blocks.into_iter().map(|(_, block)| block).collect())
    }

    /// Download a single block from a peer
    async fn download_block(&self, peer: &str, height: u64) -> Result<Block, String> {
        let p2p_port = match self.peer_manager.network {
            time_network::discovery::NetworkType::Mainnet => 24000,
            time_network::discovery::NetworkType::Testnet => 24100,
        };
        let peer_addr = format!("{}:{}", peer, p2p_port);

        tokio::time::timeout(
            tokio::time::Duration::from_secs(BLOCK_TIMEOUT_SECS),
            self.peer_manager
                .request_block_by_height(&peer_addr, height),
        )
        .await
        .map_err(|_| format!("Timeout downloading block {}", height))?
        .map_err(|e| format!("Failed to download block {}: {}", height, e))
    }

    /// Import a block into the blockchain
    async fn import_block(&self, block: Block) -> Result<(), String> {
        let height = block.header.block_number;
        let mut blockchain = self.blockchain.write().await;

        blockchain
            .add_block(block)
            .map_err(|e| format!("Failed to import block {}: {:?}", height, e))?;

        drop(blockchain);

        // Update sync gate
        self.peer_manager
            .sync_gate
            .update_local_height(height)
            .await;

        Ok(())
    }

    /// Get local blockchain height
    async fn get_local_height(&self) -> u64 {
        let blockchain = self.blockchain.read().await;
        blockchain.chain_tip_height()
    }

    /// Get network consensus height and best peer to sync from
    async fn get_network_consensus(&self) -> Result<(u64, String), String> {
        let peers = self.peer_manager.get_peer_ips().await;
        if peers.is_empty() {
            return Err("No peers available".to_string());
        }

        let p2p_port = match self.peer_manager.network {
            time_network::discovery::NetworkType::Mainnet => 24000,
            time_network::discovery::NetworkType::Testnet => 24100,
        };

        // Query all peers for their height
        let mut peer_heights = Vec::new();

        for peer_ip in peers.iter() {
            let peer_addr = format!("{}:{}", peer_ip, p2p_port);

            match tokio::time::timeout(
                tokio::time::Duration::from_secs(3),
                self.peer_manager.request_blockchain_info(&peer_addr),
            )
            .await
            {
                Ok(Ok(Some(height))) => {
                    peer_heights.push((peer_ip.clone(), height));
                }
                _ => continue,
            }
        }

        if peer_heights.is_empty() {
            return Err("No peers responded with height".to_string());
        }

        // Sort by height (highest first)
        peer_heights.sort_by_key(|(_, h)| std::cmp::Reverse(*h));

        // Use highest height as network consensus
        let (best_peer, network_height) = peer_heights[0].clone();

        println!(
            "   📡 Network consensus: height {} from {}",
            network_height, best_peer
        );

        Ok((network_height, best_peer))
    }

    /// Detect and rollback forks before syncing
    pub async fn detect_and_resolve_forks(&self) -> Result<(), String> {
        println!("   🔍 Checking for forks...");

        let our_height = self.get_local_height().await;
        if our_height == 0 {
            println!("      ✓ At genesis - no forks possible");
            return Ok(());
        }

        let (network_height, best_peer) = self.get_network_consensus().await?;

        // Find common ancestor
        let mut common_height = our_height.min(network_height);

        while common_height > 0 {
            let our_hash = {
                let blockchain = self.blockchain.read().await;
                blockchain
                    .get_block_by_height(common_height)
                    .map(|b| b.hash.clone())
            };

            if let Some(our_hash_str) = our_hash {
                match self.download_block(&best_peer, common_height).await {
                    Ok(peer_block) => {
                        if peer_block.hash == our_hash_str {
                            // Found common ancestor
                            println!("      ✓ Common ancestor at height {}", common_height);

                            // If we need to rollback
                            if common_height < our_height {
                                let blocks_to_remove = our_height - common_height;
                                println!("      🔄 Rolling back {} blocks...", blocks_to_remove);

                                let mut blockchain = self.blockchain.write().await;
                                blockchain
                                    .rollback_to_height(common_height)
                                    .map_err(|e| format!("Rollback failed: {:?}", e))?;

                                drop(blockchain);

                                self.peer_manager
                                    .sync_gate
                                    .update_local_height(common_height)
                                    .await;

                                println!("      ✅ Rolled back to height {}", common_height);
                            }

                            return Ok(());
                        }
                    }
                    Err(_) => {
                        // Peer doesn't have this block, try lower
                        common_height -= 1;
                        continue;
                    }
                }
            }

            common_height -= 1;
        }

        // If we get here, no common ancestor found (except genesis)
        println!("      ⚠️  No common ancestor found - may need full resync");
        Ok(())
    }
}
