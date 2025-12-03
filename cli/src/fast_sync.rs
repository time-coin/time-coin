use std::collections::HashMap;
use std::sync::Arc;
use time_core::block::Block;
use time_core::state::BlockchainState;
use time_network::PeerManager;
use tokio::sync::RwLock;

/// Fast synchronization system with parallel downloads and quick rollbacks
pub struct FastSync {
    blockchain: Arc<RwLock<BlockchainState>>,
    peer_manager: Arc<PeerManager>,
}

impl FastSync {
    pub fn new(blockchain: Arc<RwLock<BlockchainState>>, peer_manager: Arc<PeerManager>) -> Self {
        Self {
            blockchain,
            peer_manager,
        }
    }

    /// Perform lightning-fast synchronization
    /// 1. Find common ancestor with network
    /// 2. Rollback to common point if needed
    /// 3. Parallel download missing blocks
    /// 4. Batch validate and import
    pub async fn lightning_sync(&self) -> Result<u64, String> {
        println!("\n⚡ LIGHTNING SYNC INITIATED");

        // Step 1: Query all peers for their chain tips
        let peer_chains = self.query_peer_chains().await?;

        if peer_chains.is_empty() {
            return Err("No peers available for sync".to_string());
        }

        println!("   📡 Found {} peers", peer_chains.len());

        // Step 2: Find network consensus chain
        let consensus_chain = self.find_consensus_chain(&peer_chains).await?;
        println!(
            "   🎯 Network consensus: height {} ({}...)",
            consensus_chain.height,
            &consensus_chain.tip_hash[..16]
        );

        // Step 3: Find common ancestor with our chain
        let common_ancestor = self.find_common_ancestor(&consensus_chain).await?;

        let our_height = {
            let blockchain = self.blockchain.read().await;
            blockchain.chain_tip_height()
        };

        println!("   📊 Our height: {}", our_height);
        println!("   📍 Common ancestor: height {}", common_ancestor.height);

        // Step 4: Rollback to common ancestor if needed
        if our_height > common_ancestor.height {
            let blocks_to_remove = our_height - common_ancestor.height;
            println!(
                "   🔄 FAST ROLLBACK: Removing {} incorrect blocks...",
                blocks_to_remove
            );

            let start = std::time::Instant::now();
            self.fast_rollback(common_ancestor.height).await?;

            println!("      ✅ Rolled back in {:?}", start.elapsed());
        }

        // Step 5: Calculate blocks needed
        let start_height = common_ancestor.height + 1;
        let blocks_needed = consensus_chain.height - common_ancestor.height;

        if blocks_needed == 0 {
            println!("   ✅ Chain is up to date!");
            return Ok(0);
        }

        println!("   📥 Downloading {} blocks in parallel...", blocks_needed);

        // Step 6: Parallel download all missing blocks
        let start = std::time::Instant::now();
        let blocks = self
            .parallel_download_blocks(&consensus_chain.peer, start_height, consensus_chain.height)
            .await?;

        println!(
            "      ⚡ Downloaded {} blocks in {:?}",
            blocks.len(),
            start.elapsed()
        );

        // Step 7: Batch validate and import
        let start = std::time::Instant::now();
        self.batch_import_blocks(blocks).await?;

        println!(
            "      ✅ Imported {} blocks in {:?}",
            blocks_needed,
            start.elapsed()
        );

        println!(
            "⚡ LIGHTNING SYNC COMPLETE - {} blocks synced",
            blocks_needed
        );
        Ok(blocks_needed)
    }

    /// Query all peers for their chain information
    async fn query_peer_chains(&self) -> Result<Vec<PeerChain>, String> {
        let peer_ips = self.peer_manager.get_peer_ips().await;

        println!("   🔍 Querying {} peers...", peer_ips.len());

        // Query peers with overall timeout to prevent hanging
        let timeout_duration = std::time::Duration::from_secs(30);

        let query_all = async {
            let mut chains = Vec::new();
            for peer_ip in peer_ips {
                if let Some((height, tip_hash)) = self.probe_peer_height(&peer_ip).await {
                    chains.push(PeerChain {
                        peer: peer_ip,
                        height,
                        tip_hash,
                    });
                }
            }
            chains
        };

        match tokio::time::timeout(timeout_duration, query_all).await {
            Ok(chains) => Ok(chains),
            Err(_) => {
                eprintln!("   ⚠️  Timeout querying peers");
                Ok(Vec::new()) // Return empty vec on timeout
            }
        }
    }

    /// Probe a peer's chain height by testing high heights
    async fn probe_peer_height(&self, peer_ip: &str) -> Option<(u64, String)> {
        // Use timeout to prevent hanging
        let timeout_duration = std::time::Duration::from_secs(10);

        match tokio::time::timeout(timeout_duration, self.probe_peer_height_inner(peer_ip)).await {
            Ok(result) => result,
            Err(_) => {
                eprintln!("   ⚠️  Timeout querying peer {}", peer_ip);
                None
            }
        }
    }

