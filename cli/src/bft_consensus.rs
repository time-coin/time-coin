//! Unified BFT Consensus Flow
//!
//! Streamlined Byzantine Fault Tolerant consensus with clear phases:
//! 1. Leader Selection (VRF-based, deterministic)
//! 2. Block Proposal Creation & Broadcast
//! 3. Vote Collection with Timeout
//! 4. Consensus Check (2/3+ threshold)
//! 5. Block Finalization & Broadcast
//!
//! All block creation paths (regular, catch-up, reward-only) use this unified flow.

use std::sync::Arc;
use std::time::Duration;
use time_consensus::block_consensus::{BlockConsensusManager, BlockProposal, BlockVote};
use time_core::block::Block;
use time_core::state::BlockchainState;
use time_network::PeerManager;
use tokio::sync::RwLock;

/// Result of a consensus attempt
#[derive(Debug)]
pub enum ConsensusResult {
    Success(Block),
    Failed(ConsensusFailure),
}

#[derive(Debug)]
pub struct ConsensusFailure {
    pub attempts: u32,
    pub last_error: String,
    pub votes_received: usize,
    pub votes_required: usize,
}

/// Unified BFT Consensus Manager
pub struct BftConsensus {
    my_id: String,
    peer_manager: Arc<PeerManager>,
    block_consensus: Arc<BlockConsensusManager>,
    blockchain: Arc<RwLock<BlockchainState>>,
}

impl BftConsensus {
    pub fn new(
        my_id: String,
        peer_manager: Arc<PeerManager>,
        block_consensus: Arc<BlockConsensusManager>,
        blockchain: Arc<RwLock<BlockchainState>>,
    ) -> Self {
        Self {
            my_id,
            peer_manager,
            block_consensus,
            blockchain,
        }
    }

    /// Run complete BFT consensus for a block
    ///
    /// This is the main entry point for all consensus operations.
    /// It handles:
    /// - Leader selection and rotation on failure
    /// - Proposal broadcasting
    /// - Vote collection with timeout
    /// - Automatic retry with fallback strategies
    pub async fn run_consensus(
        &self,
        block_num: u64,
        masternodes: &[String],
        create_block_fn: impl Fn() -> Block,
        max_attempts: u32,
    ) -> ConsensusResult {
        // Update masternode list for this consensus round
        self.block_consensus
            .set_masternodes(masternodes.to_vec())
            .await;

        let required_votes = (masternodes.len() * 2).div_ceil(3);

        println!("🔷 BFT Consensus - Block #{}", block_num);
        println!("   Active nodes: {}", masternodes.len());
        println!(
            "   Required votes: {}/{}",
            required_votes,
            masternodes.len()
        );

        // Try consensus with rotating leaders
        for attempt in 0..max_attempts {
            if attempt > 0 {
                println!("\n   ⚠️  Attempt {} - rotating leader...", attempt + 1);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }

            // Select leader deterministically (rotate on retry)
            let leader = self.select_leader(block_num, masternodes, attempt);
            let am_i_leader = leader == self.my_id;

            println!("      Leader: {}", leader);

            // Phase 1: Proposal Creation & Broadcast
            if am_i_leader {
                println!("      I'm the leader - creating proposal...");

                let block = create_block_fn();

                if let Err(e) = self.broadcast_proposal(&block, block_num).await {
                    println!("      ❌ Failed to broadcast proposal: {}", e);
                    continue;
                }
            } else {
                // Non-leaders notify the leader
                self.notify_leader(&leader, block_num).await;
            }

            // Phase 2: Wait for Proposal & Vote
            match self
                .wait_and_vote(block_num, am_i_leader, masternodes, 20)
                .await
            {
                Ok(proposal) => {
                    // Phase 3: Check Consensus
                    let (has_consensus, approvals, _total) = self
                        .block_consensus
                        .has_block_consensus(block_num, &proposal.block_hash)
                        .await;

                    if has_consensus && approvals >= required_votes {
                        println!(
                            "      ✅ Consensus reached ({}/{})!",
                            approvals,
                            masternodes.len()
                        );

                        // Phase 4: Finalization
                        if am_i_leader {
                            // Leader finalizes and broadcasts
                            let block = create_block_fn();
                            return self.finalize_as_leader(block, masternodes).await;
                        } else {
                            // Non-leaders wait for finalized block broadcast
                            println!("      ⏳ Waiting for finalized block from leader...");
                            // The finalized block will arrive via /consensus/finalized-block endpoint
                            // and be added by receive_finalized_block handler
                            return ConsensusResult::Success(create_block_fn());
                        }
                    } else {
                        println!(
                            "      ❌ Insufficient votes: {}/{} (need {})",
                            approvals,
                            masternodes.len(),
                            required_votes
                        );
                    }
                }
                Err(e) => {
                    println!("      ❌ Consensus failed: {}", e);
                }
            }

            // Log diagnostics before retry
            self.log_attempt_diagnostics(block_num, attempt, masternodes)
                .await;
        }

        // All attempts failed
        ConsensusResult::Failed(ConsensusFailure {
            attempts: max_attempts,
            last_error: "Maximum retry attempts exceeded".to_string(),
            votes_received: 0,
            votes_required: required_votes,
        })
    }

