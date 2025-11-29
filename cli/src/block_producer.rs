use crate::bft_consensus::BftConsensus;
use crate::deterministic_consensus::{ConsensusResult, DeterministicConsensus};
use chrono::{TimeZone, Utc};
use crossterm::style::Stylize;
use owo_colors::OwoColorize;
use std::sync::Arc;
use std::time::Duration;
use time_consensus::block_consensus::BlockVote;
use time_consensus::ConsensusEngine;
use time_core::block::{Block, BlockHeader};
use time_core::state::BlockchainState;
use time_core::transaction::{Transaction, TxOutput};
use time_core::MasternodeTier;
use time_network::PeerManager;
use tokio::sync::RwLock;

// ============================================================================
// TESTING CONFIGURATION
// ============================================================================
// Change this to adjust block production interval for testing
// Production: 86400 seconds (24 hours)
// Testing: 600 seconds (10 minutes)
const BLOCK_INTERVAL_SECONDS: u64 = 600; // 10 minutes for testing
const IS_TESTING_MODE: bool = true;

/// Helper to safely get hash/string preview
fn truncate_str(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}

pub struct BlockProducer {
    #[allow(dead_code)]
    node_id: String,
    peer_manager: Arc<PeerManager>,
    consensus: Arc<ConsensusEngine>,
    blockchain: Arc<RwLock<BlockchainState>>,
    mempool: Arc<time_mempool::Mempool>,
    block_consensus: Arc<time_consensus::block_consensus::BlockConsensusManager>,
    #[allow(dead_code)]
    tx_consensus: Arc<time_consensus::tx_consensus::TxConsensusManager>,
    is_active: Arc<RwLock<bool>>,
    #[allow(dead_code)]
    allow_block_recreation: bool,
    quarantine: Arc<time_network::PeerQuarantine>,
    bft: Arc<BftConsensus>,
    deterministic: Arc<DeterministicConsensus>,
}

