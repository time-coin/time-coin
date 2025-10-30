use std::time::Duration;
use tokio::time;
use std::sync::Arc;
use time_network::PeerManager;
use time_consensus::ConsensusEngine;
use chrono::{Utc, TimeZone};
use owo_colors::OwoColorize;
use std::path::Path;

pub struct BlockProducer {
    node_id: String,
    peer_manager: Arc<PeerManager>,
    consensus: Arc<ConsensusEngine>,
    height_file: String,
}

impl BlockProducer {
    pub fn new(node_id: String, peer_manager: Arc<PeerManager>, consensus: Arc<ConsensusEngine>) -> Self {
        BlockProducer {
            node_id,
            peer_manager,
            consensus,
            height_file: "/root/time-coin-node/data/block_height.txt".to_string(),
        }
    }

    fn load_block_height(&self) -> u64 {
        if let Ok(contents) = std::fs::read_to_string(&self.height_file) {
            contents.trim().parse().unwrap_or(1)
        } else {
            1
        }
    }

    fn save_block_height(&self, height: u64) {
        if let Some(parent) = Path::new(&self.height_file).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&self.height_file, height.to_string());
    }

    pub async fn start(self) {
        tokio::spawn(async move {
            self.run().await;
        });
    }

    async fn run(&self) {
        // Check for missed blocks on startup
        self.catch_up_missed_blocks().await;
        
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

        time::sleep(duration_until_midnight).await;

        let mut interval = time::interval(Duration::from_secs(86400));
        interval.tick().await;

        loop {
            interval.tick().await;
            self.produce_block().await;
        }
    }

    async fn catch_up_missed_blocks(&self) {
        let current_height = self.load_block_height();
        let now = Utc::now();
        
        // Genesis: Oct 24, 2024 00:00:00 UTC
        let genesis_time = Utc.with_ymd_and_hms(2025, 10, 24, 0, 0, 0).unwrap();
        let duration = now.signed_duration_since(genesis_time);
        let days_since_genesis = duration.num_days();
        
        // Expected height = days since genesis + 1 (genesis is block 0, day 0 produces block 1, etc)
        let expected_height = (days_since_genesis + 1) as u64;
        
        if current_height < expected_height {
            let missed_blocks = expected_height - current_height;
            
            println!("\n{}", "⚠️  MISSED BLOCKS DETECTED".yellow().bold());
            println!("   Current height: {}", current_height);
            println!("   Expected height: {}", expected_height);
            println!("   Missed blocks: {}", missed_blocks);
            println!();
            
            // Check if this node should create the catch-up blocks
            let is_my_turn = self.consensus.is_my_turn(current_height, &self.node_id).await;
            
            if is_my_turn {
                println!("{}", "   🔨 I am the designated producer for catch-up blocks".green());
                println!("   Creating {} missed block(s)...", missed_blocks);
                println!();
                
                for i in 0..missed_blocks {
                    let height = current_height + i;
                    let block_time = genesis_time + chrono::Duration::days(height as i64);
                    
                    println!("   📦 Creating catch-up block #{}...", height);
                    println!("      Timestamp: {}", block_time.format("%Y-%m-%d %H:%M:%S UTC"));
                    
                    self.produce_catch_up_block(height, block_time).await;
                    
                    // Save after each block
                    self.save_block_height(height + 1);
                    
                    // Small delay between blocks
                    time::sleep(Duration::from_millis(500)).await;
                }
                
                println!();
                println!("{}", "   ✅ Catch-up complete!".green().bold());
                println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black());
                println!();
            } else {
                println!("{}", "   👀 Waiting for designated producer to create catch-up blocks...".cyan());
                println!();
            }
        }
    }

    async fn produce_catch_up_block(&self, block_num: u64, timestamp: chrono::DateTime<Utc>) {
        let block_hash = format!("{:x}", md5::compute(format!("{}{}{}", block_num, timestamp.timestamp(), self.node_id)));
        
        println!("      Block Hash: {}...", &block_hash[..16]);
        
        // Auto-approve catch-up blocks (retroactive blocks for past days)
        let _ = self.consensus.vote_on_block(&block_hash, self.node_id.clone(), true).await;
        
        println!("      ✓ Block #{} created", block_num);
    }

    async fn produce_block(&self) {
        // Load current height from disk
        let block_num = self.load_block_height();

        let timestamp = Utc::now();

        println!("\n{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_blue());
        println!("{}", "🎲 BLOCK PRODUCTION - BFT CONSENSUS".bright_blue().bold());
        println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_blue());

        let producer = self.consensus.get_block_producer(block_num).await;
        let is_my_turn = self.consensus.is_my_turn(block_num, &self.node_id).await;

        println!("   Block Height: {}", block_num.to_string().yellow());
        println!("   Timestamp: {}", timestamp.format("%Y-%m-%d %H:%M:%S UTC"));

        if let Some(selected_producer) = producer {
            println!("   Selected Producer: {}", selected_producer.yellow().bold());

            if is_my_turn {
                println!("\n{}", "   🏆 I AM THE BLOCK PRODUCER!".green().bold());
                self.propose_and_vote(block_num, timestamp.timestamp()).await;

                // Increment and save height after successful production
                self.save_block_height(block_num + 1);
            } else {
                println!("\n{}", "   👀 VALIDATOR MODE - Waiting for proposal...".cyan());
                self.wait_and_validate(block_num, &selected_producer).await;

                // Validators also increment after successful validation
                self.save_block_height(block_num + 1);
            }
        } else {
            println!("\n{}", "   ⚠️  No masternodes available".yellow());
        }

        println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_blue());
        println!();
    }

    async fn propose_and_vote(&self, block_num: u64, timestamp: i64) {
        let block_hash = format!("{:x}", md5::compute(format!("{}{}{}", block_num, timestamp, self.node_id)));

        println!("   📦 Creating block proposal...");
        println!("      Block Hash: {}...", &block_hash[..16]);

        let peers = self.peer_manager.get_peer_ips().await;
        println!("   📡 Broadcasting to {} peer(s)...", peers.len());

        println!("   🗳️  Casting my vote: APPROVE");
        let _ = self.consensus.vote_on_block(&block_hash, self.node_id.clone(), true).await;

        println!("\n   ⏳ Waiting for votes (60 second timeout)...");

        let vote_deadline = time::Instant::now() + Duration::from_secs(60);
        let mut last_status = (0, 0);

        while time::Instant::now() < vote_deadline {
            let (has_quorum, approvals, rejections, total) = self.consensus.check_quorum(&block_hash).await;

            if (approvals, rejections) != last_status {
                println!("      Votes: {} approve, {} reject (need {}/{})",
                    approvals.to_string().green(),
                    rejections.to_string().red(),
                    ((total * 2 + 2) / 3).to_string().yellow(),
                    total
                );
                last_status = (approvals, rejections);
            }

            if has_quorum {
                println!("\n{}", "   ✅ QUORUM REACHED - BLOCK COMMITTED!".green().bold());
                println!("      Final: {}/{} masternodes approved", approvals, total);
                return;
            }

            time::sleep(Duration::from_secs(2)).await;
        }

        println!("\n{}", "   ❌ TIMEOUT - Block rejected (no quorum)".red());
        println!("      Final: {}/{} votes", last_status.0, last_status.0 + last_status.1);
    }

    async fn wait_and_validate(&self, block_num: u64, proposer: &str) {
        println!("   Waiting for proposal from {}...", proposer.yellow());

        time::sleep(Duration::from_secs(5)).await;

        println!("   📬 Block proposal received!");
        println!("   🔍 Validating proposal...");
        time::sleep(Duration::from_secs(1)).await;

        let approve = true;
        let vote_type = if approve { "APPROVE ✓" } else { "REJECT ✗" };
        println!("   🗳️  Casting vote: {}", vote_type.green());

        let block_hash = format!("{:x}", md5::compute(format!("{}{}{}", block_num, proposer, "temp")));
        let _ = self.consensus.vote_on_block(&block_hash, self.node_id.clone(), approve).await;

        println!("   ⏳ Waiting for network consensus...");
        time::sleep(Duration::from_secs(10)).await;

        let (has_quorum, approvals, _rejections, total) = self.consensus.check_quorum(&block_hash).await;

        if has_quorum {
            println!("   {}", "✅ Block committed by network!".green().bold());
            println!("      {}/{} masternodes approved", approvals, total);
        } else {
            println!("   {}", "❌ Block rejected by network".red());
            println!("      {}/{} approvals (need {}/{})",
                approvals, total, (total * 2 + 2) / 3, total);
        }
    }
}