    async fn probe_peer_height_inner(&self, peer_ip: &str) -> Option<(u64, String)> {
        // Start with a reasonable upper bound (1 year of 10-minute blocks ~ 52560)
        let mut high = 100_000u64;
        let mut low = 0u64;
        let mut best_height = 0u64;
        let mut best_hash = String::new();

        // Binary search for the actual tip - but limit iterations to prevent hanging
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 20; // Log2(100k) ~ 17, so 20 is safe

        while low <= high && high - low > 1 && iterations < MAX_ITERATIONS {
            let mid = (low + high) / 2;
            iterations += 1;

            // Add timeout for each individual request
            let request_timeout = std::time::Duration::from_millis(500);
            match tokio::time::timeout(request_timeout, self.get_peer_block_hash(peer_ip, mid))
                .await
            {
                Ok(Some(block_hash)) => {
                    // Block exists at this height
                    best_height = mid;
                    best_hash = block_hash;
                    low = mid + 1; // Try higher
                }
                Ok(None) | Err(_) => {
                    // Block doesn't exist or timeout, try lower
                    high = mid - 1;
                }
            }
        }

        if !best_hash.is_empty() {
            Some((best_height, best_hash))
        } else {
            None
        }
    }

    /// Find network consensus chain (most peers on same chain)
    async fn find_consensus_chain(&self, chains: &[PeerChain]) -> Result<PeerChain, String> {
        if chains.is_empty() {
            return Err("No peer chains available".to_string());
        }

        // Group by height and hash
        let mut chain_counts: HashMap<(u64, String), Vec<String>> = HashMap::new();

        for chain in chains {
            chain_counts
                .entry((chain.height, chain.tip_hash.clone()))
                .or_default()
                .push(chain.peer.clone());
        }

        // Find the chain with most peers (consensus)
        chain_counts
            .into_iter()
            .max_by_key(|(_, peers)| peers.len())
            .map(|((height, tip_hash), peers)| PeerChain {
                peer: peers[0].clone(), // Use first peer with this chain
                height,
                tip_hash,
            })
            .ok_or_else(|| "Could not determine consensus chain".to_string())
    }

    /// Find common ancestor between our chain and network chain
    /// Uses binary search for speed
    async fn find_common_ancestor(
        &self,
        network_chain: &PeerChain,
    ) -> Result<CommonAncestor, String> {
        let our_height = {
            let blockchain = self.blockchain.read().await;
            blockchain.chain_tip_height()
        };

        // If we have no blocks, genesis is the common ancestor
        if our_height == 0 {
            return Ok(CommonAncestor {
                height: 0,
                hash: String::new(),
            });
        }

        // Binary search for common ancestor
        let mut low = 0;
        let mut high = our_height.min(network_chain.height);
        let mut common_height = 0;

        println!("   🔍 Binary search for common ancestor...");

        while low <= high {
            let mid = (low + high) / 2;

            // Get our hash at this height
            let our_hash = {
                let blockchain = self.blockchain.read().await;
                blockchain
                    .get_block_by_height(mid)
                    .map(|b| b.hash.clone())
                    .unwrap_or_default()
            };

            // Get peer's hash at this height
            let peer_hash = self
                .get_peer_block_hash(&network_chain.peer, mid)
                .await
                .unwrap_or_default();

            if our_hash == peer_hash && !our_hash.is_empty() {
                // Hashes match at this height
                common_height = mid;
                low = mid + 1; // Try higher
            } else {
                // Hashes differ, try lower
                if mid == 0 {
                    break;
                }
                high = mid - 1;
            }
        }

        let common_hash = {
            let blockchain = self.blockchain.read().await;
            blockchain
                .get_block_by_height(common_height)
                .map(|b| b.hash.clone())
                .unwrap_or_default()
        };

        Ok(CommonAncestor {
            height: common_height,
            hash: common_hash,
        })
    }

    /// Fast rollback to a specific height
    async fn fast_rollback(&self, target_height: u64) -> Result<(), String> {
        let mut blockchain = self.blockchain.write().await;

        blockchain
            .rollback_to_height(target_height)
            .map_err(|e| format!("Rollback failed: {:?}", e))?;

        // Update sync gate
        drop(blockchain);
        self.peer_manager
            .sync_gate
            .update_local_height(target_height)
            .await;

        Ok(())
    }

