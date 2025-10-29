use std::time::Duration;
use tokio::time;
use std::sync::Arc;
use time_network::PeerManager;
use time_consensus::ConsensusEngine;
use chrono::Utc;
use owo_colors::OwoColorize;

pub struct BlockProducer {
    node_id: String,
    peer_manager: Arc<PeerManager>,
    consensus: Arc<ConsensusEngine>,
    block_height: Arc<tokio::sync::RwLock<u64>>,
}

impl BlockProducer {
    pub fn new(node_id: String, peer_manager: Arc<PeerManager>, consensus: Arc<ConsensusEngine>) -> Self {
        BlockProducer {
            node_id,
            peer_manager,
            consensus,
            block_height: Arc::new(tokio::sync::RwLock::new(1)),
        }
    }

    pub async fn start(self) {
        tokio::spawn(async move {
            self.run().await;
        });
    }

    async fn run(&self) {
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

    async fn produce_block(&self) {
        let mut height = self.block_height.write().await;
        let block_num = *height;
        *height += 1;
        drop(height);

        let timestamp = Utc::now();
        
        println!("\n{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_blue());
        println!("{}", "🎲 BLOCK PRODUCTION - BFT CONSENSUS".bright_blue().bold());
        println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_blue());
        
        // Determine producer via round-robin
        let producer = self.consensus.get_block_producer(block_num).await;
        let is_my_turn = self.consensus.is_my_turn(block_num, &self.node_id).await;
        
        println!("   Block Height: {}", block_num.to_string().yellow());
        println!("   Timestamp: {}", timestamp.format("%Y-%m-%d %H:%M:%S UTC"));
        
        if let Some(selected_producer) = producer {
            println!("   Selected Producer: {}", selected_producer.yellow().bold());
            
            if is_my_turn {
                println!("\n{}", "   🏆 I AM THE BLOCK PRODUCER!".green().bold());
                self.propose_and_vote(block_num, timestamp.timestamp()).await;
            } else {
                println!("\n{}", "   👀 VALIDATOR MODE - Waiting for proposal...".cyan());
                self.wait_and_validate(block_num, &selected_producer).await;
            }
        } else {
            println!("\n{}", "   ⚠️  No masternodes available".yellow());
        }
        
        println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_blue());
        println!();
    }

    async fn propose_and_vote(&self, block_num: u64, timestamp: i64) {
        // Create block proposal
        let block_hash = format!("{:x}", md5::compute(format!("{}{}{}", block_num, timestamp, self.node_id)));
        
        println!("   📦 Creating block proposal...");
        println!("      Block Hash: {}...", &block_hash[..16]);
        
        // Broadcast proposal to all peers
        let peers = self.peer_manager.get_peer_ips().await;
        println!("   📡 Broadcasting to {} peer(s)...", peers.len());
        
        // TODO: Actually broadcast via network
        
        // Cast own vote (proposer always approves)
        println!("   🗳️  Casting my vote: APPROVE");
        let _ = self.consensus.vote_on_block(&block_hash, self.node_id.clone(), true).await;
        
        // Wait for votes from other masternodes
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
        
        // TODO: Listen for proposal from network
        // For now, simulate waiting
        
        time::sleep(Duration::from_secs(5)).await;
        
        // Simulate receiving proposal
        println!("   📬 Block proposal received!");
        
        // Validate proposal
        println!("   🔍 Validating proposal...");
        time::sleep(Duration::from_secs(1)).await;
        
        // Cast vote
        let approve = true; // TODO: Actual validation
        let vote_type = if approve { "APPROVE ✓" } else { "REJECT ✗" };
        println!("   🗳️  Casting vote: {}", vote_type.green());
        
        let block_hash = format!("{:x}", md5::compute(format!("{}{}{}", block_num, proposer, "temp")));
        let _ = self.consensus.vote_on_block(&block_hash, self.node_id.clone(), approve).await;
        
        // Wait to see if quorum is reached
        println!("   ⏳ Waiting for network consensus...");
        time::sleep(Duration::from_secs(10)).await;
        
        let (has_quorum, approvals, rejections, total) = self.consensus.check_quorum(&block_hash).await;
        
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
