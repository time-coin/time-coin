use chrono::{Timelike, Utc};
use serde::Deserialize;
use std::net::IpAddr;
use std::sync::Arc;
use time_core::block::Block;
use time_core::state::BlockchainState;
use time_network::{PeerManager, PeerQuarantine, QuarantineReason};
use tokio::sync::RwLock;

/// Helper to safely get hash preview
fn hash_preview(hash: &str) -> &str {
    if hash.len() >= 16 {
        &hash[..16]
    } else {
        hash
    }
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct BlockchainInfo {
    pub height: u64,
    pub best_block_hash: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct BlockResponse {
    block: Block,
}

/// Configuration for midnight window
#[derive(Debug, Clone)]
pub struct MidnightWindowConfig {
    /// Hours before midnight to start the window (e.g., 23 for 11 PM)
    pub start_hour: u32,
    /// Hours after midnight to end the window (e.g., 1 for 1 AM)
    pub end_hour: u32,
    /// Whether to check consensus status before skipping updates
    pub check_consensus: bool,
}

impl Default for MidnightWindowConfig {
    fn default() -> Self {
        Self {
            start_hour: 23, // 11 PM
            end_hour: 1,    // 1 AM
            check_consensus: true,
        }
    }
}

pub struct ChainSync {
    blockchain: Arc<RwLock<BlockchainState>>,
    peer_manager: Arc<PeerManager>,
    midnight_config: Option<MidnightWindowConfig>,
    is_block_producer_active: Arc<RwLock<bool>>,
    quarantine: Arc<PeerQuarantine>,
}

impl ChainSync {
    #[allow(dead_code)]
    pub fn new(blockchain: Arc<RwLock<BlockchainState>>, peer_manager: Arc<PeerManager>) -> Self {
        Self {
            blockchain,
            peer_manager,
            midnight_config: Some(MidnightWindowConfig::default()),
            is_block_producer_active: Arc::new(RwLock::new(false)),
            quarantine: Arc::new(PeerQuarantine::new()),
        }
    }

    /// Create ChainSync with custom midnight window configuration and block producer state
    pub fn with_midnight_config(
        blockchain: Arc<RwLock<BlockchainState>>,
        peer_manager: Arc<PeerManager>,
        midnight_config: Option<MidnightWindowConfig>,
        is_block_producer_active: Arc<RwLock<bool>>,
    ) -> Self {
        Self {
            blockchain,
            peer_manager,
            midnight_config,
            is_block_producer_active,
            quarantine: Arc::new(PeerQuarantine::new()),
        }
    }

    /// Get the quarantine system (for external access)
    pub fn quarantine(&self) -> Arc<PeerQuarantine> {
        self.quarantine.clone()
    }

    /// Get the correct TCP port based on network type
    fn get_p2p_port(&self) -> u16 {
        match self.peer_manager.network {
            time_network::discovery::NetworkType::Mainnet => 24000,
            time_network::discovery::NetworkType::Testnet => 24100,
        }
    }

    /// Check if current time is within the midnight window
    fn is_in_midnight_window(&self) -> bool {
        if let Some(config) = &self.midnight_config {
            let now = Utc::now();
            let hour = now.hour();

            // Handle window that spans midnight (e.g., 23:00 to 01:00)
            if config.start_hour > config.end_hour {
                hour >= config.start_hour || hour < config.end_hour
            } else {
                // Handle window that doesn't span midnight (e.g., 22:00 to 23:00)
                hour >= config.start_hour && hour < config.end_hour
            }
        } else {
            false
        }
    }

    /// Check if periodic sync should be skipped
    async fn should_skip_sync(&self) -> bool {
        // Never skip sync if we don't have genesis - critical for bootstrapping
        let blockchain = self.blockchain.read().await;
        let has_genesis = !blockchain.genesis_hash().is_empty();
        let our_height = blockchain.chain_tip_height();
        drop(blockchain);

        if !has_genesis {
            return false; // Always sync when we don't have genesis
        }

        if !self.is_in_midnight_window() {
            return false;
        }

        // We're in the midnight window - but ONLY skip if we're actually caught up
        // Check if we might be behind the network
        let peer_heights = self.query_peer_heights().await;
        if !peer_heights.is_empty() {
            let max_peer_height = peer_heights.iter().map(|(_, h, _)| h).max().unwrap_or(&0);
            if *max_peer_height > our_height {
                // We're behind - DO NOT skip sync
                println!("   ℹ️  In midnight window but behind network (our: {}, peer: {}) - syncing anyway", our_height, max_peer_height);
                return false;
            }
        }

        // We're in the midnight window and caught up
        if let Some(config) = &self.midnight_config {
            if config.check_consensus {
                // Check if consensus (block producer) is actively running
                let is_active = *self.is_block_producer_active.read().await;
                if is_active {
                    println!("   ⏸️  Skipping sync: in midnight window, caught up, and consensus is active");
                    return true;
                }
            } else {
                // Just skip during midnight window without checking consensus
                println!("   ⏸️  Skipping sync: in midnight window and caught up");
                return true;
            }
        }

        false
    }

    /// Check if we need genesis and download if missing (called on peer connect)
    ///
    /// This is the event-driven entry point called whenever a new peer connects.
    /// It checks if we're missing genesis and triggers download if needed.
    pub async fn on_peer_connected(&self) {
        // Quick check if we already have genesis (avoid expensive lock if not needed)
        let blockchain = self.blockchain.read().await;
        let has_genesis =
            blockchain.chain_tip_height() > 0 || !blockchain.genesis_hash().is_empty();
        drop(blockchain);

        if !has_genesis {
            println!("   🔍 New peer connected, missing genesis - attempting download...");
            if let Err(e) = self.try_download_genesis_from_all_peers().await {
                // Silent failure - will retry on next peer connection or periodic sync
                println!("   ℹ️  Genesis download failed: {} (will retry)", e);
            }
        }
    }

    /// Try to download genesis from any available peer
    ///
    /// This implements the network's genesis discovery protocol:
    /// 1. Query all peers to find those with a genesis block
    /// 2. Download the genesis block from the first responsive peer
    /// 3. Validate it's a proper genesis block (height 0, previous_hash = "0")
    /// 4. Accept it as THE authoritative genesis for this network
    ///
    /// This works for any network (testnet, mainnet, etc.) without hardcoding genesis blocks.
    pub async fn try_download_genesis_from_all_peers(&self) -> Result<(), String> {
        println!("   🔍 Searching all known peers for genesis block...");

        // Get ALL known peer IPs (not just connected ones)
        let peer_ips = self.peer_manager.get_peer_ips().await;
        let p2p_port = self.get_p2p_port();

        println!("   📋 Checking {} peer(s)...", peer_ips.len());

        for peer_ip in peer_ips {
            let peer_addr_with_port = format!("{}:{}", peer_ip, p2p_port);

            // Check if peer has genesis with a longer timeout (10s instead of 3s)
            // Genesis download is critical, so we give peers more time to respond
            let peer_info_result = tokio::time::timeout(
                tokio::time::Duration::from_secs(10),
                self.peer_manager
                    .request_blockchain_info(&peer_addr_with_port),
            )
            .await;

            let height = match peer_info_result {
                Ok(Ok(Some(h))) => h,
                Ok(Ok(None)) => {
                    eprintln!("   ⚠️  Peer {} has no genesis yet", peer_ip);
                    continue;
                }
                Ok(Err(e)) => {
                    eprintln!("   ⚠️  Could not query peer {} - error: {}", peer_ip, e);
                    continue;
                }
                Err(_) => {
                    eprintln!(
                        "   ⚠️  Could not query peer {} - timeout after 10s",
                        peer_ip
                    );
                    continue;
                }
            };

            println!("   📊 Peer {}: height={}", peer_ip, height);

            println!("   ✨ Peer {} has genesis block - downloading...", peer_ip);

            // Try to download genesis from this peer (always request block at height 0)
            println!("   📡 Requesting block 0 from {}...", peer_ip);
            eprintln!(
                "   [DEBUG] About to call request_block_by_height for peer {}",
                peer_ip
            );
            let genesis_block = {
                let genesis_result = tokio::time::timeout(
                    tokio::time::Duration::from_secs(10),
                    self.peer_manager
                        .request_block_by_height(&peer_addr_with_port, 0),
                )
                .await;

                eprintln!(
                    "   [DEBUG] request_block_by_height result for peer {}: {:?}",
                    peer_ip,
                    genesis_result
                        .as_ref()
                        .map(|r| r.as_ref().map(|_| "block").map_err(|e| e.to_string()))
                );

                match genesis_result {
                    Ok(Ok(block)) => {
                        println!("   ✅ Successfully received block from {}", peer_ip);
                        Some(block)
                    }
                    Ok(Err(e)) => {
                        eprintln!(
                            "   ⚠️  Download error from {}: {} - trying next",
                            peer_ip, e
                        );
                        None
                    }
                    Err(_) => {
                        eprintln!(
                            "   ⚠️  Download timeout from {} (10s) - trying next",
                            peer_ip
                        );
                        None
                    }
                }
            };

            let genesis_block = match genesis_block {
                Some(b) => b,
                None => continue,
            };

            // Validate it's actually a genesis block
            if genesis_block.header.block_number != 0 {
                println!(
                    "   ⚠️  Peer {} returned block at height {} instead of 0",
                    peer_ip, genesis_block.header.block_number
                );
                continue;
            }

            // Genesis block should have all zeros as previous_hash
            let is_zero_hash = genesis_block.header.previous_hash == "0"
                || genesis_block.header.previous_hash
                    == "0000000000000000000000000000000000000000000000000000000000000000"
                || genesis_block.header.previous_hash.chars().all(|c| c == '0');

            if !is_zero_hash {
                println!(
                    "   ⚠️  Peer {} returned invalid genesis (previous_hash = '{}')",
                    peer_ip, genesis_block.header.previous_hash
                );
                continue;
            }

            println!("   📥 Received valid genesis block from peer!");
            println!("   🔍 Block hash: {}", hash_preview(&genesis_block.hash));
            println!("   ℹ️  This is now THE authoritative genesis for this network");

            // Import the genesis block
            let mut blockchain = self.blockchain.write().await;
            if let Err(e) = blockchain.add_block(genesis_block.clone()) {
                println!(
                    "   ⚠️  Failed to import genesis from {}: {} - trying next",
                    peer_ip, e
                );
                continue;
            }

            println!(
                "✅ Genesis block downloaded and initialized: {}...",
                hash_preview(&genesis_block.hash)
            );
            println!("   ✓ Genesis block imported from {}", peer_ip);
            return Ok(());
        }

        Err("No peers with genesis block found".to_string())
    }

    /// Query all peers and find the highest blockchain height
    pub async fn query_peer_heights(&self) -> Vec<(String, u64, String)> {
        // Use all known peer IPs instead of just connected peers
        // This allows us to query peers even if we only have inbound connections from them
        let peer_ips = self.peer_manager.get_peer_ips().await;
        let mut peer_heights = Vec::new();

        // Query peers silently

        for peer_ip in peer_ips {
            // Skip quarantined peers
            if let Ok(peer_addr) = peer_ip.parse::<IpAddr>() {
                if self.quarantine.is_quarantined(&peer_addr).await {
                    if let Some(reason) = self.quarantine.get_reason(&peer_addr).await {
                        println!(
                            "   🚫 Skipping quarantined peer {} (reason: {})",
                            peer_ip, reason
                        );
                    }
                    continue;
                }
            }

            // Use TCP protocol instead of HTTP API
            let p2p_port = self.get_p2p_port();
            let peer_addr_with_port = format!("{}:{}", peer_ip, p2p_port);

            // Get peer info with aggressive 3-second timeout
            let peer_info = tokio::time::timeout(
                tokio::time::Duration::from_secs(3),
                self.peer_manager
                    .request_blockchain_info(&peer_addr_with_port),
            )
            .await
            .ok()
            .and_then(|r| r.ok())
            .flatten();

            if let Some(height) = peer_info {
                // Get the hash too - for now use a placeholder until we extend the protocol
                // The height is what matters most for sync
                peer_heights.push((peer_ip.clone(), height, String::new()));
            }
        }

        peer_heights
    }

    /// Download a specific block from a peer with timeout
    async fn download_block(&self, peer_ip: &str, height: u64) -> Option<Block> {
        let p2p_port = self.get_p2p_port();
        let peer_addr_with_port = format!("{}:{}", peer_ip, p2p_port);

        // Add 5-second timeout for block downloads
        tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            self.peer_manager
                .request_block_by_height(&peer_addr_with_port, height),
        )
        .await
        .ok()
        .and_then(|r| r.ok())
    }

    /// Validate a block before importing
    fn validate_block(&self, block: &Block, expected_prev_hash: &str) -> Result<(), String> {
        // Skip previous hash check for genesis block (height 0)
        if block.header.block_number > 0 {
            // Check previous hash matches
            if block.header.previous_hash != expected_prev_hash {
                return Err(format!(
                    "Invalid previous hash: expected {}, got {}",
                    expected_prev_hash, block.header.previous_hash
                ));
            }
        }

        // Check hash is correctly calculated
        let calculated_hash = block.calculate_hash();
        if calculated_hash != block.hash {
            return Err(format!(
                "Invalid block hash: expected {}, calculated {}",
                block.hash, calculated_hash
            ));
        }

        // Check block has transactions
        if block.transactions.is_empty() {
            return Err("Block has no transactions".to_string());
        }

        // Check first transaction is coinbase
        if !block.transactions[0].inputs.is_empty() {
            return Err("First transaction is not coinbase".to_string());
        }

        Ok(())
    }

    /// Sync blockchain from peers
    pub async fn sync_from_peers(&self) -> Result<u64, String> {
        let (our_height, genesis_time) = {
            let blockchain = self.blockchain.read().await;

            // Check if we have any blocks yet
            if blockchain.chain_tip_height() == 0 && blockchain.genesis_hash().is_empty() {
                // No blockchain yet - use a default time (October 12, 2025 as per original genesis)
                // We'll download genesis from peers and it will have the actual timestamp
                (
                    0,
                    chrono::NaiveDate::from_ymd_opt(2025, 10, 12)
                        .unwrap()
                        .and_hms_opt(0, 0, 0)
                        .unwrap()
                        .and_utc()
                        .timestamp(),
                )
            } else {
                let genesis_block = blockchain
                    .get_block_by_height(0)
                    .ok_or("Genesis block not found")?;
                (
                    blockchain.chain_tip_height(),
                    genesis_block.header.timestamp.timestamp(),
                )
            }
        };

        // Query all peers (already filters quarantined peers)
        let peer_heights = self.query_peer_heights().await;

        if peer_heights.is_empty() {
            return Err("No peers available for sync".to_string());
        }

        // Double-check: filter out any quarantined peers from results
        let peer_heights_copy = peer_heights.clone();
        let mut filtered_peer_heights = Vec::new();
        for (peer_ip, height, hash) in &peer_heights_copy {
            if let Ok(peer_addr) = peer_ip.parse::<IpAddr>() {
                if self.quarantine.is_quarantined(&peer_addr).await {
                    println!(
                        "   🚫 Filtering quarantined peer {} from sync candidates",
                        peer_ip
                    );
                    continue;
                }
            }
            filtered_peer_heights.push((peer_ip.clone(), *height, hash.clone()));
        }

        if filtered_peer_heights.is_empty() {
            // All peers are quarantined - as a last resort, allow downloading from quarantined peers
            // but skip them in the order they were quarantined (oldest quarantine first)
            println!("   ⚠️  All peers quarantined - trying anyway as last resort");
            filtered_peer_heights = peer_heights;
        }

        // Find the longest valid chain by checking multiple peers
        // Group peers by height to find consensus
        let mut height_groups: std::collections::HashMap<u64, Vec<(&str, String)>> =
            std::collections::HashMap::new();

        for (peer_ip, height, hash) in &filtered_peer_heights {
            height_groups
                .entry(*height)
                .or_default()
                .push((peer_ip.as_str(), hash.clone()));
        }

        // Find the highest height that has multiple peers agreeing
        let best_height_and_peer = height_groups
            .iter()
            .filter(|(_, peers)| !peers.is_empty()) // At least one peer
            .max_by_key(|(height, peers)| (*height, peers.len()))
            .and_then(|(height, peers)| {
                // Pick the peer with the most common hash at this height
                let mut hash_counts: std::collections::HashMap<&str, usize> =
                    std::collections::HashMap::new();
                for (_, hash) in peers {
                    *hash_counts.entry(hash.as_str()).or_insert(0) += 1;
                }

                hash_counts
                    .iter()
                    .max_by_key(|(_, count)| *count)
                    .and_then(|(target_hash, _)| {
                        // Find a peer with this hash
                        peers
                            .iter()
                            .find(|(_, hash)| hash == target_hash)
                            .map(|(peer, hash)| (*peer, *height, hash.clone()))
                    })
            })
            .ok_or("No valid peer heights")?;

        let (best_peer, max_height, best_hash) = best_height_and_peer;

        println!(
            "   🎯 Selected longest chain: peer {} at height {} (hash: {}...)",
            best_peer,
            max_height,
            hash_preview(&best_hash)
        );

        // Validate that the height is reasonable based on time elapsed
        let now = chrono::Utc::now().timestamp();
        let elapsed_seconds = now - genesis_time;

        // TIME Coin uses 10-minute blocks (600 seconds per block)
        // Calculate maximum possible blocks based on time since genesis
        let max_possible_blocks = (elapsed_seconds / 600) as u64;

        // Add tolerance: allow up to 20% more blocks than theoretical maximum
        // This accounts for clock drift, faster block production during testing, etc.
        let max_expected_height = max_possible_blocks + (max_possible_blocks / 5);

        if max_height > max_expected_height {
            println!(
                "   ⚠️  Peer height {} exceeds expected maximum {} (based on time since genesis)",
                max_height, max_expected_height
            );
            println!("      Seconds since genesis: {}", elapsed_seconds);
            println!(
                "      Theoretical max blocks (10min each): {}",
                max_possible_blocks
            );
            println!("      This may indicate a fork or imposter chain");

            // Quarantine the peer with suspicious height
            if let Ok(peer_addr) = best_peer.parse::<IpAddr>() {
                self.quarantine
                    .quarantine_peer(
                        peer_addr,
                        QuarantineReason::SuspiciousHeight {
                            their_height: max_height,
                            max_expected: max_expected_height,
                        },
                    )
                    .await;
            }

            return Err("Best peer has invalid height".to_string());
        }

        if max_height <= our_height {
            // Special case: if both are at height 0, check if we actually have genesis
            if our_height == 0 && max_height == 0 {
                let blockchain = self.blockchain.read().await;
                let has_genesis = !blockchain.genesis_hash().is_empty();
                drop(blockchain);

                if !has_genesis {
                    // We don't have genesis - try to download from any peer at height 0
                    println!("   📥 Downloading genesis block (block 0)...");

                    // Get all peers at height 0
                    let peers_at_zero: Vec<&str> = filtered_peer_heights
                        .iter()
                        .filter(|(_, height, _)| *height == 0)
                        .map(|(peer, _, _)| peer.as_str())
                        .collect();

                    if peers_at_zero.is_empty() {
                        return Err("No peers found at height 0".to_string());
                    }

                    // Try each peer until we successfully download genesis
                    for peer in peers_at_zero {
                        if let Some(block) = self.download_block(peer, 0).await {
                            let mut blockchain = self.blockchain.write().await;
                            match blockchain.add_block(block) {
                                Ok(()) => {
                                    println!("   ✓ Genesis block imported from {}", peer);
                                    return Ok(1);
                                }
                                Err(e) => {
                                    eprintln!(
                                        "   ⚠️  Failed to import genesis from {}: {}",
                                        peer, e
                                    );
                                    continue; // Try next peer
                                }
                            }
                        } else {
                            eprintln!("   ⚠️  Failed to download from {} - trying next peer", peer);
                            continue;
                        }
                    }

                    return Err("Failed to download genesis block from any peer".to_string());
                }
            }

            // Chain is up to date - message already printed in calling code
            return Ok(0);
        }

        // Determine starting height: if we have no blocks, start from genesis (0)
        let start_height = if our_height == 0 {
            let blockchain = self.blockchain.read().await;
            if blockchain.genesis_hash().is_empty() {
                0 // No genesis yet, download from block 0
            } else {
                1 // We have genesis, start from block 1
            }
        } else {
            our_height + 1
        };

        println!(
            "   Peer {} has height {} (we have {})",
            best_peer, max_height, our_height
        );
        println!(
            "   Downloading {} missing blocks...",
            max_height - start_height + 1
        );

        let mut synced_blocks = 0;

        // Download and import missing blocks
        for height in start_height..=max_height {
            println!("   📥 Downloading block {}...", height);

            if let Some(block) = self.download_block(best_peer, height).await {
                // Get expected previous hash (empty for genesis block)
                let prev_hash = {
                    let blockchain = self.blockchain.read().await;
                    if height == 0 {
                        String::new() // Genesis has no previous hash requirement
                    } else {
                        blockchain.chain_tip_hash().to_string()
                    }
                };

                // Validate block
                match self.validate_block(&block, &prev_hash) {
                    Ok(_) => {
                        // Store block hash before moving block
                        let block_hash_for_comparison = block.hash.clone();

                        // Import block
                        let mut blockchain = self.blockchain.write().await;
                        match blockchain.add_block(block) {
                            Ok(_) => {
                                synced_blocks += 1;
                                println!("   ✓ Block {} imported", height);

                                // FORK PREVENTION: Update local height after importing
                                drop(blockchain);
                                self.peer_manager
                                    .sync_gate
                                    .update_local_height(height)
                                    .await;
                            }
                            Err(e) => {
                                // Check if this is an InvalidCoinbase error
                                let is_coinbase_error = matches!(
                                    &e,
                                    time_core::state::StateError::BlockError(
                                        time_core::block::BlockError::InvalidCoinbase
                                    )
                                );

                                if is_coinbase_error {
                                    // InvalidCoinbase: Check if other peers have same block
                                    println!("   ⚠️  Block {} has invalid coinbase", height);
                                    println!("      Reason: {}", e);

                                    // Compare with other peers to see if this is widespread
                                    println!("      🔍 Checking other peers for this block...");
                                    let mut peers_with_same_block = 0;
                                    let mut total_peers_checked = 0;

                                    let peers = self.peer_manager.get_connected_peers().await;
                                    for peer in peers.iter() {
                                        let peer_ip = peer.address.ip().to_string();
                                        if peer_ip != best_peer {
                                            total_peers_checked += 1;
                                            if let Some(peer_block) =
                                                self.download_block(&peer_ip, height).await
                                            {
                                                if peer_block.hash == block_hash_for_comparison {
                                                    peers_with_same_block += 1;
                                                }
                                            }
                                        }
                                    }

                                    if total_peers_checked > 0 {
                                        println!(
                                            "      📊 {}/{} other peers have this same block",
                                            peers_with_same_block, total_peers_checked
                                        );

                                        if peers_with_same_block >= total_peers_checked / 2 {
                                            println!("      ⚠️  Majority of peers have this block - may indicate local validation issue");
                                            println!("      💡 Consider: This node may need to accept this block or reset chain");
                                        } else {
                                            println!("      ✓ Minority of peers have this block - peer likely has incompatible chain");
                                        }
                                    }

                                    if let Ok(peer_addr) = best_peer.parse::<IpAddr>() {
                                        self.quarantine
                                            .quarantine_peer(
                                                peer_addr,
                                                QuarantineReason::InvalidBlock {
                                                    height,
                                                    reason: format!("Invalid coinbase: {:?}", e),
                                                },
                                            )
                                            .await;
                                    }

                                    return Err(format!(
                                        "Block {} validation failed: Invalid coinbase (will retry with different peer)",
                                        height
                                    ));
                                } else if matches!(&e, time_core::state::StateError::DuplicateBlock)
                                {
                                    // DuplicateBlock: We already have a block at this height
                                    // This likely means we're on a different chain branch
                                    println!("   ⚠️  Block {} already exists locally with different hash", height);
                                    println!("      Local chain may have diverged - this is a fork situation");

                                    // Safe hash display - handle empty hashes
                                    let peer_hash_display = if block_hash_for_comparison.len() >= 16
                                    {
                                        &block_hash_for_comparison[..16]
                                    } else {
                                        &block_hash_for_comparison
                                    };
                                    println!("      Peer's block hash: {}...", peer_hash_display);

                                    let our_block_hash = blockchain
                                        .get_block_by_height(height)
                                        .map(|b| b.hash.clone())
                                        .unwrap_or_default();

                                    let our_hash_display = if our_block_hash.len() >= 16 {
                                        &our_block_hash[..16]
                                    } else if !our_block_hash.is_empty() {
                                        &our_block_hash
                                    } else {
                                        "<none>"
                                    };
                                    println!("      Our block hash:    {}...", our_hash_display);

                                    // CRITICAL FIX: Immediately rollback to resolve the fork
                                    // We need to rewind to one block before the fork point
                                    if height > 0 {
                                        println!("   🔄 Triggering immediate rollback to resolve fork...");

                                        // Rollback to the block before the fork
                                        let rollback_to = height - 1;
                                        match blockchain.rollback_to_height(rollback_to) {
                                            Ok(_) => {
                                                println!(
                                                    "   ✅ Rolled back to block {}",
                                                    rollback_to
                                                );
                                                println!("   🔄 Will re-sync from block {} with network consensus", height);
                                            }
                                            Err(e) => {
                                                println!("   ❌ Rollback failed: {:?}", e);
                                            }
                                        }
                                    }

                                    // Don't quarantine - this is a fork, not a bad peer
                                    // Return error to trigger retry with clean chain
                                    return Err(format!(
                                        "Fork detected and resolved: Block {} rolled back (will re-sync)",
                                        height
                                    ));
                                } else {
                                    // Other errors: Quarantine peer for sending block that failed to import
                                    if let Ok(peer_addr) = best_peer.parse::<IpAddr>() {
                                        self.quarantine
                                            .quarantine_peer(
                                                peer_addr,
                                                QuarantineReason::InvalidBlock {
                                                    height,
                                                    reason: format!("Import failed: {:?}", e),
                                                },
                                            )
                                            .await;
                                    }
                                    return Err(format!(
                                        "Failed to import block {}: {:?}",
                                        height, e
                                    ));
                                }
                            }
                        }
                    }
                    Err(validation_error) => {
                        // Check if this is a previous hash mismatch - indicates a fork
                        if validation_error.contains("Invalid previous hash") {
                            println!(
                                "   ⚠️  Previous hash mismatch detected - fork at height {}",
                                height - 1
                            );
                            println!("      Will trigger fork resolution and retry sync");

                            // Don't quarantine - this is a legitimate fork situation
                            // The fork resolution process will handle it
                            return Err(format!(
                                "Fork detected at height {} - {}",
                                height - 1,
                                validation_error
                            ));
                        }

                        // Other validation errors: Quarantine peer for sending invalid block
                        println!("   ✗ Block validation failed: {}", validation_error);
                        if let Ok(peer_addr) = best_peer.parse::<IpAddr>() {
                            self.quarantine
                                .quarantine_peer(
                                    peer_addr,
                                    QuarantineReason::InvalidBlock {
                                        height,
                                        reason: validation_error.clone(),
                                    },
                                )
                                .await;
                        }
                        return Err(format!(
                            "Block {} validation failed: {}",
                            height, validation_error
                        ));
                    }
                }
            } else {
                return Err(format!("Failed to download block {}", height));
            }
        }

        Ok(synced_blocks)
    }

    /// Detect and resolve blockchain forks with aggressive timeout
    /// Returns Ok(true) if rollback occurred (caller should skip sync)
    /// Returns Ok(false) if fork resolved normally
    pub async fn detect_and_resolve_forks(&self) -> Result<bool, String> {
        // CRITICAL: Add 30-second timeout to prevent indefinite hanging
        match tokio::time::timeout(
            tokio::time::Duration::from_secs(30),
            self.detect_and_resolve_forks_impl(),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => {
                println!("⚠️  Fork detection timed out after 30 seconds - continuing with sync");
                Ok(false)
            }
        }
    }

    /// Internal implementation of fork detection
    async fn detect_and_resolve_forks_impl(&self) -> Result<bool, String> {
        let (our_height, our_genesis) = {
            let blockchain = self.blockchain.read().await;
            (
                blockchain.chain_tip_height(),
                blockchain.genesis_hash().to_string(),
            )
        };

        // If we have no genesis yet, we're starting fresh - skip fork detection
        if our_genesis.is_empty() {
            return Ok(false);
        }

        // Query all peers for their blocks at our current height
        let peer_heights = self.query_peer_heights().await;

        if peer_heights.is_empty() {
            return Ok(false);
        }

        // Collect genesis blocks from all peers to determine network consensus
        let mut peer_genesis_blocks = Vec::new();
        for (peer_ip, _, _) in &peer_heights {
            if let Some(genesis_block) = self.download_block(peer_ip, 0).await {
                peer_genesis_blocks.push((peer_ip.clone(), genesis_block));
            }
        }

        if peer_genesis_blocks.is_empty() {
            println!("   ⚠️  Could not download genesis from any peer");
            return Ok(false);
        }

        // Count genesis block hashes to find consensus
        let mut genesis_counts = std::collections::HashMap::new();
        for (_peer_ip, block) in &peer_genesis_blocks {
            *genesis_counts.entry(block.hash.clone()).or_insert(0) += 1;
        }

        // Find the genesis with majority consensus
        let consensus_genesis = genesis_counts
            .iter()
            .max_by_key(|(_hash, count)| *count)
            .map(|(hash, _count)| hash.clone());

        let consensus_genesis = match consensus_genesis {
            Some(hash) => hash,
            None => {
                println!("   ⚠️  Could not determine genesis consensus");
                return Ok(false);
            }
        };

        let consensus_count = genesis_counts.get(&consensus_genesis).unwrap();
        let total_peers = peer_genesis_blocks.len();

        println!("\n🔍 Genesis consensus check:");
        println!(
            "   Network consensus: {}... ({}/{} peers)",
            hash_preview(&consensus_genesis),
            consensus_count,
            total_peers
        );

        // Check if OUR genesis matches network consensus
        if our_genesis != consensus_genesis {
            println!("\n⚠️  Our genesis block does not match network consensus!");
            println!(
                "   Network:   {}... ({}/{} peers)",
                hash_preview(&consensus_genesis),
                consensus_count,
                total_peers
            );
            println!("   We have:   {}...", hash_preview(&our_genesis));
            println!("   🔄 Downloading consensus genesis from peers...");

            // Find a peer with the consensus genesis
            for (peer_ip, block) in &peer_genesis_blocks {
                if block.hash == consensus_genesis {
                    println!("   ✓ Found consensus genesis on peer {}", peer_ip);

                    // Replace our invalid genesis with the consensus one
                    let mut blockchain = self.blockchain.write().await;
                    let _ = blockchain.rollback_to_height(u64::MAX); // Remove all blocks
                    match blockchain.add_block(block.clone()) {
                        Ok(_) => {
                            println!("   ✅ Successfully adopted consensus genesis block");
                            // Genesis corrected - will be checked on next fork detection cycle
                            return Ok(false);
                        }
                        Err(e) => {
                            println!("   ❌ Failed to add consensus genesis: {}", e);
                        }
                    }
                }
            }

            println!("   ⚠️  Could not adopt consensus genesis");
            return Ok(false);
        }

        println!("   ✓ Our genesis matches network consensus");

        // Quarantine peers with non-consensus genesis blocks
        for (peer_ip, block) in &peer_genesis_blocks {
            if block.hash != consensus_genesis {
                let peer_addr: IpAddr = match peer_ip.parse() {
                    Ok(addr) => addr,
                    Err(_) => continue,
                };

                // Skip if already quarantined
                if self.quarantine.is_quarantined(&peer_addr).await {
                    continue;
                }

                println!(
                    "\n⛔ GENESIS MISMATCH: Peer {} has non-consensus genesis!",
                    peer_ip
                );
                println!(
                    "   Network:   {}... ({}/{} peers)",
                    hash_preview(&consensus_genesis),
                    consensus_count,
                    total_peers
                );
                println!("   Peer has:  {}...", hash_preview(&block.hash));
                println!("   ⚠️  This peer will be quarantined");

                // Quarantine this peer - they have the wrong genesis
                self.quarantine
                    .quarantine_peer(
                        peer_addr,
                        QuarantineReason::GenesisMismatch {
                            our_genesis: consensus_genesis.clone(),
                            their_genesis: block.hash.clone(),
                        },
                    )
                    .await;
            }
        }

        // SKIP fork detection if we're at genesis (height 0)
        // Genesis blocks should never fork - they're either consensus or the peer is quarantined
        if our_height == 0 {
            println!("   ✓ At genesis - skipping fork detection (genesis blocks handled above)");
            return Ok(false);
        }

        // Check if any peer has a different block at our height
        // OR if peers ahead of us have a chain that diverges from ours
        let our_hash = {
            let blockchain = self.blockchain.read().await;
            blockchain.chain_tip_hash().to_string()
        };

        let mut competing_blocks = Vec::new();

        for (peer_ip, peer_height, peer_hash) in &peer_heights {
            // Check for fork at our current height
            if *peer_height == our_height && peer_hash != &our_hash {
                // This peer has a different block at same height - potential fork!
                if let Some(peer_block) = self.download_block(peer_ip, our_height).await {
                    competing_blocks.push((peer_ip.clone(), peer_block));
                }
            }
            // Also check if peer is ahead - download their block at our height to verify chain compatibility
            else if *peer_height > our_height {
                // Download the peer's block at our height to see if chains diverged
                if let Some(peer_block_at_our_height) =
                    self.download_block(peer_ip, our_height).await
                {
                    if peer_block_at_our_height.hash != our_hash {
                        // Peer's chain diverged at our height!
                        println!(
                            "   ⚠️  Peer {} has different block at height {} (we are at {})",
                            peer_ip, our_height, our_height
                        );
                        competing_blocks.push((peer_ip.clone(), peer_block_at_our_height));
                    }
                }
            }
        }

        if competing_blocks.is_empty() {
            // No fork at current height, but check if nodes ahead have valid chains
            let nodes_ahead: Vec<_> = peer_heights
                .iter()
                .filter(|(_, h, _)| *h > our_height)
                .collect();

            if nodes_ahead.len() >= peer_heights.len() / 2 {
                println!("\n   ℹ️  No fork at current height, but majority of network is ahead");
                println!("   🔍 Validating chains of ahead nodes...");

                let mut invalid_peers = Vec::new();

                for (peer_ip, _peer_height, _) in nodes_ahead {
                    // Download their block at our height and the next block
                    if let Some(their_block_at_our_height) =
                        self.download_block(peer_ip, our_height).await
                    {
                        if their_block_at_our_height.hash == our_hash {
                            // They have the same block as us at this height
                            // Now check if their next block chains correctly
                            if let Some(next_block) =
                                self.download_block(peer_ip, our_height + 1).await
                            {
                                if next_block.header.previous_hash != their_block_at_our_height.hash
                                {
                                    println!("   ⚠️  Peer {} has invalid chain - block {} doesn't chain correctly", peer_ip, our_height + 1);
                                    println!(
                                        "      Expected prev_hash: {}",
                                        their_block_at_our_height.hash
                                    );
                                    println!(
                                        "      Got prev_hash: {}",
                                        next_block.header.previous_hash
                                    );
                                    invalid_peers.push(peer_ip.clone());
                                }
                            }
                        }
                    }
                }

                // Quarantine invalid peers
                if !invalid_peers.is_empty() {
                    println!(
                        "   🚨 Quarantining {} peer(s) with invalid chains:",
                        invalid_peers.len()
                    );
                    for peer_ip in &invalid_peers {
                        if let Ok(peer_addr) = peer_ip.parse::<std::net::IpAddr>() {
                            self.quarantine
                                .quarantine_peer(
                                    peer_addr,
                                    time_network::QuarantineReason::InvalidBlock {
                                        height: our_height + 1,
                                        reason: format!(
                                            "Block doesn't chain correctly at height {}",
                                            our_height + 1
                                        ),
                                    },
                                )
                                .await;
                            println!("      ⛔ Quarantined: {}", peer_ip);
                        }
                    }
                    println!(
                        "   ℹ️  Invalid chains detected - blocks will be recreated via consensus"
                    );
                }
            }

            return Ok(false); // No fork detected at current height
        }

        // Get our current block FIRST to check for identical blocks early
        let our_block = {
            let blockchain = self.blockchain.read().await;
            blockchain
                .get_block_by_height(our_height)
                .ok_or("Cannot find our own block")?
                .clone()
        };

        // Add our block to the competition
        let mut all_blocks = vec![("self".to_string(), our_block.clone())];
        all_blocks.extend(competing_blocks);

        // Check if all blocks are identical BEFORE printing "FORK DETECTED"
        let all_identical = all_blocks
            .iter()
            .all(|(_, block)| block.hash == our_block.hash);

        if all_identical {
            // No actual fork - all nodes generated the same block (deterministic consensus working)
            println!(
                "   ✅ Consensus check: All {} nodes generated identical block at height {}",
                all_blocks.len(),
                our_height
            );
            return Ok(false); // Not a real fork, just detection artifact
        }

        // Real fork detected - announce it
        println!("\n⚠️  FORK DETECTED at height {}!", our_height);
        println!("   Found {} competing blocks", all_blocks.len());

        // CRITICAL: Check if any nodes have moved beyond this height
        // If so, follow the longest chain (most work) principle
        let nodes_ahead: Vec<_> = peer_heights
            .iter()
            .filter(|(_, h, _)| *h > our_height)
            .collect();

        println!("   🔍 Fork resolution debug:");
        println!("      Total peers queried: {}", peer_heights.len());
        println!(
            "      Nodes ahead (height > {}): {}",
            our_height,
            nodes_ahead.len()
        );
        println!("      Majority threshold: {}", peer_heights.len() / 2);

        // Follow longest chain if ANY nodes are ahead (not just majority)
        // This ensures we always follow the chain with more work
        if !nodes_ahead.is_empty() {
            // Any nodes ahead - follow their chain (longest chain wins)
            println!(
                "   ℹ️  Network has nodes ahead ({} peers) at height {}",
                nodes_ahead.len(),
                nodes_ahead
                    .iter()
                    .map(|(_, h, _)| h)
                    .max()
                    .unwrap_or(&our_height)
            );
            println!("   🔄 Following longest chain principle...");

            // Download the winning block from the longest chain, trying peers by height (highest first)
            let mut sorted_nodes_ahead: Vec<_> = nodes_ahead.iter().collect();
            sorted_nodes_ahead.sort_by_key(|(_, h, _)| std::cmp::Reverse(*h));

            for (peer_ip, _peer_height, _) in sorted_nodes_ahead {
                if let Some(their_block_at_our_height) =
                    self.download_block(peer_ip, our_height).await
                {
                    // Validate block timestamp is not from the future
                    let now = chrono::Utc::now();
                    let block_time = their_block_at_our_height.header.timestamp;
                    let max_drift = chrono::Duration::seconds(300); // Allow 5 minutes of clock drift

                    if block_time > now + max_drift {
                        let diff = (block_time - now).num_seconds();
                        println!(
                            "   ⚠️  Block from peer {} is from the future (diff: {}s)",
                            peer_ip, diff
                        );
                        println!("   ⚠️  Skipping this peer - trying next...");
                        continue;
                    }

                    // NOTE: We DON'T validate chain consistency here because:
                    // 1. their_block_at_our_height might be different (that's the fork!)
                    // 2. We're following longest chain principle
                    // 3. Block validation will happen during import
                    // The fork detection ALREADY identified this as the winning chain

                    println!("   ✓ Adopting block from longest chain (peer {})", peer_ip);
                    self.revert_and_replace_block(our_height, their_block_at_our_height.clone())
                        .await?;

                    println!("   ✓ Fork resolved - now on longest chain");
                    return Ok(false); // Fork resolved, continue with sync
                }
            }

            println!("   ⚠️  Could not adopt any block from longest chain");

            // CRITICAL: Don't rollback genesis block (height 0)
            if our_height == 0 {
                println!("   ⚠️  Fork at genesis height - cannot rollback further");
                println!("   ℹ️  Using timestamp-based selection to resolve genesis fork");
                // Fall through to timestamp-based selection
            } else {
                // CRITICAL: None of the peer chains validated - this means network-wide corruption
                // Best solution: Rollback to previous height and recreate missing blocks
                println!("   🚨 Network-wide chain inconsistency detected!");
                println!(
                    "   🔧 Solution: Rolling back to height {} and recreating blocks...",
                    our_height - 1
                );

                {
                    let mut blockchain = self.blockchain.write().await;
                    match blockchain.rollback_to_height(our_height - 1) {
                        Ok(_) => {
                            println!("   ✅ Rollback successful");
                            println!(
                                "   🔄 Blocks {} and {} will be recreated by consensus",
                                our_height,
                                our_height + 1
                            );
                            println!(
                                "   ℹ️  The block producer will recreate missing blocks on next cycle"
                            );
                            println!("   🚫 Skipping sync to prevent re-downloading bad blocks");
                            return Ok(true); // Rollback occurred, skip sync
                        }
                        Err(e) => {
                            println!("   ❌ Rollback failed: {:?}", e);
                            // Continue to timestamp-based resolution as fallback
                        }
                    }
                }
            }
        }

        // If we reach here, all competing nodes are at same height
        // Use timestamp-based selection combined with peer consensus
        println!("   ℹ️  All competing nodes at same height - using consensus rules");

        // Check if we can find consensus among the competing blocks
        let mut block_counts = std::collections::HashMap::new();
        for (_source, block) in &all_blocks {
            *block_counts.entry(block.hash.clone()).or_insert(0) += 1;
        }

        // Find the block with most occurrences
        let consensus_block = block_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(hash, count)| (hash.clone(), *count));

        if let Some((consensus_hash, consensus_count)) = consensus_block {
            if consensus_count > all_blocks.len() / 2 {
                // We have majority consensus on a specific block
                let winner = all_blocks
                    .iter()
                    .find(|(_, block)| block.hash == consensus_hash)
                    .map(|(_, block)| block.clone());

                if let Some(winner) = winner {
                    println!(
                        "   ✓ Found consensus block ({}/{} nodes agree)",
                        consensus_count,
                        all_blocks.len()
                    );

                    if winner.hash != our_block.hash {
                        println!("\n   🔄 Our block lost to consensus - reverting and accepting winner...");
                        self.revert_and_replace_block(our_height, winner).await?;
                        println!("   ✓ Fork resolved - now on consensus chain");
                    } else {
                        println!("   ✓ Our block matches consensus - no action needed");
                    }

                    return Ok(false); // Fork resolved, continue with sync
                }
            }
        }

        // No clear consensus - use timestamp-based selection as fallback
        println!("   ℹ️  No clear consensus - using timestamp-based selection");
        let winner = self.select_winning_block(&all_blocks)?;

        println!("   📊 Block comparison:");
        for (source, block) in &all_blocks {
            let is_winner = block.hash == winner.hash;
            let marker = if is_winner { "✓ WINNER" } else { "✗" };
            println!(
                "      {} {} - Timestamp: {}, Hash: {}...",
                marker,
                source,
                block.header.timestamp,
                &block.hash[..16]
            );
        }

        // If our block lost, revert it and accept the winner
        if winner.hash != our_block.hash {
            println!("\n   🔄 Our block lost - reverting and accepting winner...");
            self.revert_and_replace_block(our_height, winner).await?;
            println!("   ✓ Fork resolved - now on correct chain");
        } else {
            println!("   ✓ Our block won - no action needed");
        }

        Ok(false) // Fork resolved, continue with sync
    }

    /// Select the winning block from competing blocks
    fn select_winning_block(&self, blocks: &[(String, Block)]) -> Result<Block, String> {
        if blocks.is_empty() {
            return Err("No blocks to compare".to_string());
        }

        let mut best = &blocks[0].1;

        for (_, block) in blocks.iter().skip(1) {
            // Rule 1: Earlier timestamp wins
            if block.header.timestamp < best.header.timestamp {
                best = block;
                continue;
            }

            if block.header.timestamp > best.header.timestamp {
                continue;
            }

            // Rule 2: If timestamps are equal, calculate weight
            let block_weight = self.calculate_block_weight(block);
            let best_weight = self.calculate_block_weight(best);

            if block_weight > best_weight {
                best = block;
            } else if block_weight == best_weight {
                // Rule 3: If weights are equal, use hash (deterministic)
                if block.hash < best.hash {
                    best = block;
                }
            }
        }

        Ok(best.clone())
    }

    /// Calculate block weight based on producer tier and other factors
    fn calculate_block_weight(&self, block: &Block) -> u64 {
        let mut weight = 0u64;

        // Base weight from timestamp (earlier = higher weight)
        weight += (i64::MAX - block.header.timestamp.timestamp()) as u64;

        // Tier weight bonus (parsed from validator address if available)
        // Gold = +4000, Silver = +3000, Bronze = +2000, Free = +1000
        let validator = &block.header.validator_address;
        if validator.contains("gold") {
            weight += 4000;
        } else if validator.contains("silver") {
            weight += 3000;
        } else if validator.contains("bronze") {
            weight += 2000;
        } else if validator.contains("free") {
            weight += 1000;
        }

        // Transaction count bonus (more transactions = slightly higher weight)
        weight += block.transactions.len() as u64;

        weight
    }

    /// Revert our block and replace with the winning block
    async fn revert_and_replace_block(
        &self,
        height: u64,
        winning_block: Block,
    ) -> Result<(), String> {
        println!("   🔄 FORK RESOLUTION: Our block lost");
        println!("   📥 Correct block: {}...", &winning_block.hash[..16]);

        // Replace the forked block with the consensus block
        let mut blockchain = self.blockchain.write().await;
        match blockchain.replace_block(height, winning_block) {
            Ok(_) => {
                println!("   ✅ Block replaced successfully");
                Ok(())
            }
            Err(e) => {
                println!("   ⚠️  Failed to replace block: {:?}", e);
                Err(format!("Fork resolution failed: {:?}", e))
            }
        }
    }

    /// Generate catchup blocks when we're behind the network
    #[allow(dead_code)]
    async fn generate_catchup_blocks(&self) -> Result<(), String> {
        use chrono::TimeZone;

        let (current_height, genesis_block) = {
            let blockchain = self.blockchain.read().await;
            let genesis = blockchain
                .get_block_by_height(0)
                .ok_or("Genesis block not found")?
                .clone();
            (blockchain.chain_tip_height(), genesis)
        };

        let now = Utc::now();
        let _current_date = now.date_naive();
        let genesis_date = genesis_block.header.timestamp.date_naive();

        // CRITICAL FIX: Use network consensus height, not time-based calculation
        // Query all peers to find actual network height
        let network_max_height = {
            let peer_heights = self.query_peer_heights().await;
            peer_heights
                .iter()
                .map(|(_, h, _)| *h)
                .max()
                .unwrap_or(current_height)
        };

        // Use network consensus as expected height
        let expected_height = network_max_height;

        if current_height >= expected_height {
            println!(
                "   ℹ️  No catchup needed - chain is synced with network (height {})",
                current_height
            );
            return Ok(());
        }

        let missing_blocks = expected_height - current_height;
        println!("   📊 Need to sync {} blocks from network", missing_blocks);
        println!(
            "   ℹ️  Current height: {}, Network height: {}",
            current_height, expected_height
        );

        // Create each missing block
        for height in (current_height + 1)..=expected_height {
            println!("\n   🔧 Creating catchup block #{}...", height);

            // Calculate the timestamp for this block (midnight UTC on the day)
            let timestamp_date = genesis_date + chrono::Duration::days(height as i64);
            let timestamp = Utc.from_utc_datetime(&timestamp_date.and_hms_opt(0, 0, 0).unwrap());

            // Create the block
            let block = {
                let mut blockchain = self.blockchain.write().await;
                let previous_hash = blockchain.chain_tip_hash().to_string();
                let masternode_counts = blockchain.masternode_counts().clone();
                let active_masternodes: Vec<(String, time_core::MasternodeTier)> = blockchain
                    .get_active_masternodes()
                    .iter()
                    .map(|mn| (mn.wallet_address.clone(), mn.tier))
                    .collect();

                // Get the node ID for the validator
                let my_id = std::env::var("NODE_PUBLIC_IP").unwrap_or_else(|_| {
                    if let Ok(ip) = local_ip_address::local_ip() {
                        ip.to_string()
                    } else {
                        "catchup_node".to_string()
                    }
                });

                // Create coinbase transaction with masternode rewards
                let coinbase_tx = time_core::block::create_coinbase_transaction(
                    height,
                    &active_masternodes,
                    &masternode_counts,
                    0, // No transaction fees in catchup blocks
                    timestamp.timestamp(),
                );

                use time_core::block::{Block, BlockHeader};

                let mut block = Block {
                    hash: String::new(),
                    header: BlockHeader {
                        block_number: height,
                        timestamp,
                        previous_hash,
                        merkle_root: String::new(),
                        validator_signature: my_id.clone(),
                        validator_address: my_id.clone(),
                        masternode_counts,
                        proof_of_time: None,
                        checkpoints: Vec::new(),
                    },
                    transactions: vec![coinbase_tx],
                };

                block.header.merkle_root = block.calculate_merkle_root();
                block.hash = block.calculate_hash();

                // Add the block to the chain
                match blockchain.add_block(block.clone()) {
                    Ok(_) => {
                        println!("      ✅ Block #{} created successfully", height);
                        println!(
                            "         Timestamp: {}",
                            timestamp.format("%Y-%m-%d %H:%M:%S UTC")
                        );
                        println!("         Hash: {}...", &block.hash[..16]);
                        println!("         Rewards: {} masternodes", active_masternodes.len());
                    }
                    Err(e) => {
                        return Err(format!("Failed to add block {}: {:?}", height, e));
                    }
                }

                block
            };

            // Small delay between blocks to avoid overwhelming the system
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // Broadcast the new block to peers via TCP
            let peers = self.peer_manager.get_peer_ips().await;
            let block_height = block.header.block_number;
            let block_hash = block.hash.clone();

            for peer_ip in peers {
                let peer_manager = self.peer_manager.clone();
                let hash = block_hash.clone();

                tokio::spawn(async move {
                    // Send via TCP using UpdateTip message
                    if let Ok(peer_addr) = peer_ip.parse::<IpAddr>() {
                        let message = time_network::protocol::NetworkMessage::UpdateTip {
                            height: block_height,
                            hash,
                        };
                        let _ = peer_manager.send_to_peer_tcp(peer_addr, message).await;
                    }
                });
            }
        }

        println!(
            "\n   ✅ Catchup complete! Created {} blocks",
            missing_blocks
        );
        Ok(())
    }

    /// Start periodic sync task
    pub async fn start_periodic_sync(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes

            loop {
                interval.tick().await;

                // Check if we should skip this sync
                if self.should_skip_sync().await {
                    continue;
                }

                println!("\n🔄 Running periodic chain sync...");

                // First, check if we need genesis
                {
                    let blockchain = self.blockchain.read().await;
                    let has_genesis = blockchain.chain_tip_height() > 0
                        || blockchain.get_block_by_height(0).is_some();
                    drop(blockchain);

                    if !has_genesis {
                        println!("   🔍 Missing genesis block - attempting to download...");
                        if let Err(e) = self.try_download_genesis_from_all_peers().await {
                            println!("   ⚠️  Could not download genesis: {}", e);
                            println!("   ℹ️  Will retry on next sync interval (5 minutes)");
                            continue;
                        }
                    }
                }

                // First check for forks
                let _rollback_occurred = match self.detect_and_resolve_forks().await {
                    Ok(rollback) => rollback,
                    Err(e) => {
                        println!("   ⚠️  Fork detection failed: {}", e);
                        false
                    }
                };

                // After rollback, continue to sync missing blocks from peers
                // Block recreation is only for when peers don't have the blocks

                // Then sync missing blocks
                match self.sync_from_peers().await {
                    Ok(0) => {
                        println!("   ✓ Chain is up to date");
                    }
                    Ok(n) => {
                        println!("   ✓ Synced {} blocks", n);
                        // After successful sync, check if we're still behind
                        // If so, the periodic sync will try again next interval
                    }
                    Err(e) => {
                        // Check if this is a fork-related error
                        if e.contains("Fork detected") {
                            println!("   ⚠️  {}", e);
                            println!("   🔄 Re-running fork resolution...");
                            match self.detect_and_resolve_forks().await {
                                Ok(true) => {
                                    // Rollback occurred, skip sync
                                    println!("   ⏭️  Rollback completed - skipping sync");
                                }
                                Ok(false) => {
                                    // Fork resolved, try sync again
                                    match self.sync_from_peers().await {
                                        Ok(0) => {
                                            println!(
                                                "   ✓ Chain is up to date after fork resolution"
                                            )
                                        }
                                        Ok(n) => {
                                            println!(
                                                "   ✓ Synced {} blocks after fork resolution",
                                                n
                                            )
                                        }
                                        Err(e2) => {
                                            println!("   ⚠️  Sync failed: {}", e2);
                                            println!(
                                                "   ℹ️  Will retry on next sync interval (5 minutes)"
                                            );
                                        }
                                    }
                                }
                                Err(fork_err) => {
                                    println!("   ⚠️  Fork resolution failed: {}", fork_err);
                                }
                            }
                        } else {
                            println!("   ⚠️  Sync failed: {}", e);
                            println!("   ℹ️  Will retry on next sync interval (5 minutes)");
                        }
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Timelike, Utc};

    #[test]
    fn test_midnight_window_default_config() {
        let config = MidnightWindowConfig::default();
        assert_eq!(config.start_hour, 23);
        assert_eq!(config.end_hour, 1);
        assert!(config.check_consensus);
    }

    #[test]
    fn test_is_in_midnight_window_spanning_midnight() {
        // Test with a window that spans midnight (23:00 - 01:00)
        let config = MidnightWindowConfig {
            start_hour: 23,
            end_hour: 1,
            check_consensus: true,
        };

        // Check that hour 23 (11 PM) is in window
        let now = Utc::now()
            .date_naive()
            .and_hms_opt(23, 30, 0)
            .unwrap()
            .and_utc();
        let hour = now.hour();
        let in_window = if config.start_hour > config.end_hour {
            hour >= config.start_hour || hour < config.end_hour
        } else {
            hour >= config.start_hour && hour < config.end_hour
        };
        assert!(in_window, "Hour 23 should be in midnight window");

        // Check that hour 0 (midnight) is in window
        let now = Utc::now()
            .date_naive()
            .and_hms_opt(0, 30, 0)
            .unwrap()
            .and_utc();
        let hour = now.hour();
        let in_window = if config.start_hour > config.end_hour {
            hour >= config.start_hour || hour < config.end_hour
        } else {
            hour >= config.start_hour && hour < config.end_hour
        };
        assert!(in_window, "Hour 0 should be in midnight window");

        // Check that hour 1 (1 AM) is NOT in window (exclusive end)
        let now = Utc::now()
            .date_naive()
            .and_hms_opt(1, 0, 0)
            .unwrap()
            .and_utc();
        let hour = now.hour();
        let in_window = if config.start_hour > config.end_hour {
            hour >= config.start_hour || hour < config.end_hour
        } else {
            hour >= config.start_hour && hour < config.end_hour
        };
        assert!(
            !in_window,
            "Hour 1 should not be in midnight window (exclusive)"
        );

        // Check that hour 12 (noon) is NOT in window
        let now = Utc::now()
            .date_naive()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc();
        let hour = now.hour();
        let in_window = if config.start_hour > config.end_hour {
            hour >= config.start_hour || hour < config.end_hour
        } else {
            hour >= config.start_hour && hour < config.end_hour
        };
        assert!(!in_window, "Hour 12 should not be in midnight window");
    }

    #[test]
    fn test_is_in_midnight_window_not_spanning_midnight() {
        // Test with a window that doesn't span midnight (22:00 - 23:00)
        let config = MidnightWindowConfig {
            start_hour: 22,
            end_hour: 23,
            check_consensus: true,
        };

        // Check that hour 22 is in window
        let now = Utc::now()
            .date_naive()
            .and_hms_opt(22, 30, 0)
            .unwrap()
            .and_utc();
        let hour = now.hour();
        let in_window = if config.start_hour > config.end_hour {
            hour >= config.start_hour || hour < config.end_hour
        } else {
            hour >= config.start_hour && hour < config.end_hour
        };
        assert!(in_window, "Hour 22 should be in window");

        // Check that hour 23 is NOT in window (exclusive end)
        let now = Utc::now()
            .date_naive()
            .and_hms_opt(23, 0, 0)
            .unwrap()
            .and_utc();
        let hour = now.hour();
        let in_window = if config.start_hour > config.end_hour {
            hour >= config.start_hour || hour < config.end_hour
        } else {
            hour >= config.start_hour && hour < config.end_hour
        };
        assert!(!in_window, "Hour 23 should not be in window (exclusive)");

        // Check that hour 21 is NOT in window
        let now = Utc::now()
            .date_naive()
            .and_hms_opt(21, 30, 0)
            .unwrap()
            .and_utc();
        let hour = now.hour();
        let in_window = if config.start_hour > config.end_hour {
            hour >= config.start_hour || hour < config.end_hour
        } else {
            hour >= config.start_hour && hour < config.end_hour
        };
        assert!(!in_window, "Hour 21 should not be in window");
    }
}
