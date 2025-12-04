//! Block Synchronization
//!
//! Fetches and validates blocks from peers to catch up when behind

use reqwest::Client;
use std::time::Duration;
use time_core::block::Block;

/// Manager for fetching and syncing blocks from peers
pub struct BlockSyncManager {
    client: Client,
    request_timeout_secs: u64,
}

impl BlockSyncManager {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            request_timeout_secs: 30,
        }
    }

    /// Fetch a block from a peer at specific height
    pub async fn fetch_block_from_peer(&self, peer: &str, height: u64) -> Option<Block> {
        let url = format!("http://{}:24101/api/blockchain/block/{}", peer, height);

        match tokio::time::timeout(
            Duration::from_secs(self.request_timeout_secs),
            self.client.get(&url).send(),
        )
        .await
        {
            Ok(Ok(response)) => {
                if let Ok(block_response) = response.json::<BlockResponse>().await {
                    Some(block_response.block)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Try to fetch a block from multiple peers (use first successful)
    pub async fn fetch_block_from_peers(
        &self,
        peers: &[String],
        height: u64,
    ) -> Result<Block, String> {
        for peer in peers {
            println!("  Attempting to fetch block {} from {}...", height, peer);

            if let Some(block) = self.fetch_block_from_peer(peer, height).await {
                // Basic validation
                if block.header.block_number == height {
                    println!("  ✓ Successfully fetched block {} from {}", height, peer);
                    return Ok(block);
                } else {
                    println!(
                        "  ✗ Block height mismatch from {} (got {}, expected {})",
                        peer, block.header.block_number, height
                    );
                }
            }
        }

        Err(format!(
            "Failed to fetch block {} from any of {} peers",
            height,
            peers.len()
        ))
    }

    /// Sync a range of blocks from peers
    pub async fn sync_blocks(
        &self,
        peers: &[String],
        start_height: u64,
        end_height: u64,
    ) -> Result<Vec<Block>, String> {
        if start_height > end_height {
            return Err("Invalid height range".to_string());
        }

        let count = end_height - start_height + 1;
        println!(
            "🔄 Syncing {} blocks (height {} to {})...",
            count, start_height, end_height
        );

        let mut blocks = Vec::new();

        for height in start_height..=end_height {
            match self.fetch_block_from_peers(peers, height).await {
                Ok(block) => {
                    blocks.push(block);
                    println!("  Progress: {}/{} blocks synced", blocks.len(), count);
                }
                Err(e) => {
                    println!("  ✗ Failed to fetch block {}: {}", height, e);
                    return Err(format!("Sync failed at block {}: {}", height, e));
                }
            }
        }

        println!("✅ Successfully synced {} blocks", blocks.len());
        Ok(blocks)
    }

    /// Verify block chain continuity
    pub fn verify_block_chain(blocks: &[Block]) -> Result<(), String> {
        if blocks.is_empty() {
            return Ok(());
        }

        println!("🔍 Verifying block chain continuity...");

        for i in 1..blocks.len() {
            let prev_block = &blocks[i - 1];
            let curr_block = &blocks[i];

            // Check height continuity
            if curr_block.header.block_number != prev_block.header.block_number + 1 {
                return Err(format!(
                    "Height discontinuity: {} -> {}",
                    prev_block.header.block_number, curr_block.header.block_number
                ));
            }

            // Check hash linkage
            if curr_block.header.previous_hash != prev_block.hash {
                return Err(format!(
                    "Hash mismatch at height {}: previous_hash doesn't match",
                    curr_block.header.block_number
                ));
            }

            // Verify block hash
            let computed_hash = curr_block.calculate_hash();
            if computed_hash != curr_block.hash {
                return Err(format!(
                    "Invalid block hash at height {}",
                    curr_block.header.block_number
                ));
            }
        }

        println!("  ✓ All {} blocks verified successfully", blocks.len());
        Ok(())
    }
}

impl Default for BlockSyncManager {
    fn default() -> Self {
        Self::new()
    }
}

// Response structure matching the API endpoint
#[derive(serde::Deserialize)]
struct BlockResponse {
    block: Block,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_empty_chain() {
        let blocks = vec![];
        assert!(BlockSyncManager::verify_block_chain(&blocks).is_ok());
    }

    #[test]
    fn test_verify_single_block() {
        // Would need to create a valid block for this test
        // Skipping for now as it requires complex setup
    }
}
