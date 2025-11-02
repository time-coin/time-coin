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
use chrono::{Utc, TimeZone, NaiveDate, Timelike};
use owo_colors::OwoColorize;
use std::path::Path;
use serde::Deserialize;

#[derive(Deserialize)]
struct BlockchainInfo {
    height: u64,
}

pub struct BlockProducer {
    #[allow(dead_code)]
    node_id: String,
    peer_manager: Arc<PeerManager>,
    consensus: Arc<ConsensusEngine>,
    blockchain: Arc<RwLock<BlockchainState>>,
    height_file: String,
    mempool: Arc<time_mempool::Mempool>,
    block_consensus: Arc<time_consensus::block_consensus::BlockConsensusManager>,
    tx_consensus: Arc<time_consensus::tx_consensus::TxConsensusManager>,
}

impl BlockProducer {
    pub fn new(
        node_id: String,
        peer_manager: Arc<PeerManager>,
        consensus: Arc<ConsensusEngine>,
        blockchain: Arc<RwLock<BlockchainState>>,
        mempool: Arc<time_mempool::Mempool>,
        block_consensus: Arc<time_consensus::block_consensus::BlockConsensusManager>,
        tx_consensus: Arc<time_consensus::tx_consensus::TxConsensusManager>,
        data_dir: String,
    ) -> Self {
        let height_file = format!("{}/block_height.txt", data_dir);
        BlockProducer {
            node_id,
            peer_manager,
            consensus,
            height_file,
            blockchain,
            mempool,
            block_consensus,
            tx_consensus,
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
        let path = Path::new(&self.height_file);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&self.height_file, height.to_string());
    }

    pub async fn start(&self) {
        println!("🔨 Starting block producer...");
        
        // Run catch-up check
        self.catch_up_missed_blocks().await;
        
        println!("✓ Block producer started (24-hour interval)");

        let mut interval = time::interval(Duration::from_secs(60));
        
        loop {
            interval.tick().await;
            // Check for missed blocks (runs every minute)
            self.catch_up_missed_blocks().await;
            
            let now = Utc::now();
            
            // Check if it's midnight UTC
            if now.time().hour() == 0 && now.time().minute() == 0 {
                self.create_and_propose_block().await;
                
                // Sleep for 2 minutes to avoid duplicate triggers
                tokio::time::sleep(Duration::from_secs(120)).await;
            }
        }
    }

