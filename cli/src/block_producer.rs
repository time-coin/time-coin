use crate::bft_consensus::BftConsensus;
use crate::chain_sync::BlockchainInfo;
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

        println!("Block producer started (24-hour interval)");

        if self.allow_block_recreation {
            self.spawn_periodic_catchup_task();
        }

        // Main loop: sleep until midnight, then produce block
        loop {
            let now = Utc::now();

            // Calculate next midnight UTC
            let tomorrow = now.date_naive() + chrono::Duration::days(1);
            let next_midnight = tomorrow.and_hms_opt(0, 0, 0).unwrap().and_utc();

            let duration_until_midnight = (next_midnight - now)
                .to_std()
                .unwrap_or(Duration::from_secs(60));

            let hours = duration_until_midnight.as_secs() / 3600;
            let minutes = (duration_until_midnight.as_secs() % 3600) / 60;
            let seconds = duration_until_midnight.as_secs() % 60;

            println!(
                "Next block at {} UTC (in {}h {}m {}s)",
                next_midnight.format("%Y-%m-%d %H:%M:%S"),
                hours,
                minutes,
                seconds
            );

            // Sleep until midnight
            tokio::time::sleep(duration_until_midnight).await;
            println!("Midnight reached - producing block...");

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
        let current_date = now.date_naive();

        // Get genesis date from blockchain state
        let blockchain = self.blockchain.read().await;
        let genesis_block = match blockchain.get_block_by_height(0) {
            Some(block) => block,
            None => {
                // No genesis block yet - node is still syncing
                println!("⏳ Waiting for genesis block to be downloaded...");
                drop(blockchain);
                return; // Exit catch-up, will retry on next cycle
            }
        };
        let genesis_date = genesis_block.header.timestamp.date_naive();
        drop(blockchain);

        let days_since_genesis = (current_date - genesis_date).num_days();
        let expected_height = days_since_genesis as u64;

        let actual_height = self.load_block_height().await;

        println!("🔍 Catch-up check:");
        println!("   Current height: {}", actual_height);
        println!("   Expected height: {}", expected_height);

        if actual_height >= expected_height {
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
                    let url = format!("http://{}:24101/blockchain/info", peer_ip);
                    if let Ok(response) = reqwest::get(&url).await {
                        if let Ok(info) = response.json::<BlockchainInfo>().await {
                            peer_heights.push((peer_ip.clone(), info.height));
                        }
                    }
                }

                // Sort by height descending
                peer_heights.sort_by(|a, b| b.1.cmp(&a.1));

                // Try downloading complete chain from longest chains first
                for (peer_ip, peer_height) in peer_heights {
                    if peer_height == 0 {
                        continue;
                    }

                    println!(
                        "      🔗 Trying full chain download from {} (height {})...",
                        peer_ip, peer_height
                    );

                    let sync_to_height = std::cmp::min(peer_height, expected_height);
                    let mut all_blocks = Vec::new();
                    let mut download_success = true;

                    // Download all blocks first before adding any
                    for height in 1..=sync_to_height {
                        let url = format!("http://{}:24101/blockchain/block/{}", peer_ip, height);

                        match reqwest::Client::new()
                            .get(&url)
                            .timeout(std::time::Duration::from_secs(10))
                            .send()
                            .await
                        {
                            Ok(resp) => {
                                if let Ok(json) = resp.json::<serde_json::Value>().await {
                                    if let Some(block_data) = json.get("block") {
                                        if let Ok(block) =
                                            serde_json::from_value::<time_core::block::Block>(
                                                block_data.clone(),
                                            )
                                        {
                                            all_blocks.push((height, block));
                                        } else {
                                            download_success = false;
                                            break;
                                        }
                                    } else {
                                        download_success = false;
                                        break;
                                    }
                                } else {
                                    download_success = false;
                                    break;
                                }
                            }
                            Err(_) => {
                                download_success = false;
                                break;
                            }
                        }
                    }

                    if download_success && !all_blocks.is_empty() {
                        println!(
                            "      ✓ Downloaded {} blocks, validating chain...",
                            all_blocks.len()
                        );

                        // Try to add all blocks sequentially
                        let mut added = 0;
                        let mut blockchain = self.blockchain.write().await;
                        for (height, block) in all_blocks {
                            match blockchain.add_block(block) {
                                Ok(_) => {
                                    added += 1;
                                    if added % 5 == 0 {
                                        println!("         ✓ Validated {} blocks...", added);
                                    }
                                }
                                Err(e) => {
                                    println!(
                                        "         ✗ Block #{} validation failed: {:?}",
                                        height, e
                                    );
                                    // This peer's chain is invalid, try next peer
                                    break;
                                }
                            }
                        }
                        drop(blockchain);

                        if added > 0 {
                            println!(
                                "      ✅ Successfully synced {} blocks from {}!",
                                added, peer_ip
                            );
                            synced_any = true;
                            break; // Successfully synced, exit peer loop
                        }
                    } else {
                        println!("      ✗ Failed to download complete chain from {}", peer_ip);
                    }
                }
            } else {
                // Not at genesis, use incremental sync
                for peer_ip in &peers {
                    let url = format!("http://{}:24101/blockchain/info", peer_ip);
                    if let Ok(response) = reqwest::get(&url).await {
                        if let Ok(info) = response.json::<BlockchainInfo>().await {
                            // If peer has blocks we need, try to sync
                            if info.height > current_height {
                                println!(
                                    "      🔗 Peer {} has height {}, downloading blocks {}-{}...",
                                    peer_ip,
                                    info.height,
                                    current_height + 1,
                                    std::cmp::min(info.height, expected_height)
                                );

                                let sync_to_height = std::cmp::min(info.height, expected_height);
                                let mut downloaded = 0;

                                for height in (current_height + 1)..=sync_to_height {
                                    let url = format!(
                                        "http://{}:24101/blockchain/block/{}",
                                        peer_ip, height
                                    );

                                    match reqwest::Client::new()
                                        .get(&url)
                                        .timeout(std::time::Duration::from_secs(10))
                                        .send()
                                        .await
                                    {
                                        Ok(resp) => {
                                            if let Ok(json) = resp.json::<serde_json::Value>().await
                                            {
                                                if let Some(block_data) = json.get("block") {
                                                    if let Ok(block) = serde_json::from_value::<
                                                        time_core::block::Block,
                                                    >(
                                                        block_data.clone()
                                                    ) {
                                                        let mut blockchain =
                                                            self.blockchain.write().await;
                                                        match blockchain.add_block(block) {
                                                            Ok(_) => {
                                                                downloaded += 1;
                                                                println!(
                                                                    "         ✓ Block #{} downloaded",
                                                                    height
                                                                );
                                                                synced_any = true;
                                                            }
                                                            Err(e) => {
                                                                println!(
                                                                    "         ✗ Failed to add block #{}: {:?}",
                                                                    height, e
                                                                );
                                                                // Skip this peer and try next
                                                                break;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        Err(_) => {
                                            // Network error, try next peer
                                            break;
                                        }
                                    }
                                }

                                if downloaded > 0 {
                                    println!(
                                        "      ✓ Downloaded {} blocks from {}",
                                        downloaded, peer_ip
                                    );
                                }
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
            let url = format!("http://{}:24101/blockchain/info", masternode_ip);
            match reqwest::get(&url).await {
                Ok(response) => {
                    if let Ok(info) = response.json::<BlockchainInfo>().await {
                        peer_heights.push((masternode_ip.clone(), info.height));
                        println!("      {} is at height {}", masternode_ip, info.height);
                    }
                }
                Err(_) => {
                    println!("      ⚠️  Could not reach {}", masternode_ip);
                }
            }
        }

        // Check if anyone is significantly behind
        let min_height = peer_heights.iter().map(|(_, h)| *h).min().unwrap_or(0);
        if min_height + 1 < start_from_height {
            println!(
                "   ⚠️  Cannot create blocks - some masternodes don't have required base blocks"
            );
            println!(
                "      We're at height {}, but lowest peer is at height {}",
                our_height, min_height
            );
            println!(
                "      All peers must have block {} before we can create block {}",
                start_from_height - 1,
                start_from_height
            );
            println!("   ℹ️  Waiting for peers to sync genesis/base blocks first...");
            return;
        }

        // Create catch-up blocks
        let still_missing = expected_height - self.load_block_height().await;
        println!(
            "   Processing with BFT consensus: {} missed block(s)...",
            still_missing
        );

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

            let timestamp_date = genesis_date + chrono::Duration::days(block_num as i64);
            let timestamp = Utc.from_utc_datetime(&timestamp_date.and_hms_opt(0, 0, 0).unwrap());

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
                        }

                        // Give it a moment to be processed
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        let current_height = self.load_block_height().await;
                        if current_height >= block_num {
                            block_received = true;
                            break;
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

        // Initialize health tracking for any new masternodes
        for mn in &all_masternodes {
            self.block_consensus
                .init_masternode_health(mn.clone())
                .await;
        }

        // Get only active masternodes for consensus
        let mut masternodes = self
            .block_consensus
            .get_active_masternodes(&all_masternodes)
            .await;

        // Filter out quarantined masternodes
        let mut quarantined_count = 0;
        masternodes.retain(|mn| {
            if let Ok(ip_addr) = mn.parse::<std::net::IpAddr>() {
                let is_quarantined = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(self.quarantine.is_quarantined(&ip_addr))
                });
                if is_quarantined {
                    quarantined_count += 1;
                    return false;
                }
            }
            true
        });

        if quarantined_count > 0 {
            println!(
                "   🚨 {} masternode(s) quarantined and excluded from consensus",
                quarantined_count
            );
        }

        let required_votes = ((masternodes.len() * 2) / 3) + 1;

        if masternodes.len() < all_masternodes.len() {
            println!(
                "   ⚠️  {} masternode(s) excluded from consensus",
                all_masternodes.len() - masternodes.len()
            );
        }

        let selected_producer = self.consensus.get_leader(block_num).await;
        let my_id = self.get_node_id();
        let am_i_leader = selected_producer
            .as_ref()
            .map(|p| p == &my_id)
            .unwrap_or(false);

        // Log leader selection for debugging
        println!(
            "   🎯 Leader selection (VRF-based on {} total masternodes)",
            all_masternodes.len()
        );
        println!(
            "      Selected leader: {}",
            selected_producer.as_ref().unwrap_or(&"none".to_string())
        );
        println!("      Active masternodes: {}", masternodes.len());

        if am_i_leader {
            println!("{}", "   🟢 I am the block producer".green().bold());

            // Get transactions from mempool that are valid for this block's timestamp
            let block_timestamp = now.timestamp();
            let all_transactions = self.mempool.get_all_transactions().await;
            let mut transactions: Vec<_> = all_transactions
                .into_iter()
                .filter(|tx| tx.timestamp <= block_timestamp)
                .collect();

            // Sort transactions deterministically by txid to ensure same merkle root
            transactions.sort_by(|a, b| a.txid.cmp(&b.txid));

            // Check for approved proposals and add treasury grants
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
                        // Create treasury grant transaction
                        let grant_tx = time_core::transaction::Transaction {
                            txid: format!("treasury_grant_{}", proposal.id),
                            version: 1,
                            inputs: vec![], // No inputs = treasury grant
                            outputs: vec![time_core::transaction::TxOutput::new(
                                proposal.amount,
                                proposal.recipient.clone(),
                            )],
                            lock_time: 0,
                            timestamp: chrono::Utc::now().timestamp(),
                        };

                        println!(
                            "      ✓ Grant: {} TIME to {}",
                            proposal.amount as f64 / 100_000_000.0,
                            proposal.recipient
                        );
                        println!("         Reason: {}", proposal.reason);

                        transactions.push(grant_tx);

                        // Mark proposal as executed (will be persisted after block)
                        let _ = proposal_manager
                            .mark_executed(&proposal.id, format!("treasury_grant_{}", proposal.id))
                            .await;
                    }
                }
            }

            // Get blockchain state atomically (all data retrieved while holding read lock)
            let blockchain = self.blockchain.read().await;
            let previous_hash = blockchain.chain_tip_hash().to_string();
            let masternode_counts = blockchain.masternode_counts().clone();

            // Get active masternodes with their wallet addresses and tiers
            // Note: masternode_counts and active_masternodes represent the same state
            let active_masternodes: Vec<(String, time_core::MasternodeTier)> = blockchain
                .get_active_masternodes()
                .iter()
                .map(|mn| (mn.wallet_address.clone(), mn.tier))
                .collect();

            drop(blockchain);

            // Check for empty mempool - use deterministic reward-only block path
            let is_reward_only = transactions.is_empty();

            if is_reward_only {
                println!("   📭 Mempool is empty - creating deterministic reward-only block");
                println!("   ⚡ This should achieve instant consensus (identical blocks)");

                // Use deterministic reward-only block creation
                let block = time_core::block::create_reward_only_block(
                    block_num,
                    previous_hash.clone(),
                    my_id.clone(),
                    &active_masternodes,
                    &masternode_counts,
                );

                // Create proposal with reward-only flag
                let proposal = time_consensus::block_consensus::BlockProposal {
                    block_height: block_num,
                    proposer: my_id.clone(),
                    block_hash: block.hash.clone(),
                    merkle_root: block.header.merkle_root.clone(),
                    previous_hash: previous_hash.clone(),
                    timestamp: block.header.timestamp.timestamp(),
                    is_reward_only: true,
                    strategy: None, // Regular block production doesn't use foolproof strategies
                };

                self.block_consensus.store_proposal(proposal.clone()).await;

                // Leader auto-votes on their own proposal
                let leader_vote = time_consensus::block_consensus::BlockVote {
                    block_height: block_num,
                    block_hash: block.hash.clone(),
                    voter: my_id.clone(),
                    approve: true,
                    timestamp: chrono::Utc::now().timestamp(),
                };
                if let Err(e) = self
                    .block_consensus
                    .vote_on_block(leader_vote.clone())
                    .await
                {
                    println!("   ⚠️  Leader failed to record own vote: {}", e);
                } else {
                    println!("   ✓ Leader auto-voted APPROVE");
                }

                let proposal_json = serde_json::to_value(&proposal).unwrap();
                self.peer_manager
                    .broadcast_block_proposal(proposal_json)
                    .await;

                // Also broadcast leader's vote
                let vote_json = serde_json::to_value(&leader_vote).unwrap();
                self.peer_manager.broadcast_block_vote(vote_json).await;

                println!("   📡 Reward-only proposal and vote broadcast");

                // Give peers time to receive and process proposal (2 second buffer)
                println!("   ⏳ Waiting 2s for peers to receive proposal...");
                tokio::time::sleep(Duration::from_secs(2)).await;

                println!(
                    "   ▶️ Collecting votes (need {}/{})...",
                    required_votes,
                    masternodes.len()
                );

                // Collect votes for reward-only block
                let (approved, total) = self
                    .block_consensus
                    .collect_votes_with_timeout(block_num, required_votes, 60)
                    .await;

                println!("   🗳️  Votes: {}/{}", approved, total);

                if approved >= required_votes {
                    println!("   ✔ Quorum reached! Finalizing reward-only block...");

                    // Apply the deterministic block
                    let mut blockchain_write = self.blockchain.write().await;
                    match blockchain_write.add_block(block.clone()) {
                        Ok(_) => {
                            println!("   ✅ Reward-only block {} finalized", block_num);
                            // Broadcast to peers
                            drop(blockchain_write);
                            self.broadcast_finalized_block(&block, &masternodes).await;
                        }
                        Err(e) => {
                            println!("   ❌ Failed to finalize reward-only block: {:?}", e);
                        }
                    }
                } else {
                    println!("   ❌ FAILED: Insufficient votes ({}/{})", approved, total);
                }

                return;
            }

            // Regular path: mempool has transactions
            println!("   📋 {} mempool transactions", transactions.len());

            let total_fees = self.calculate_total_fees(&transactions).await;

            if total_fees > 0 {
                println!(
                    "   💵 Total transaction fees: {} satoshis ({} TIME)",
                    total_fees,
                    total_fees as f64 / 100_000_000.0
                );
            }

            // Log masternode reward distribution
            if !active_masternodes.is_empty() {
                println!(
                    "   💰 Distributing rewards to {} masternodes:",
                    active_masternodes.len()
                );

                // Calculate what each tier will receive
                let total_pool =
                    time_core::block::calculate_total_masternode_reward(&masternode_counts);
                let total_weight = masternode_counts.total_weight();
                let per_weight = if total_weight > 0 {
                    total_pool / total_weight
                } else {
                    0
                };

                println!(
                    "      Total reward pool: {} satoshis ({} TIME)",
                    total_pool,
                    total_pool / 100_000_000
                );
                println!("      Total weight: {}", total_weight);
                println!("      Per weight unit: {} satoshis", per_weight);

                // Show breakdown by tier
                let mut tier_summary: std::collections::HashMap<
                    time_core::MasternodeTier,
                    (usize, u64),
                > = std::collections::HashMap::new();

                for (wallet_addr, tier) in &active_masternodes {
                    let reward = per_weight * tier.weight();
                    let entry = tier_summary.entry(*tier).or_insert((0, 0));
                    entry.0 += 1;
                    entry.1 += reward;

                    println!(
                        "      - {:?} tier ({} weight): {} → {} satoshis ({:.2} TIME)",
                        tier,
                        tier.weight(),
                        if wallet_addr.len() >= 20 {
                            &wallet_addr[..20]
                        } else {
                            wallet_addr.as_str()
                        },
                        reward,
                        reward as f64 / 100_000_000.0
                    );
                }

                println!("   📊 Reward Summary:");
                for (tier, (count, total_reward)) in tier_summary.iter() {
                    println!(
                        "      {:?}: {} nodes, {} satoshis total ({:.2} TIME each)",
                        tier,
                        count,
                        total_reward,
                        (*total_reward as f64 / *count as f64) / 100_000_000.0
                    );
                }
            } else {
                println!("   ⚠️  No active masternodes registered for rewards");
            }

            let block_timestamp = now.timestamp();
            let coinbase_tx = time_core::block::create_coinbase_transaction(
                block_num,
                &active_masternodes,
                &masternode_counts,
                total_fees,
                block_timestamp,
            );

            let mut all_transactions = vec![coinbase_tx];
            let mempool_count = transactions.len();
            all_transactions.extend(transactions);

            self.log_coinbase_info(&all_transactions, mempool_count);

            let merkle_root = self.calc_merkle(&all_transactions);
            let block_hash = self.calculate_block_hash(
                block_num,
                &now,
                &previous_hash,
                &merkle_root,
                &my_id,
                &masternode_counts,
            );

            let proposal = time_consensus::block_consensus::BlockProposal {
                block_height: block_num,
                proposer: my_id.clone(),
                block_hash: block_hash.clone(),
                merkle_root: merkle_root.clone(),
                previous_hash: previous_hash.clone(),
                timestamp: now.timestamp(),
                is_reward_only: false,
                strategy: None, // Regular block production doesn't use foolproof strategies
            };

            self.block_consensus.store_proposal(proposal.clone()).await;

            // Leader auto-votes on their own proposal
            let leader_vote = time_consensus::block_consensus::BlockVote {
                block_height: block_num,
                block_hash: block_hash.clone(),
                voter: my_id.clone(),
                approve: true,
                timestamp: chrono::Utc::now().timestamp(),
            };
            if let Err(e) = self
                .block_consensus
                .vote_on_block(leader_vote.clone())
                .await
            {
                println!("   ⚠️  Leader failed to record own vote: {}", e);
            } else {
                println!("   ✓ Leader auto-voted APPROVE");
            }

            let proposal_json = serde_json::to_value(&proposal).unwrap();
            self.peer_manager
                .broadcast_block_proposal(proposal_json)
                .await;

            // Also broadcast leader's vote
            let vote_json = serde_json::to_value(&leader_vote).unwrap();
            self.peer_manager.broadcast_block_vote(vote_json).await;

            println!("   📡 Proposal and vote broadcast");

            // Give peers time to receive and process proposal (2 second buffer)
            println!("   ⏳ Waiting 2s for peers to receive proposal...");
            tokio::time::sleep(Duration::from_secs(2)).await;

            println!(
                "   ▶️ Collecting votes (need {}/{})...",
                required_votes,
                masternodes.len()
            );

            self.collect_and_finalize_votes(
                &all_transactions,
                &previous_hash,
                &merkle_root,
                block_num,
                required_votes,
                &masternodes,
                &proposal,
            )
            .await;
        } else {
            println!(
                "   ℹ️  Producer: {}",
                selected_producer.as_deref().unwrap_or("unknown")
            );
            println!("   ⏳ Waiting for proposal...");

            if let Some(proposal) = self.block_consensus.wait_for_proposal(block_num).await {
                println!("   📨 Received from {}", proposal.proposer);

                let blockchain = self.blockchain.read().await;
                let chain_tip_hash = blockchain.chain_tip_hash().to_string();
                let chain_tip_height = blockchain.chain_tip_height();
                let masternode_counts = blockchain.masternode_counts().clone();
                let active_masternodes: Vec<(String, time_core::MasternodeTier)> = blockchain
                    .get_active_masternodes()
                    .iter()
                    .map(|mn| (mn.wallet_address.clone(), mn.tier))
                    .collect();
                drop(blockchain);

                let mut is_valid = self.block_consensus.validate_proposal(
                    &proposal,
                    &chain_tip_hash,
                    chain_tip_height,
                );

                // Fast-path validation for reward-only blocks
                if proposal.is_reward_only {
                    println!("   🚀 Reward-only block - using fast-path validation");

                    // Recreate the deterministic block locally
                    let expected_block = time_core::block::create_reward_only_block(
                        block_num,
                        chain_tip_hash.clone(),
                        proposal.proposer.clone(),
                        &active_masternodes,
                        &masternode_counts,
                    );

                    // Verify the block hash matches
                    if expected_block.hash == proposal.block_hash {
                        println!("   ✅ Reward-only block verified - auto-approving");
                        is_valid = true;
                    } else {
                        println!("   ❌ Reward-only block mismatch - rejecting");
                        println!("      Expected hash: {}", expected_block.hash);
                        println!("      Received hash: {}", proposal.block_hash);
                        is_valid = false;
                    }
                }

                let vote = time_consensus::block_consensus::BlockVote {
                    block_height: block_num,
                    block_hash: proposal.block_hash.clone(),
                    voter: my_id.clone(),
                    approve: is_valid,
                    timestamp: Utc::now().timestamp(),
                };

                // Use vote_on_block() instead of store_vote() to ensure proper validation
                if let Err(e) = self.block_consensus.vote_on_block(vote.clone()).await {
                    println!("   ⚠️  Failed to record vote: {}", e);
                }

                let vote_json = serde_json::to_value(&vote).unwrap();
                self.peer_manager.broadcast_block_vote(vote_json).await;

                println!(
                    "   {} Voted {}",
                    if is_valid { "✓" } else { "✗" },
                    if is_valid { "APPROVE" } else { "REJECT" }
                );

                let (approved, _total) = self
                    .block_consensus
                    .collect_votes(block_num, required_votes)
                    .await;

                if approved >= required_votes {
                    // For reward-only blocks, we can recreate locally instead of fetching
                    if proposal.is_reward_only {
                        println!("   ✅ Reward-only block approved - applying locally...");

                        let reward_block = time_core::block::create_reward_only_block(
                            block_num,
                            chain_tip_hash,
                            proposal.proposer.clone(),
                            &active_masternodes,
                            &masternode_counts,
                        );

                        let mut blockchain = self.blockchain.write().await;
                        match blockchain.add_block(reward_block) {
                            Ok(_) => {
                                println!("   ✅ Reward-only block {} applied locally", block_num);

                                // Save UTXO snapshot to persist state
                                if let Err(e) = blockchain.save_utxo_snapshot() {
                                    println!("   ⚠️  Failed to save UTXO snapshot: {}", e);
                                } else {
                                    println!("   💾 UTXO snapshot saved");
                                }
                            }
                            Err(e) => {
                                println!("   ⚠️  Failed to apply reward-only block: {:?}", e);
                            }
                        }
                    } else {
                        println!("   ✅ Block approved - fetching finalized block...");

                        // Actively fetch the finalized block from producer
                        if let Some(producer_id) = selected_producer {
                            if let Some(block) = self
                                .fetch_finalized_block(
                                    &producer_id,
                                    block_num,
                                    &proposal.merkle_root,
                                )
                                .await
                            {
                                // Apply the finalized block
                                let mut blockchain = self.blockchain.write().await;
                                match blockchain.add_block(block) {
                                    Ok(_) => {
                                        println!("   ✅ Block {} applied from producer", block_num);

                                        // Save UTXO snapshot to persist state
                                        if let Err(e) = blockchain.save_utxo_snapshot() {
                                            println!("   ⚠️  Failed to save UTXO snapshot: {}", e);
                                        } else {
                                            println!("   💾 UTXO snapshot saved");
                                        }
                                    }
                                    Err(e) => {
                                        println!("   ⚠️  Failed to apply fetched block: {:?}", e);
                                        println!("   ⏳ Falling back to catch-up...");
                                    }
                                }
                            } else {
                                println!("   ⚠️  Failed to fetch block, falling back to catch-up");
                            }
                        }
                    }
                } else {
                    println!("   ✗ Block rejected");
                }
            } else {
                println!("   ⚠️  Timeout");
            }
        }
    }

    fn calc_merkle(&self, transactions: &[time_core::Transaction]) -> String {
        if transactions.is_empty() {
            return "0".repeat(64);
        }

        use sha3::{Digest, Sha3_256};

        // Build proper merkle tree (matching Block::calculate_merkle_root in core/src/block.rs)
        let mut hashes: Vec<String> = transactions.iter().map(|tx| tx.txid.clone()).collect();

        // Build merkle tree iteratively
        while hashes.len() > 1 {
            let mut next_level = Vec::new();

            for i in (0..hashes.len()).step_by(2) {
                let left = &hashes[i];
                let right = if i + 1 < hashes.len() {
                    &hashes[i + 1]
                } else {
                    left // Duplicate if odd number
                };

                let combined = format!("{}{}", left, right);
                let hash = Sha3_256::digest(combined.as_bytes());
                next_level.push(hex::encode(hash));
            }

            hashes = next_level;
        }

        hashes[0].clone()
    }

    /// Broadcast finalized block to peers (best-effort)
    async fn broadcast_finalized_block(
        &self,
        block: &time_core::block::Block,
        masternodes: &[String],
    ) {
        let block_json = match serde_json::to_value(block) {
            Ok(json) => json,
            Err(e) => {
                println!("   ⚠️  Failed to serialize block for broadcast: {:?}", e);
                return;
            }
        };

        let payload = serde_json::json!({
            "block": block_json
        });

        for node in masternodes {
            let url = format!("http://{}:24101/consensus/finalized-block", node);
            let payload_clone = payload.clone();

            // Fire-and-forget, best-effort broadcast
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                if let Err(e) = client
                    .post(&url)
                    .json(&payload_clone)
                    .timeout(std::time::Duration::from_secs(2))
                    .send()
                    .await
                {
                    // Log warning but don't fail - best effort only
                    eprintln!("   ⚠️  Failed to broadcast to {}: {:?}", url, e);
                }
            });
        }
    }

    /// Attempt to fetch finalized block from producer with retries
    async fn fetch_finalized_block(
        &self,
        producer: &str,
        height: u64,
        expected_merkle: &str,
    ) -> Option<time_core::block::Block> {
        const MAX_ATTEMPTS: u32 = 8;
        const RETRY_DELAY_MS: u64 = 500;

        for attempt in 1..=MAX_ATTEMPTS {
            let url = format!("http://{}:24101/consensus/block/{}", producer, height);

            match reqwest::Client::new()
                .get(&url)
                .timeout(std::time::Duration::from_secs(2))
                .send()
                .await
            {
                Ok(response) => {
                    if let Ok(json) = response.json::<serde_json::Value>().await {
                        if let Some(block_data) = json.get("block") {
                            if let Ok(block) = serde_json::from_value::<time_core::block::Block>(
                                block_data.clone(),
                            ) {
                                // Validate merkle root matches proposal
                                if block.header.merkle_root == expected_merkle {
                                    println!("   ✅ Fetched finalized block from {}", producer);
                                    return Some(block);
                                } else {
                                    println!(
                                        "   ⚠️  Merkle mismatch: expected {}, got {}",
                                        &expected_merkle[..16],
                                        &block.header.merkle_root[..16]
                                    );
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    if attempt < MAX_ATTEMPTS {
                        println!(
                            "   ⏳ Fetch attempt {}/{} failed, retrying... ({:?})",
                            attempt, MAX_ATTEMPTS, e
                        );
                        tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                    } else {
                        println!("   ⚠️  All fetch attempts failed: {:?}", e);
                    }
                }
            }
        }

        None
    }

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
                } else {
                    println!("   💾 UTXO snapshot saved");
                }

                // Remove finalized transactions that were included in this block
                for tx in block.transactions.iter().skip(1) {
                    // Skip coinbase (first transaction)
                    if let Err(e) = blockchain.remove_finalized_tx(&tx.txid) {
                        println!(
                            "   ⚠️  Failed to remove finalized tx {}: {}",
                            &tx.txid[..16],
                            e
                        );
                    }
                }

                let block_hash = block.hash.clone();
                drop(blockchain);

                self.peer_manager
                    .broadcast_tip_update(block_num, block_hash)
                    .await;

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
        println!("      Block Hash: {}...", &block.hash[..16]);

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

            println!("      ✓ Block created: {}...", &block.hash[..16]);

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

            // Broadcast proposal to all peers IN PARALLEL
            self.broadcast_block_proposal(proposal.clone(), masternodes)
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
                println!("      ✓ Voted APPROVE on block {}", &block.hash[..16]);
            }

            // Broadcast vote IN PARALLEL
            self.broadcast_block_vote(vote, masternodes).await;

            // ULTRA-FAST consensus check - 3 second timeout, 50ms polling
            println!("      ⚡ Ultra-fast consensus check...");
            let start_time = std::time::Instant::now();
            let timeout = Duration::from_secs(3);
            let mut last_log_time = start_time;
            let mut last_vote_count = 0;
            let mut stall_count = 0;
            const MAX_STALLS: u32 = 6; // Exit after 300ms of no progress (6 * 50ms)

            // NO artificial delay - check immediately
            while start_time.elapsed() < timeout {
                // FAST polling - check every 50ms instead of 1 second
                tokio::time::sleep(Duration::from_millis(50)).await;

                if let Some(proposal) = self.block_consensus.get_proposal(block_num).await {
                    // Check consensus (need 2/3+ votes)
                    let required_votes = ((masternodes.len() * 2) / 3) + 1;
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
                let required_votes = ((masternodes.len() * 2) / 3) + 1;
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

                // FALLBACK: If on final attempt and we have at least 1 vote (ourselves),
                // unilaterally finalize to keep chain moving during version upgrades
                if attempt == 2 && approvals >= 1 {
                    println!("         ⚠️  EMERGENCY FALLBACK: Finalizing with single vote");
                    println!("         ℹ️  This allows progress during version upgrades");
                    // Use the block we already created and voted on
                    return self.finalize_agreed_block(block.clone(), masternodes).await;
                }
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

        // Get ALL registered masternodes (not just active ones)
        let all_masternodes = self.consensus.get_masternodes().await;

        let active_masternodes: Vec<(String, time_core::MasternodeTier)> = blockchain
            .get_active_masternodes()
            .iter()
            .map(|mn| (mn.wallet_address.clone(), mn.tier))
            .collect();

        drop(blockchain);

        // Get pending transactions from mempool that belong to this block's timeframe
        // Only include transactions that were created BEFORE or ON this block's timestamp
        let block_timestamp = timestamp.timestamp();
        let all_mempool_txs = self.mempool.get_all_transactions().await;
        let mut mempool_txs: Vec<_> = all_mempool_txs
            .into_iter()
            .filter(|tx| tx.timestamp <= block_timestamp)
            .collect();

        println!(
            "      💰 Catch-up block will reward {} masternodes",
            active_masternodes.len()
        );
        println!(
            "      🔍 Using {} registered masternodes for consensus",
            all_masternodes.len()
        );
        println!(
            "      📦 Including {} transactions from mempool (filtered by timestamp <= {})",
            mempool_txs.len(),
            timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );

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
            &block.hash[..16]
        );
        block
    }

    #[allow(dead_code)]
    async fn broadcast_block_proposal(
        &self,
        proposal: time_consensus::block_consensus::BlockProposal,
        masternodes: &[String],
    ) {
        // ULTRA-FAST PARALLEL BROADCAST - all requests fire simultaneously
        let mut handles = Vec::new();

        for node in masternodes {
            let url = format!("http://{}:24101/consensus/block-proposal", node);
            let proposal_clone = proposal.clone();

            let handle = tokio::spawn(async move {
                // OPTIMIZED: 100ms timeout for LAN, fire-and-forget
                let _ = reqwest::Client::new()
                    .post(&url)
                    .json(&proposal_clone)
                    .timeout(Duration::from_millis(100)) // REDUCED from 2s to 100ms
                    .send()
                    .await;
            });
            handles.push(handle);
        }

        // Wait for all broadcasts to complete (or timeout)
        let _ = futures::future::join_all(handles).await;
    }

    #[allow(dead_code)]
    async fn broadcast_block_vote(
        &self,
        vote: time_consensus::block_consensus::BlockVote,
        masternodes: &[String],
    ) {
        // ULTRA-FAST PARALLEL BROADCAST - all votes sent simultaneously
        let mut handles = Vec::new();

        for node in masternodes {
            let url = format!("http://{}:24101/consensus/block-vote", node);
            let vote_clone = vote.clone();

            let handle = tokio::spawn(async move {
                // OPTIMIZED: 100ms timeout for LAN speed
                let _ = reqwest::Client::new()
                    .post(&url)
                    .json(&vote_clone)
                    .timeout(Duration::from_millis(100)) // REDUCED from 2s to 100ms
                    .send()
                    .await;
            });
            handles.push(handle);
        }

        // Wait for all broadcasts (or timeout)
        let _ = futures::future::join_all(handles).await;
    }

    #[allow(dead_code)]
    async fn notify_leader_to_produce_block(
        &self,
        leader_ip: &str,
        block_height: u64,
        requester_ip: &str,
    ) {
        let request = serde_json::json!({
            "block_height": block_height,
            "leader_ip": leader_ip,
            "requester_ip": requester_ip,
        });

        let url = format!(
            "http://{}:24101/consensus/request-block-proposal",
            leader_ip
        );
        let result = reqwest::Client::new()
            .post(&url)
            .json(&request)
            .timeout(Duration::from_secs(3))
            .send()
            .await;

        match result {
            Ok(response) if response.status().is_success() => {
                println!("      ✓ Leader {} acknowledged the request", leader_ip);
            }
            Ok(response) => {
                println!(
                    "      ⚠️  Leader {} responded with status: {}",
                    leader_ip,
                    response.status()
                );
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
        println!("      📋 Block hash: {}...", &block.hash[..16]);

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

                // Remove transactions from mempool (skip coinbase at index 0)
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
        for peer_ip in peers {
            let url = format!("http://{}:24101/consensus/finalized-block", peer_ip);
            let block_clone = block.clone();
            tokio::spawn(async move {
                let _ = reqwest::Client::new()
                    .post(&url)
                    .json(&block_clone)
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await;
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
            "   📡 Notifying {} peer(s) of catch-up need...",
            peers.len()
        );

        for peer_ip in peers {
            let url = format!("http://{}:24101/network/catch-up-request", peer_ip);
            let request = serde_json::json!({
                "requester": node_id,
                "current_height": current_height,
                "expected_height": expected_height,
            });

            // Fire and forget - don't wait for responses
            tokio::spawn(async move {
                let _ = reqwest::Client::new()
                    .post(&url)
                    .json(&request)
                    .timeout(std::time::Duration::from_secs(3))
                    .send()
                    .await;
            });
        }
    }
}