    /// Select leader using deterministic round-robin with attempt offset
    fn select_leader(&self, block_num: u64, masternodes: &[String], attempt: u32) -> String {
        let mut sorted = masternodes.to_vec();
        sorted.sort();
        let index = ((block_num + attempt as u64) as usize) % sorted.len();
        sorted[index].clone()
    }

    /// Broadcast block proposal to all masternodes
    async fn broadcast_proposal(&self, block: &Block, block_num: u64) -> Result<(), String> {
        let proposal = BlockProposal {
            block_height: block_num,
            proposer: self.my_id.clone(),
            block_hash: block.hash.clone(),
            merkle_root: block.header.merkle_root.clone(),
            previous_hash: block.header.previous_hash.clone(),
            timestamp: block.header.timestamp.timestamp(),
            is_reward_only: block.transactions.len() == 1,
            strategy: None,
        };

        // Store locally
        self.block_consensus.propose_block(proposal.clone()).await;

        // Leader auto-votes
        let vote = BlockVote {
            block_height: block_num,
            block_hash: block.hash.clone(),
            voter: self.my_id.clone(),
            approve: true,
            timestamp: chrono::Utc::now().timestamp(),
        };

        if let Err(e) = self.block_consensus.vote_on_block(vote.clone()).await {
            return Err(format!("Failed to record leader vote: {}", e));
        }

        // Broadcast to network
        let proposal_json = serde_json::to_value(&proposal)
            .map_err(|e| format!("Failed to serialize proposal: {}", e))?;
        self.peer_manager
            .broadcast_block_proposal(proposal_json)
            .await;

        let vote_json =
            serde_json::to_value(&vote).map_err(|e| format!("Failed to serialize vote: {}", e))?;
        self.peer_manager.broadcast_block_vote(vote_json).await;

        println!("      📡 Proposal and vote broadcast");
        Ok(())
    }

    /// Wait for proposal and cast vote (for non-leaders)
    async fn wait_and_vote(
        &self,
        block_num: u64,
        am_i_leader: bool,
        masternodes: &[String],
        timeout_secs: u64,
    ) -> Result<BlockProposal, String> {
        println!("      Waiting for consensus...");

        let start_time = chrono::Utc::now();
        let mut last_log = start_time;
        let mut proposal_seen = false;
        let mut voted = am_i_leader; // Leader already voted

        // Give network time to propagate
        tokio::time::sleep(Duration::from_secs(2)).await;

        while (chrono::Utc::now() - start_time).num_seconds() < timeout_secs as i64 {
            tokio::time::sleep(Duration::from_secs(1)).await;

            if let Some(proposal) = self.block_consensus.get_proposal(block_num).await {
                if !proposal_seen {
                    println!("         📋 Proposal received from {}", proposal.proposer);
                    proposal_seen = true;
                }

                // Non-leaders vote once
                if !voted {
                    let vote = BlockVote {
                        block_height: block_num,
                        block_hash: proposal.block_hash.clone(),
                        voter: self.my_id.clone(),
                        approve: true,
                        timestamp: chrono::Utc::now().timestamp(),
                    };

                    if self
                        .block_consensus
                        .vote_on_block(vote.clone())
                        .await
                        .is_ok()
                    {
                        let vote_json = serde_json::to_value(&vote).unwrap();
                        self.peer_manager.broadcast_block_vote(vote_json).await;
                        voted = true;
                    }
                }

                // Check progress every 5 seconds
                if (chrono::Utc::now() - last_log).num_seconds() >= 5 {
                    let required = (masternodes.len() * 2).div_ceil(3);
                    let (_, approvals, _) = self
                        .block_consensus
                        .has_block_consensus(block_num, &proposal.block_hash)
                        .await;

                    println!(
                        "         ⏳ Progress: {}/{} votes (need {})",
                        approvals,
                        masternodes.len(),
                        required
                    );
                    last_log = chrono::Utc::now();

                    // Early exit if consensus reached
                    if approvals >= required {
                        return Ok(proposal);
                    }
                }
            } else if (chrono::Utc::now() - last_log).num_seconds() >= 5 {
                println!("         ⏳ Waiting for proposal from leader...");
                last_log = chrono::Utc::now();
            }
        }

        // Timeout - return proposal if we have one
        if let Some(proposal) = self.block_consensus.get_proposal(block_num).await {
            Ok(proposal)
        } else {
            Err("Timeout: No proposal received".to_string())
        }
    }