    async fn catch_up_missed_blocks(&self) {
        let now = Utc::now();
        let current_date = now.date_naive();
        
        let genesis_date = NaiveDate::from_ymd_opt(2025, 10, 24).unwrap();
        let days_since_genesis = (current_date - genesis_date).num_days();
        let expected_height = days_since_genesis as u64;

        let actual_height = self.load_block_height();

        println!("🔍 Catch-up check:");
        println!("   Current height: {}", actual_height);
        println!("   Expected height: {}", expected_height);

        if actual_height >= expected_height {
            return;
        }

        let missing_blocks = expected_height - actual_height;
        println!("⚠️  MISSED BLOCKS DETECTED");
        println!("   Missing {} block(s)", missing_blocks);

        // CRITICAL: Check consensus mode FIRST - NEVER create blocks in BOOTSTRAP mode
        let consensus_mode = self.consensus.consensus_mode().await;
        if consensus_mode != time_consensus::ConsensusMode::BFT {
            println!("   ⚠️  Cannot create catch-up blocks in BOOTSTRAP mode");
            println!("   ℹ️  Chain sync will download blocks from peers");
            println!("   ⏳ Waiting for BFT mode (need 3+ masternodes)...");
            return;
        }

        // Check if we have enough masternodes
        let masternode_count = self.consensus.masternode_count().await;
        if masternode_count < 3 {
            println!("   ⚠️  Cannot create catch-up blocks: Only {} masternodes", masternode_count);
            println!("   ⏳ Need at least 3 masternodes for catch-up");
            return;
        }

        // CRITICAL: Try to download from peers first
        println!("   📡 Checking if peers have these blocks...");
        
        let peers = self.peer_manager.get_peer_ips().await;
        if !peers.is_empty() {
            for peer_ip in &peers {
                println!("      Checking {}...", peer_ip);
                let url = format!("http://{}:24101/blockchain/info", peer_ip);
                if let Ok(response) = reqwest::get(&url).await {
                    if let Ok(info) = response.json::<BlockchainInfo>().await {
                        if info.height >= expected_height {
                            println!("      Peer height: {}", info.height);
                            println!("      ✓ Peer has all blocks! Syncing from peer...");
                            
                            // Download blocks from peer
                            let mut blockchain = self.blockchain.write().await;
                            let current_height = blockchain.chain_tip_height();
                            
                            for height in (current_height + 1)..=expected_height {
                                println!("      📥 Downloading block #{}...", height);
                                
                                match reqwest::get(format!("http://{}:24101/blockchain/block/{}", peer_ip, height)).await {
                                    Ok(resp) => {
                                        match resp.json::<serde_json::Value>().await {
                                            Ok(json) => {
                                                if let Some(block_data) = json.get("block") {
                                                    match serde_json::from_value::<time_core::block::Block>(block_data.clone()) {
                                                        Ok(block) => {
                                                            match blockchain.add_block(block.clone()) {
                                                                Ok(_) => {
                                                                    println!("         ✓ Block #{} synced", height);
                                                                    self.save_block_height(height);
                                                                }
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
                            println!("      ✅ Sync complete!");
                            return;
                        }
                    }
                }
            }
            println!("      ℹ No peers have the missing blocks yet");
        }

        // Wait for BFT consensus to stabilize
        println!("   ⏳ Waiting for BFT consensus...");
        tokio::time::sleep(Duration::from_secs(30)).await;

        // Recheck consensus mode after wait
        let consensus_mode = self.consensus.consensus_mode().await;
        if consensus_mode != time_consensus::ConsensusMode::BFT {
            println!("   ⚠ BFT not yet active, aborting catch-up");
            return;
        }

        // Determine which node should create catch-up blocks
        let masternodes = self.consensus.get_masternodes().await;
        println!("   🔍 Masternode list: {:?}", masternodes);

        // Create catch-up blocks
        println!("   Processing with BFT consensus: {} missed block(s)...", missing_blocks);
        
        for block_num in (actual_height + 1)..=expected_height {
            let timestamp_date = genesis_date + chrono::Duration::days(block_num as i64);
            let timestamp = Utc.from_utc_datetime(
                &timestamp_date.and_hms_opt(0, 0, 0).unwrap()
            );
            
            let success = self.produce_catchup_block_with_bft_consensus(block_num, timestamp, &masternodes).await;
            if !success {
                println!("   ✗ Failed to create block {}", block_num);
                break;
            }
        }

        println!("   ✅ Catch-up complete!");
    }

    fn select_block_producer(&self, masternodes: &[String], block_height: u64) -> Option<String> {
        if masternodes.is_empty() {
            return None;
        }
        
        let mut sorted_nodes = masternodes.to_vec();
        sorted_nodes.sort();
        
        let index = (block_height as usize) % sorted_nodes.len();
        Some(sorted_nodes[index].clone())
    }

    async fn create_and_propose_block(&self) {
        let now = Utc::now();
        let block_num = self.load_block_height() + 1;

        println!("\n{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan().bold());
        println!("{} {}", "⏰ BLOCK PRODUCTION TIME".cyan().bold(), now.format("%Y-%m-%d %H:%M:%S UTC"));
        println!("{} {}", "   Block Height:".bright_black(), block_num.to_string().cyan().bold());
        println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan().bold());

        // Check consensus mode
        let consensus_mode = self.consensus.consensus_mode().await;
        if consensus_mode != time_consensus::ConsensusMode::BFT {
            println!("{}", "⚠️  Not in BFT mode - skipping block production".yellow());
            return;
        }

        // Determine block producer
        let masternodes = self.consensus.get_masternodes().await;
        let selected_producer = self.select_block_producer(&masternodes, block_num);

        let my_id = if let Ok(ip) = local_ip_address::local_ip() {
            ip.to_string()
        } else {
            "unknown".to_string()
        };

        if let Some(producer) = selected_producer {
            if producer != my_id {
                println!("   ℹ️  Block producer: {} (not me)", producer);
                return;
            }
        }

        println!("{}", "   🔨 I am the designated block producer".green().bold());

        // Step 1: Get transactions from mempool
        let transactions = self.mempool.get_all_transactions().await;
        
        if transactions.is_empty() {
            println!("   📋 No pending transactions in mempool");
        } else {
            println!("   📋 Including {} transactions", transactions.len());
        }

        let mut blockchain = self.blockchain.write().await;

        let previous_hash = if block_num == 0 {
            "0000000000000000000000000000000000000000000000000000000000000000".to_string()
        } else {
            blockchain.chain_tip_hash().to_string()
        };

        let masternode_counts = blockchain.masternode_counts().clone();

        // Step 2: Create coinbase transaction with rewards
        let mut outputs = vec![
            TxOutput {
                amount: time_core::block::calculate_treasury_reward(),
                address: "TIME1treasury00000000000000000000000000".to_string()
            }
        ];

        // Get list of masternodes that voted (participated in consensus)
        let agreed_tx_set = self.tx_consensus.get_agreed_tx_set(block_num).await;
        let voters = if let Some(proposal) = agreed_tx_set {
            self.tx_consensus.get_voters(block_num, &proposal.merkle_root).await
        } else {
            Vec::new()
        };

        // Only reward masternodes that participated in voting
        let active_masternodes = self.consensus.get_masternodes_with_wallets().await;
        let participating_masternodes: Vec<_> = active_masternodes.into_iter()
            .filter(|(node_id, _)| voters.contains(node_id))
            .collect();

        if !participating_masternodes.is_empty() {
            println!("   💰 Rewarding {} participating masternodes", participating_masternodes.len());
            
            let tiers = [MasternodeTier::Free, MasternodeTier::Bronze, MasternodeTier::Silver, MasternodeTier::Gold];
            for tier in tiers {
                let tier_reward = time_core::block::calculate_tier_reward(tier, &masternode_counts);
                if tier_reward > 0 {
                    let tier_nodes: Vec<_> = participating_masternodes.iter()
                        .filter(|(node_id, _)| node_id.starts_with(&format!("{:?}", tier).to_lowercase()))
                        .collect();

                    if !tier_nodes.is_empty() {
                        let reward_per_node = tier_reward / tier_nodes.len() as u64;
                        for (_, wallet_addr) in tier_nodes {
                            outputs.push(TxOutput { amount: reward_per_node, address: wallet_addr.clone() });
                        }
                    }
                }
            }
        } else {
            println!("   ⚠️  No masternodes participated in voting - no rewards distributed");
        }

        let coinbase_tx = Transaction {
            txid: format!("coinbase_{}", block_num),
            version: 1,
            inputs: vec![],
            outputs,
            lock_time: 0,
            timestamp: now.timestamp(),
        };

        let mut block_transactions = vec![coinbase_tx];
        block_transactions.extend(transactions);

        // Step 3: Create block
        let mut block = Block {
            hash: String::new(),
            header: BlockHeader {
                block_number: block_num,
                timestamp: now,
                previous_hash,
                merkle_root: String::new(),
                validator_address: my_id.clone(),
                validator_signature: my_id.clone(),
            },
            transactions: block_transactions,
        };

        // Calculate merkle root and hash
        block.header.merkle_root = block.calculate_merkle_root();
        block.hash = block.calculate_hash();

        println!("   📦 Block created:");
        println!("      Hash: {}...", &block.hash[..16]);
        println!("      Transactions: {}", block.transactions.len());

        // Step 4: Add to blockchain
        match blockchain.add_block(block.clone()) {
            Ok(_) => {
                self.save_block_height(block_num);
                println!("{}", "   ✅ BLOCK ADDED TO CHAIN".green().bold());
                
                // Remove transactions from mempool
                for tx in block.transactions.iter().skip(1) {
                    let _ = self.mempool.remove_transaction(&tx.txid).await;
                }
            }
            Err(e) => {
                println!("{} {:?}", "   ✗ Failed to add block:".red(), e);
                return;
            }
        }

        // Step 5: Announce to network
        let height = blockchain.chain_tip_height();
        let tip_hash = blockchain.chain_tip_hash().to_string();
        drop(blockchain);

        let peers = self.peer_manager.get_peer_ips().await;
        let (consensus_reached, _agreements, disagreements) =
            self.consensus.announce_chain_state(height, tip_hash, peers).await;

        if !consensus_reached && !disagreements.is_empty() {
            println!("{}", "⚠️  WARNING: Network disagrees with our chain state!".yellow().bold());
        }

        println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan().bold());
        println!("⏰ Next block production at: {}", (now + chrono::Duration::days(1)).format("%Y-%m-%d %H:%M:%S UTC"));
        let hours_left = 23 - now.time().hour();
        let minutes_left = 60 - now.time().minute();
        println!("   Waiting {} hours {} minutes...", hours_left, minutes_left);
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

        // For catch-up blocks, also filter by participation
        let agreed_tx_set = self.tx_consensus.get_agreed_tx_set(block_num).await;
        let voters = if let Some(proposal) = agreed_tx_set {
            self.tx_consensus.get_voters(block_num, &proposal.merkle_root).await
        } else {
            Vec::new()
        };

        let active_masternodes = self.consensus.get_masternodes_with_wallets().await;
        let participating_masternodes: Vec<_> = active_masternodes.into_iter()
            .filter(|(node_id, _)| voters.contains(node_id))
            .collect();

        if !participating_masternodes.is_empty() {
            let tiers = [MasternodeTier::Free, MasternodeTier::Bronze, MasternodeTier::Silver, MasternodeTier::Gold];
            for tier in tiers {
                let tier_reward = calculate_tier_reward(tier, &masternode_counts);
                if tier_reward > 0 {
                    let tier_nodes: Vec<_> = participating_masternodes.iter()
                        .filter(|(node_id, _)| node_id.starts_with(&format!("{:?}", tier).to_lowercase()))
                        .collect();

                    if !tier_nodes.is_empty() {
                        let reward_per_node = tier_reward / tier_nodes.len() as u64;
                        for (_, wallet_addr) in tier_nodes {
                            outputs.push(TxOutput { amount: reward_per_node, address: wallet_addr.clone() });
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

        let my_id = if let Ok(ip) = local_ip_address::local_ip() {
            ip.to_string()
        } else {
            "unknown".to_string()
        };

        let mut block = Block {
            hash: String::new(),
            header: BlockHeader {
                block_number: block_num,
                timestamp,
                previous_hash,
                merkle_root: String::new(),
                validator_signature: my_id.clone(),
                validator_address: my_id.clone(),
            },
            transactions: vec![coinbase_tx],
        };

        // Calculate merkle root and hash
        block.header.merkle_root = block.calculate_merkle_root();
        block.hash = block.calculate_hash();

        println!("   📦 Creating catch-up block #{}...", block_num);
        println!("      Timestamp: {}", timestamp.format("%Y-%m-%d %H:%M:%S UTC"));
        println!("      Block Hash: {}...", &block.hash[..16]);

        match blockchain.add_block(block) {
            Ok(_) => {
                self.save_block_height(block_num);
                println!("      ✓ Block #{} created and stored", block_num);
                true
            }
            Err(e) => {
                println!("      ✗ Failed to create block {}: {:?}", block_num, e);
                false
            }
        }
    }

    async fn produce_catchup_block_with_bft_consensus(
        &self,
        block_num: u64,
        timestamp: chrono::DateTime<Utc>,
        masternodes: &[String],
    ) -> bool {
        use time_consensus::block_consensus::{BlockProposal, BlockVote};
        
        // Determine producer for THIS specific block
        let selected_producer = self.select_block_producer(masternodes, block_num);
        
        let my_id = if let Ok(ip) = local_ip_address::local_ip() {
            ip.to_string()
        } else {
            "unknown".to_string()
        };
        
        println!("   📦 Block #{} - Producer: {:?}", block_num, selected_producer);
        
        // Step 1: If I'm the producer, create and broadcast proposal
        if let Some(ref producer) = selected_producer {
            if producer == &my_id {
                println!("      🔨 I'm the producer - creating block proposal...");
                
                // Create the block (without adding to chain yet)
                let block = self.create_catchup_block_structure(block_num, timestamp).await;
                
                // Create proposal
                let proposal = BlockProposal {
                    block_height: block_num,
                    proposer: my_id.clone(),
                    block_hash: block.hash.clone(),
                    merkle_root: block.header.merkle_root.clone(),
                    previous_hash: block.header.previous_hash.clone(),
                    timestamp: timestamp.timestamp(),
                };
                
                // Store locally
                self.block_consensus.propose_block(proposal.clone()).await;
                
                // Broadcast to all masternodes
                self.broadcast_block_proposal(proposal, masternodes).await;
                
                // Auto-vote approve
                let vote = BlockVote {
                    block_height: block_num,
                    block_hash: block.hash.clone(),
                    voter: my_id.clone(),
                    approve: true,
                    timestamp: chrono::Utc::now().timestamp(),
                };
                let _ = self.block_consensus.vote_on_block(vote.clone()).await;
                self.broadcast_block_vote(vote, masternodes).await;
            }
        }
        
        // Step 2: Wait for proposal and vote (all nodes including producer)
        println!("      ⏳ Waiting for block proposal and consensus...");
        
        for attempt in 0..30 {
            tokio::time::sleep(Duration::from_secs(1)).await;
            
            // Check if we have a proposal
            if let Some(proposal) = self.block_consensus.get_proposal(block_num).await {
                // Vote if we haven't already (non-producers vote here)
                if selected_producer.as_ref() != Some(&my_id) {
                    let vote = BlockVote {
                        block_height: block_num,
                        block_hash: proposal.block_hash.clone(),
                        voter: my_id.clone(),
                        approve: true,
                        timestamp: chrono::Utc::now().timestamp(),
                    };
                    
                    if let Ok(_) = self.block_consensus.vote_on_block(vote.clone()).await {
                        self.broadcast_block_vote(vote, masternodes).await;
                    }
                }
                
                // Check for consensus
                let (has_consensus, approvals, total) = self.block_consensus
                    .has_block_consensus(block_num, &proposal.block_hash).await;
                
                if has_consensus {
                    println!("      ✅ Consensus reached! ({}/{} votes)", approvals, total);
                    
                    // Get list of voters for rewards
                    let voters = self.block_consensus.get_voters(block_num, &proposal.block_hash).await;
                    
                    // Finalize the block with rewards to voters
                    return self.finalize_catchup_block_with_rewards(
                        block_num,
                        timestamp,
                        &voters
                    ).await;
                } else if attempt % 5 == 0 {
                    println!("      ⏳ Waiting for consensus: {}/{} votes", approvals, total);
                }
            }
        }
        
        println!("      ⚠️  Timeout - no consensus reached for block {}", block_num);
        false
    }

    async fn create_catchup_block_structure(
        &self,
        block_num: u64,
        timestamp: chrono::DateTime<Utc>,
    ) -> time_core::block::Block {
        use time_core::block::{Block, BlockHeader};
        use time_core::transaction::{Transaction, TxOutput};
        use time_core::MasternodeTier;
        
        let blockchain = self.blockchain.read().await;
        let previous_hash = blockchain.chain_tip_hash().to_string();
        let masternode_counts = blockchain.masternode_counts().clone();
        drop(blockchain);
        
        let my_id = if let Ok(ip) = local_ip_address::local_ip() {
            ip.to_string()
        } else {
            "unknown".to_string()
        };
        
        // Create coinbase with treasury reward only (no MN rewards yet)
        let coinbase_tx = Transaction {
            txid: format!("coinbase_{}", block_num),
            version: 1,
            inputs: vec![],
            outputs: vec![TxOutput {
                amount: time_core::block::calculate_treasury_reward(),
                address: "TIME1treasury00000000000000000000000000".to_string()
            }],
            lock_time: 0,
            timestamp: timestamp.timestamp(),
        };
        
        let mut block = Block {
            hash: String::new(),
            header: BlockHeader {
                block_number: block_num,
                timestamp,
                previous_hash,
                merkle_root: String::new(),
                validator_signature: my_id.clone(),
                validator_address: my_id.clone(),
            },
            transactions: vec![coinbase_tx],
        };
        
        block.header.merkle_root = block.calculate_merkle_root();
        block.hash = block.calculate_hash();
        block
    }

    async fn broadcast_block_proposal(
        &self,
        proposal: time_consensus::block_consensus::BlockProposal,
        masternodes: &[String],
    ) {
        for node in masternodes {
            let url = format!("http://{}:24101/consensus/block-proposal", node);
            let _ = reqwest::Client::new()
                .post(&url)
                .json(&proposal)
                .send()
                .await;
        }
    }

    async fn broadcast_block_vote(
        &self,
        vote: time_consensus::block_consensus::BlockVote,
        masternodes: &[String],
    ) {
        for node in masternodes {
            let url = format!("http://{}:24101/consensus/block-vote", node);
            let _ = reqwest::Client::new()
                .post(&url)
                .json(&vote)
                .send()
                .await;
        }
    }

    async fn finalize_catchup_block_with_rewards(
        &self,
        block_num: u64,
        timestamp: chrono::DateTime<Utc>,
        voters: &[String],
    ) -> bool {
        use time_core::block::{Block, BlockHeader};
        use time_core::transaction::{Transaction, TxOutput};
        use time_core::MasternodeTier;
        use time_core::block::{calculate_treasury_reward, calculate_tier_reward};
        
        let mut blockchain = self.blockchain.write().await;
        let previous_hash = blockchain.chain_tip_hash().to_string();
        let masternode_counts = blockchain.masternode_counts().clone();
        
        let my_id = if let Ok(ip) = local_ip_address::local_ip() {
            ip.to_string()
        } else {
            "unknown".to_string()
        };
        
        // Get wallet addresses by querying each voter's API
        let mut voter_wallets: Vec<(String, String)> = Vec::new();
        println!("      🐛 DEBUG: voters = {:?}", voters);
        
        for voter in voters {
            let url = format!("http://{}:24101/wallet/address", voter);
            if let Ok(response) = reqwest::Client::new()
                .get(&url)
                .timeout(std::time::Duration::from_secs(2))
                .send()
                .await
            {
                if let Ok(wallet_info) = response.json::<serde_json::Value>().await {
                    if let Some(address) = wallet_info.get("address").and_then(|a| a.as_str()) {
                        println!("      🐛 DEBUG: {} wallet = {}", voter, address);
                        voter_wallets.push((voter.clone(), address.to_string()));
                    }
                }
            }
        }
        println!("      🐛 DEBUG: voter_wallets = {:?}", voter_wallets);
        println!("      🐛 DEBUG: voter_wallets = {:?}", voter_wallets);
        
        // Build outputs with treasury + voter rewards
        let mut outputs = vec![TxOutput {
            amount: calculate_treasury_reward(),
            address: "TIME1treasury00000000000000000000000000".to_string()
        }];
        
        if !voter_wallets.is_empty() {
            println!("      💰 Rewarding {} voters", voter_wallets.len());
            
            let tiers = [MasternodeTier::Free, MasternodeTier::Bronze, MasternodeTier::Silver, MasternodeTier::Gold];
            for tier in tiers {
                let tier_reward = calculate_tier_reward(tier, &masternode_counts);
                if tier_reward > 0 {
                    let tier_nodes: Vec<_> = voter_wallets.iter()
                        .filter(|(node_id, _)| node_id.starts_with(&format!("{:?}", tier).to_lowercase()))
                        .collect();
                    
                    if !tier_nodes.is_empty() {
                        let reward_per_node = tier_reward / tier_nodes.len() as u64;
                        for (_, wallet_addr) in tier_nodes {
                            outputs.push(TxOutput {
                                amount: reward_per_node,
                                address: wallet_addr.clone()
                            });
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
            hash: String::new(),
            header: BlockHeader {
                block_number: block_num,
                timestamp,
                previous_hash,
                merkle_root: String::new(),
                validator_signature: my_id.clone(),
                validator_address: my_id.clone(),
            },
            transactions: vec![coinbase_tx],
        };
        
        block.header.merkle_root = block.calculate_merkle_root();
        block.hash = block.calculate_hash();
        
        println!("      📦 Finalizing block #{}...", block_num);
        match blockchain.add_block(block) {
            Ok(_) => {
                self.save_block_height(block_num);
                println!("      ✓ Block #{} finalized and stored", block_num);
                true
            }
            Err(e) => {
                println!("      ✗ Failed to finalize block: {:?}", e);
                false
            }
        }
    }
}
