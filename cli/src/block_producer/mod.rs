//! Block Production System
//! 
//! Leader selection:
//! - < 3 nodes: Round-robin without BFT (testnet mode)
//! - ≥ 3 nodes: BFT consensus required

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use time_core::Block;
use time_network::PeerManager;

const BLOCK_REWARD: u64 = 100 * 100_000_000; // 100 TIME
const MASTERNODE_REWARD: u64 = 95 * 100_000_000; // 95 TIME  
const TREASURY_REWARD: u64 = 5 * 100_000_000; // 5 TIME

pub struct BlockProducer {
    current_height: Arc<RwLock<u64>>,
    last_block_hash: Arc<RwLock<String>>,
    node_id: String,
    peer_manager: Arc<PeerManager>,
}

impl BlockProducer {
    pub fn new(node_id: String, peer_manager: Arc<PeerManager>) -> Self {
        Self {
            current_height: Arc::new(RwLock::new(0)),
            last_block_hash: Arc::new(RwLock::new(
                "00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048".to_string()
            )),
            node_id,
            peer_manager,
        }
    }

    pub async fn start(&self) {
        let height = self.current_height.clone();
        let last_hash = self.last_block_hash.clone();
        let node_id = self.node_id.clone();
        let peer_manager = self.peer_manager.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(86400)); // 24 hours
            interval.tick().await; // Skip first immediate tick
            
            loop {
                interval.tick().await;
                
                let mut current_height = height.write().await;
                let mut last_block_hash = last_hash.write().await;
                
                *current_height += 1;
                let block_num = *current_height;
                
                // Get active nodes (self + connected peers)
                let peer_count = peer_manager.peer_count().await;
                let total_nodes = peer_count + 1; // Include self
                
                // Get sorted list of node IPs for deterministic ordering
                let mut node_ips = peer_manager.get_peer_ips().await;
                node_ips.push(node_id.clone());
                node_ips.sort();
                
                // Determine leader: block_height % num_nodes
                let leader_index = (block_num as usize - 1) % total_nodes;
                let leader_ip = &node_ips[leader_index];
                let is_leader = leader_ip == &node_id;
                
                println!("📋 Block #{} - {} nodes active", block_num, total_nodes);
                println!("   Leader: {} {}", leader_ip, if is_leader { "(👑 ME)" } else { "" });
                
                if is_leader {
                    // This node produces the block
                    println!("🔨 Producing block #{}...", block_num);
                    
                    let block = Block::new(block_num, last_block_hash.clone());
                    *last_block_hash = block.hash.clone();
                    
                    println!("✅ Block #{} produced: {}", block_num, &block.hash[..16]);
                    println!("💰 Rewards: {} TIME total ({} → Masternodes, {} → Treasury)", 
                        BLOCK_REWARD / 100_000_000,
                        MASTERNODE_REWARD / 100_000_000,
                        TREASURY_REWARD / 100_000_000
                    );
                    
                    // TODO: Broadcast block to peers
                    // TODO: Distribute rewards
                } else {
                    println!("⏳ Waiting for block #{} from leader {}...", block_num, leader_ip);
                    // TODO: Listen for block from leader
                    // TODO: Validate and accept block
                }
            }
        });
    }
}