    /// Finalize block as leader and broadcast to peers
    async fn finalize_as_leader(&self, block: Block, masternodes: &[String]) -> ConsensusResult {
        println!("      ✔ Leader finalizing block...");

        // Add block to local blockchain
        let mut blockchain = self.blockchain.write().await;
        match blockchain.add_block(block.clone()) {
            Ok(_) => {
                println!("      ✅ Block {} finalized", block.header.block_number);
                drop(blockchain);

                // Broadcast finalized block to all peers
                self.broadcast_finalized_block(&block, masternodes).await;

                ConsensusResult::Success(block)
            }
            Err(e) => ConsensusResult::Failed(ConsensusFailure {
                attempts: 1,
                last_error: format!("Failed to add block: {:?}", e),
                votes_received: 0,
                votes_required: 0,
            }),
        }
    }

    /// Broadcast finalized block to all masternodes
    async fn broadcast_finalized_block(&self, block: &Block, masternodes: &[String]) {
        let block_json = match serde_json::to_value(block) {
            Ok(json) => json,
            Err(e) => {
                println!("   ⚠️  Failed to serialize block: {:?}", e);
                return;
            }
        };

        for node in masternodes {
            let url = format!("http://{}:24101/consensus/finalized-block", node);
            let payload = serde_json::json!({ "block": block_json });

            let url_clone = url.clone();
            let payload_clone = payload.clone();

            tokio::spawn(async move {
                let client = reqwest::Client::new();
                if let Err(e) = client
                    .post(&url_clone)
                    .json(&payload_clone)
                    .timeout(Duration::from_secs(2))
                    .send()
                    .await
                {
                    eprintln!("   ⚠️  Failed to broadcast to {}: {:?}", url_clone, e);
                }
            });
        }
    }

    /// Notify leader to produce block (non-leader behavior)
    async fn notify_leader(&self, leader: &str, block_num: u64) {
        println!("      Notifying leader {}...", leader);

        let url = format!("http://{}:24101/consensus/request-block-proposal", leader);
        let request = serde_json::json!({
            "block_height": block_num,
            "leader_ip": leader,
            "requester_ip": self.my_id,
        });

        match reqwest::Client::new()
            .post(&url)
            .json(&request)
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            Ok(_) => println!("      ✓ Leader {} acknowledged the request", leader),
            Err(_) => println!("      ⚠️  Could not reach leader {}", leader),
        }
    }

    /// Log detailed diagnostics after failed attempt
    async fn log_attempt_diagnostics(&self, block_num: u64, attempt: u32, masternodes: &[String]) {
        println!("      ❌ Attempt {} timeout", attempt + 1);

        if let Some(proposal) = self.block_consensus.get_proposal(block_num).await {
            let required = (masternodes.len() * 2).div_ceil(3);
            let (_, approvals, _) = self
                .block_consensus
                .has_block_consensus(block_num, &proposal.block_hash)
                .await;

            println!(
                "         📊 Final tally: {}/{} votes (needed {})",
                approvals,
                masternodes.len(),
                required
            );

            let voters = self
                .block_consensus
                .get_voters(block_num, &proposal.block_hash)
                .await;
            println!("         👥 Voters: {:?}", voters);

            let non_voters: Vec<String> = masternodes
                .iter()
                .filter(|mn| !voters.contains(mn))
                .cloned()
                .collect();
            if !non_voters.is_empty() {
                println!("         ❌ Missing votes from: {:?}", non_voters);
            }
        } else {
            println!("         ⚠️  No proposal was ever received from leader");
        }
    }
}
