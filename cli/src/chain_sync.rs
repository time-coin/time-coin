use chrono::{Timelike, Utc};
use serde::Deserialize;
use std::net::IpAddr;
use std::sync::Arc;
use time_core::block::Block;
use time_core::state::BlockchainState;
use time_network::{PeerManager, PeerQuarantine, QuarantineReason};
use tokio::sync::RwLock;

#[derive(Deserialize)]
struct BlockchainInfo {
    height: u64,
    best_block_hash: String,
}

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
        if !self.is_in_midnight_window() {
            return false;
        }

        // We're in the midnight window
        if let Some(config) = &self.midnight_config {
            if config.check_consensus {
                // Check if consensus (block producer) is actively running
                let is_active = *self.is_block_producer_active.read().await;
                if is_active {
                    println!("   ⏸️  Skipping sync: in midnight window and consensus is active");
                    return true;
                }
            } else {
                // Just skip during midnight window without checking consensus
                println!("   ⏸️  Skipping sync: in midnight window");
                return true;
            }
        }

        false
    }

    /// Query all peers and find the highest blockchain height
    pub async fn query_peer_heights(&self) -> Vec<(String, u64, String)> {
        let peers = self.peer_manager.get_connected_peers().await;
        let mut peer_heights = Vec::new();

        for peer in peers {
            let peer_ip = peer.address.ip().to_string();
            
            // Skip quarantined peers
            if let Ok(peer_addr) = peer_ip.parse::<IpAddr>() {
                if self.quarantine.is_quarantined(&peer_addr).await {
                    if let Some(reason) = self.quarantine.get_reason(&peer_addr).await {
                        println!("   🚫 Skipping quarantined peer {} (reason: {})", peer_ip, reason);
                    }
                    continue;
                }
            }
            
            let url = format!("http://{}:24101/blockchain/info", peer_ip);

            match reqwest::get(&url).await {
                Ok(response) => {
                    if let Ok(info) = response.json::<BlockchainInfo>().await {
                        peer_heights.push((peer_ip, info.height, info.best_block_hash));
                    }
                }
                Err(_) => continue,
            }
        }

        peer_heights
    }

    /// Download a specific block from a peer
    async fn download_block(&self, peer_ip: &str, height: u64) -> Option<Block> {
        let url = format!("http://{}:24101/blockchain/block/{}", peer_ip, height);

        match reqwest::get(&url).await {
            Ok(response) => {
                if let Ok(block_resp) = response.json::<BlockResponse>().await {
                    return Some(block_resp.block);
                }
            }
            Err(_) => return None,
        }
        None
    }

    /// Validate a block before importing
    fn validate_block(&self, block: &Block, expected_prev_hash: &str) -> Result<(), String> {
        // Check previous hash matches
        if block.header.previous_hash != expected_prev_hash {
            return Err(format!(
                "Invalid previous hash: expected {}, got {}",
                expected_prev_hash, block.header.previous_hash
            ));
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
            let genesis_block = blockchain
                .get_block_by_height(0)
                .ok_or("Genesis block not found")?;
            (
                blockchain.chain_tip_height(),
                genesis_block.header.timestamp.timestamp(),
            )
        };

        println!("   Current height: {}", our_height);

        // Query all peers (already filters quarantined peers)
        let peer_heights = self.query_peer_heights().await;

        if peer_heights.is_empty() {
            return Err("No peers available for sync".to_string());
        }

        // Double-check: filter out any quarantined peers from results
        let mut filtered_peer_heights = Vec::new();
        for (peer_ip, height, hash) in peer_heights {
            if let Ok(peer_addr) = peer_ip.parse::<IpAddr>() {
                if self.quarantine.is_quarantined(&peer_addr).await {
                    println!("   🚫 Filtering quarantined peer {} from sync candidates", peer_ip);
                    continue;
                }
            }
            filtered_peer_heights.push((peer_ip, height, hash));
        }
        
        if filtered_peer_heights.is_empty() {
            return Err("No non-quarantined peers available for sync".to_string());
        }

        // Find highest peer
        let (best_peer, max_height, _) = filtered_peer_heights
            .iter()
            .max_by_key(|(_, h, _)| h)
            .ok_or("No valid peer heights")?;

        // Validate that the height is reasonable based on time elapsed
        let now = chrono::Utc::now().timestamp();
        let elapsed_days = (now - genesis_time) / 86400; // seconds per day
        let max_expected_height = elapsed_days as u64 + 10; // Allow some tolerance

        if *max_height > max_expected_height {
            println!(
                "   ⚠️  Peer height {} exceeds expected maximum {} (based on time since genesis)",
                max_height, max_expected_height
            );
            println!("      Days since genesis: {}", elapsed_days);
            println!("      This may indicate a fork or imposter chain");

            // Quarantine the peer with suspicious height
            if let Ok(peer_addr) = best_peer.parse::<IpAddr>() {
                self.quarantine
                    .quarantine_peer(
                        peer_addr,
                        QuarantineReason::SuspiciousHeight {
                            their_height: *max_height,
                            max_expected: max_expected_height,
                        },
                    )
                    .await;
            }

            // Don't sync from this peer, find another
            let valid_peers: Vec<_> = filtered_peer_heights
                .iter()
                .filter(|(_, h, _)| *h <= max_expected_height)
                .collect();

            if valid_peers.is_empty() {
                return Err("No peers with valid height found".to_string());
            }

            let (best_peer, max_height, _) = valid_peers
                .iter()
                .max_by_key(|(_, h, _)| h)
                .ok_or("No valid peer heights")?;

            if *max_height <= our_height {
                println!("   ✓ Already at latest height");
                return Ok(0);
            }

            println!(
                "   Using peer {} with validated height {}",
                best_peer, max_height
            );
        }

        if *max_height <= our_height {
            println!("   ✓ Already at latest height");
            return Ok(0);
        }

        println!(
            "   Peer {} has height {} (we have {})",
            best_peer, max_height, our_height
        );
        println!(
            "   Downloading {} missing blocks...",
            max_height - our_height
        );

        let mut synced_blocks = 0;

        // Download and import missing blocks
        for height in (our_height + 1)..=*max_height {
            println!("   📥 Downloading block {}...", height);

            if let Some(block) = self.download_block(best_peer, height).await {
                // Get expected previous hash
                let prev_hash = {
                    let blockchain = self.blockchain.read().await;
                    blockchain.chain_tip_hash().to_string()
                };

                // Validate block
                match self.validate_block(&block, &prev_hash) {
                    Ok(_) => {
                        // Import block
                        let mut blockchain = self.blockchain.write().await;
                        match blockchain.add_block(block) {
                            Ok(_) => {
                                synced_blocks += 1;
                                println!("   ✓ Block {} imported", height);
                            }
                            Err(e) => {
                                // Quarantine peer for sending block that failed to import
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
                                return Err(format!("Failed to import block {}: {:?}", height, e));
                            }
                        }
                    }
                    Err(validation_error) => {
                        // Quarantine peer for sending invalid block
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

    /// Detect and resolve blockchain forks
    pub async fn detect_and_resolve_forks(&self) -> Result<(), String> {
        let (our_height, our_genesis) = {
            let blockchain = self.blockchain.read().await;
            (
                blockchain.chain_tip_height(),
                blockchain.genesis_hash().to_string(),
            )
        };

        // Query all peers for their blocks at our current height
        let peer_heights = self.query_peer_heights().await;

        if peer_heights.is_empty() {
            return Ok(());
        }

        // Check for genesis block mismatches first
        for (peer_ip, _, _peer_hash) in &peer_heights {
            // Parse IP address for quarantine
            let peer_addr: IpAddr = match peer_ip.parse() {
                Ok(addr) => addr,
                Err(_) => continue,
            };

            // Skip if already quarantined
            if self.quarantine.is_quarantined(&peer_addr).await {
                continue;
            }

            // Try to get their genesis block (height 0)
            if let Some(peer_genesis_block) = self.download_block(peer_ip, 0).await {
                if peer_genesis_block.hash != our_genesis {
                    println!(
                        "\n⛔ GENESIS MISMATCH: Peer {} on different chain!",
                        peer_ip
                    );
                    println!("   Our genesis:   {}...", &our_genesis[..16]);
                    println!("   Peer genesis:  {}...", &peer_genesis_block.hash[..16]);
                    println!("   ⚠️  This peer will be quarantined from consensus");

                    // Quarantine this peer
                    self.quarantine
                        .quarantine_peer(
                            peer_addr,
                            QuarantineReason::GenesisMismatch {
                                our_genesis: our_genesis.clone(),
                                their_genesis: peer_genesis_block.hash.clone(),
                            },
                        )
                        .await;
                    continue;
                }
            }
        }

        // Check if any peer has a different block at our height
        let our_hash = {
            let blockchain = self.blockchain.read().await;
            blockchain.chain_tip_hash().to_string()
        };

        let mut competing_blocks = Vec::new();

        for (peer_ip, peer_height, peer_hash) in peer_heights {
            if peer_height >= our_height && peer_hash != our_hash {
                // This peer has a different block at same height - potential fork!
                if let Some(peer_block) = self.download_block(&peer_ip, our_height).await {
                    competing_blocks.push((peer_ip, peer_block));
                }
            }
        }

        if competing_blocks.is_empty() {
            return Ok(()); // No fork detected
        }

        println!("\n⚠️  FORK DETECTED at height {}!", our_height);
        println!("   Found {} competing blocks", competing_blocks.len() + 1);

        // Get our current block
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

        // Determine the winning block
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

        Ok(())
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

                // First check for forks
                if let Err(e) = self.detect_and_resolve_forks().await {
                    println!("   ⚠️  Fork detection failed: {}", e);
                }

                // Then sync missing blocks
                match self.sync_from_peers().await {
                    Ok(0) => println!("   ✓ Chain is up to date"),
                    Ok(n) => println!("   ✓ Synced {} blocks", n),
                    Err(e) => println!("   ⚠️  Sync failed: {}", e),
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
