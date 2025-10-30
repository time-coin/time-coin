use tokio::sync::RwLock;
use time_core::state::BlockchainState;
use time_core::block::{Block, BlockHeader};
use time_core::transaction::{Transaction, TxOutput};
use time_core::MasternodeTier;
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
    blockchain: Arc<RwLock<BlockchainState>>,
    height_file: String,
}

impl BlockProducer {
    pub fn new(node_id: String, peer_manager: Arc<PeerManager>, consensus: Arc<ConsensusEngine>, blockchain: Arc<RwLock<BlockchainState>>) -> Self {
        BlockProducer {
            node_id,
            peer_manager,
            consensus,
            height_file: "/root/time-coin-node/data/block_height.txt".to_string(),
            blockchain,
        }
    }

    fn load_block_height(&self) -> u64 {
        if let Ok(contents) = std::fs::read_to_string(&self.height_file) {
            contents.trim().parse().unwrap_or(0)
        } else {
            0
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
        let current_height = self.blockchain.read().await.chain_tip_height();
        let now = Utc::now();
        
        // Genesis: Oct 24, 2025 00:00:00 UTC
        let genesis_time = Utc.with_ymd_and_hms(2025, 10, 24, 0, 0, 0).unwrap();
        let duration = now.signed_duration_since(genesis_time);
        let days_since_genesis = duration.num_days();
        
        let expected_height = days_since_genesis as u64;
        
        println!("🔍 Catch-up check:");
        println!("   Current height: {}", current_height);
        println!("   Expected height: {}", expected_height);
        
        if current_height < expected_height {
            let missed_blocks = expected_height - current_height;
            
            println!("\n{}", "⚠️  MISSED BLOCKS DETECTED".yellow().bold());
            println!("   Missing {} block(s)", missed_blocks);
            println!();
            
            // STEP 1: Check if any peers already have these blocks
            println!("{}", "   📡 Checking if peers have these blocks...".cyan());
            let peers = self.peer_manager.get_peer_ips().await;
            
            if !peers.is_empty() {
                // Try to get blockchain info from peers
                for peer in peers.iter().take(3) {  // Check up to 3 peers
                    println!("      Checking {}...", peer.bright_black());
                    
                    // Try to get their blockchain height via API
                    if let Ok(response) = tokio::time::timeout(
                        Duration::from_secs(5),
                        reqwest::get(format!("http://{}:24101/blockchain/info", peer))
                    ).await {
                        if let Ok(resp) = response {
                            if let Ok(info) = resp.json::<serde_json::Value>().await {
                                if let Some(peer_height) = info.get("height").and_then(|h| h.as_u64()) {
                                    println!("      Peer height: {}", peer_height);
                                    
                                    if peer_height >= expected_height {
                                        println!("{}", "      ✓ Peer has all blocks! Syncing from peer...".green());
                                        

                                        // Sync blocks from peer

                                        let mut blockchain = self.blockchain.write().await;

                                        let current_height = blockchain.chain_tip_height();

                                        

                                        for height in (current_height + 1)..=expected_height {

                                            println!("      📥 Downloading block #{}...", height);

                                            

                                            match reqwest::get(format!("http://{}:24101/blockchain/block/{}", peer, height)).await {

                                                Ok(resp) => {

                                                    match resp.json::<serde_json::Value>().await {

                                                        Ok(json) => {

                                                            if let Some(block_data) = json.get("block") {

                                                                match serde_json::from_value::<time_core::block::Block>(block_data.clone()) {

                                                                    Ok(block) => {

                                                                        match blockchain.add_block(block.clone()) {

                                                                            Ok(_) => println!("         ✓ Block #{} synced", height),

                                                                            Err(e) => {

                                                                                println!("         ✗ Failed to add block #{}: {:?}", height, e);

                                                                                println!("      ⚠ Sync failed, stopping");

                                                                                return;

                                                                            }

                                                                        }

                                                                    }

                                                                    Err(e) => {

                                                                        println!("         ✗ Failed to parse block: {:?}", e);

                                                                        return;

                                                                    }

                                                                }

                                                            }

                                                        }

                                                        Err(e) => {

                                                            println!("         ✗ Failed to parse response: {:?}", e);

                                                            return;

                                                        }

                                                    }

                                                }

                                                Err(e) => {

                                                    println!("         ✗ Failed to download block: {:?}", e);

                                                    return;

                                                }

                                            }

                                        }

                                        

                                        println!("{}", "      ✅ Sync complete!".green());

                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
                
                println!("{}", "      ℹ No peers have the missing blocks yet".bright_black());
            }
            
            // STEP 2: Wait for BFT consensus before producing blocks
            println!();
            println!("{}", "   ⏳ Waiting for BFT consensus...".cyan());
            
            // Wait up to 30 seconds for BFT to activate
            for i in 0..30 {
                if self.consensus.has_bft_quorum().await {
                    println!("{}", "   ✓ BFT consensus active!".green());
                    break;
                }
                
                if i == 29 {
                    println!("{}", "   ⚠ BFT not yet active, proceeding anyway...".yellow());
                }
                
                time::sleep(Duration::from_secs(1)).await;
            }
            
            println!();
            
            // DEBUG: Show masternode list and selection
            let masternodes = self.consensus.get_masternodes().await;
            println!("   🔍 Masternode list: {:?}", masternodes);
            let selected = self.consensus.get_block_producer(current_height).await;
            println!("   🎯 Selected producer for block {}: {:?}", current_height, selected);
            println!("   🆔 My ID: {}", self.node_id);
            println!();
            // STEP 3: Check if this node should create the catch-up blocks
            let is_my_turn = self.consensus.is_my_turn(current_height, &self.node_id).await;
            
            if is_my_turn {
                println!("{}", "   🔨 I am the designated producer for catch-up blocks".green());
                println!("   Creating {} missed block(s)...", missed_blocks);
            let mut all_success = true;
                println!();
                
                for i in 0..missed_blocks {
                    let height = current_height + 1 + i;
                    let block_time = genesis_time + chrono::Duration::days(height as i64);
                    
                    println!("   📦 Creating catch-up block #{}...", height);
                    println!("      Timestamp: {}", block_time.format("%Y-%m-%d %H:%M:%S UTC"));
                    
                    if !self.produce_catch_up_block(height, block_time).await {
                        println!("   ⚠ Stopping catch-up due to error");
                        all_success = false;
                        break;
                    }
                    
                    // Save after each block
                    self.save_block_height(height + 1);
                    
                    // Small delay between blocks
                    time::sleep(Duration::from_millis(500)).await;
                }
                
                println!();
            if all_success {
                println!("{}", "   ✅ Catch-up complete!".green().bold());
            } else {
                println!("{}", "   ❌ Catch-up failed!".red());
            }
                println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black());
                println!();
            } else {
                println!("{}", "   👀 Not my turn - waiting for designated producer...".cyan());
                println!("{}", "   (Other node should create blocks shortly)".bright_black());
                println!();
                
                // Wait and monitor for blocks to appear
                println!("{}", "   ⏳ Monitoring for new blocks...".cyan());
                for _attempt in 0..60 {
                    time::sleep(Duration::from_secs(2)).await;
                    let new_height = self.load_block_height();
                    if new_height >= expected_height {
                        println!("{}", "   ✓ Blocks received from designated producer!".green());
                        return;
                    }
                }
                
                println!("{}", "   ⚠ Timeout waiting for blocks".yellow());
            }
        } else {
            println!("   ✓ No missed blocks\n");
        }

        // Verify our chain state with peers after sync
        let blockchain = self.blockchain.read().await;
        let height = blockchain.chain_tip_height();
        let tip_hash = blockchain.chain_tip_hash().to_string();
        drop(blockchain);
        let peers = self.peer_manager.get_peer_ips().await;
        
        let (consensus_reached, _agreements, disagreements) =
            self.consensus.announce_chain_state(height, tip_hash, peers).await;
        
        if !consensus_reached && !disagreements.is_empty() {
            println!("{}", "⚠️  WARNING: Network disagrees with our chain state!".yellow().bold());
        }
    }

    async fn produce_catch_up_block(&self, block_num: u64, timestamp: chrono::DateTime<Utc>) -> bool {
        use time_core::block::{calculate_treasury_reward, calculate_tier_reward};
        
        let mut blockchain = self.blockchain.write().await;
        
        let previous_hash = if block_num == 0 {
            "0000000000000000000000000000000000000000000000000000000000000000".to_string()
        } else {
            blockchain.chain_tip_hash().to_string()
        };
        
        let masternode_counts = blockchain.masternode_counts().clone();
        
        let mut outputs = vec![
            TxOutput { 
                amount: calculate_treasury_reward(),
                address: "TIME1treasury00000000000000000000000000".to_string()
            }
        ];
        
        let active_masternodes = self.consensus.get_masternodes().await;
        if !active_masternodes.is_empty() {
            let tiers = [MasternodeTier::Free, MasternodeTier::Bronze, MasternodeTier::Silver, MasternodeTier::Gold];
            
            for tier in tiers {
                let tier_reward = calculate_tier_reward(tier, &masternode_counts);
                if tier_reward > 0 {
                    let tier_nodes: Vec<_> = active_masternodes.iter()
                        .filter(|mn| mn.starts_with(&format!("{:?}", tier).to_lowercase()))
                        .collect();
                    
                    if !tier_nodes.is_empty() {
                        let reward_per_node = tier_reward / tier_nodes.len() as u64;
                        for node in tier_nodes {
                            outputs.push(TxOutput { amount: reward_per_node, address: node.clone() });
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
        
        let mut block = Block {
            header: BlockHeader {
                block_number: block_num,
                timestamp,
                previous_hash,
                merkle_root: String::new(),
                validator_signature: self.node_id.clone(),
                validator_address: self.node_id.clone(),
            },
            transactions: vec![coinbase_tx],
            hash: String::new(),
        };
        
        block.header.merkle_root = block.calculate_merkle_root();
        block.hash = block.calculate_hash();
        
        println!("      Block Hash: {}...", &block.hash[..16]);
        
        match blockchain.add_block(block.clone()) {
            Ok(_) => {
                println!("      ✓ Block #{} created and stored", block_num);
                let _ = self.consensus.vote_on_block(&block.hash, self.node_id.clone(), true).await;
                true
            }
            Err(e) => {
                println!("      ✗ Failed to add block: {:?}", e);
                false
            }
        }
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
