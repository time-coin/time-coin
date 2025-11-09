use chrono::{NaiveDate, TimeZone, Utc};
use owo_colors::OwoColorize;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use time_consensus::ConsensusEngine;
use time_core::block::{Block, BlockHeader};
use time_core::state::BlockchainState;
use time_core::transaction::{Transaction, TxOutput};
use time_core::MasternodeTier;
use time_network::PeerManager;
use tokio::sync::RwLock;

/// Treasury address for block rewards
const TREASURY_ADDRESS: &str = "TIME1treasury00000000000000000000000000";

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
    mempool: Arc<time_mempool::Mempool>,
    block_consensus: Arc<time_consensus::block_consensus::BlockConsensusManager>,
    #[allow(dead_code)]
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
        #[allow(dead_code)] tx_consensus: Arc<time_consensus::tx_consensus::TxConsensusManager>,
    ) -> Self {
        BlockProducer {
            node_id,
            peer_manager,
            consensus,
            blockchain,
            mempool,
            block_consensus,
            tx_consensus,
        }
    }

    async fn load_block_height(&self) -> u64 {
        let blockchain = self.blockchain.read().await;
        blockchain.chain_tip_height()
    }

    pub async fn start(&self) {
        println!("Starting block producer...");

        // Run initial catch-up check

        println!("Block producer started (24-hour interval)");

        // Main loop: sleep until midnight, then produce block
        loop {
            let now = Utc::now();

            // Run a catch-up check each iteration
            self.catch_up_missed_blocks().await;

            // Calculate next midnight UTC
            let tomorrow = now.date_naive() + chrono::Duration::days(1);
            let next_midnight = tomorrow.and_hms_opt(0, 0, 0).unwrap().and_utc();

            let duration_until_midnight = (next_midnight - now)
                .to_std()
                .unwrap_or(Duration::from_secs(60));

            let hours = duration_until_midnight.as_secs() / 3600;
            let minutes = (duration_until_midnight.as_secs() % 3600) / 60;
            let seconds = duration_until_midnight.as_secs() % 60;

            println!(
                "Next block at {} UTC (in {}h {}m {}s)",
                next_midnight.format("%Y-%m-%d %H:%M:%S"),
                hours,
                minutes,
                seconds
            );

            // Sleep until midnight
            tokio::time::sleep(duration_until_midnight).await;

            // It's midnight! Produce block immediately
            println!("Midnight reached - producing block...");
            self.create_and_propose_block().await;

            // Sleep a few seconds to avoid duplicate triggers
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    async fn catch_up_missed_blocks(&self) {
        let now = Utc::now();
        let current_date = now.date_naive();

        let genesis_date = NaiveDate::from_ymd_opt(2025, 10, 24).unwrap();
        let days_since_genesis = (current_date - genesis_date).num_days();
        let expected_height = days_since_genesis as u64;

        let actual_height = self.load_block_height().await;

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
            println!("   ▶️ Waiting for BFT mode (need 3+ masternodes)...");
            return;
        }

        // Check if we have enough masternodes
        let masternode_count = self.consensus.masternode_count().await;
        if masternode_count < 3 {
            println!(
                "   ⚠️  Cannot create catch-up blocks: Only {} masternodes",
                masternode_count
            );
            println!("   ▶️ Need at least 3 masternodes for catch-up");
            return;
        }

        // CRITICAL: Try to download from peers first
        println!("   🔍 Checking if peers have these blocks...");

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
                                println!("      🔽 Downloading block #{}...", height);

                                match reqwest::get(format!(
                                    "http://{}:24101/blockchain/block/{}",
                                    peer_ip, height
                                ))
                                .await
                                {
                                    Ok(resp) => match resp.json::<serde_json::Value>().await {
                                        Ok(json) => {
                                            if let Some(block_data) = json.get("block") {
                                                match serde_json::from_value::<
                                                    time_core::block::Block,
                                                >(
                                                    block_data.clone()
                                                ) {
                                                    Ok(block) => {
                                                        match blockchain.add_block(block.clone()) {
                                                            Ok(_) => {
                                                                println!(
                                                                    "         ✓ Block #{} synced",
                                                                    height
                                                                );
                                                            }
                                                            Err(e) => {
                                                                println!("         ✗ Failed to add block #{}: {:?}", height, e);
                                                                println!("      ⚠️ Sync failed, stopping");
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
                                            println!(
                                                "         ✗ Failed to parse response: {:?}",
                                                e
                                            );
                                            return;
                                        }
                                    },
                                    Err(e) => {
                                        println!("         ✗ Failed to download block: {:?}", e);
                                        return;
                                    }
                                }
                            }
                            println!("      ✔ Sync complete!");
                            return;
                        }
                    }
                }
            }
            println!("      ℹ️ No peers have the missing blocks yet");
        }

        // Wait for BFT consensus to stabilize
        println!("   ▶️ Waiting for BFT consensus...");
        tokio::time::sleep(Duration::from_secs(30)).await;

        // Recheck consensus mode after wait
        let consensus_mode = self.consensus.consensus_mode().await;
        if consensus_mode != time_consensus::ConsensusMode::BFT {
            println!("   ⚠️ BFT not yet active, aborting catch-up");
            return;
        }

        // Determine which node should create catch-up blocks
        let masternodes = self.consensus.get_masternodes().await;
        println!("   🔍 Masternode list: {:?}", masternodes);

        // Create catch-up blocks
        println!(
            "   Processing with BFT consensus: {} missed block(s)...",
            missing_blocks
        );

        for block_num in (actual_height + 1)..=expected_height {
            let timestamp_date = genesis_date + chrono::Duration::days(block_num as i64);
            let timestamp = Utc.from_utc_datetime(&timestamp_date.and_hms_opt(0, 0, 0).unwrap());

            // Retry up to 5 times with increasing delays
            let mut success = false;
            for retry_attempt in 0..5 {
                if retry_attempt > 0 {
                    let delay_secs = 15 + (retry_attempt * 5);
                    println!(
                        "   🔄 Retry attempt {}/5 for block {} (waiting {}s)...",
                        retry_attempt + 1,
                        block_num,
                        delay_secs
                    );
                    tokio::time::sleep(Duration::from_secs(delay_secs as u64)).await;
                }

                success = self
                    .produce_catchup_block_with_bft_consensus(block_num, timestamp, &masternodes)
                    .await;

                if success {
                    println!("   ✅ Block {} created successfully!", block_num);
                    break;
                } else if retry_attempt < 4 {
                    println!(
                        "   ⚠️  Attempt {}/5 failed, will retry...",
                        retry_attempt + 1
                    );
                }
            }

            if !success {
                println!(
                    "   ❌ Failed to create block {} after 5 attempts",
                    block_num
                );
                println!("   ℹ️  Ensure all nodes are running same version");
                break;
            }
        }

        println!("   ✔ Catch-up complete!");
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
        let block_num = self.load_block_height().await + 1;

        println!(
            "
────────────────────────────────────────────────────────────────"
        );
        println!(
            "{} {}",
            "⨯ BLOCK PRODUCTION TIME".cyan().bold(),
            now.format("%Y-%m-%d %H:%M:%S UTC")
        );
        println!(
            "{} {}",
            "   Block Height:".bright_black(),
            block_num.to_string().cyan().bold()
        );
        println!("────────────────────────────────────────────────────────────────");

        let consensus_mode = self.consensus.consensus_mode().await;
        if consensus_mode != time_consensus::ConsensusMode::BFT {
            println!("{}", "⚠️  Not in BFT mode".yellow());
            return;
        }

        let all_masternodes = self.consensus.get_masternodes().await;

        // Initialize health tracking for any new masternodes
        for mn in &all_masternodes {
            self.block_consensus
                .init_masternode_health(mn.clone())
                .await;
        }

        // Get only active masternodes for consensus
        let masternodes = self
            .block_consensus
            .get_active_masternodes(&all_masternodes)
            .await;
        let required_votes = ((masternodes.len() * 2) / 3) + 1;

        if masternodes.len() < all_masternodes.len() {
            println!(
                "   ⚠️  {} masternode(s) excluded from consensus",
                all_masternodes.len() - masternodes.len()
            );
        }

        let selected_producer = self.select_block_producer(&masternodes, block_num);
        let my_id = std::env::var("NODE_PUBLIC_IP").unwrap_or_else(|_| {
            if let Ok(ip) = local_ip_address::local_ip() {
                ip.to_string()
            } else {
                "unknown".to_string()
            }
        });

        let am_i_leader = selected_producer
            .as_ref()
            .map(|p| p == &my_id)
            .unwrap_or(false);

        if am_i_leader {
            println!("{}", "   🟢 I am the block producer".green().bold());

            let mut transactions = self.mempool.get_all_transactions().await;
            // Sort transactions deterministically by txid to ensure same merkle root
            transactions.sort_by(|a, b| a.txid.cmp(&b.txid));
            println!("   📋 {} mempool transactions", transactions.len());

            // Get blockchain state atomically (all data retrieved while holding read lock)
            let blockchain = self.blockchain.read().await;
            let previous_hash = blockchain.chain_tip_hash().to_string();
            let masternode_counts = blockchain.masternode_counts().clone();

            // Get active masternodes with their wallet addresses and tiers
            // Note: masternode_counts and active_masternodes represent the same state
            let active_masternodes: Vec<(String, time_core::MasternodeTier)> = blockchain
                .get_active_masternodes()
                .iter()
                .map(|mn| (mn.wallet_address.clone(), mn.tier))
                .collect();

            drop(blockchain);

            // Calculate total transaction fees (currently 0 as we don't have UTXO validation yet)
            let total_fees: u64 = 0;

            // Log masternode reward distribution
            if !active_masternodes.is_empty() {
                println!(
                    "   💰 Distributing rewards to {} masternodes:",
                    active_masternodes.len()
                );
                for (wallet_addr, tier) in &active_masternodes {
                    println!("      - {:?} tier: {}", tier, wallet_addr);
                }
            } else {
                println!("   ⚠️  No active masternodes registered for rewards");
            }

            // Create coinbase transaction with all rewards
            let coinbase_tx = time_core::block::create_coinbase_transaction(
                block_num,
                TREASURY_ADDRESS,
                &active_masternodes,
                &masternode_counts,
                total_fees,
            );

            // Prepend coinbase to transactions list
            let mut all_transactions = vec![coinbase_tx];
            let mempool_count = transactions.len();
            all_transactions.extend(transactions);

            if mempool_count == 0 {
                println!("   📦 Block will contain ONLY coinbase transaction (zero regular txs)");
            }

            println!(
                "   📋 {} total transactions (1 coinbase + {} mempool)",
                all_transactions.len(),
                mempool_count
            );

            let merkle_root = self.calc_merkle(&all_transactions);

            let proposal = time_consensus::block_consensus::BlockProposal {
                block_height: block_num,
                proposer: my_id.clone(),
                block_hash: "".to_string(),
                merkle_root: merkle_root.clone(),
                previous_hash: previous_hash.clone(),
                timestamp: now.timestamp(),
            };

            self.block_consensus.store_proposal(proposal.clone()).await;

            let proposal_json = serde_json::to_value(&proposal).unwrap();
            self.peer_manager
                .broadcast_block_proposal(proposal_json)
                .await;

            println!("   📡 Proposal broadcast");
            println!(
                "   ▶️ Collecting votes (need {}/{})...",
                required_votes,
                masternodes.len()
            );

            let (approved, total) = self
                .block_consensus
                .collect_votes(block_num, required_votes)
                .await;

            // Track missed votes for health monitoring
            for mn in &masternodes {
                let voters = self
                    .block_consensus
                    .get_voters(block_num, &proposal.block_hash)
                    .await;
                if !voters.contains(mn) {
                    self.block_consensus.record_missed_vote(mn).await;
                }
            }

            println!("   🗳️ Votes: {}/{} approved", approved, total);

            if approved >= required_votes {
                println!("   ✔ Quorum reached! Finalizing...");
                self.finalize_block_bft(&all_transactions, &previous_hash, &merkle_root, block_num)
                    .await;
            } else {
                println!("   ✗ Quorum failed ({} < {})", approved, required_votes);
            }
        } else {
            println!(
                "   ℹ️  Producer: {}",
                selected_producer.as_deref().unwrap_or("unknown")
            );
            println!("   ⏳ Waiting for proposal...");

            if let Some(proposal) = self.block_consensus.wait_for_proposal(block_num).await {
                println!("   📨 Received from {}", proposal.proposer);

                let blockchain = self.blockchain.read().await;
                let is_valid = self.block_consensus.validate_proposal(
                    &proposal,
                    blockchain.chain_tip_hash(),
                    blockchain.chain_tip_height(),
                );
                drop(blockchain);

                let vote = time_consensus::block_consensus::BlockVote {
                    block_height: block_num,
                    block_hash: proposal.block_hash.clone(),
                    voter: my_id.clone(),
                    approve: is_valid,
                    timestamp: Utc::now().timestamp(),
                };

                self.block_consensus.store_vote(vote.clone()).await;

                let vote_json = serde_json::to_value(&vote).unwrap();
                self.peer_manager.broadcast_block_vote(vote_json).await;

                println!(
                    "   {} Voted {}",
                    if is_valid { "✓" } else { "✗" },
                    if is_valid { "APPROVE" } else { "REJECT" }
                );

                let (approved, _total) = self
                    .block_consensus
                    .collect_votes(block_num, required_votes)
                    .await;

                if approved >= required_votes {
                    println!("   ✅ Block approved - fetching finalized block...");

                    // Actively fetch the finalized block from producer
                    if let Some(producer_id) = selected_producer {
                        if let Some(block) = self
                            .fetch_finalized_block(&producer_id, block_num, &proposal.merkle_root)
                            .await
                        {
                            // Apply the finalized block
                            let mut blockchain = self.blockchain.write().await;
                            match blockchain.add_block(block) {
                                Ok(_) => {
                                    println!("   ✅ Block {} applied from producer", block_num);
                                }
                                Err(e) => {
                                    println!("   ⚠️  Failed to apply fetched block: {:?}", e);
                                    println!("   ⏳ Falling back to catch-up...");
                                }
                            }
                        } else {
                            println!("   ⚠️  Failed to fetch block, falling back to catch-up");
                        }
                    }
                } else {
                    println!("   ✗ Block rejected");
                }
            } else {
                println!("   ⚠️  Timeout");
            }
        }
    }

    fn calc_merkle(&self, transactions: &[time_core::Transaction]) -> String {
        if transactions.is_empty() {
            return "0".repeat(64);
        }

        use sha3::{Digest, Sha3_256};

        // Build proper merkle tree (matching Block::calculate_merkle_root in core/src/block.rs)
        let mut hashes: Vec<String> = transactions.iter().map(|tx| tx.txid.clone()).collect();

        // Build merkle tree iteratively
        while hashes.len() > 1 {
            let mut next_level = Vec::new();

            for i in (0..hashes.len()).step_by(2) {
                let left = &hashes[i];
                let right = if i + 1 < hashes.len() {
                    &hashes[i + 1]
                } else {
                    left // Duplicate if odd number
                };

                let combined = format!("{}{}", left, right);
                let hash = Sha3_256::digest(combined.as_bytes());
                next_level.push(hex::encode(hash));
            }

            hashes = next_level;
        }

        hashes[0].clone()
    }

    /// Broadcast finalized block to peers (best-effort)
    async fn broadcast_finalized_block(
        &self,
        block: &time_core::block::Block,
        masternodes: &[String],
    ) {
        let block_json = match serde_json::to_value(block) {
            Ok(json) => json,
            Err(e) => {
                println!("   ⚠️  Failed to serialize block for broadcast: {:?}", e);
                return;
            }
        };

        let payload = serde_json::json!({
            "block": block_json
        });

        for node in masternodes {
            let url = format!("http://{}:24101/consensus/finalized-block", node);
            let payload_clone = payload.clone();

            // Fire-and-forget, best-effort broadcast
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                if let Err(e) = client
                    .post(&url)
                    .json(&payload_clone)
                    .timeout(std::time::Duration::from_secs(2))
                    .send()
                    .await
                {
                    // Log warning but don't fail - best effort only
                    eprintln!("   ⚠️  Failed to broadcast to {}: {:?}", url, e);
                }
            });
        }
    }

    /// Attempt to fetch finalized block from producer with retries
    async fn fetch_finalized_block(
        &self,
        producer: &str,
        height: u64,
        expected_merkle: &str,
    ) -> Option<time_core::block::Block> {
        const MAX_ATTEMPTS: u32 = 8;
        const RETRY_DELAY_MS: u64 = 500;

        for attempt in 1..=MAX_ATTEMPTS {
            let url = format!("http://{}:24101/consensus/block/{}", producer, height);

            match reqwest::Client::new()
                .get(&url)
                .timeout(std::time::Duration::from_secs(2))
                .send()
                .await
            {
                Ok(response) => {
                    if let Ok(json) = response.json::<serde_json::Value>().await {
                        if let Some(block_data) = json.get("block") {
                            if let Ok(block) = serde_json::from_value::<time_core::block::Block>(
                                block_data.clone(),
                            ) {
                                // Validate merkle root matches proposal
                                if block.header.merkle_root == expected_merkle {
                                    println!("   ✅ Fetched finalized block from {}", producer);
                                    return Some(block);
                                } else {
                                    println!(
                                        "   ⚠️  Merkle mismatch: expected {}, got {}",
                                        &expected_merkle[..16],
                                        &block.header.merkle_root[..16]
                                    );
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    if attempt < MAX_ATTEMPTS {
                        println!(
                            "   ⏳ Fetch attempt {}/{} failed, retrying... ({:?})",
                            attempt, MAX_ATTEMPTS, e
                        );
                        tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                    } else {
                        println!("   ⚠️  All fetch attempts failed: {:?}", e);
                    }
                }
            }
        }

        None
    }

    async fn finalize_block_bft(
        &self,
        transactions: &[time_core::Transaction],
        previous_hash: &str,
        merkle_root: &str,
        block_num: u64,
    ) {
        use sha2::{Digest, Sha256};
        use time_core::{Block, BlockHeader};

        let my_id = std::env::var("NODE_PUBLIC_IP").unwrap_or_else(|_| {
            if let Ok(ip) = local_ip_address::local_ip() {
                ip.to_string()
            } else {
                "unknown".to_string()
            }
        });

        let header = BlockHeader {
            block_number: block_num,
            timestamp: Utc::now(),
            previous_hash: previous_hash.to_string(),
            merkle_root: merkle_root.to_string(),
            validator_signature: {
                use sha2::{Digest, Sha256};
                let sig_data = format!("{}{}{}", block_num, previous_hash, merkle_root);
                let mut hasher = Sha256::new();
                hasher.update(sig_data.as_bytes());
                hasher.update(my_id.as_bytes());
                format!("{:x}", hasher.finalize())
            },
            validator_address: my_id,
        };

        let header_json = serde_json::to_string(&header).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(header_json.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        let block = Block {
            header,
            transactions: transactions.to_vec(),
            hash,
        };

        // Broadcast finalized block to peers before storing (best-effort)
        let masternodes = self.consensus.get_masternodes().await;
        self.broadcast_finalized_block(&block, &masternodes).await;

        let mut blockchain = self.blockchain.write().await;
        match blockchain.add_block(block.clone()) {
            Ok(_) => {
                println!("   ✔ Block {} finalized", block_num);
                drop(blockchain);

                // Remove transactions from mempool (skip first transaction as it's coinbase)
                for tx in transactions.iter().skip(1) {
                    self.mempool.remove_transaction(&tx.txid).await;
                }

                // Broadcast the finalized block to peers (best-effort).
                let all_masternodes = self.consensus.get_masternodes().await;
                let active_masternodes = self
                    .block_consensus
                    .get_active_masternodes(&all_masternodes)
                    .await;
                self.broadcast_finalized_block(&block, &active_masternodes)
                    .await;
            }
            Err(e) => {
                println!("   ✗ Failed: {:?}", e);
            }
        }
    }

    #[allow(dead_code)]
    async fn produce_catch_up_block(
        &self,
        block_num: u64,
        timestamp: chrono::DateTime<Utc>,
    ) -> bool {
        use time_core::block::{calculate_tier_reward, calculate_treasury_reward};

        let mut blockchain = self.blockchain.write().await;

        let previous_hash = if block_num == 0 {
            "0000000000000000000000000000000000000000000000000000000000000000".to_string()
        } else {
            blockchain.chain_tip_hash().to_string()
        };

        let masternode_counts = blockchain.masternode_counts().clone();

        let mut outputs = vec![TxOutput {
            amount: calculate_treasury_reward(),
            address: "TIME1treasury00000000000000000000000000".to_string(),
        }];

        // For catch-up blocks, also filter by participation
        let agreed_tx_set = self.tx_consensus.get_agreed_tx_set(block_num).await;
        let voters = if let Some(proposal) = agreed_tx_set {
            self.tx_consensus
                .get_voters(block_num, &proposal.merkle_root)
                .await
        } else {
            Vec::new()
        };

        // --- FIX: derive participating_masternodes from voters ---
        let active_masternodes = self.consensus.get_masternodes_with_wallets().await;
        let participating_masternodes: Vec<_> = active_masternodes
            .into_iter()
            .filter(|(node_id, _)| voters.contains(node_id))
            .collect();

        if !participating_masternodes.is_empty() {
            println!(
                "      💡 Rewarding {} participating masternodes",
                participating_masternodes.len()
            );
            let tiers = [
                MasternodeTier::Free,
                MasternodeTier::Bronze,
                MasternodeTier::Silver,
                MasternodeTier::Gold,
            ];
            for tier in tiers {
                let tier_reward = calculate_tier_reward(tier, &masternode_counts);
                if tier_reward > 0 {
                    let tier_nodes: Vec<_> = participating_masternodes
                        .iter()
                        .filter(|(node_id, _)| {
                            node_id.starts_with(&format!("{:?}", tier).to_lowercase())
                        })
                        .collect();

                    if !tier_nodes.is_empty() {
                        let reward_per_node = tier_reward / tier_nodes.len() as u64;
                        for (_, wallet_addr) in tier_nodes {
                            outputs.push(TxOutput {
                                amount: reward_per_node,
                                address: wallet_addr.clone(),
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

        let my_id = std::env::var("NODE_PUBLIC_IP").unwrap_or_else(|_| {
            if let Ok(ip) = local_ip_address::local_ip() {
                ip.to_string()
            } else {
                "unknown".to_string()
            }
        });

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

        println!("   🔧 Creating catch-up block #{}...", block_num);
        println!(
            "      Timestamp: {}",
            timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );
        println!("      Block Hash: {}...", &block.hash[..16]);

        match blockchain.add_block(block.clone()) {
            Ok(_) => {
                println!("      ✔ Block #{} created and stored", block_num);
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

        let my_id = std::env::var("NODE_PUBLIC_IP").unwrap_or_else(|_| {
            if let Ok(ip) = local_ip_address::local_ip() {
                ip.to_string()
            } else {
                "unknown".to_string()
            }
        });

        println!(
            "   🔧 Block #{} - Producer: {:?}",
            block_num, selected_producer
        );

        // Step 1: If I'm the producer, create and broadcast proposal
        if let Some(ref producer) = selected_producer {
            if producer == &my_id {
                println!("      📝 I'm the producer - creating block proposal...");

                // Create the block (without adding to chain yet)
                let block = self
                    .create_catchup_block_structure(block_num, timestamp)
                    .await;

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
        println!("      ▶️ Waiting for block proposal and consensus...");

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

                    if (self.block_consensus.vote_on_block(vote.clone()).await).is_ok() {
                        self.broadcast_block_vote(vote, masternodes).await;
                    }
                }

                // PHASE 1: Try consensus with all masternodes
                let (has_consensus, approvals, total) = self
                    .block_consensus
                    .has_block_consensus(block_num, &proposal.block_hash)
                    .await;

                if has_consensus {
                    println!("      ✔ Consensus reached! ({}/{} votes)", approvals, total);

                    let voters = self
                        .block_consensus
                        .get_voters(block_num, &proposal.block_hash)
                        .await;

                    return self
                        .finalize_catchup_block_with_rewards(block_num, timestamp, &voters)
                        .await;
                }

                // PHASE 2: After 15 attempts, try version-filtered consensus
                if attempt >= 15 {
                    let my_version = time_network::protocol::full_version();
                    let (has_version_consensus, version_approvals, version_total) = self
                        .block_consensus
                        .has_block_consensus_with_version_filter(
                            block_num,
                            &proposal.block_hash,
                            Some(&my_version),
                        )
                        .await;

                    if has_version_consensus && version_total >= 3 {
                        println!(
                            "      ✔ Version-filtered consensus reached! ({}/{} votes from v{})",
                            version_approvals, version_total, my_version
                        );
                        println!(
                            "      ℹ️  Excluded {} nodes with incompatible versions",
                            total - version_total
                        );

                        let voters = self
                            .block_consensus
                            .get_voters(block_num, &proposal.block_hash)
                            .await;

                        let matching_version_nodes = self
                            .block_consensus
                            .get_masternodes_by_version(&my_version)
                            .await;

                        let version_filtered_voters: Vec<String> = voters
                            .into_iter()
                            .filter(|v| matching_version_nodes.contains(v))
                            .collect();

                        return self
                            .finalize_catchup_block_with_rewards(
                                block_num,
                                timestamp,
                                &version_filtered_voters,
                            )
                            .await;
                    }
                }

                if attempt % 5 == 0 {
                    println!("      ▶️ Waiting for consensus...");
                }
            }
        }

        println!(
            "      ⚠️  Timeout - no consensus reached for block {}",
            block_num
        );
        false
    }

    async fn create_catchup_block_structure(
        &self,
        block_num: u64,
        timestamp: chrono::DateTime<Utc>,
    ) -> time_core::block::Block {
        use time_core::block::{create_coinbase_transaction, Block, BlockHeader};

        let blockchain = self.blockchain.read().await;
        let previous_hash = blockchain.chain_tip_hash().to_string();
        let masternode_counts = blockchain.masternode_counts().clone();

        let active_masternodes: Vec<(String, time_core::MasternodeTier)> = blockchain
            .get_active_masternodes()
            .iter()
            .map(|mn| (mn.wallet_address.clone(), mn.tier))
            .collect();

        drop(blockchain);

        let my_id = std::env::var("NODE_PUBLIC_IP").unwrap_or_else(|_| {
            if let Ok(ip) = local_ip_address::local_ip() {
                ip.to_string()
            } else {
                "unknown".to_string()
            }
        });

        // Log masternode reward distribution for catch-up block
        println!(
            "      💰 Catch-up block will reward {} masternodes",
            active_masternodes.len()
        );

        let coinbase_tx = create_coinbase_transaction(
            block_num,
            "TIME1treasury00000000000000000000000000",
            &active_masternodes,
            &masternode_counts,
            0,
        );

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
        println!(
            "      💰 Proposal includes rewards for {} masternodes",
            active_masternodes.len()
        );
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
            let _ = reqwest::Client::new().post(&url).json(&vote).send().await;
        }
    }

    // --- finalize_catchup_block_with_rewards kept inside impl ---

    async fn finalize_catchup_block_with_rewards(
        &self,
        block_num: u64,
        timestamp: chrono::DateTime<Utc>,
        voters: &[String],
    ) -> bool {
        use time_core::block::{calculate_treasury_reward, distribute_masternode_rewards};
        use time_core::block::{Block, BlockHeader};
        use time_core::transaction::{Transaction, TxOutput};

        let mut blockchain = self.blockchain.write().await;
        let previous_hash = blockchain.chain_tip_hash().to_string();
        let masternode_counts = blockchain.masternode_counts().clone();

        let my_id = std::env::var("NODE_PUBLIC_IP").unwrap_or_else(|_| {
            if let Ok(ip) = local_ip_address::local_ip() {
                ip.to_string()
            } else {
                "unknown".to_string()
            }
        });

        // Build outputs with treasury
        let mut outputs = vec![TxOutput {
            amount: calculate_treasury_reward(),
            address: "TIME1treasury00000000000000000000000000".to_string(),
        }];

        // Get registered masternodes with their wallet addresses from blockchain
        let registered_masternodes: Vec<(String, time_core::MasternodeTier)> = blockchain
            .get_all_masternodes()
            .iter()
            .filter_map(|mn| {
                if mn.is_active {
                    Some((mn.wallet_address.clone(), mn.tier))
                } else {
                    None
                }
            })
            .collect();

        println!(
            "      💡 DEBUG: registered masternodes = {}",
            registered_masternodes.len()
        );
        println!("      💡 DEBUG: voters = {:?}", voters);

        // If we have registered masternodes, distribute rewards
        if !registered_masternodes.is_empty() {
            println!(
                "      💰 Distributing rewards to {} registered masternodes",
                registered_masternodes.len()
            );

            // Use the built-in distribution function
            let masternode_outputs =
                distribute_masternode_rewards(&registered_masternodes, &masternode_counts);

            outputs.extend(masternode_outputs);

            println!(
                "      ✓ Added {} masternode reward outputs",
                registered_masternodes.len()
            );
        } else {
            println!("      ⚠️  No registered masternodes - treasury reward only");
            println!(
                "      💡 To receive rewards, masternodes must be registered in blockchain state"
            );
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

        println!("      🔧 Finalizing block #{}...", block_num);
        match blockchain.add_block(block.clone()) {
            Ok(_) => {
                println!("      ✔ Block #{} finalized and stored", block_num);
                true
            }
            Err(e) => {
                println!("      ✗ Failed to finalize block: {:?}", e);
                false
            }
        }
    }
}
