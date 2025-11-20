use chrono::{TimeZone, Utc};
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
    is_active: Arc<RwLock<bool>>,
    allow_block_recreation: bool,
}

impl BlockProducer {
    #[allow(dead_code)]
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
            is_active: Arc::new(RwLock::new(false)),
            allow_block_recreation: false, // Default to false for safety
        }
    }

    /// Create a BlockProducer with a shared active state
    #[allow(clippy::too_many_arguments)]
    pub fn with_shared_state(
        node_id: String,
        peer_manager: Arc<PeerManager>,
        consensus: Arc<ConsensusEngine>,
        blockchain: Arc<RwLock<BlockchainState>>,
        mempool: Arc<time_mempool::Mempool>,
        block_consensus: Arc<time_consensus::block_consensus::BlockConsensusManager>,
        tx_consensus: Arc<time_consensus::tx_consensus::TxConsensusManager>,
        is_active: Arc<RwLock<bool>>,
        allow_block_recreation: bool,
    ) -> Self {
        BlockProducer {
            node_id,
            peer_manager,
            consensus,
            blockchain,
            mempool,
            block_consensus,
            tx_consensus,
            is_active,
            allow_block_recreation,
        }
    }

    /// Get a reference to the block producer active state
    #[allow(dead_code)]
    pub fn get_active_state(&self) -> Arc<RwLock<bool>> {
        self.is_active.clone()
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

            // Mark block producer as active during block production
            *self.is_active.write().await = true;
            self.create_and_propose_block().await;
            *self.is_active.write().await = false;

            // Sleep a few seconds to avoid duplicate triggers
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    async fn catch_up_missed_blocks(&self) {
        let now = Utc::now();
        let current_date = now.date_naive();

        // Get genesis date from blockchain state
        let blockchain = self.blockchain.read().await;
        let genesis_block = blockchain
            .get_block_by_height(0)
            .expect("Genesis block must exist");
        let genesis_date = genesis_block.header.timestamp.date_naive();
        drop(blockchain);

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

            // Wait and periodically re-check if BFT mode is reached
            for _ in 0..12 {
                // Check every 5 seconds for 1 minute
                tokio::time::sleep(Duration::from_secs(5)).await;
                let mode = self.consensus.consensus_mode().await;
                if mode == time_consensus::ConsensusMode::BFT {
                    println!("   ✅ BFT mode activated! Proceeding with catch-up...");
                    break;
                }
            }

            // Final check - if still not BFT, give up for now
            let final_mode = self.consensus.consensus_mode().await;
            if final_mode != time_consensus::ConsensusMode::BFT {
                println!("   ⚠️  Still in BOOTSTRAP mode after waiting - will retry next cycle");
                return;
            }
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

        // CRITICAL: Try to download from peers first - retry multiple times
        println!("   🔍 Attempting to download blocks from peers...");

        let max_sync_attempts = 3;
        for attempt in 1..=max_sync_attempts {
            let current_height = self.blockchain.read().await.chain_tip_height();

            if current_height >= expected_height {
                println!("      ✔ Fully synced to height {}!", current_height);
                return;
            }

            println!("      📥 Sync attempt {}/{}", attempt, max_sync_attempts);

            let peers = self.peer_manager.get_peer_ips().await;
            if peers.is_empty() {
                println!("      ⚠️  No peers available for sync");
                break;
            }

            let mut synced_any = false;

            for peer_ip in &peers {
                let url = format!("http://{}:24101/blockchain/info", peer_ip);
                if let Ok(response) = reqwest::get(&url).await {
                    if let Ok(info) = response.json::<BlockchainInfo>().await {
                        // If peer has blocks we need, try to sync
                        if info.height > current_height {
                            println!(
                                "      🔗 Peer {} has height {}, downloading blocks {}-{}...",
                                peer_ip,
                                info.height,
                                current_height + 1,
                                std::cmp::min(info.height, expected_height)
                            );

                            let sync_to_height = std::cmp::min(info.height, expected_height);
                            let mut downloaded = 0;

                            for height in (current_height + 1)..=sync_to_height {
                                let url =
                                    format!("http://{}:24101/blockchain/block/{}", peer_ip, height);

                                match reqwest::Client::new()
                                    .get(&url)
                                    .timeout(std::time::Duration::from_secs(10))
                                    .send()
                                    .await
                                {
                                    Ok(resp) => {
                                        if let Ok(json) = resp.json::<serde_json::Value>().await {
                                            if let Some(block_data) = json.get("block") {
                                                if let Ok(block) = serde_json::from_value::<
                                                    time_core::block::Block,
                                                >(
                                                    block_data.clone()
                                                ) {
                                                    let mut blockchain =
                                                        self.blockchain.write().await;
                                                    match blockchain.add_block(block) {
                                                        Ok(_) => {
                                                            downloaded += 1;
                                                            println!(
                                                                "         ✓ Block #{} downloaded",
                                                                height
                                                            );
                                                            synced_any = true;
                                                        }
                                                        Err(e) => {
                                                            println!(
                                                                "         ✗ Failed to add block #{}: {:?}",
                                                                height, e
                                                            );
                                                            // Skip this peer and try next
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        // Network error, try next peer
                                        break;
                                    }
                                }
                            }

                            if downloaded > 0 {
                                println!(
                                    "      ✓ Downloaded {} blocks from {}",
                                    downloaded, peer_ip
                                );
                            }
                        }
                    }
                }
            }

            if !synced_any && attempt < max_sync_attempts {
                println!("      ⏳ Waiting 10s before retry...");
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }

        // Final check after all sync attempts
        let final_height = self.blockchain.read().await.chain_tip_height();
        if final_height >= expected_height {
            println!(
                "   ✅ Successfully synced all blocks to height {}!",
                final_height
            );
            return;
        } else if final_height > actual_height {
            println!(
                "   ℹ️  Partially synced to height {} (need {})",
                final_height, expected_height
            );
            println!("   ▶️  Will create remaining blocks via consensus");
        } else {
            println!("   ⚠️  Could not download any blocks from peers");
        }

        // Check if block recreation is allowed
        if !self.allow_block_recreation {
            println!("   ⚠️  Block recreation is disabled in config");
            println!("   ℹ️  Historical blocks can only be downloaded, not recreated");
            println!("   ℹ️  Set 'allow_block_recreation = true' in config to enable");
            println!("   ⏸️  Will retry sync on next cycle");
            return;
        }

        // SYNCHRONIZED NODES: Allow block creation since nodes are now in consensus
        // Nodes have agreed on blocks 0-3 via fork resolution and can safely build forward
        println!("   ⚠️  Could not download blocks from peers");
        println!("   ✅ Block recreation enabled - creating blocks via BFT consensus");
        println!("   📝 This ensures complete blockchain from genesis to present");

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

        // CRITICAL: Process blocks sequentially, ONE AT A TIME
        // Wait for each block to be fully consensus-accepted before starting the next
        for block_num in (actual_height + 1)..=expected_height {
            println!("\n╔═══════════════════════════════════════════════════════════╗");
            println!("║  BLOCK #{:<50} ║", block_num);
            println!("╚═══════════════════════════════════════════════════════════╝");

            // Verify current chain height before attempting next block
            let current_height = self.load_block_height().await;
            if current_height >= block_num {
                println!("   ℹ️  Block {} already exists, skipping...", block_num);
                continue;
            }

            if current_height + 1 != block_num {
                println!(
                    "   ⚠️  Chain height mismatch: have {}, trying to create {}",
                    current_height, block_num
                );
                println!("   ⏸️  Pausing catch-up - chain not ready for this block");
                break;
            }

            let timestamp_date = genesis_date + chrono::Duration::days(block_num as i64);
            let timestamp = Utc.from_utc_datetime(&timestamp_date.and_hms_opt(0, 0, 0).unwrap());

            // Single attempt with improved consensus (fast-track + emergency fallback)
            let success = self
                .produce_catchup_block_with_bft_consensus(block_num, timestamp, &masternodes)
                .await;

            if success {
                println!("   ✅ Block {} created successfully!", block_num);
                // Give network MORE time to settle and propagate
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            } else {
                println!("   ❌ Failed to create block {}", block_num);
                println!("   ⏸️  Pausing catch-up - will retry on next cycle");
                break;
            }
        }

        println!("   ✔ Catch-up complete!");
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

        // CRITICAL: Use consensus engine's deterministic leader selection on ALL masternodes
        // This ensures all nodes agree on the leader regardless of local health state
        let selected_producer = self.consensus.get_leader(block_num).await;

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

        // Log leader selection for debugging
        println!(
            "   🎯 Leader selection (VRF-based on {} total masternodes)",
            all_masternodes.len()
        );
        println!(
            "      Selected leader: {}",
            selected_producer.as_ref().unwrap_or(&"none".to_string())
        );
        println!("      Active masternodes: {}", masternodes.len());

        if am_i_leader {
            println!("{}", "   🟢 I am the block producer".green().bold());

            let mut transactions = self.mempool.get_all_transactions().await;
            // Sort transactions deterministically by txid to ensure same merkle root
            transactions.sort_by(|a, b| a.txid.cmp(&b.txid));

            // Check for approved proposals and add treasury grants
            if let Some(proposal_manager) = self.consensus.proposal_manager() {
                let masternode_count = self.consensus.masternode_count().await;
                let approved_proposals = proposal_manager
                    .get_approved_pending(masternode_count)
                    .await;

                if !approved_proposals.is_empty() {
                    println!(
                        "   💰 Processing {} approved proposal(s)",
                        approved_proposals.len()
                    );

                    for proposal in approved_proposals {
                        // Create treasury grant transaction
                        let grant_tx = time_core::transaction::Transaction {
                            txid: format!("treasury_grant_{}", proposal.id),
                            version: 1,
                            inputs: vec![], // No inputs = treasury grant
                            outputs: vec![time_core::transaction::TxOutput::new(
                                proposal.amount,
                                proposal.recipient.clone(),
                            )],
                            lock_time: 0,
                            timestamp: chrono::Utc::now().timestamp(),
                        };

                        println!(
                            "      ✓ Grant: {} TIME to {}",
                            proposal.amount as f64 / 100_000_000.0,
                            proposal.recipient
                        );
                        println!("         Reason: {}", proposal.reason);

                        transactions.push(grant_tx);

                        // Mark proposal as executed (will be persisted after block)
                        let _ = proposal_manager
                            .mark_executed(&proposal.id, format!("treasury_grant_{}", proposal.id))
                            .await;
                    }
                }
            }

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

            // Check for empty mempool - use deterministic reward-only block path
            let is_reward_only = transactions.is_empty();

            if is_reward_only {
                println!("   📭 Mempool is empty - creating deterministic reward-only block");
                println!("   ⚡ This should achieve instant consensus (identical blocks)");

                // Use deterministic reward-only block creation
                let block = time_core::block::create_reward_only_block(
                    block_num,
                    previous_hash.clone(),
                    my_id.clone(),
                    &active_masternodes,
                    &masternode_counts,
                );

                // Create proposal with reward-only flag
                let proposal = time_consensus::block_consensus::BlockProposal {
                    block_height: block_num,
                    proposer: my_id.clone(),
                    block_hash: block.hash.clone(),
                    merkle_root: block.header.merkle_root.clone(),
                    previous_hash: previous_hash.clone(),
                    timestamp: block.header.timestamp.timestamp(),
                    is_reward_only: true,
                    strategy: None, // Regular block production doesn't use foolproof strategies
                };

                self.block_consensus.store_proposal(proposal.clone()).await;

                // Leader auto-votes on their own proposal
                let leader_vote = time_consensus::block_consensus::BlockVote {
                    block_height: block_num,
                    block_hash: block.hash.clone(),
                    voter: my_id.clone(),
                    approve: true,
                    timestamp: chrono::Utc::now().timestamp(),
                };
                if let Err(e) = self
                    .block_consensus
                    .vote_on_block(leader_vote.clone())
                    .await
                {
                    println!("   ⚠️  Leader failed to record own vote: {}", e);
                } else {
                    println!("   ✓ Leader auto-voted APPROVE");
                }

                let proposal_json = serde_json::to_value(&proposal).unwrap();
                self.peer_manager
                    .broadcast_block_proposal(proposal_json)
                    .await;

                // Also broadcast leader's vote
                let vote_json = serde_json::to_value(&leader_vote).unwrap();
                self.peer_manager.broadcast_block_vote(vote_json).await;

                println!("   📡 Reward-only proposal and vote broadcast");

                // Give peers time to receive and process proposal (2 second buffer)
                println!("   ⏳ Waiting 2s for peers to receive proposal...");
                tokio::time::sleep(Duration::from_secs(2)).await;

                println!(
                    "   ▶️ Collecting votes (need {}/{})...",
                    required_votes,
                    masternodes.len()
                );

                // Collect votes for reward-only block
                let (approved, total) = self
                    .block_consensus
                    .collect_votes_with_timeout(block_num, required_votes, 60)
                    .await;

                println!("   🗳️  Votes: {}/{}", approved, total);

                if approved >= required_votes {
                    println!("   ✔ Quorum reached! Finalizing reward-only block...");

                    // Apply the deterministic block
                    let mut blockchain_write = self.blockchain.write().await;
                    match blockchain_write.add_block(block.clone()) {
                        Ok(_) => {
                            println!("   ✅ Reward-only block {} finalized", block_num);
                            // Broadcast to peers
                            drop(blockchain_write);
                            self.broadcast_finalized_block(&block, &masternodes).await;
                        }
                        Err(e) => {
                            println!("   ❌ Failed to finalize reward-only block: {:?}", e);
                        }
                    }
                } else {
                    println!("   ❌ FAILED: Insufficient votes ({}/{})", approved, total);
                }

                return;
            }

            // Regular path: mempool has transactions
            println!("   📋 {} mempool transactions", transactions.len());

            // Calculate total transaction fees from mempool transactions
            let mut total_fees: u64 = 0;
            {
                let blockchain = self.blockchain.read().await;
                let utxo_map = blockchain.utxo_set().utxos();

                for tx in &transactions {
                    match tx.fee(utxo_map) {
                        Ok(fee) => {
                            total_fees += fee;
                            println!("      📊 TX {} fee: {} satoshis", &tx.txid[..8], fee);
                        }
                        Err(e) => {
                            println!(
                                "      ⚠️  Could not calculate fee for {}: {:?}",
                                &tx.txid[..8],
                                e
                            );
                            // Skip transaction if fee can't be calculated
                        }
                    }
                }
                drop(blockchain);
            }

            if total_fees > 0 {
                println!(
                    "   💵 Total transaction fees: {} satoshis ({} TIME)",
                    total_fees,
                    total_fees as f64 / 100_000_000.0
                );
            }

            // Log masternode reward distribution
            if !active_masternodes.is_empty() {
                println!(
                    "   💰 Distributing rewards to {} masternodes:",
                    active_masternodes.len()
                );

                // Calculate what each tier will receive
                let total_pool =
                    time_core::block::calculate_total_masternode_reward(&masternode_counts);
                let total_weight = masternode_counts.total_weight();
                let per_weight = if total_weight > 0 {
                    total_pool / total_weight
                } else {
                    0
                };

                println!(
                    "      Total reward pool: {} satoshis ({} TIME)",
                    total_pool,
                    total_pool / 100_000_000
                );
                println!("      Total weight: {}", total_weight);
                println!("      Per weight unit: {} satoshis", per_weight);

                // Show breakdown by tier
                let mut tier_summary: std::collections::HashMap<
                    time_core::MasternodeTier,
                    (usize, u64),
                > = std::collections::HashMap::new();

                for (wallet_addr, tier) in &active_masternodes {
                    let reward = per_weight * tier.weight();
                    let entry = tier_summary.entry(*tier).or_insert((0, 0));
                    entry.0 += 1;
                    entry.1 += reward;

                    println!(
                        "      - {:?} tier ({} weight): {} → {} satoshis ({:.2} TIME)",
                        tier,
                        tier.weight(),
                        if wallet_addr.len() >= 20 {
                            &wallet_addr[..20]
                        } else {
                            wallet_addr.as_str()
                        },
                        reward,
                        reward as f64 / 100_000_000.0
                    );
                }

                println!("   📊 Reward Summary:");
                for (tier, (count, total_reward)) in tier_summary.iter() {
                    println!(
                        "      {:?}: {} nodes, {} satoshis total ({:.2} TIME each)",
                        tier,
                        count,
                        total_reward,
                        (*total_reward as f64 / *count as f64) / 100_000_000.0
                    );
                }
            } else {
                println!("   ⚠️  No active masternodes registered for rewards");
            }

            // Create coinbase transaction with all rewards
            // CRITICAL: Use current time as block timestamp for determinism across all nodes
            let block_timestamp = now.timestamp();
            let coinbase_tx = time_core::block::create_coinbase_transaction(
                block_num,
                &active_masternodes,
                &masternode_counts,
                total_fees,
                block_timestamp,
            );

            // Prepend coinbase to transactions list
            let mut all_transactions = vec![coinbase_tx];
            let mempool_count = transactions.len();
            all_transactions.extend(transactions);

            if mempool_count == 0 {
                println!("   📦 Block will contain ONLY coinbase transaction (zero regular txs)");
                println!("   ℹ️  This is NORMAL and EXPECTED for TIME Coin");
                println!("   ℹ️  Coinbase includes treasury + masternode rewards");
            }

            println!(
                "   📋 {} total transactions (1 coinbase + {} mempool)",
                all_transactions.len(),
                mempool_count
            );

            // Verify coinbase transaction
            let coinbase_outputs = all_transactions[0].outputs.len();
            let coinbase_amount: u64 = all_transactions[0].outputs.iter().map(|o| o.amount).sum();
            println!(
                "   💰 Coinbase: {} outputs, {} satoshis total ({} TIME)",
                coinbase_outputs,
                coinbase_amount,
                coinbase_amount / 100_000_000
            );

            let merkle_root = self.calc_merkle(&all_transactions);

            // Create a temporary block to calculate the hash
            use sha2::{Digest, Sha256};
            use time_core::BlockHeader;

            let temp_header = BlockHeader {
                block_number: block_num,
                timestamp: now,
                previous_hash: previous_hash.clone(),
                merkle_root: merkle_root.clone(),
                validator_signature: {
                    let sig_data = format!("{}{}{}", block_num, previous_hash, merkle_root);
                    let mut hasher = Sha256::new();
                    hasher.update(sig_data.as_bytes());
                    hasher.update(my_id.as_bytes());
                    format!("{:x}", hasher.finalize())
                },
                validator_address: my_id.clone(),
            };

            let header_json = serde_json::to_string(&temp_header).unwrap();
            let mut hasher = Sha256::new();
            hasher.update(header_json.as_bytes());
            let block_hash = format!("{:x}", hasher.finalize());

            let proposal = time_consensus::block_consensus::BlockProposal {
                block_height: block_num,
                proposer: my_id.clone(),
                block_hash: block_hash.clone(),
                merkle_root: merkle_root.clone(),
                previous_hash: previous_hash.clone(),
                timestamp: now.timestamp(),
                is_reward_only: false,
                strategy: None, // Regular block production doesn't use foolproof strategies
            };

            self.block_consensus.store_proposal(proposal.clone()).await;

            // Leader auto-votes on their own proposal
            let leader_vote = time_consensus::block_consensus::BlockVote {
                block_height: block_num,
                block_hash: block_hash.clone(),
                voter: my_id.clone(),
                approve: true,
                timestamp: chrono::Utc::now().timestamp(),
            };
            if let Err(e) = self
                .block_consensus
                .vote_on_block(leader_vote.clone())
                .await
            {
                println!("   ⚠️  Leader failed to record own vote: {}", e);
            } else {
                println!("   ✓ Leader auto-voted APPROVE");
            }

            let proposal_json = serde_json::to_value(&proposal).unwrap();
            self.peer_manager
                .broadcast_block_proposal(proposal_json)
                .await;

            // Also broadcast leader's vote
            let vote_json = serde_json::to_value(&leader_vote).unwrap();
            self.peer_manager.broadcast_block_vote(vote_json).await;

            println!("   📡 Proposal and vote broadcast");

            // Give peers time to receive and process proposal (2 second buffer)
            println!("   ⏳ Waiting 2s for peers to receive proposal...");
            tokio::time::sleep(Duration::from_secs(2)).await;

            println!(
                "   ▶️ Collecting votes (need {}/{})...",
                required_votes,
                masternodes.len()
            );

            // Enhanced voting collection with multiple retries
            let mut attempt = 0;
            const MAX_VOTE_ATTEMPTS: u32 = 3;
            let mut best_approved = 0;
            let mut best_total = 0;

            while attempt < MAX_VOTE_ATTEMPTS {
                attempt += 1;

                let timeout_secs = if attempt == 1 { 60 } else { 30 }; // First attempt: 60s, retries: 30s
                println!(
                    "   🗳️  Vote collection attempt {}/{} ({}s timeout)",
                    attempt, MAX_VOTE_ATTEMPTS, timeout_secs
                );

                let (approved, total) = self
                    .block_consensus
                    .collect_votes_with_timeout(block_num, required_votes, timeout_secs)
                    .await;

                best_approved = approved.max(best_approved);
                best_total = total.max(best_total);

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

                println!("   🗳️  Votes: {}/{}", approved, total);

                if approved >= required_votes {
                    println!("   ✔ Quorum reached! Finalizing...");
                    self.finalize_block_bft(
                        &all_transactions,
                        &previous_hash,
                        &merkle_root,
                        block_num,
                    )
                    .await;
                    return;
                } else {
                    println!(
                        "   ⚠️  Attempt {}: Quorum not reached ({} < {})",
                        attempt, approved, required_votes
                    );

                    // On final attempt, check for emergency fallback
                    if attempt == MAX_VOTE_ATTEMPTS {
                        // If we have more than 50% votes, create block anyway
                        if approved > total / 2 {
                            println!("   🚨 EMERGENCY: Simple majority reached ({}/{}), finalizing block", approved, total);
                            self.finalize_block_bft(
                                &all_transactions,
                                &previous_hash,
                                &merkle_root,
                                block_num,
                            )
                            .await;
                            return;
                        } else {
                            println!(
                                "   ❌ FAILED: Insufficient votes ({}/{}), block not created",
                                approved, total
                            );
                            println!("   ℹ️  Block will be created during next catch-up cycle");
                        }
                    } else {
                        println!("   🔄 Retrying vote collection...");
                        // Brief pause before retry
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                }
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
                let chain_tip_hash = blockchain.chain_tip_hash().to_string();
                let chain_tip_height = blockchain.chain_tip_height();
                let masternode_counts = blockchain.masternode_counts().clone();
                let active_masternodes: Vec<(String, time_core::MasternodeTier)> = blockchain
                    .get_active_masternodes()
                    .iter()
                    .map(|mn| (mn.wallet_address.clone(), mn.tier))
                    .collect();
                drop(blockchain);

                let mut is_valid = self.block_consensus.validate_proposal(
                    &proposal,
                    &chain_tip_hash,
                    chain_tip_height,
                );

                // Fast-path validation for reward-only blocks
                if proposal.is_reward_only {
                    println!("   🚀 Reward-only block - using fast-path validation");

                    // Recreate the deterministic block locally
                    let expected_block = time_core::block::create_reward_only_block(
                        block_num,
                        chain_tip_hash.clone(),
                        proposal.proposer.clone(),
                        &active_masternodes,
                        &masternode_counts,
                    );

                    // Verify the block hash matches
                    if expected_block.hash == proposal.block_hash {
                        println!("   ✅ Reward-only block verified - auto-approving");
                        is_valid = true;
                    } else {
                        println!("   ❌ Reward-only block mismatch - rejecting");
                        println!("      Expected hash: {}", expected_block.hash);
                        println!("      Received hash: {}", proposal.block_hash);
                        is_valid = false;
                    }
                }

                let vote = time_consensus::block_consensus::BlockVote {
                    block_height: block_num,
                    block_hash: proposal.block_hash.clone(),
                    voter: my_id.clone(),
                    approve: is_valid,
                    timestamp: Utc::now().timestamp(),
                };

                // Use vote_on_block() instead of store_vote() to ensure proper validation
                if let Err(e) = self.block_consensus.vote_on_block(vote.clone()).await {
                    println!("   ⚠️  Failed to record vote: {}", e);
                }

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
                    // For reward-only blocks, we can recreate locally instead of fetching
                    if proposal.is_reward_only {
                        println!("   ✅ Reward-only block approved - applying locally...");

                        let reward_block = time_core::block::create_reward_only_block(
                            block_num,
                            chain_tip_hash,
                            proposal.proposer.clone(),
                            &active_masternodes,
                            &masternode_counts,
                        );

                        let mut blockchain = self.blockchain.write().await;
                        match blockchain.add_block(reward_block) {
                            Ok(_) => {
                                println!("   ✅ Reward-only block {} applied locally", block_num);
                            }
                            Err(e) => {
                                println!("   ⚠️  Failed to apply reward-only block: {:?}", e);
                            }
                        }
                    } else {
                        println!("   ✅ Block approved - fetching finalized block...");

                        // Actively fetch the finalized block from producer
                        if let Some(producer_id) = selected_producer {
                            if let Some(block) = self
                                .fetch_finalized_block(
                                    &producer_id,
                                    block_num,
                                    &proposal.merkle_root,
                                )
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
                let block_hash = block.hash.clone();
                drop(blockchain);

                // Broadcast tip update to all connected peers
                self.peer_manager
                    .broadcast_tip_update(block_num, block_hash)
                    .await;

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
        use time_core::block::calculate_tier_reward;

        let mut blockchain = self.blockchain.write().await;

        let previous_hash = if block_num == 0 {
            "0000000000000000000000000000000000000000000000000000000000000000".to_string()
        } else {
            blockchain.chain_tip_hash().to_string()
        };

        let masternode_counts = blockchain.masternode_counts().clone();

        // Initialize outputs with masternode rewards only (no treasury pre-allocation)
        let mut outputs = vec![];

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
        use time_consensus::foolproof_block::{
            BlockCreationStrategy, FoolproofBlockManager, FoolproofConfig,
        };

        // CRITICAL: Update block_consensus manager's masternode list before starting
        // This ensures votes from all masternodes (including self) are authorized
        self.block_consensus
            .set_masternodes(masternodes.to_vec())
            .await;

        // Initialize foolproof system
        let foolproof = FoolproofBlockManager::new(FoolproofConfig::default());
        foolproof.start_round().await;

        let my_id = std::env::var("NODE_PUBLIC_IP").unwrap_or_else(|_| {
            if let Ok(ip) = local_ip_address::local_ip() {
                ip.to_string()
            } else {
                "unknown".to_string()
            }
        });

        println!("   🆔 My node ID: {}", my_id);
        println!("   📋 Masternode list: {:?}", masternodes);
        println!(
            "   ℹ️  This node is now entering catch-up mode and will check if it's the leader"
        );

        // Test connectivity to all masternodes
        println!("   🔍 Testing connectivity to masternodes...");
        for node in masternodes {
            let url = format!("http://{}:24101/health", node);
            match reqwest::Client::new()
                .get(&url)
                .timeout(Duration::from_secs(2))
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {
                    println!("      ✓ {} is reachable", node);
                }
                Ok(response) => {
                    println!(
                        "      ⚠️  {} responded with status: {}",
                        node,
                        response.status()
                    );
                }
                Err(e) => {
                    println!("      ✗ {} is NOT reachable: {}", node, e);
                }
            }
        }

        // Try each strategy in the foolproof chain
        loop {
            let strategy = foolproof.current_strategy().await;
            let timeout_secs = strategy.timeout_secs();

            println!();
            println!("╔═══════════════════════════════════════════════════════════╗");
            println!("║  Strategy: {:?}", strategy);
            println!("║  Timeout: {}s", timeout_secs);
            println!("║  Block: #{}", block_num);
            println!("╚═══════════════════════════════════════════════════════════╝");

            // Determine leader using consensus engine's VRF selection
            // For LeaderRotation strategy, we'll override after getting base selection
            let base_selected_producer = if strategy == BlockCreationStrategy::LeaderRotation {
                // For rotation, use simple round-robin on the masternode list
                let attempt_count = foolproof.attempt_count().await;
                let index = (block_num as usize + attempt_count) % masternodes.len();
                let mut sorted = masternodes.to_vec();
                sorted.sort();
                Some(sorted[index].clone())
            } else {
                // Use VRF-based selection for normal strategies
                self.consensus.get_leader(block_num).await
            };

            let selected_producer = base_selected_producer;
            let am_i_leader = selected_producer.as_ref() == Some(&my_id);

            println!(
                "   Leader: {:?} (Strategy: {:?})",
                selected_producer, strategy
            );
            println!(
                "   Am I leader? {} (my_id='{}' vs selected='{:?}')",
                am_i_leader, my_id, selected_producer
            );

            // Notify the leader if I'm not the leader
            if !am_i_leader {
                if let Some(ref leader_ip) = selected_producer {
                    println!(
                        "   📨 Notifying leader {} to create block proposal...",
                        leader_ip
                    );
                    self.notify_leader_to_produce_block(leader_ip, block_num, &my_id)
                        .await;
                }
            }

            // Step 1: Leader creates and broadcasts proposal
            if am_i_leader {
                println!("   📝 I'm the leader - creating block proposal...");

                let block = if strategy.includes_mempool_txs() {
                    // Full block with mempool transactions
                    self.create_catchup_block_structure(block_num, timestamp)
                        .await
                } else {
                    // Reward-only or emergency block
                    self.create_minimal_catchup_block(
                        block_num,
                        timestamp,
                        strategy == BlockCreationStrategy::Emergency,
                    )
                    .await
                };

                let proposal = BlockProposal {
                    block_height: block_num,
                    proposer: my_id.clone(),
                    block_hash: block.hash.clone(),
                    merkle_root: block.header.merkle_root.clone(),
                    previous_hash: block.header.previous_hash.clone(),
                    timestamp: timestamp.timestamp(),
                    is_reward_only: false, // Catch-up blocks are not marked as reward-only
                    strategy: Some(format!("{:?}", strategy)), // Include strategy for synchronization
                };

                self.block_consensus.propose_block(proposal.clone()).await;
                self.broadcast_block_proposal(proposal.clone(), masternodes)
                    .await;

                // Leader auto-votes
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

            // Step 2: Wait for consensus with strategy-specific timeout
            // NOTE: All nodes wait for ANY valid proposal, regardless of strategy mismatch
            // This ensures network converges even if nodes are on different strategy rounds
            println!(
                "   ▶️ Waiting for consensus (timeout: {}s)...",
                timeout_secs
            );
            println!("   ℹ️  Will accept proposals from ANY strategy to ensure convergence");

            let start_time = Utc::now();
            let mut best_votes = 0;
            let mut best_total = masternodes.len();
            let mut consensus_reached = false;

            // For emergency strategy, always succeed
            if strategy == BlockCreationStrategy::Emergency {
                println!("   🚨 EMERGENCY MODE: Creating block without consensus");

                // Finalize immediately
                let success = self
                    .finalize_catchup_block_with_rewards(block_num, timestamp, masternodes)
                    .await;

                foolproof
                    .record_attempt(
                        strategy,
                        selected_producer.unwrap_or_else(|| "emergency".to_string()),
                        1, // Emergency assumes 1 vote (self)
                        masternodes.len(),
                        success,
                        if success {
                            None
                        } else {
                            Some("Emergency finalization failed".to_string())
                        },
                    )
                    .await;

                foolproof.log_summary().await;
                return success;
            }

            // Wait and check for consensus
            while (Utc::now() - start_time).num_seconds() < timeout_secs as i64 {
                tokio::time::sleep(Duration::from_secs(1)).await;

                if let Some(proposal) = self.block_consensus.get_proposal(block_num).await {
                    // Non-leaders vote here
                    if !am_i_leader {
                        let vote = BlockVote {
                            block_height: block_num,
                            block_hash: proposal.block_hash.clone(),
                            voter: my_id.clone(),
                            approve: true,
                            timestamp: chrono::Utc::now().timestamp(),
                        };

                        if self
                            .block_consensus
                            .vote_on_block(vote.clone())
                            .await
                            .is_ok()
                        {
                            self.broadcast_block_vote(vote, masternodes).await;
                        }
                    }

                    // Check consensus with strategy-specific threshold
                    let (_has_consensus, approvals, total) = self
                        .block_consensus
                        .has_block_consensus(block_num, &proposal.block_hash)
                        .await;

                    best_votes = approvals.max(best_votes);
                    best_total = total;

                    // Check with foolproof threshold
                    if foolproof
                        .check_consensus_with_strategy(approvals, total)
                        .await
                    {
                        consensus_reached = true;
                        println!("   ✅ Consensus reached! ({}/{} votes)", approvals, total);
                        break;
                    }

                    // Log progress
                    if (Utc::now() - start_time).num_seconds() % 5 == 0 {
                        let (num, denom) = strategy.vote_threshold();
                        let required = (total * num).div_ceil(denom);
                        println!("   ⏳ Votes: {}/{} (need {})", approvals, total, required);
                    }
                }
            }

            // Record attempt result
            if consensus_reached {
                let success = self
                    .finalize_catchup_block_with_rewards(block_num, timestamp, masternodes)
                    .await;

                foolproof
                    .record_attempt(
                        strategy,
                        selected_producer.unwrap_or_else(|| "unknown".to_string()),
                        best_votes,
                        best_total,
                        success,
                        if success {
                            None
                        } else {
                            Some("Finalization failed".to_string())
                        },
                    )
                    .await;

                if success {
                    foolproof.log_summary().await;
                    return true;
                } else {
                    // Finalization failed - try next strategy
                    println!("   ⚠️  Finalization failed despite consensus - trying next strategy");
                    if foolproof.advance_strategy().await.is_some() {
                        continue;
                    } else {
                        foolproof.log_summary().await;
                        return false;
                    }
                }
            } else {
                foolproof
                    .record_attempt(
                        strategy,
                        selected_producer.unwrap_or_else(|| "unknown".to_string()),
                        best_votes,
                        best_total,
                        false,
                        Some(format!("Timeout after {}s without consensus", timeout_secs)),
                    )
                    .await;

                // Try next strategy
                if foolproof.advance_strategy().await.is_some() {
                    // Clear consensus state for next attempt
                    continue;
                } else {
                    // No more strategies - this should never happen as Emergency is last
                    println!("   ❌ All strategies exhausted - CRITICAL ERROR");
                    foolproof.log_summary().await;
                    return false;
                }
            }
        }
    }

    /// Create a minimal catchup block (reward-only or emergency)
    #[allow(dead_code)]
    async fn create_minimal_catchup_block(
        &self,
        block_num: u64,
        timestamp: chrono::DateTime<Utc>,
        emergency: bool,
    ) -> time_core::block::Block {
        use time_core::block::{Block, BlockHeader};
        use time_core::transaction::Transaction;

        let blockchain = self.blockchain.read().await;
        let previous_hash = blockchain.chain_tip_hash().to_string();
        drop(blockchain);

        let my_id = std::env::var("NODE_PUBLIC_IP").unwrap_or_else(|_| {
            if let Ok(ip) = local_ip_address::local_ip() {
                ip.to_string()
            } else {
                "unknown".to_string()
            }
        });

        // Create minimal coinbase with masternode rewards (no treasury pre-allocation)
        let outputs = if emergency {
            // Emergency: No outputs (empty block, should rarely happen)
            vec![]
        } else {
            // Reward-only: Masternode rewards only
            let blockchain = self.blockchain.read().await;
            let masternode_counts = blockchain.masternode_counts().clone();
            let active_masternodes: Vec<(String, time_core::MasternodeTier)> = blockchain
                .get_active_masternodes()
                .iter()
                .map(|mn| (mn.wallet_address.clone(), mn.tier))
                .collect();
            drop(blockchain);

            time_core::block::distribute_masternode_rewards(&active_masternodes, &masternode_counts)
        };

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

        println!(
            "      💰 Minimal block: {} transaction(s)",
            block.transactions.len()
        );

        block
    }

    #[allow(dead_code)]
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
            &active_masternodes,
            &masternode_counts,
            0,
            timestamp.timestamp(), // Use block timestamp for determinism
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    async fn notify_leader_to_produce_block(
        &self,
        leader_ip: &str,
        block_height: u64,
        requester_ip: &str,
    ) {
        let request = serde_json::json!({
            "block_height": block_height,
            "leader_ip": leader_ip,
            "requester_ip": requester_ip,
        });

        let url = format!(
            "http://{}:24101/consensus/request-block-proposal",
            leader_ip
        );
        let result = reqwest::Client::new()
            .post(&url)
            .json(&request)
            .timeout(Duration::from_secs(3))
            .send()
            .await;

        match result {
            Ok(response) if response.status().is_success() => {
                println!("      ✓ Leader {} acknowledged the request", leader_ip);
            }
            Ok(response) => {
                println!(
                    "      ⚠️  Leader {} responded with status: {}",
                    leader_ip,
                    response.status()
                );
            }
            Err(e) => {
                println!("      ✗ Failed to notify leader {}: {}", leader_ip, e);
            }
        }
    }

    // --- finalize_catchup_block_with_rewards kept inside impl ---

    async fn finalize_catchup_block_with_rewards(
        &self,
        block_num: u64,
        timestamp: chrono::DateTime<Utc>,
        _masternodes: &[String],
    ) -> bool {
        use time_core::block::{Block, BlockHeader};

        let mut blockchain = self.blockchain.write().await;

        // Check if block already exists at this height
        if let Some(existing_block) = blockchain.get_block_by_height(block_num) {
            println!(
                "      ℹ️  Block #{} already exists (hash: {}...), skipping creation",
                block_num,
                &existing_block.hash[..16]
            );
            return true; // Not an error, just already done
        }

        let previous_hash = blockchain.chain_tip_hash().to_string();
        let masternode_counts = blockchain.masternode_counts().clone();

        let my_id = std::env::var("NODE_PUBLIC_IP").unwrap_or_else(|_| {
            if let Ok(ip) = local_ip_address::local_ip() {
                ip.to_string()
            } else {
                "unknown".to_string()
            }
        });

        // Pay all registered masternodes (simplified approach)
        let all_masternodes: Vec<(String, time_core::MasternodeTier)> = blockchain
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
            "      💰 Distributing rewards to {} registered masternodes",
            all_masternodes.len()
        );

        // Use the standard coinbase creation function which handles treasury correctly
        let coinbase_tx = time_core::block::create_coinbase_transaction(
            block_num,
            &all_masternodes,
            &masternode_counts,
            0, // No transaction fees in catch-up blocks
            timestamp.timestamp(),
        );

        println!(
            "      ✓ Created coinbase with {} outputs (incl. treasury)",
            coinbase_tx.outputs.len()
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

        println!("      🔧 Finalizing block #{}...", block_num);
        match blockchain.add_block(block.clone()) {
            Ok(_) => {
                println!("      ✔ Block #{} finalized and stored", block_num);
                drop(blockchain); // Release lock before broadcasting

                // CRITICAL: Broadcast finalized block to all peers so they can sync
                println!("      📡 Broadcasting finalized block to peers...");
                let peers = self.peer_manager.get_peer_ips().await;
                for peer_ip in &peers {
                    let url = format!("http://{}:24101/consensus/finalized-block", peer_ip);
                    let block_clone = block.clone();
                    tokio::spawn(async move {
                        let _ = reqwest::Client::new()
                            .post(&url)
                            .json(&block_clone)
                            .timeout(std::time::Duration::from_secs(5))
                            .send()
                            .await;
                    });
                }

                true
            }
            Err(e) => {
                println!("      ✗ Failed to finalize block: {:?}", e);
                false
            }
        }
    }
}
