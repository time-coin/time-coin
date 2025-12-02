//! Midnight Consensus Orchestrator
//!
//! Simple, clean midnight block creation:
//! 1. All nodes wake up at midnight
//! 2. Deterministically agree on leader via VRF
//! 3. Leader creates block and broadcasts proposal
//! 4. Nodes verify and vote
//! 5. If transaction mismatch: sync missing txs, validate, recreate block
//!
//! Note: Transaction syncing is handled at the network layer via
//! TransactionSyncManager, not directly in consensus.

use crate::simplified::{BlockProposal, SimplifiedConsensus};
use std::sync::Arc;
use time_core::block::Block;
use time_core::transaction::Transaction;

pub struct MidnightConsensusOrchestrator {
    consensus: Arc<SimplifiedConsensus>,
    my_ip: String,
}

impl MidnightConsensusOrchestrator {
    pub fn new(consensus: Arc<SimplifiedConsensus>, my_ip: String) -> Self {
        Self { consensus, my_ip }
    }

    /// Main midnight consensus flow
    pub async fn run_midnight_consensus(
        &self,
        height: u64,
        previous_hash: String,
        mempool: Vec<Transaction>,
        create_coinbase_fn: impl Fn() -> Transaction,
    ) -> Result<Block, String> {
        println!("🕛 MIDNIGHT CONSENSUS - Block #{}", height);
        println!("─────────────────────────────────────────");

        // Step 1: Determine leader (deterministic VRF)
        let leader = self
            .consensus
            .select_leader(height, &previous_hash)
            .await
            .ok_or("No masternodes available")?;

        println!("👑 Leader selected: {}", leader);

        let am_leader = leader == self.my_ip;

        if am_leader {
            println!("✅ I am the leader - creating block proposal");
            self.leader_create_and_propose(
                height,
                previous_hash.clone(),
                mempool,
                create_coinbase_fn,
            )
            .await?;
        } else {
            println!("⏳ Waiting for leader's proposal...");
        }

        // Step 2: Wait for proposal
        let proposal = self.wait_for_proposal(height).await?;

        // Step 3: Validate proposal
        println!("🔍 Validating proposal...");
        match self.consensus.validate_proposal(&proposal).await {
            Ok(()) => {
                println!("✅ All transactions present - approving");
                self.consensus
                    .vote(
                        height,
                        proposal.block_hash.clone(),
                        self.my_ip.clone(),
                        true,
                        None,
                    )
                    .await?;
            }
            Err(missing) => {
                println!("⚠️  Missing {} transaction(s)", missing.len());

                // Vote rejection with missing tx info
                let reason = format!("missing_tx:{}", missing.join(","));
                self.consensus
                    .vote(
                        height,
                        proposal.block_hash.clone(),
                        self.my_ip.clone(),
                        false,
                        Some(reason),
                    )
                    .await?;

                // Request missing transactions from peers
                // This is now handled automatically by the TransactionSyncManager
                // running in the network layer
                println!("📡 Missing transactions will be synced via network layer");

                return Err("Missing transactions - consensus deferred".to_string());
            }
        }

        // Step 4: Wait for consensus
        println!("🗳️  Collecting votes...");
        let (has_consensus, approvals, required, rejections) = self
            .wait_for_consensus(height, &proposal.block_hash, 30)
            .await;

        if has_consensus {
            println!("✅ CONSENSUS REACHED ({}/{})", approvals, required);

            if !rejections.is_empty() {
                println!(
                    "   Note: {} rejection(s) but reached threshold",
                    rejections.len()
                );
                for rejection in &rejections {
                    println!("      - {}", rejection);
                }
            }

            // Build the actual block
            // TODO: Fetch full block from leader or reconstruct
            Err("Block reconstruction not yet implemented".to_string())
        } else {
            println!("❌ CONSENSUS FAILED ({}/{} votes)", approvals, required);

            // Check if we need to handle missing transactions
            let missing = self.consensus.get_missing_transactions(height).await;

            if !missing.is_empty() {
                println!("🔄 {} transaction(s) missing across network", missing.len());
                println!("   This requires synchronization and block recreation");

                // If I'm the leader, I should recreate the block
                if am_leader {
                    println!("👑 As leader, I will recreate block after sync");
                    // TODO: Wait for missing txs to arrive, then recreate
                }
            }

            Err("Consensus not reached".to_string())
        }
    }

    /// Leader creates block and broadcasts proposal
    async fn leader_create_and_propose(
        &self,
        height: u64,
        previous_hash: String,
        mempool: Vec<Transaction>,
        create_coinbase_fn: impl Fn() -> Transaction,
    ) -> Result<(), String> {
        println!("📦 Creating block proposal...");

        // Create coinbase transaction
        let coinbase = create_coinbase_fn();

        // Combine coinbase + mempool transactions
        let mut transactions = vec![coinbase];
        transactions.extend(mempool);

        println!(
            "   Coinbase + {} mempool transactions",
            transactions.len() - 1
        );

        // Build block (simplified - real impl would be more complex)
        let transaction_ids: Vec<String> = transactions.iter().map(|tx| tx.txid.clone()).collect();
        let merkle_root = self.calculate_merkle_root(&transaction_ids);
        let block_hash = format!("block_hash_{}", height); // Simplified

        let proposal = BlockProposal {
            height,
            leader: self.my_ip.clone(),
            block_hash: block_hash.clone(),
            previous_hash,
            merkle_root,
            transaction_ids,
            timestamp: chrono::Utc::now().timestamp(),
        };

        // Store proposal
        self.consensus.propose_block(proposal).await?;

        println!("📡 Broadcasting proposal to network...");
        // TODO: Network broadcast

        Ok(())
    }

    /// Wait for block proposal (with timeout)
    async fn wait_for_proposal(&self, height: u64) -> Result<BlockProposal, String> {
        for i in 0..300 {
            // 30 seconds (100ms intervals)
            if let Some(proposal) = self.consensus.get_proposal(height).await {
                return Ok(proposal);
            }

            if i % 50 == 0 {
                // Every 5 seconds
                println!("   Still waiting... ({}s)", i / 10);
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Err("Timeout waiting for proposal".to_string())
    }

    /// Wait for consensus with timeout
    async fn wait_for_consensus(
        &self,
        height: u64,
        block_hash: &str,
        timeout_secs: u64,
    ) -> (bool, usize, usize, Vec<String>) {
        let iterations = timeout_secs * 10; // 100ms intervals

        for i in 0..iterations {
            let (has_consensus, approvals, required, rejections) =
                self.consensus.has_consensus(height, block_hash).await;

            if has_consensus {
                return (true, approvals, required, rejections);
            }

            if i % 50 == 0 && i > 0 {
                // Every 5 seconds
                println!("   Votes: {}/{} ({}s elapsed)", approvals, required, i / 10);
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Timeout - return final state
        self.consensus.has_consensus(height, block_hash).await
    }

    /// Calculate merkle root (simplified)
    fn calculate_merkle_root(&self, transaction_ids: &[String]) -> String {
        use sha2::{Digest, Sha256};

        if transaction_ids.is_empty() {
            return "0".repeat(64);
        }

        let mut hasher = Sha256::new();
        for txid in transaction_ids {
            hasher.update(txid.as_bytes());
        }

        format!("{:x}", hasher.finalize())
    }
}