    /// Download blocks in parallel with batching
    async fn parallel_download_blocks(
        &self,
        peer: &str,
        start_height: u64,
        end_height: u64,
    ) -> Result<Vec<Block>, String> {
        const BATCH_SIZE: u64 = 50; // Download 50 blocks at a time

        let mut all_blocks = Vec::new();
        let total_blocks = end_height - start_height + 1;

        let mut current_height = start_height;

        while current_height <= end_height {
            let batch_end = (current_height + BATCH_SIZE - 1).min(end_height);
            let blocks_in_batch = batch_end - current_height + 1;

            // Create download tasks for this batch
            let mut tasks = Vec::new();

            for height in current_height..=batch_end {
                let peer_clone = peer.to_string();
                let peer_manager = self.peer_manager.clone();

                tasks.push(tokio::spawn(async move {
                    let p2p_port = match peer_manager.network {
                        time_network::discovery::NetworkType::Mainnet => 24000,
                        time_network::discovery::NetworkType::Testnet => 24100,
                    };
                    let peer_addr = format!("{}:{}", peer_clone, p2p_port);

                    (height, peer_manager.request_block_by_height(&peer_addr, height).await)
                }));
            }

            // Wait for all downloads in parallel
            let results = futures::future::join_all(tasks).await;

            let mut batch_blocks = Vec::new();
            let mut failed_heights = Vec::new();
            
            for result in results {
                match result {
                    Ok((height, Ok(block))) => batch_blocks.push(block),
                    Ok((height, Err(e))) => {
                        eprintln!("      ⚠️  Failed to download block {}: {}", height, e);
                        failed_heights.push(height);
                    }
                    Err(e) => {
                        eprintln!("      ⚠️  Task panicked: {}", e);
                    }
                }
            }
            
            // Retry failed downloads once
            if !failed_heights.is_empty() {
                eprintln!("      🔄 Retrying {} failed blocks...", failed_heights.len());
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                
                for height in failed_heights {
                    let p2p_port = match self.peer_manager.network {
                        time_network::discovery::NetworkType::Mainnet => 24000,
                        time_network::discovery::NetworkType::Testnet => 24100,
                    };
                    let peer_addr = format!("{}:{}", peer, p2p_port);
                    
                    if let Ok(block) = self.peer_manager
                        .request_block_by_height(&peer_addr, height)
                        .await
                    {
                        batch_blocks.push(block);
                    } else {
                        eprintln!("      ❌ Retry failed for block {}", height);
                    }
                }
            }

            // Sort blocks by height to maintain order
            batch_blocks.sort_by_key(|b| b.header.block_number);

            all_blocks.extend(batch_blocks);

            // Progress indicator
            let progress = ((current_height - start_height + blocks_in_batch) as f64
                / total_blocks as f64
                * 100.0) as u64;
            println!(
                "      📊 Progress: {}% ({}/{})",
                progress,
                all_blocks.len(),
                total_blocks
            );

            current_height = batch_end + 1;

            // Throttle to avoid overwhelming peer
            if current_height <= end_height {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }

        if all_blocks.len() as u64 != total_blocks {
            return Err(format!(
                "Failed to download all blocks: got {}, expected {}",
                all_blocks.len(),
                total_blocks
            ));
        }

        Ok(all_blocks)
    }

    /// Batch import blocks with validation
    async fn batch_import_blocks(&self, blocks: Vec<Block>) -> Result<(), String> {
        let mut blockchain = self.blockchain.write().await;

        for (idx, block) in blocks.iter().enumerate() {
            blockchain.add_block(block.clone()).map_err(|e| {
                format!(
                    "Failed to import block {}: {:?}",
                    block.header.block_number, e
                )
            })?;

            // Progress indicator every 10 blocks
            if (idx + 1) % 10 == 0 {
                let progress = ((idx + 1) as f64 / blocks.len() as f64 * 100.0) as u64;
                println!("      📊 Import progress: {}%", progress);
            }
        }

        // Save UTXO snapshot after bulk import
        if let Err(e) = blockchain.save_utxo_snapshot() {
            println!("      ⚠️  Warning: Failed to save UTXO snapshot: {}", e);
        }

        let final_height = blocks.last().map(|b| b.header.block_number).unwrap_or(0);
        drop(blockchain);

        // Update sync gate with final height
        self.peer_manager
            .sync_gate
            .update_local_height(final_height)
            .await;

        Ok(())
    }

    /// Get block hash from peer at specific height
    async fn get_peer_block_hash(&self, peer: &str, height: u64) -> Option<String> {
        let p2p_port = match self.peer_manager.network {
            time_network::discovery::NetworkType::Mainnet => 24000,
            time_network::discovery::NetworkType::Testnet => 24100,
        };
        let peer_addr = format!("{}:{}", peer, p2p_port);

        self.peer_manager
            .request_block_by_height(&peer_addr, height)
            .await
            .ok()
            .map(|block| block.hash)
    }
}

#[derive(Clone)]
struct PeerChain {
    peer: String,
    height: u64,
    tip_hash: String,
}

struct CommonAncestor {
    height: u64,
    #[allow(dead_code)]
    hash: String,
}
