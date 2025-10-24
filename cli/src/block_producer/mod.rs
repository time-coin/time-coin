//! Block Production System
//! 
//! Produces one block every 24 hours with:
//! - Validator selection (round-robin for now)
//! - Reward distribution (95 TIME to masternodes, 5 TIME to treasury)
//! - Block broadcasting

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use time_core::Block;

// Constants
const BLOCK_REWARD: u64 = 100 * 100_000_000; // 100 TIME
const MASTERNODE_REWARD: u64 = 95 * 100_000_000; // 95 TIME
const TREASURY_REWARD: u64 = 5 * 100_000_000; // 5 TIME

pub struct BlockProducer {
    current_height: Arc<RwLock<u64>>,
    last_block_hash: Arc<RwLock<String>>,
}

impl BlockProducer {
    pub fn new() -> Self {
        Self {
            current_height: Arc::new(RwLock::new(0)),
            last_block_hash: Arc::new(RwLock::new(
                "00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048".to_string()
            )),
        }
    }

    pub async fn start(&self) {
        let height = self.current_height.clone();
        let last_hash = self.last_block_hash.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(86400)); // 24 hours
            
            loop {
                interval.tick().await;
                
                let mut current_height = height.write().await;
                let mut last_block_hash = last_hash.write().await;
                
                *current_height += 1;
                
                println!("🔨 Producing block #{}...", *current_height);
                
                // Create new block
                let block = Block::new(*current_height, last_block_hash.clone());
                
                // Update last block hash
                *last_block_hash = block.hash.clone();
                
                println!("✅ Block #{} produced: {}", *current_height, &block.hash[..16]);
                println!(
                    "💰 Block reward: {} TIME (Masternodes: {}, Treasury: {})",
                    BLOCK_REWARD / 100_000_000,
                    MASTERNODE_REWARD / 100_000_000,
                    TREASURY_REWARD / 100_000_000
                );
                
                // TODO: Distribute rewards
                // TODO: Broadcast block to network
            }
        });
    }

    #[allow(dead_code)]
    pub async fn get_height(&self) -> u64 {
        *self.current_height.read().await
    }

    #[allow(dead_code)]
    pub async fn get_last_block_hash(&self) -> String {
        self.last_block_hash.read().await.clone()
    }
}
