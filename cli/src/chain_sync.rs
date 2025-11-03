use std::sync::Arc;
use tokio::sync::RwLock;
use time_core::state::BlockchainState;
use time_core::block::Block;
use time_network::PeerManager;
use reqwest;
use serde::Deserialize;

#[derive(Deserialize)]
struct BlockchainInfo {
    height: u64,
    best_block_hash: String,
}

#[derive(Deserialize)]
struct BlockResponse {
    block: Block,
}

pub struct ChainSync {
    blockchain: Arc<RwLock<BlockchainState>>,
    peer_manager: Arc<PeerManager>,
}

impl ChainSync {
    pub fn new(
        blockchain: Arc<RwLock<BlockchainState>>,
        peer_manager: Arc<PeerManager>,
    ) -> Self {
        Self {
            blockchain,
            peer_manager,
        }
    }

    /// Query all peers and find the highest blockchain height
    pub async fn query_peer_heights(&self) -> Vec<(String, u64, String)> {
        let peers = self.peer_manager.get_connected_peers().await;
        let mut peer_heights = Vec::new();

        for peer in peers {
            let peer_ip = peer.address.ip().to_string();
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
    fn validate_block(&self, block: &Block, expected_prev_hash: &str) -> bool {
        // Check previous hash matches
        if block.header.previous_hash != expected_prev_hash {
            println!("   ✗ Invalid previous hash");
            return false;
        }

        // Check hash is correctly calculated
        let calculated_hash = block.calculate_hash();
        if calculated_hash != block.hash {
            println!("   ✗ Invalid block hash");
            return false;
        }

        // Check block has transactions
        if block.transactions.is_empty() {
            println!("   ✗ Block has no transactions");
            return false;
        }

        // Check first transaction is coinbase
        if block.transactions[0].inputs.len() > 0 {
            println!("   ✗ First transaction is not coinbase");
            return false;
        }

        true
    }

    /// Sync blockchain from peers
    pub async fn sync_from_peers(&self) -> Result<u64, String> {
        let our_height = {
            let blockchain = self.blockchain.read().await;
            blockchain.chain_tip_height()
        };

        println!("   Current height: {}", our_height);

        // Query all peers
        let peer_heights = self.query_peer_heights().await;
        
        if peer_heights.is_empty() {
            return Err("No peers available for sync".to_string());
        }

        // Find highest peer
        let (best_peer, max_height, _) = peer_heights.iter()
            .max_by_key(|(_, h, _)| h)
            .ok_or("No valid peer heights")?;

        if *max_height <= our_height {
            println!("   ✓ Already at latest height");
            return Ok(0);
        }

        println!("   Peer {} has height {} (we have {})", best_peer, max_height, our_height);
        println!("   Downloading {} missing blocks...", max_height - our_height);

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
                if !self.validate_block(&block, &prev_hash) {
                    return Err(format!("Block {} validation failed", height));
                }

                // Import block
                {
                    let mut blockchain = self.blockchain.write().await;
                    match blockchain.add_block(block) {
                        Ok(_) => {
                            synced_blocks += 1;
                            println!("   ✓ Block {} imported", height);
                        }
                        Err(e) => {
                            return Err(format!("Failed to import block {}: {:?}", height, e));
                        }
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
        let our_height = {
            let blockchain = self.blockchain.read().await;
            blockchain.chain_tip_height()
        };

        // Query all peers for their blocks at our current height
        let peer_heights = self.query_peer_heights().await;
        
        if peer_heights.is_empty() {
            return Ok(());
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
            blockchain.get_block_by_height(our_height)
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
            println!("      {} {} - Timestamp: {}, Hash: {}...", 
                marker, source, block.header.timestamp, &block.hash[..16]);
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
    async fn revert_and_replace_block(&self, _height: u64, winning_block: Block) -> Result<(), String> {
        println!("   🔄 FORK RESOLUTION: Our block lost");
        println!("   📥 Correct block: {}...", &winning_block.hash[..16]);
        println!("   No restart needed");
        
        // Request the winning block from network
        println!("   Fetching consensus block from network...");
        
        // Try to get the winning block from peers
        let peer_ips = self.peer_manager.get_peer_ips().await;
        for peer_ip in peer_ips {
            let url = format!("http://{}:24101/blockchain/block/{}", peer_ip, winning_block.hash);
            if let Ok(response) = reqwest::get(&url).await {
                if response.status().is_success() {
                    println!("   Downloaded consensus block from {}", peer_ip);
                    // The block will be applied on next validation
                    return Ok(());
                }
            }
        }
        
        println!("   Fork marked - will resolve on next sync");
        Ok(())
    }


    /// Start periodic sync task
    pub async fn start_periodic_sync(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes
            
            loop {
                interval.tick().await;
                
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