impl BlockProducer {
    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        node_id: String,
        peer_manager: Arc<PeerManager>,
        consensus: Arc<ConsensusEngine>,
        blockchain: Arc<RwLock<BlockchainState>>,
        mempool: Arc<time_mempool::Mempool>,
        block_consensus: Arc<time_consensus::block_consensus::BlockConsensusManager>,
        #[allow(dead_code)] tx_consensus: Arc<time_consensus::tx_consensus::TxConsensusManager>,
        allow_block_recreation: bool,
        quarantine: Arc<time_network::PeerQuarantine>,
    ) -> Self {
        let bft = Arc::new(BftConsensus::new(
            node_id.clone(),
            peer_manager.clone(),
            block_consensus.clone(),
            blockchain.clone(),
        ));

        let deterministic = Arc::new(DeterministicConsensus::new(
            node_id.clone(),
            peer_manager.clone(),
            blockchain.clone(),
        ));

        BlockProducer {
            node_id,
            peer_manager,
            consensus,
            blockchain,
            mempool,
            block_consensus,
            tx_consensus,
            is_active: Arc::new(RwLock::new(false)),
            allow_block_recreation,
            quarantine,
            bft,
            deterministic,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_shared_state(
        node_id: String,
        peer_manager: Arc<PeerManager>,
        consensus: Arc<ConsensusEngine>,
        blockchain: Arc<RwLock<BlockchainState>>,
        mempool: Arc<time_mempool::Mempool>,
        block_consensus: Arc<time_consensus::block_consensus::BlockConsensusManager>,
        tx_consensus: Arc<time_consensus::tx_consensus::TxConsensusManager>,
        is_active: Arc<RwLock<bool>>,
        allow_block_recreation: bool,
        quarantine: Arc<time_network::PeerQuarantine>,
    ) -> Self {
        let bft = Arc::new(BftConsensus::new(
            node_id.clone(),
            peer_manager.clone(),
            block_consensus.clone(),
            blockchain.clone(),
        ));

        let deterministic = Arc::new(DeterministicConsensus::new(
            node_id.clone(),
            peer_manager.clone(),
            blockchain.clone(),
        ));

        BlockProducer {
            node_id,
            peer_manager,
            consensus,
            blockchain,
            mempool,
            block_consensus,
            tx_consensus,
            is_active,
            allow_block_recreation,
            quarantine,
            bft,
            deterministic,
        }
    }

    #[allow(dead_code)]
    pub fn get_active_state(&self) -> Arc<RwLock<bool>> {
        self.is_active.clone()
    }

    #[allow(dead_code)]
    pub async fn force_create_block(&self) {
        println!("🔧 MANUAL BLOCK CREATION TRIGGERED");
        *self.is_active.write().await = true;
        self.create_and_propose_block().await;
        *self.is_active.write().await = false;
    }

    async fn load_block_height(&self) -> u64 {
        self.blockchain.read().await.chain_tip_height()
    }

    fn get_node_id(&self) -> String {
        std::env::var("NODE_PUBLIC_IP").unwrap_or_else(|_| {
            local_ip_address::local_ip()
                .map(|ip| ip.to_string())
                .unwrap_or_else(|_| "unknown".to_string())
        })
    }

    pub async fn start(&self) {
        println!("Starting block producer...");

        // Run initial catch-up check if enabled
        if self.allow_block_recreation {
            self.catch_up_missed_blocks().await;
        }

        if IS_TESTING_MODE {
            println!(
                "⚠️  TESTING MODE: Block interval = {} seconds ({} minutes)",
                BLOCK_INTERVAL_SECONDS,
                BLOCK_INTERVAL_SECONDS / 60
            );
        } else {
            println!("Block producer started (24-hour interval)");
        }

        if self.allow_block_recreation {
            self.spawn_periodic_catchup_task();
        }

        // Main loop: sleep until next interval, then produce block
        loop {
            let now = Utc::now();

            // Calculate next block time based on mode
            let next_block_time = if IS_TESTING_MODE {
                // Testing: round up to next 10-minute interval
                let current_seconds = now.timestamp();
                let next_interval = ((current_seconds / BLOCK_INTERVAL_SECONDS as i64) + 1)
                    * BLOCK_INTERVAL_SECONDS as i64;
                Utc.timestamp_opt(next_interval, 0).unwrap()
            } else {
                // Production: next midnight UTC
                let tomorrow = now.date_naive() + chrono::Duration::days(1);
                tomorrow.and_hms_opt(0, 0, 0).unwrap().and_utc()
            };

            let duration_until_next = (next_block_time - now)
                .to_std()
                .unwrap_or(Duration::from_secs(60));

            let hours = duration_until_next.as_secs() / 3600;
            let minutes = (duration_until_next.as_secs() % 3600) / 60;
            let seconds = duration_until_next.as_secs() % 60;

            println!(
                "Next block at {} UTC (in {}h {}m {}s)",
                next_block_time.format("%Y-%m-%d %H:%M:%S"),
                hours,
                minutes,
                seconds
            );

            // Sleep until next block time
            tokio::time::sleep(duration_until_next).await;

            if IS_TESTING_MODE {
                println!("⏰ Block interval reached - producing block...");
            } else {
                println!("Midnight reached - producing block...");
            }

            *self.is_active.write().await = true;
            self.create_and_propose_block().await;
            *self.is_active.write().await = false;
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    fn spawn_periodic_catchup_task(&self) {
        let self_clone = Self {
            node_id: self.node_id.clone(),
            peer_manager: self.peer_manager.clone(),
            consensus: self.consensus.clone(),
            blockchain: self.blockchain.clone(),
            mempool: self.mempool.clone(),
            block_consensus: self.block_consensus.clone(),
            tx_consensus: self.tx_consensus.clone(),
            is_active: self.is_active.clone(),
            allow_block_recreation: self.allow_block_recreation,
            quarantine: self.quarantine.clone(),
            bft: self.bft.clone(),
            deterministic: self.deterministic.clone(),
        };

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300));
            interval.tick().await;
            loop {
                interval.tick().await;
                if !*self_clone.is_active.read().await {
                    println!("🔍 Periodic catch-up check...");
                    self_clone.catch_up_missed_blocks().await;
                }
            }
        });
    }

    #[allow(dead_code)]
    async fn catch_up_missed_blocks(&self) {
        let now = Utc::now();

        // Get genesis date from blockchain state
        let blockchain = self.blockchain.read().await;
        let genesis_block = match blockchain.get_block_by_height(0) {
            Some(block) => block.clone(),
            None => {
                // No genesis block yet - node is still syncing
                println!("⏳ Waiting for genesis block to be downloaded...");
                drop(blockchain);
                return; // Exit catch-up, will retry on next cycle
            }
        };
        drop(blockchain);

        let _genesis_timestamp = genesis_block.header.timestamp.timestamp();
        let _genesis_date = genesis_block.header.timestamp.date_naive();

        let actual_height = self.load_block_height().await;

        // CRITICAL FIX: Use network consensus height, not time-based calculation
        // Time-based fails when network is offline (testnet scenario)
        let network_max_height = {
            let peer_heights = self.peer_manager.get_peer_ips().await;
            let mut max_height = actual_height;

            for peer_ip in peer_heights {
                if let Ok(Some(height)) = self.peer_manager.request_blockchain_info(&peer_ip).await
                {
                    max_height = max_height.max(height);
                }
            }
            max_height
        };

        // Use network consensus as expected height
        let expected_height = network_max_height;

        println!("🔍 Catch-up check:");
        println!("   Current height: {}", actual_height);
        println!("   Network consensus height: {}", expected_height);

        if actual_height >= expected_height {
            println!("   ✅ Node is synced with network");
            return;
        }

        let missing_blocks = expected_height - actual_height;
        println!("⚠️  MISSED BLOCKS DETECTED");
        println!("   Missing {} block(s)", missing_blocks);

        // Broadcast catch-up request to coordinate with other nodes
        println!("   📢 Broadcasting catch-up request to network...");
        self.broadcast_catch_up_request(actual_height, expected_height)
            .await;

        // Wait a moment for other nodes to acknowledge and prepare
        tokio::time::sleep(Duration::from_secs(2)).await;

        // If we don't have genesis, trigger a sync which will download it
        if actual_height == 0 {
            let has_genesis = {
                let blockchain = self.blockchain.read().await;
                !blockchain.genesis_hash().is_empty()
            };

            if !has_genesis {
                println!("   📥 No genesis block - triggering sync to download...");

                // Create a chain sync instance and trigger sync
                let sync = crate::chain_sync::ChainSync::new(
                    self.blockchain.clone(),
                    self.peer_manager.clone(),
                );

                // Attempt to sync genesis
                match sync.sync_from_peers().await {
                    Ok(_) => {
                        println!("   ✅ Genesis sync completed!");
                    }
                    Err(e) => {
                        println!("   ⚠️  Genesis sync failed: {}", e);
                    }
                }
            }
        }

        // CRITICAL: Check consensus mode FIRST - NEVER create blocks in BOOTSTRAP mode
        let consensus_mode = self.consensus.consensus_mode().await;
        if consensus_mode != time_consensus::ConsensusMode::BFT {
            println!("   ⚠️  Cannot create catch-up blocks in BOOTSTRAP mode");
            println!("   ℹ️  Chain sync will download blocks from peers");
            println!("   ▶️ Waiting for BFT mode (need 3+ masternodes)...");

            // Wait and periodically re-check if BFT mode is reached
            for _ in 0..12 {
                // Check every 5 seconds for 1 minute
                tokio::time::sleep(Duration::from_secs(5)).await;
                let mode = self.consensus.consensus_mode().await;
                if mode == time_consensus::ConsensusMode::BFT {
                    println!("   ✅ BFT mode activated! Proceeding with catch-up...");
                    break;
                }
            }

            // Final check - if still not BFT, give up for now
            let final_mode = self.consensus.consensus_mode().await;
            if final_mode != time_consensus::ConsensusMode::BFT {
                println!("   ⚠️  Still in BOOTSTRAP mode after waiting - will retry next cycle");
                return;
            }
        }

        // Check if we have enough masternodes
        let masternode_count = self.consensus.masternode_count().await;
        if masternode_count < 3 {
            println!(
                "   ⚠️  Cannot create catch-up blocks: Only {} masternodes",
                masternode_count
            );
            println!("   ▶️ Need at least 3 masternodes for catch-up");
            return;
        }

        // CRITICAL: Try to download from peers first - retry multiple times
        println!("   🔍 Attempting to download blocks from peers...");

        let max_sync_attempts = 3;
        for attempt in 1..=max_sync_attempts {
            let current_height = self.blockchain.read().await.chain_tip_height();

            if current_height >= expected_height {
                println!("      ✔ Fully synced to height {}!", current_height);
                return;
            }

            println!("      📥 Sync attempt {}/{}", attempt, max_sync_attempts);

            let peers = self.peer_manager.get_peer_ips().await;
            if peers.is_empty() {
                println!("      ⚠️  No peers available for sync");
                break;
            }

            let mut synced_any = false;

            // If we're at genesis (height 0), try to download the entire chain from one valid peer
            if current_height == 0 {
                println!("      🔄 At genesis - attempting full chain sync from longest chain...");

                // Find peers with the longest valid chain
                let mut peer_heights: Vec<(String, u64)> = Vec::new();
                for peer_ip in &peers {
                    // Get network-aware port
                    let p2p_port = match self.peer_manager.network {
                        time_network::discovery::NetworkType::Mainnet => 24000,
                        time_network::discovery::NetworkType::Testnet => 24100,
                    };
                    let peer_addr = format!("{}:{}", peer_ip, p2p_port);

                    if let Ok(Some(height)) =
                        self.peer_manager.request_blockchain_info(&peer_addr).await
                    {
                        peer_heights.push((peer_ip.clone(), height));
                    }
                }

                // Sort by height descending
                peer_heights.sort_by(|a, b| b.1.cmp(&a.1));

                // Try downloading complete chain from longest chains first
                for (peer_ip, peer_height) in peer_heights {
                    println!(
                        "      🔗 Trying full chain download from {} (height {})...",
                        peer_ip, peer_height
                    );

                    let sync_to_height = std::cmp::min(peer_height, expected_height);

                    // Get network-aware port
                    let p2p_port = match self.peer_manager.network {
                        time_network::discovery::NetworkType::Mainnet => 24000,
                        time_network::discovery::NetworkType::Testnet => 24100,
                    };
                    let peer_addr = format!("{}:{}", peer_ip, p2p_port);

                    // Download blocks from genesis to sync_to_height
                    let mut downloaded_count = 0;
                    for height in 0..=sync_to_height {
                        match self
                            .peer_manager
                            .request_block_by_height(&peer_addr, height)
                            .await
                        {
                            Ok(block) => {
                                // Validate and add block
                                let mut blockchain = self.blockchain.write().await;
                                match blockchain.add_block(block.clone()) {
                                    Ok(_) => {
                                        downloaded_count += 1;
                                        if downloaded_count % 10 == 0 || height == sync_to_height {
                                            println!(
                                                "      ✓ Downloaded {} blocks (current: {})",
                                                downloaded_count, height
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        println!("      ⚠️  Failed to add block {}: {}", height, e);
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                println!("      ⚠️  Failed to download block {}: {}", height, e);
                                break;
                            }
                        }
                    }

                    if downloaded_count > 0 {
                        println!(
                            "      ✅ Successfully downloaded {} blocks from {}",
                            downloaded_count, peer_ip
                        );
                        synced_any = true;
                        break; // Successfully synced, no need to try other peers
                    }

                    // If download failed, continue to next peer
                }
            } else {
                // Not at genesis, use incremental sync
                for peer_ip in &peers {
                    // Get network-aware port
                    let p2p_port = match self.peer_manager.network {
                        time_network::discovery::NetworkType::Mainnet => 24000,
                        time_network::discovery::NetworkType::Testnet => 24100,
                    };
                    let peer_addr = format!("{}:{}", peer_ip, p2p_port);

                    if let Ok(Some(peer_height)) =
                        self.peer_manager.request_blockchain_info(&peer_addr).await
                    {
                        // If peer has blocks we need, try to sync
                        if peer_height > current_height {
                            let sync_to_height = std::cmp::min(peer_height, expected_height);

                            println!(
                                "      🔗 Peer {} has height {}, downloading blocks {}-{}...",
                                peer_ip,
                                peer_height,
                                current_height + 1,
                                sync_to_height
                            );

                            // Download missing blocks
                            let mut downloaded_count = 0;
                            for height in (current_height + 1)..=sync_to_height {
                                match self
                                    .peer_manager
                                    .request_block_by_height(&peer_addr, height)
                                    .await
                                {
                                    Ok(block) => {
                                        // Validate and add block
                                        let mut blockchain = self.blockchain.write().await;
                                        match blockchain.add_block(block.clone()) {
                                            Ok(_) => {
                                                downloaded_count += 1;
                                                if downloaded_count % 10 == 0
                                                    || height == sync_to_height
                                                {
                                                    println!(
                                                        "      ✓ Downloaded {} blocks (current: {})",
                                                        downloaded_count, height
                                                    );
                                                }
                                            }
                                            Err(e) => {
                                                println!(
                                                    "      ⚠️  Failed to add block {}: {}",
                                                    height, e
                                                );
                                                break;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!(
                                            "      ⚠️  Failed to download block {}: {}",
                                            height, e
                                        );
                                        break;
                                    }
                                }
                            }

                            if downloaded_count > 0 {
                                println!(
                                    "      ✅ Successfully downloaded {} blocks from {}",
                                    downloaded_count, peer_ip
                                );
                                synced_any = true;
                                break; // Successfully synced, no need to try other peers
                            }
                        }
                    }
                }
            }

            if !synced_any && attempt < max_sync_attempts {
                println!("      ⏳ Waiting 10s before retry...");
                tokio::time::sleep(Duration::from_secs(10)).await;
            } else if synced_any {
                // Successfully synced some blocks, check if we're done
                break;
            }
        }

        // Final check after all sync attempts
        let final_height = self.blockchain.read().await.chain_tip_height();
        if final_height >= expected_height {
            println!(
                "   ✅ Successfully synced all blocks to height {}!",
                final_height
            );
            return;
        } else if final_height > actual_height {
            println!(
                "   ℹ️  Partially synced to height {} (need {})",
                final_height, expected_height
            );
            println!("   ▶️  Will create remaining blocks via consensus");
        } else {
            println!("   ⚠️  Could not download any blocks from peers");
        }

        // Check if block recreation is allowed
        if !self.allow_block_recreation {
            println!("   ⚠️  Block recreation is disabled in config");
            println!("   ℹ️  Historical blocks can only be downloaded, not recreated");
            println!("   ℹ️  Set 'allow_block_recreation = true' in config to enable");
            println!("   ⏸️  Will retry sync on next cycle");
            return;
        }

        // SYNCHRONIZED NODES: Allow block creation since nodes are now in consensus
        // Nodes have agreed on blocks 0-3 via fork resolution and can safely build forward
        println!("   ⚠️  Could not download blocks from peers");
        println!("   ✅ Block recreation enabled - creating blocks via BFT consensus");
        println!("   📝 This ensures complete blockchain from genesis to present");

        // Wait and watch - see if other nodes are creating blocks
        println!("   ▶️ Waiting 10s to observe network activity...");
        tokio::time::sleep(Duration::from_secs(10)).await;

        // Recheck - did we sync more blocks while waiting?
        let current_height_after_wait = self.load_block_height().await;
        if current_height_after_wait >= expected_height {
            println!(
                "   ✅ Blocks synced while waiting! Now at height {}",
                current_height_after_wait
            );
            return;
        }

        // If we gained blocks, other nodes are working - wait longer
        if current_height_after_wait > actual_height {
            println!(
                "   📊 Height increased from {} to {} - other nodes are building blocks",
                actual_height, current_height_after_wait
            );
            println!("   ⏸️  Waiting for them to complete...");

            // Watch for progress every 15 seconds, up to 2 minutes
            for _ in 0..8 {
                tokio::time::sleep(Duration::from_secs(15)).await;
                let check_height = self.load_block_height().await;
                println!("   📊 Current height: {}", check_height);

                if check_height >= expected_height {
                    println!("   ✅ Caught up to height {}!", check_height);
                    return;
                }
            }

            println!("   ⚠️  Still missing blocks after 2 minutes of waiting");
        }

        // Recheck consensus mode
        let consensus_mode = self.consensus.consensus_mode().await;
        if consensus_mode != time_consensus::ConsensusMode::BFT {
            println!("   ⚠️ BFT not yet active, aborting catch-up");
            return;
        }

        // Determine which node should create catch-up blocks
        let all_masternodes = self.consensus.get_masternodes().await;

        // Filter out quarantined peers from consensus
        let mut masternodes = Vec::new();
        let mut quarantined_count = 0;

        for node_ip in all_masternodes {
            let peer_addr: std::net::IpAddr = node_ip
                .parse()
                .unwrap_or_else(|_| "0.0.0.0".parse().unwrap());

            if self.quarantine.is_quarantined(&peer_addr).await {
                quarantined_count += 1;
                if let Some(reason) = self.quarantine.get_reason(&peer_addr).await {
                    println!(
                        "   🚫 Excluding quarantined peer {} from consensus (reason: {})",
                        node_ip, reason
                    );
                }
            } else {
                masternodes.push(node_ip);
            }
        }

        if quarantined_count > 0 {
            println!(
                "   ✅ Filtered {} quarantined peer(s) from consensus",
                quarantined_count
            );
        }

        println!("   🔍 Active masternode list: {:?}", masternodes);

        // CRITICAL: Check if all masternodes have the required base blocks
        // Before creating block N, all nodes must have block N-1
        let our_height = self.load_block_height().await;
        let start_from_height = our_height + 1;

        // Query all masternodes to see their heights
        println!("   🔍 Checking masternode heights before catch-up...");
        let mut peer_heights: Vec<(String, u64)> = Vec::new();

        for masternode_ip in &masternodes {
            // Get network-aware port
            let p2p_port = match self.peer_manager.network {
                time_network::discovery::NetworkType::Mainnet => 24000,
                time_network::discovery::NetworkType::Testnet => 24100,
            };
            let peer_addr = format!("{}:{}", masternode_ip, p2p_port);

            match self.peer_manager.request_blockchain_info(&peer_addr).await {
                Ok(Some(height)) => {
                    peer_heights.push((masternode_ip.clone(), height));
                    println!("      {} is at height {}", masternode_ip, height);
                }
                Ok(None) => {
                    println!("      {} has no genesis yet", masternode_ip);
                }
                Err(_) => {
                    println!("      ⚠️  Could not reach {}", masternode_ip);
                }
            }
        }

        // Log peer status but don't block - BFT consensus handles this
        let min_height = peer_heights.iter().map(|(_, h)| *h).min().unwrap_or(0);
        if min_height + 1 < start_from_height {
            println!(
                "   ℹ️  Some peers are behind (lowest: {}, we're at: {})",
                min_height, our_height
            );
            println!("   ℹ️  Proceeding with BFT consensus - behind peers will catch up");
        }

        // Create catch-up blocks
        let still_missing = expected_height - self.load_block_height().await;
        println!(
            "   Processing with BFT consensus: {} missed block(s)...",
            still_missing
        );

        // Get genesis block for timestamp calculations
        let blockchain = self.blockchain.read().await;
        let genesis_block = match blockchain.get_block_by_height(0) {
            Some(block) => block.clone(),
            None => {
                println!("   ⚠️  No genesis block found, cannot create catch-up blocks");
                drop(blockchain);
                return;
            }
        };
        drop(blockchain);

        let genesis_timestamp = genesis_block.header.timestamp.timestamp();
        let genesis_date = genesis_block.header.timestamp.date_naive();

        // CRITICAL: Process blocks sequentially, ONE AT A TIME
        // Wait for each block to be fully consensus-accepted before starting the next
        let start_from = self.load_block_height().await + 1;
        for block_num in start_from..=expected_height {
            println!("\n╔═══════════════════════════════════════════════════════════╗");
            println!("║  BLOCK #{:<50} ║", block_num);
            println!("╚═══════════════════════════════════════════════════════════╝");

            // Verify current chain height before attempting next block
            let current_height = self.load_block_height().await;
            if current_height >= block_num {
                println!("   ℹ️  Block {} already exists, skipping...", block_num);
                continue;
            }

            if current_height + 1 != block_num {
                println!(
                    "   ⚠️  Chain height mismatch: have {}, trying to create {}",
                    current_height, block_num
                );
                println!("   ⏸️  Pausing catch-up - chain not ready for this block");
                break;
            }

            // Calculate timestamp based on mode
            let timestamp = if IS_TESTING_MODE {
                // Testing: blocks every 10 minutes from genesis
                let block_timestamp =
                    genesis_timestamp + (block_num as i64 * BLOCK_INTERVAL_SECONDS as i64);
                Utc.timestamp_opt(block_timestamp, 0).unwrap()
            } else {
                // Production: blocks every 24 hours (midnight)
                let timestamp_date = genesis_date + chrono::Duration::days(block_num as i64);
                Utc.from_utc_datetime(&timestamp_date.and_hms_opt(0, 0, 0).unwrap())
            };

            // Single attempt with improved consensus (fast-track + emergency fallback)
            let success = self
                .produce_catchup_block_with_bft_consensus(block_num, timestamp, &masternodes)
                .await;

            if success {
                println!("   ✅ Block {} created successfully!", block_num);
                // Give network time to settle - add randomized delay to prevent all nodes rushing
                let delay_secs = 10 + (block_num % 5); // 10-14 seconds
                println!("   ⏸️  Waiting {}s before next block...", delay_secs);
                tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)).await;
            } else {
                println!("   ❌ Failed to create block {}", block_num);
                println!("   👂 Listening for block from other masternodes...");

                // Wait for block from network - either via proposal or blockchain sync
                let wait_duration = Duration::from_secs(45);
                let start = tokio::time::Instant::now();
                let mut block_received = false;

                while start.elapsed() < wait_duration {
                    // Check if the block appeared in the blockchain
                    let current_height = self.load_block_height().await;
                    if current_height >= block_num {
                        println!("   ✅ Block {} received from network!", block_num);
                        block_received = true;
                        break;
                    }

                    // Also check for consensus proposal
                    if let Some(proposal) = self.block_consensus.wait_for_proposal(block_num).await
                    {
                        println!("   📋 Block {} proposal received from network!", block_num);

                        // Check if block was already finalized while we were waiting
                        let current_height = self.load_block_height().await;
                        if current_height >= block_num {
                            println!("   ✅ Block {} already finalized!", block_num);
                            block_received = true;
                            break;
                        }

                        // Always auto-vote on proposals received from network to help consensus
                        // Since blocks are deterministic, multiple nodes will create identical blocks
                        // and we should vote on matching proposals even if we already voted locally
                        {
                            // Auto-vote on the proposal to help reach consensus
                            println!("   🗳️  Auto-voting APPROVE to help consensus...");

                            // Create and send the vote
                            let vote = BlockVote {
                                block_height: block_num,
                                voter: self.node_id.clone(),
                                block_hash: proposal.block_hash.clone(),
                                approve: true,
                                timestamp: Utc::now().timestamp(),
                            };

                            // Send vote to consensus manager
                            if let Err(e) = self.block_consensus.vote_on_block(vote.clone()).await {
                                eprintln!("   ⚠️  Auto-vote failed: {}", e);
                            } else {
                                println!("   ✅ Auto-vote successful!");

                                // Broadcast the vote to other nodes
                                if let Ok(vote_value) = serde_json::to_value(&vote) {
                                    self.peer_manager.broadcast_block_vote(vote_value).await;
                                }
                            }

                            // Give it a moment to be processed
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            let current_height = self.load_block_height().await;
                            if current_height >= block_num {
                                block_received = true;
                                break;
                            }
                        }
                    }

                    tokio::time::sleep(Duration::from_secs(5)).await;
                }

                if !block_received {
                    break;
                }
            }
        }
    }

    async fn create_and_propose_block(&self) {
        let now = Utc::now();
        let block_num = self.load_block_height().await + 1;

        println!(
            "
────────────────────────────────────────────────────────────────"
        );
        println!(
            "{} {}",
            "⨯ BLOCK PRODUCTION TIME".cyan().bold(),
            now.format("%Y-%m-%d %H:%M:%S UTC")
        );
        println!(
            "{} {}",
            "   Block Height:".bright_black(),
            block_num.to_string().cyan().bold()
        );
        println!("────────────────────────────────────────────────────────────────");

        let consensus_mode = self.consensus.consensus_mode().await;
        if consensus_mode != time_consensus::ConsensusMode::BFT {
            println!("{}", "⚠️  Not in BFT mode".yellow());
            return;
        }

        let all_masternodes = self.consensus.get_masternodes().await;

        // Filter out quarantined masternodes
        let mut masternodes = Vec::new();
        let mut quarantined_count = 0;

        for mn in &all_masternodes {
            if let Ok(ip_addr) = mn.parse::<std::net::IpAddr>() {
                if self.quarantine.is_quarantined(&ip_addr).await {
                    quarantined_count += 1;
                    continue;
                }
            }
            masternodes.push(mn.clone());
        }

        if quarantined_count > 0 {
            println!(
                "   ℹ️  Consensus pool: {} active, {} excluded",
                masternodes.len(),
                quarantined_count
            );
        }

        println!("   🔷 Deterministic Consensus - all nodes generate identical block");
        println!("      Active masternodes: {}", masternodes.len());

        // Get pending transactions
        let mut transactions = self.mempool.get_all_transactions().await;

        // Handle approved treasury proposals
        if let Some(proposal_manager) = self.consensus.proposal_manager() {
            let masternode_count = self.consensus.masternode_count().await;
            let approved_proposals = proposal_manager
                .get_approved_pending(masternode_count)
                .await;

            if !approved_proposals.is_empty() {
                println!(
                    "   💰 Processing {} approved proposal(s)",
                    approved_proposals.len()
                );

                for proposal in approved_proposals {
                    let grant_tx = Transaction {
                        txid: format!("treasury_grant_{}", proposal.id),
                        version: 1,
                        inputs: vec![],
                        outputs: vec![TxOutput::new(proposal.amount, proposal.recipient.clone())],
                        lock_time: 0,
                        timestamp: now.timestamp(),
                    };

                    println!(
                        "      ✓ Grant: {} TIME to {}",
                        proposal.amount as f64 / 100_000_000.0,
                        proposal.recipient
                    );
                    transactions.push(grant_tx);

                    let _ = proposal_manager
                        .mark_executed(&proposal.id, format!("treasury_grant_{}", proposal.id))
                        .await;
                }
            }
        }

        // Use deterministic timestamp based on mode
        let timestamp = if IS_TESTING_MODE {
            // Testing: round to 10-minute interval
            let current_seconds = now.timestamp();
            let interval_timestamp =
                (current_seconds / BLOCK_INTERVAL_SECONDS as i64) * BLOCK_INTERVAL_SECONDS as i64;
            Utc.timestamp_opt(interval_timestamp, 0).unwrap()
        } else {
            // Production: midnight UTC
            now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc()
        };

        println!("   📡 Running deterministic consensus...");

        // Calculate total fees
        let total_fees = self.calculate_total_fees(&transactions).await;

        // Run deterministic consensus
        match self
            .deterministic
            .run_consensus(block_num, timestamp, &masternodes, transactions, total_fees)
            .await
        {
            ConsensusResult::Consensus(block) => {
                println!("   ✅ CONSENSUS REACHED - Block finalized!");
                self.finalize_and_broadcast_block(block).await;
            }
            ConsensusResult::NeedsReconciliation {
                our_block,
                peer_blocks,
                differences,
            } => {
                println!("   ⚠️  Block differences detected - reconciling...");

                if let Some(reconciled_block) = self
                    .deterministic
                    .reconcile_and_finalize(
                        block_num,
                        timestamp,
                        our_block,
                        peer_blocks,
                        differences,
                    )
                    .await
                {
                    println!("   ✅ Reconciliation successful!");
                    self.finalize_and_broadcast_block(reconciled_block).await;
                } else {
                    println!("   ❌ Reconciliation failed - will retry next block");
                }
            }
        }
    }

    /// Finalize and broadcast a block to the network
    async fn finalize_and_broadcast_block(&self, block: Block) {
        let block_num = block.header.block_number;

        // Add block to blockchain
        let mut blockchain = self.blockchain.write().await;
        match blockchain.add_block(block.clone()) {
            Ok(_) => {
                println!("   💾 Block {} added to blockchain", block_num);
                drop(blockchain);

                // Clear processed transactions from mempool
                for tx in &block.transactions {
                    if !tx.inputs.is_empty() {
                        // Skip coinbase (first tx with no inputs)
                        self.mempool.remove_transaction(&tx.txid).await;
                    }
                }

                // Broadcast via UpdateTip message
                let masternodes = self.consensus.get_masternodes().await;
                self.broadcast_finalized_block(&block, &masternodes).await;
            }
            Err(e) => {
                println!("   ❌ Failed to add block {}: {:?}", block_num, e);
            }
        }
    }

    async fn broadcast_finalized_block(
        &self,
        block: &time_core::block::Block,
        masternodes: &[String],
    ) {
        let height = block.header.block_number;
        let hash = hex::encode(&block.hash);

        println!(
            "   📡 Broadcasting finalized block #{} to peers via TCP...",
            height
        );

        for node_ip in masternodes {
            let peer_manager = self.peer_manager.clone();
            let node_ip_owned = node_ip.clone();
            let hash_clone = hash.clone();

            // Fire-and-forget, best-effort broadcast via TCP
            tokio::spawn(async move {
                // Parse IP from node string
                let ip: std::net::IpAddr = match node_ip_owned.parse() {
                    Ok(ip) => ip,
                    Err(_) => {
                        eprintln!("   ⚠️  Invalid IP address: {}", node_ip_owned);
                        return;
                    }
                };

                let message = time_network::protocol::NetworkMessage::UpdateTip {
                    height,
                    hash: hash_clone,
                };

                if let Err(e) = peer_manager.send_to_peer_tcp(ip, message).await {
                    eprintln!(
                        "   ⚠️  Failed to broadcast UpdateTip to {}: {}",
                        node_ip_owned, e
                    );
                }
            });
        }
    }

    /// Attempt to fetch finalized block from producer with retries
    #[allow(dead_code)]
    async fn fetch_finalized_block(
        &self,
        producer: &str,
        height: u64,
        _expected_merkle: &str,
    ) -> Option<time_core::block::Block> {
        // Try to fetch block from the producer
        let p2p_port = match self.peer_manager.network {
            time_network::discovery::NetworkType::Mainnet => 24000,
            time_network::discovery::NetworkType::Testnet => 24100,
        };
        let peer_addr = format!("{}:{}", producer, p2p_port);

        println!(
            "   📡 Attempting to download block {} from {}...",
            height, producer
        );

        match self
            .peer_manager
            .request_block_by_height(&peer_addr, height)
            .await
        {
            Ok(block) => {
                println!(
                    "   ✅ Successfully downloaded block {} from {}",
                    height, producer
                );
                Some(block)
            }
            Err(e) => {
                println!("   ⚠️  Failed to download block {}: {}", height, e);
                println!("   ℹ️  Block will be recreated via BFT consensus");
                None
            }
        }
    }

    #[allow(dead_code)]
    async fn finalize_block_bft(
        &self,
        transactions: &[time_core::Transaction],
        previous_hash: &str,
        merkle_root: &str,
        block_num: u64,
    ) {
        use sha2::{Digest, Sha256};
        use time_core::{Block, BlockHeader};

        let my_id = self.get_node_id();
        let sig_data = format!("{}{}{}", block_num, previous_hash, merkle_root);
        let mut hasher = Sha256::new();
        hasher.update(sig_data.as_bytes());
        hasher.update(my_id.as_bytes());
        let validator_signature = format!("{:x}", hasher.finalize());

        let header = BlockHeader {
            block_number: block_num,
            timestamp: Utc::now(),
            previous_hash: previous_hash.to_string(),
            merkle_root: merkle_root.to_string(),
            validator_signature,
            validator_address: my_id,
            masternode_counts: time_core::MasternodeCounts {
                free: 0,
                bronze: 0,
                silver: 0,
                gold: 0,
            },
        };

        let mut block = Block {
            header,
            transactions: transactions.to_vec(),
            hash: String::new(), // Temporary, will be calculated
        };

        // Calculate hash using the proper method
        block.hash = block.calculate_hash();

        let masternodes = self.consensus.get_masternodes().await;
        self.broadcast_finalized_block(&block, &masternodes).await;

        let mut blockchain = self.blockchain.write().await;
        match blockchain.add_block(block.clone()) {
            Ok(_) => {
                println!("   ✔ Block {} finalized", block_num);

                // Save UTXO snapshot to persist state
                if let Err(e) = blockchain.save_utxo_snapshot() {
                    println!("   ⚠️  Failed to save UTXO snapshot: {}", e);
                    println!("   ⚠️  NOT removing transactions from mempool due to save failure");
                    println!("   ⚠️  This prevents UTXO loss - transactions will be retried");
                    drop(blockchain);
                    return; // Critical failure - don't continue
                } else {
                    println!("   💾 UTXO snapshot saved");
                }

                // Remove finalized transactions that were included in this block
                for tx in block.transactions.iter().skip(1) {
                    // Skip coinbase (first transaction)
                    if let Err(e) = blockchain.remove_finalized_tx(&tx.txid) {
                        println!(
                            "   ⚠️  Failed to remove finalized tx {}: {}",
                            truncate_str(&tx.txid, 16),
                            e
                        );
                    }
                }

                let block_hash = block.hash.clone();
                drop(blockchain);

                self.peer_manager
                    .broadcast_tip_update(block_num, block_hash)
                    .await;

                // Remove transactions from mempool ONLY after successful UTXO snapshot save
                for tx in transactions.iter().skip(1) {
                    self.mempool.remove_transaction(&tx.txid).await;
                }

                let all_masternodes = self.consensus.get_masternodes().await;
                let active_masternodes = self
                    .block_consensus
                    .get_active_masternodes(&all_masternodes)
                    .await;
                self.broadcast_finalized_block(&block, &active_masternodes)
                    .await;
            }
            Err(e) => {
                println!("   ✗ Failed: {:?}", e);
            }
        }
    }

    async fn calculate_total_fees(&self, transactions: &[Transaction]) -> u64 {
        let blockchain = self.blockchain.read().await;
        let utxo_map = blockchain.utxo_set().utxos();
        let mut total_fees = 0u64;

        for tx in transactions {
            match tx.fee(utxo_map) {
                Ok(fee) => {
                    total_fees += fee;
                    println!("      📊 TX {} fee: {} satoshis", &tx.txid[..8], fee);
                }
                Err(e) => {
                    println!(
                        "      ⚠️  Could not calculate fee for {}: {:?}",
                        &tx.txid[..8],
                        e
                    );
                }
            }
        }
        total_fees
    }

    #[allow(dead_code)]
    fn log_coinbase_info(&self, all_transactions: &[Transaction], mempool_count: usize) {
        if mempool_count == 0 {
            println!("   📦 Block will contain ONLY coinbase transaction (zero regular txs)");
            println!("   ℹ️  This is NORMAL and EXPECTED for TIME Coin");
            println!("   ℹ️  Coinbase includes treasury + masternode rewards");
        }
        println!(
            "   📋 {} total transactions (1 coinbase + {} mempool)",
            all_transactions.len(),
            mempool_count
        );

        let coinbase_outputs = all_transactions[0].outputs.len();
        let coinbase_amount: u64 = all_transactions[0].outputs.iter().map(|o| o.amount).sum();
        println!(
            "   💰 Coinbase: {} outputs, {} satoshis total ({} TIME)",
            coinbase_outputs,
            coinbase_amount,
            coinbase_amount / 100_000_000
        );
    }

    #[allow(dead_code)]
    fn calculate_block_hash(
        &self,
        block_num: u64,
        now: &chrono::DateTime<Utc>,
        previous_hash: &str,
        merkle_root: &str,
        my_id: &str,
        masternode_counts: &time_core::MasternodeCounts,
    ) -> String {
        use sha2::{Digest, Sha256};
        use time_core::BlockHeader;

        let sig_data = format!("{}{}{}", block_num, previous_hash, merkle_root);
        let mut hasher = Sha256::new();
        hasher.update(sig_data.as_bytes());
        hasher.update(my_id.as_bytes());
        let validator_signature = format!("{:x}", hasher.finalize());

        let temp_header = BlockHeader {
            block_number: block_num,
            timestamp: *now,
            previous_hash: previous_hash.to_string(),
            merkle_root: merkle_root.to_string(),
            validator_signature,
            validator_address: my_id.to_string(),
            masternode_counts: masternode_counts.clone(),
        };

        // Create a temporary block to calculate hash properly
        let temp_block = time_core::Block {
            header: temp_header,
            hash: String::new(),
            transactions: vec![],
        };

        // Calculate hash using the proper method
        temp_block.calculate_hash()
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(dead_code)]
    async fn collect_and_finalize_votes(
        &self,
        all_transactions: &[Transaction],
        previous_hash: &str,
        merkle_root: &str,
        block_num: u64,
        required_votes: usize,
        masternodes: &[String],
        proposal: &time_consensus::block_consensus::BlockProposal,
    ) {
        const MAX_VOTE_ATTEMPTS: u32 = 3;

        for attempt in 1..=MAX_VOTE_ATTEMPTS {
            let timeout_secs = if attempt == 1 { 60 } else { 30 };
            println!(
                "   🗳️  Vote collection attempt {}/{} ({}s timeout)",
                attempt, MAX_VOTE_ATTEMPTS, timeout_secs
            );

            let (approved, total) = self
                .block_consensus
                .collect_votes_with_timeout(block_num, required_votes, timeout_secs)
                .await;

            for mn in masternodes {
                let voters = self
                    .block_consensus
                    .get_voters(block_num, &proposal.block_hash)
                    .await;
                if !voters.contains(mn) {
                    self.block_consensus.record_missed_vote(mn).await;
                }
            }

            println!("   🗳️  Votes: {}/{}", approved, total);

            if approved >= required_votes {
                println!("   ✔ Quorum reached! Finalizing...");
                self.finalize_block_bft(all_transactions, previous_hash, merkle_root, block_num)
                    .await;
                return;
            }

            println!(
                "   ⚠️  Attempt {}: Quorum not reached ({} < {})",
                attempt, approved, required_votes
            );

            if attempt == MAX_VOTE_ATTEMPTS {
                if approved > total / 2 {
                    println!(
                        "   🚨 EMERGENCY: Simple majority reached ({}/{}), finalizing block",
                        approved, total
                    );
                    self.finalize_block_bft(
                        all_transactions,
                        previous_hash,
                        merkle_root,
                        block_num,
                    )
                    .await;
                    return;
                } else {
                    println!(
                        "   ❌ FAILED: Insufficient votes ({}/{}), block not created",
                        approved, total
                    );
                    println!("   ℹ️  Block will be created during next catch-up cycle");
                }
            } else {
                println!("   🔄 Retrying vote collection...");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }

    #[allow(dead_code)]
    async fn produce_catch_up_block(
        &self,
        block_num: u64,
        timestamp: chrono::DateTime<Utc>,
    ) -> bool {
        use time_core::block::calculate_tier_reward;

        let mut blockchain = self.blockchain.write().await;

        let previous_hash = if block_num == 0 {
            "0000000000000000000000000000000000000000000000000000000000000000".to_string()
        } else {
            blockchain.chain_tip_hash().to_string()
        };

        let masternode_counts = blockchain.masternode_counts().clone();

        // Initialize outputs with masternode rewards only (no treasury pre-allocation)
        let mut outputs = vec![];

        // For catch-up blocks, also filter by participation
        let agreed_tx_set = self.tx_consensus.get_agreed_tx_set(block_num).await;
        let voters = if let Some(proposal) = agreed_tx_set {
            self.tx_consensus
                .get_voters(block_num, &proposal.merkle_root)
                .await
        } else {
            Vec::new()
        };

        // --- FIX: derive participating_masternodes from voters ---
        let active_masternodes = self.consensus.get_masternodes_with_wallets().await;
        let participating_masternodes: Vec<_> = active_masternodes
            .into_iter()
            .filter(|(node_id, _)| voters.contains(node_id))
            .collect();

        if !participating_masternodes.is_empty() {
            println!(
                "      💡 Rewarding {} participating masternodes",
                participating_masternodes.len()
            );
            let tiers = [
                MasternodeTier::Free,
                MasternodeTier::Bronze,
                MasternodeTier::Silver,
                MasternodeTier::Gold,
            ];
            for tier in tiers {
                let tier_reward = calculate_tier_reward(tier, &masternode_counts);
                if tier_reward > 0 {
                    let tier_nodes: Vec<_> = participating_masternodes
                        .iter()
                        .filter(|(node_id, _)| {
                            node_id.starts_with(&format!("{:?}", tier).to_lowercase())
                        })
                        .collect();

                    if !tier_nodes.is_empty() {
                        let reward_per_node = tier_reward / tier_nodes.len() as u64;
                        for (_, wallet_addr) in tier_nodes {
                            outputs.push(TxOutput {
                                amount: reward_per_node,
                                address: wallet_addr.clone(),
                            });
                        }
                    }
                }
            }
        }

        let coinbase_tx = Transaction {
            txid: format!("coinbase_{}", block_num),
            version: 1,
            inputs: vec![],
            outputs,
            lock_time: 0,
            timestamp: timestamp.timestamp(),
        };

        let my_id = self.get_node_id();

        let mut block = Block {
            hash: String::new(),
            header: BlockHeader {
                block_number: block_num,
                timestamp,
                previous_hash,
                merkle_root: String::new(),
                validator_signature: my_id.clone(),
                validator_address: my_id.clone(),
                masternode_counts: time_core::MasternodeCounts {
                    free: 0,
                    bronze: 0,
                    silver: 0,
                    gold: 0,
                },
            },
            transactions: vec![coinbase_tx],
        };

        // Calculate merkle root and hash
        block.header.merkle_root = block.calculate_merkle_root();
        block.hash = block.calculate_hash();

        println!("   🔧 Creating catch-up block #{}...", block_num);
        println!(
            "      Timestamp: {}",
            timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );
        println!("      Block Hash: {}...", truncate_str(&block.hash, 16));

        match blockchain.add_block(block.clone()) {
            Ok(_) => {
                println!("      ✔ Block #{} created and stored", block_num);

                // Save UTXO snapshot to persist state
                if let Err(e) = blockchain.save_utxo_snapshot() {
                    println!("      ⚠️  Failed to save UTXO snapshot: {}", e);
                } else {
                    println!("      💾 UTXO snapshot saved");
                }

                true
            }
            Err(e) => {
                println!("      ✗ Failed to create block {}: {:?}", block_num, e);
                false
            }
        }
    }

    #[allow(dead_code)]
    async fn produce_catchup_block_with_bft_consensus(
        &self,
        block_num: u64,
        timestamp: chrono::DateTime<Utc>,
        masternodes: &[String],
    ) -> bool {
        use time_consensus::block_consensus::{BlockProposal, BlockVote};

        // Update block_consensus manager's masternode list
        self.block_consensus
            .set_masternodes(masternodes.to_vec())
            .await;

        let my_id = self.get_node_id();
        println!("   🔨 Creating catchup block #{}", block_num);
        println!(
            "      ℹ️  All {} nodes create identical deterministic blocks",
            masternodes.len()
        );

        // Try up to 3 times with FAST retries (no artificial delays)
        for attempt in 0..3 {
            if attempt > 0 {
                println!("      ⚠️  Attempt {} - instant retry", attempt + 1);
                // NO DELAY - instant retry for speed
            }

            // ALL nodes create the same deterministic block
            let block = self
                .create_catchup_block_structure(block_num, timestamp)
                .await;

            println!(
                "      ✓ Block created: {}...",
                truncate_str(&block.hash, 16)
            );

            let proposal = BlockProposal {
                block_height: block_num,
                proposer: my_id.clone(),
                block_hash: block.hash.clone(),
                merkle_root: block.header.merkle_root.clone(),
                previous_hash: block.header.previous_hash.clone(),
                timestamp: timestamp.timestamp(),
                is_reward_only: false,
                strategy: None,
            };

            // Store proposal locally (first-proposal-wins)
            let accepted = self.block_consensus.propose_block(proposal.clone()).await;

            if accepted {
                println!("      ✓ Proposal stored locally");
            } else {
                println!("      ℹ️  Another proposal already stored");
            }

            // Broadcast proposal to all peers via TCP
            let proposal_json = serde_json::to_value(&proposal).unwrap();
            self.peer_manager
                .broadcast_block_proposal(proposal_json)
                .await;

            // ALL nodes vote on their own calculated block hash
            let vote = BlockVote {
                block_height: block_num,
                block_hash: block.hash.clone(),
                voter: my_id.clone(),
                approve: true,
                timestamp: chrono::Utc::now().timestamp(),
            };

            if let Err(e) = self.block_consensus.vote_on_block(vote.clone()).await {
                println!("      ⚠️  Failed to record vote: {}", e);
            } else {
                println!(
                    "      ✓ Voted APPROVE on block {}",
                    truncate_str(&block.hash, 16)
                );
            }

            // Broadcast vote via TCP
            let vote_json = serde_json::to_value(&vote).unwrap();
            self.peer_manager.broadcast_block_vote(vote_json).await;

            // ULTRA-FAST consensus check - 3 second timeout, 50ms polling
            println!("      ⚡ Ultra-fast consensus check...");
            let start_time = std::time::Instant::now();
            let timeout = Duration::from_secs(3);
            let mut last_log_time = start_time;
            let mut last_vote_count = 0;
            let mut stall_count = 0;
            const MAX_STALLS: u32 = 40; // Exit after 2 seconds of no progress (40 * 50ms)

            // NO artificial delay - check immediately
            while start_time.elapsed() < timeout {
                // FAST polling - check every 50ms instead of 1 second
                tokio::time::sleep(Duration::from_millis(50)).await;

                if let Some(proposal) = self.block_consensus.get_proposal(block_num).await {
                    // Check consensus (need 2/3+ votes)
                    let required_votes = (masternodes.len() * 2).div_ceil(3);
                    let (has_consensus, approvals, _total) = self
                        .block_consensus
                        .has_block_consensus(block_num, &proposal.block_hash)
                        .await;

                    // Early exit if votes have stalled
                    if approvals == last_vote_count {
                        stall_count += 1;
                        if stall_count >= MAX_STALLS {
                            println!(
                                "      ⚠️  Vote stalled at {}/{} after {}ms - ending attempt",
                                approvals,
                                masternodes.len(),
                                start_time.elapsed().as_millis()
                            );
                            break;
                        }
                    } else {
                        last_vote_count = approvals;
                        stall_count = 0; // Reset stall counter on progress
                    }

                    // Log progress every 1 second for real-time feedback (reduced spam)
                    if last_log_time.elapsed() >= Duration::from_secs(1) {
                        println!(
                            "         ⚡ {}/{} votes (need {}) - {}ms elapsed",
                            approvals,
                            masternodes.len(),
                            required_votes,
                            start_time.elapsed().as_millis()
                        );
                        last_log_time = std::time::Instant::now();
                    }

                    if has_consensus && approvals >= required_votes {
                        let elapsed_ms = start_time.elapsed().as_millis();
                        println!(
                            "      ✅ ULTRA-FAST CONSENSUS in {}ms ({}/{})!",
                            elapsed_ms,
                            approvals,
                            masternodes.len()
                        );

                        // ALL nodes finalize the exact same block they voted on
                        return self.finalize_agreed_block(block, masternodes).await;
                    }
                }
            }

            // Timeout - provide diagnostics
            let elapsed_ms = start_time.elapsed().as_millis();
            println!(
                "      ⚠️  Attempt {} timeout after {}ms",
                attempt + 1,
                elapsed_ms
            );

            if let Some(proposal) = self.block_consensus.get_proposal(block_num).await {
                let required_votes = (masternodes.len() * 2).div_ceil(3);
                let (_, approvals, _) = self
                    .block_consensus
                    .has_block_consensus(block_num, &proposal.block_hash)
                    .await;

                println!(
                    "         📊 Final tally: {}/{} votes (needed {})",
                    approvals,
                    masternodes.len(),
                    required_votes
                );

                // Get list of who voted
                let voters = self
                    .block_consensus
                    .get_voters(block_num, &proposal.block_hash)
                    .await;
                println!("         👥 Voters: {:?}", voters);

                // Show who didn't vote
                let non_voters: Vec<String> = masternodes
                    .iter()
                    .filter(|mn| !voters.contains(mn))
                    .cloned()
                    .collect();
                if !non_voters.is_empty() {
                    println!("         ❌ Missing votes from: {:?}", non_voters);
                }

                // Don't proceed without proper consensus
                println!("         ⚠️  Insufficient votes - cannot finalize block");
            } else {
                println!("         ⚠️  No proposal was received");
            }
        }

        println!("      ❌ All attempts failed for block #{}", block_num);
        false
    }

    #[allow(dead_code)]
    async fn create_minimal_catchup_block(
        &self,
        block_num: u64,
        timestamp: chrono::DateTime<Utc>,
        emergency: bool,
    ) -> time_core::block::Block {
        use time_core::block::{Block, BlockHeader};
        use time_core::transaction::Transaction;

        let blockchain = self.blockchain.read().await;
        let previous_hash = blockchain.chain_tip_hash().to_string();
        let masternode_counts = blockchain.masternode_counts().clone();
        let active_masternodes: Vec<(String, time_core::MasternodeTier)> = blockchain
            .get_active_masternodes()
            .iter()
            .map(|mn| (mn.wallet_address.clone(), mn.tier))
            .collect();
        drop(blockchain);

        let my_id = self.get_node_id();
        let outputs = if emergency {
            vec![]
        } else {
            time_core::block::distribute_masternode_rewards(&active_masternodes, &masternode_counts)
        };

        let coinbase_tx = Transaction {
            txid: format!("coinbase_{}", block_num),
            version: 1,
            inputs: vec![],
            outputs,
            lock_time: 0,
            timestamp: timestamp.timestamp(),
        };

        let mut block = Block {
            hash: String::new(),
            header: BlockHeader {
                block_number: block_num,
                timestamp,
                previous_hash,
                merkle_root: String::new(),
                validator_signature: my_id.clone(),
                validator_address: my_id.clone(),
                masternode_counts: time_core::block::MasternodeCounts {
                    free: 0,
                    bronze: 0,
                    silver: 0,
                    gold: 0,
                },
            },
            transactions: vec![coinbase_tx],
        };

        block.header.merkle_root = block.calculate_merkle_root();
        block.hash = block.calculate_hash();
        println!(
            "      💰 Minimal block: {} transaction(s)",
            block.transactions.len()
        );
        block
    }

    #[allow(dead_code)]
    async fn create_catchup_block_structure(
        &self,
        block_num: u64,
        timestamp: chrono::DateTime<Utc>,
    ) -> time_core::block::Block {
        use time_core::block::{create_coinbase_transaction, Block, BlockHeader};

        let blockchain = self.blockchain.read().await;
        let previous_hash = blockchain.chain_tip_hash().to_string();
        let masternode_counts = blockchain.masternode_counts().clone();

        // CRITICAL: Use the same active masternodes logic as normal block production
        // This ensures catch-up blocks are identical to what the leader produced
        let active_masternodes: Vec<(String, time_core::MasternodeTier)> = blockchain
            .get_active_masternodes()
            .iter()
            .map(|mn| (mn.wallet_address.clone(), mn.tier))
            .collect();

        // Get all masternodes for consensus tracking
        let all_masternodes = self.consensus.get_masternodes().await;

        drop(blockchain);

        // Get pending transactions from mempool
        // For catch-up blocks (past dates), only include transactions from before that block's time
        // For current/future blocks, include all pending transactions
        let block_timestamp = timestamp.timestamp();
        let current_time = chrono::Utc::now().timestamp();
        let all_mempool_txs = self.mempool.get_all_transactions().await;

        let mut mempool_txs: Vec<_> = if block_timestamp < current_time {
            // This is a catch-up block for a past date - only include old transactions
            all_mempool_txs
                .into_iter()
                .filter(|tx| tx.timestamp <= block_timestamp)
                .collect()
        } else {
            // This is a current/future block - include all pending transactions
            all_mempool_txs
        };

        println!(
            "      💰 Catch-up block will reward all {} active masternodes",
            active_masternodes.len()
        );
        println!(
            "      🔍 Using {} registered masternodes for consensus",
            all_masternodes.len()
        );
        if block_timestamp < current_time {
            println!(
                "      📦 Including {} transactions from mempool (filtered by timestamp <= {})",
                mempool_txs.len(),
                timestamp.format("%Y-%m-%d %H:%M:%S UTC")
            );
        } else {
            println!(
                "      📦 Including {} transactions from mempool (all pending)",
                mempool_txs.len()
            );
        }

        let coinbase_tx = create_coinbase_transaction(
            block_num,
            &active_masternodes,
            &masternode_counts,
            0,
            timestamp.timestamp(), // Use block timestamp for determinism
        );

        // CRITICAL: Use deterministic validator info so all nodes create identical blocks
        let deterministic_validator = format!("consensus_block_{}", block_num);

        // Start with coinbase, then add mempool transactions
        let mut transactions = vec![coinbase_tx];
        transactions.append(&mut mempool_txs);

        let mut block = Block {
            hash: String::new(),
            header: BlockHeader {
                block_number: block_num,
                timestamp,
                previous_hash,
                merkle_root: String::new(),
                validator_signature: deterministic_validator.clone(),
                validator_address: deterministic_validator,
                masternode_counts: masternode_counts.clone(),
            },
            transactions,
        };

        block.header.merkle_root = block.calculate_merkle_root();
        block.hash = block.calculate_hash();
        println!(
            "      ✓ Deterministic block created: {}...",
            truncate_str(&block.hash, 16)
        );
        block
    }

    #[allow(dead_code)]
    async fn notify_leader_to_produce_block(
        &self,
        leader_ip: &str,
        block_height: u64,
        requester_ip: &str,
    ) {
        println!(
            "   📤 Notifying leader {} to produce block #{} via TCP...",
            leader_ip, block_height
        );

        // Parse IP
        let ip: std::net::IpAddr = match leader_ip.parse() {
            Ok(ip) => ip,
            Err(_) => {
                eprintln!("      ✗ Invalid leader IP: {}", leader_ip);
                return;
            }
        };

        let message = time_network::protocol::NetworkMessage::RequestBlockProposal {
            block_height,
            leader_ip: leader_ip.to_string(),
            requester_ip: requester_ip.to_string(),
        };

        match self.peer_manager.send_to_peer_tcp(ip, message).await {
            Ok(_) => {
                println!("      ✓ Leader {} acknowledged the request", leader_ip);
            }
            Err(e) => {
                println!("      ✗ Failed to notify leader {}: {}", leader_ip, e);
            }
        }
    }

    /// Finalize the agreed-upon block that all nodes voted on
    async fn finalize_agreed_block(
        &self,
        block: time_core::block::Block,
        _masternodes: &[String],
    ) -> bool {
        let block_num = block.header.block_number;
        let mut blockchain = self.blockchain.write().await;

        // Check if block already exists at this height
        if let Some(existing_block) = blockchain.get_block_by_height(block_num) {
            println!(
                "      ℹ️  Block #{} already exists (hash: {}...), skipping",
                block_num,
                &existing_block.hash[..16]
            );
            return true;
        }

        println!("      🔧 Finalizing agreed block #{}...", block_num);
        println!("      📋 Block hash: {}...", truncate_str(&block.hash, 16));

        match blockchain.add_block(block.clone()) {
            Ok(_) => {
                println!("      ✔ Block #{} finalized and stored", block_num);

                // Save UTXO snapshot to persist state
                if let Err(e) = blockchain.save_utxo_snapshot() {
                    println!("      ⚠️  Failed to save UTXO snapshot: {}", e);
                    println!(
                        "      ⚠️  NOT removing transactions from mempool due to save failure"
                    );
                    println!("      ⚠️  This prevents UTXO loss - transactions will be retried");
                    drop(blockchain);
                    return false; // Critical failure - don't continue
                } else {
                    println!("      💾 UTXO snapshot saved");
                }

                drop(blockchain);

                // Remove transactions from mempool (skip coinbase at index 0)
                // This is ONLY done after successful UTXO snapshot save to prevent UTXO loss
                for tx in block.transactions.iter().skip(1) {
                    self.mempool.remove_transaction(&tx.txid).await;
                }

                if block.transactions.len() > 1 {
                    println!(
                        "      🧹 Removed {} transactions from mempool",
                        block.transactions.len() - 1
                    );
                }

                println!("      📡 Broadcasting finalized block to peers...");
                self.broadcast_block_to_peers(&block).await;
                true
            }
            Err(e) => {
                println!("      ✗ Failed to finalize block: {:?}", e);
                false
            }
        }
    }

    #[allow(dead_code)]
    async fn finalize_catchup_block_with_rewards(
        &self,
        block_num: u64,
        timestamp: chrono::DateTime<Utc>,
        _masternodes: &[String],
    ) -> bool {
        use time_core::block::{Block, BlockHeader};

        let mut blockchain = self.blockchain.write().await;

        // Check if block already exists at this height
        if let Some(existing_block) = blockchain.get_block_by_height(block_num) {
            println!(
                "      ℹ️  Block #{} already exists (hash: {}...), skipping creation",
                block_num,
                &existing_block.hash[..16]
            );
            return true; // Not an error, just already done
        }

        let previous_hash = blockchain.chain_tip_hash().to_string();
        let masternode_counts = blockchain.masternode_counts().clone();

        let my_id = self.get_node_id();
        let all_masternodes: Vec<(String, time_core::MasternodeTier)> = blockchain
            .get_all_masternodes()
            .iter()
            .filter(|&mn| mn.is_active)
            .map(|mn| (mn.wallet_address.clone(), mn.tier))
            .collect();

        println!(
            "      💰 Distributing rewards to {} registered masternodes",
            all_masternodes.len()
        );

        let coinbase_tx = time_core::block::create_coinbase_transaction(
            block_num,
            &all_masternodes,
            &masternode_counts,
            0,
            timestamp.timestamp(),
        );

        println!(
            "      ✓ Created coinbase with {} outputs (incl. treasury)",
            coinbase_tx.outputs.len()
        );

        let mut block = Block {
            hash: String::new(),
            header: BlockHeader {
                block_number: block_num,
                timestamp,
                previous_hash,
                merkle_root: String::new(),
                validator_signature: my_id.clone(),
                validator_address: my_id.clone(),
                masternode_counts: masternode_counts.clone(),
            },
            transactions: vec![coinbase_tx],
        };

        block.header.merkle_root = block.calculate_merkle_root();
        block.hash = block.calculate_hash();

        println!("      🔧 Finalizing block #{}...", block_num);
        match blockchain.add_block(block.clone()) {
            Ok(_) => {
                println!("      ✔ Block #{} finalized and stored", block_num);

                // Save UTXO snapshot to persist state
                if let Err(e) = blockchain.save_utxo_snapshot() {
                    println!("      ⚠️  Failed to save UTXO snapshot: {}", e);
                } else {
                    println!("      💾 UTXO snapshot saved");
                }

                drop(blockchain);

                println!("      📡 Broadcasting finalized block to peers...");
                self.broadcast_block_to_peers(&block).await;
                true
            }
            Err(e) => {
                println!("      ✗ Failed to finalize block: {:?}", e);
                false
            }
        }
    }

    async fn broadcast_block_to_peers(&self, block: &time_core::block::Block) {
        let peers = self.peer_manager.get_peer_ips().await;
        let height = block.header.block_number;
        let hash = hex::encode(&block.hash);

        println!(
            "   📡 Broadcasting block #{} to {} peers via TCP...",
            height,
            peers.len()
        );

        for peer_ip in peers {
            let peer_manager = self.peer_manager.clone();
            let hash_clone = hash.clone();
            let peer_ip_clone = peer_ip.clone();

            tokio::spawn(async move {
                // Parse IP from peer string
                let ip: std::net::IpAddr = match peer_ip_clone
                    .split(':')
                    .next()
                    .unwrap_or(&peer_ip_clone)
                    .parse()
                {
                    Ok(ip) => ip,
                    Err(_) => return,
                };

                let message = time_network::protocol::NetworkMessage::UpdateTip {
                    height,
                    hash: hash_clone,
                };

                let _ = peer_manager.send_to_peer_tcp(ip, message).await;
            });
        }
    }

    async fn broadcast_catch_up_request(&self, current_height: u64, expected_height: u64) {
        let node_id = self.node_id.clone();
        let peers = self.peer_manager.get_peer_ips().await;

        if peers.is_empty() {
            return;
        }

        println!(
            "   📡 Notifying {} peer(s) of catch-up need via TCP...",
            peers.len()
        );

        for peer_ip in peers {
            let peer_manager = self.peer_manager.clone();
            let requester = node_id.clone();
            let peer_ip_clone = peer_ip.clone();

            // Fire and forget - don't wait for responses
            tokio::spawn(async move {
                // Parse IP from peer string
                let ip: std::net::IpAddr = match peer_ip_clone
                    .split(':')
                    .next()
                    .unwrap_or(&peer_ip_clone)
                    .parse()
                {
                    Ok(ip) => ip,
                    Err(_) => return,
                };

                let message = time_network::protocol::NetworkMessage::CatchUpRequest {
                    requester,
                    current_height,
                    expected_height,
                };

                let _ = peer_manager.send_to_peer_tcp(ip, message).await;
            });
        }
    }
}
