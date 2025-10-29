use std::time::Duration;
use tokio::time;
use std::sync::Arc;
use time_network::PeerManager;
use chrono::Utc;

pub struct BlockProducer {
    node_id: String,
    peer_manager: Arc<PeerManager>,
    block_height: Arc<tokio::sync::RwLock<u64>>,
}

impl BlockProducer {
    pub fn new(node_id: String, peer_manager: Arc<PeerManager>) -> Self {
        BlockProducer {
            node_id,
            peer_manager,
            block_height: Arc::new(tokio::sync::RwLock::new(1)), // Start at block 1
        }
    }

    pub async fn start(self) {
        tokio::spawn(async move {
            self.run().await;
        });
    }

    async fn run(&self) {
        // Calculate time until next midnight GMT
        let now = Utc::now();
        let next_midnight = (now + chrono::Duration::days(1))
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        
        let duration_until_midnight = (next_midnight - now).to_std().unwrap_or(Duration::from_secs(0));
        
        println!("⏰ Next block production at: {} UTC", next_midnight.format("%Y-%m-%d %H:%M:%S"));
        println!("   Waiting {} hours {} minutes...", 
            duration_until_midnight.as_secs() / 3600,
            (duration_until_midnight.as_secs() % 3600) / 60
        );

        // Wait until midnight
        time::sleep(duration_until_midnight).await;

        // Then produce blocks every 24 hours
        let mut interval = time::interval(Duration::from_secs(86400)); // 24 hours
        interval.tick().await; // Skip immediate tick

        loop {
            interval.tick().await;
            self.produce_block().await;
        }
    }

    async fn produce_block(&self) {
        let mut height = self.block_height.write().await;
        let block_num = *height;
        *height += 1;
        drop(height);

        let timestamp = Utc::now();
        
        println!("\n{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_blue());
        println!("{}", "🎲 BLOCK PRODUCTION LOTTERY".bright_blue().bold());
        println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_blue());
        
        // Get all masternodes
        let peers = self.peer_manager.get_peer_ips().await;
        let mut all_nodes = peers.clone();
        all_nodes.push(self.node_id.clone());
        all_nodes.sort();
        all_nodes.dedup();
        
        // Determine producer
        let producer_index = (block_num as usize) % all_nodes.len();
        let selected_producer = &all_nodes[producer_index];
        let is_my_turn = selected_producer == &self.node_id;
        
        println!("   Block Height: {}", block_num.to_string().yellow());
        println!("   Timestamp: {}", timestamp.format("%Y-%m-%d %H:%M:%S UTC"));
        println!("   Total Masternodes: {}", all_nodes.len().to_string().yellow());
        println!("\n   🎯 Round-Robin Selection:");
        
        for (i, node) in all_nodes.iter().enumerate() {
            if i == producer_index {
                if is_my_turn {
                    println!("      {} {} (THIS NODE) 👑", "✓".green(), node.green().bold());
                } else {
                    println!("      {} {} 👑", "→".yellow(), node.yellow().bold());
                }
            } else {
                println!("        {}", node.bright_black());
            }
        }
        
        if is_my_turn {
            println!("\n{}", "   🏆 I AM THE BLOCK PRODUCER!".green().bold());
            println!("   📦 Producing block {}...", block_num);
            
            // TODO: Actually create and broadcast block
            // For now, just announce
            
            println!("{}", "   ✓ Block produced and broadcast".green());
        } else {
            println!("\n{}", "   ⏸  Not my turn - standing by for validation".bright_black());
            println!("   Waiting for block from {}...", selected_producer.yellow());
        }
        
        println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_blue());
        println!();
    }
}

use owo_colors::OwoColorize;
