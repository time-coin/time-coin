//! Deterministic Block Consensus
//!
//! Simple, robust consensus mechanism:
//! 1. All nodes generate the SAME deterministic block at midnight
//! 2. Nodes compare their blocks with peers
//! 3. If blocks match → accept and finalize
//! 4. If blocks differ → reconcile differences (transactions, masternodes, etc.)
//! 5. Invalid transactions are rejected and wallets notified
//!
//! This eliminates leader selection, timeouts, and single points of failure.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use time_core::block::Block;
use time_core::state::BlockchainState;
use time_core::transaction::Transaction;
use time_network::PeerManager;
use tokio::sync::RwLock;

/// Result of deterministic consensus
#[derive(Debug)]
pub enum ConsensusResult {
    /// Block matches network consensus and is finalized
    Consensus(Block),
    /// Block has differences that need reconciliation
    NeedsReconciliation {
        our_block: Block,
        peer_blocks: Vec<(String, Block)>,
        differences: BlockDifferences,
    },
    /// Not enough peers to verify consensus
    InsufficientPeers,
}

/// Differences found between blocks
#[derive(Debug)]
pub struct BlockDifferences {
    pub transaction_mismatches: Vec<TransactionMismatch>,
    pub masternode_mismatches: Vec<MasternodeMismatch>,
    #[allow(dead_code)]
    pub hash_mismatches: bool,
}

#[derive(Debug)]
pub struct TransactionMismatch {
    pub tx_hash: String,
    pub present_on: Vec<String>, // Node IPs that have this transaction
    pub missing_on: Vec<String>, // Node IPs missing this transaction
}

#[derive(Debug)]
pub struct MasternodeMismatch {
    #[allow(dead_code)]
    pub wallet_address: String,
    #[allow(dead_code)]
    pub expected_reward: u64,
    #[allow(dead_code)]
    pub actual_rewards: HashMap<String, u64>, // Node IP -> reward amount
}

pub struct DeterministicConsensus {
    my_id: String,
    peer_manager: Arc<PeerManager>,
    blockchain: Arc<RwLock<BlockchainState>>,
}

impl DeterministicConsensus {
    pub fn new(
        my_id: String,
        peer_manager: Arc<PeerManager>,
        blockchain: Arc<RwLock<BlockchainState>>,
    ) -> Self {
        Self {
            my_id,
            peer_manager,
            blockchain,
        }
    }

    /// Get the correct TCP port based on network type
    fn get_p2p_port(&self) -> u16 {
        match self.peer_manager.network {
            time_network::discovery::NetworkType::Mainnet => 24000,
            time_network::discovery::NetworkType::Testnet => 24100,
        }
    }

    /// Run deterministic consensus for a block
    ///
    /// Steps:
    /// 1. Create deterministic block locally
    /// 2. Request blocks from all peers
    /// 3. Compare blocks
    /// 4. If 2/3+ match → finalize
    /// 5. If differences → reconcile
    pub async fn run_consensus(
        &self,
        block_num: u64,
        timestamp: DateTime<Utc>,
        masternodes: &[String],
        transactions: Vec<Transaction>,
        total_fees: u64,
    ) -> ConsensusResult {
        println!("\n🔷 Deterministic Consensus - Block #{}", block_num);
        println!("   Active nodes: {}", masternodes.len());

        // Step 1: Create our deterministic block
        let our_block = self
            .create_deterministic_block(
                block_num,
                timestamp,
                masternodes,
                transactions.clone(),
                total_fees,
            )
            .await;

        println!(
            "   ✓ Created local block: {}...",
            &our_block.hash[..our_block.hash.len().min(16)]
        );
        println!("      Transactions: {}", our_block.transactions.len());
        println!("      Merkle root: {}", &our_block.header.merkle_root);

        // Step 2: Request blocks from all peers
        let peer_ips = self.peer_manager.get_peer_ips().await;
        if peer_ips.is_empty() {
            println!("   ⚠️  No peers available - accepting local block");
            return ConsensusResult::Consensus(our_block);
        }

        println!(
            "   📡 Requesting block {} from {} peers...",
            block_num,
            peer_ips.len()
        );
        let peer_blocks = self.request_blocks_from_peers(&peer_ips, block_num).await;

        if peer_blocks.is_empty() {
            println!("   ⚠️  No peer responses - insufficient for consensus");
            return ConsensusResult::InsufficientPeers;
        }

        println!("   ✓ Received {} peer blocks", peer_blocks.len());

        // Step 3: Compare blocks
        let (matches, differences) = self.compare_blocks(&our_block, &peer_blocks).await;

        let total_nodes = peer_blocks.len() + 1; // peers + us
        let required_matches = (total_nodes * 2).div_ceil(3); // 2/3+ threshold

        println!("   📊 Consensus check:");
        println!(
            "      Matching blocks: {}/{}",
            matches + 1, // +1 for our block
            total_nodes
        );
        println!("      Required: {}", required_matches);

        // Step 4: Check for consensus
        if matches + 1 >= required_matches {
            // 2/3+ consensus achieved
            println!("   ✅ CONSENSUS REACHED - Block finalized!");
            ConsensusResult::Consensus(our_block)
        } else {
            // Differences found - need reconciliation
            println!("   ⚠️  DIFFERENCES DETECTED - Reconciliation needed");
            self.print_differences(&differences);

            ConsensusResult::NeedsReconciliation {
                our_block,
                peer_blocks,
                differences,
            }
        }
    }

    /// Create a deterministic block that all nodes should generate identically
    async fn create_deterministic_block(
        &self,
        block_num: u64,
        timestamp: DateTime<Utc>,
        _masternodes: &[String],
        mut transactions: Vec<Transaction>,
        total_fees: u64,
    ) -> Block {
        let blockchain = self.blockchain.read().await;
        let previous_hash = blockchain.chain_tip_hash().to_string();
        let masternode_counts = blockchain.masternode_counts().clone();

        // Get active masternodes with tiers - MUST BE DETERMINISTICALLY SORTED
        let mut active_masternodes: Vec<(String, time_core::MasternodeTier)> = blockchain
            .get_active_masternodes()
            .iter()
            .map(|mn| (mn.wallet_address.clone(), mn.tier))
            .collect();

        // Sort by wallet address to ensure all nodes have same order
        active_masternodes.sort_by(|a, b| a.0.cmp(&b.0));

        drop(blockchain);

        // Create deterministic coinbase transaction
        let coinbase_tx = time_core::block::create_coinbase_transaction(
            block_num,
            &active_masternodes,
            &masternode_counts,
            total_fees,
            timestamp.timestamp(),
        );

        // Sort transactions deterministically (by txid)
        transactions.sort_by(|a, b| a.txid.cmp(&b.txid));

        // Combine coinbase + sorted transactions
        let mut all_transactions = vec![coinbase_tx];
        all_transactions.extend(transactions);

        // Use deterministic validator ID (not any specific node)
        let validator_id = format!("consensus_block_{}", block_num);

        let mut block = Block {
            hash: String::new(),
            header: time_core::block::BlockHeader {
                block_number: block_num,
                timestamp,
                previous_hash,
                merkle_root: String::new(),
                validator_signature: validator_id.clone(),
                validator_address: validator_id,
                masternode_counts,
            },
            transactions: all_transactions,
        };

        // Calculate merkle root and hash
        block.header.merkle_root = block.calculate_merkle_root();
        block.hash = block.calculate_hash();

        block
    }

    /// Request blocks from multiple peers
    async fn request_blocks_from_peers(
        &self,
        peer_ips: &[String],
        block_num: u64,
    ) -> Vec<(String, Block)> {
        let p2p_port = self.get_p2p_port();
        let mut peer_blocks = Vec::new();

        for peer_ip in peer_ips {
            let peer_addr = format!("{}:{}", peer_ip, p2p_port);

            // Try to get the block from this peer
            match tokio::time::timeout(
                tokio::time::Duration::from_secs(5),
                self.peer_manager
                    .request_block_by_height(&peer_addr, block_num),
            )
            .await
            {
                Ok(Ok(block)) => {
                    println!("      ✓ Received block from {}", peer_ip);
                    peer_blocks.push((peer_ip.clone(), block));
                }
                Ok(Err(e)) => {
                    println!("      ✗ Error from {}: {}", peer_ip, e);
                }
                Err(_) => {
                    println!("      ✗ Timeout from {}", peer_ip);
                }
            }
        }

        peer_blocks
    }

    /// Compare our block with peer blocks
    ///
    /// Returns: (number of matching blocks, differences if any)
    async fn compare_blocks(
        &self,
        our_block: &Block,
        peer_blocks: &[(String, Block)],
    ) -> (usize, BlockDifferences) {
        let mut matches = 0;
        let mut transaction_mismatches = Vec::new();
        let mut masternode_mismatches = Vec::new();
        let mut hash_mismatches = false;

        for (peer_ip, peer_block) in peer_blocks {
            if peer_block.hash == our_block.hash {
                // Perfect match
                matches += 1;
                println!("      ✓ {} - MATCH", peer_ip);
            } else {
                println!("      ⚠️  {} - DIFFERENT", peer_ip);
                hash_mismatches = true;

                // Analyze differences
                self.analyze_block_differences(
                    our_block,
                    peer_block,
                    peer_ip,
                    &mut transaction_mismatches,
                    &mut masternode_mismatches,
                );
            }
        }

        (
            matches,
            BlockDifferences {
                transaction_mismatches,
                masternode_mismatches,
                hash_mismatches,
            },
        )
    }

    /// Analyze specific differences between two blocks
    fn analyze_block_differences(
        &self,
        our_block: &Block,
        peer_block: &Block,
        peer_ip: &str,
        transaction_mismatches: &mut Vec<TransactionMismatch>,
        _masternode_mismatches: &mut Vec<MasternodeMismatch>,
    ) {
        // Compare transactions
        let our_tx_hashes: std::collections::HashSet<_> =
            our_block.transactions.iter().map(|tx| &tx.txid).collect();
        let peer_tx_hashes: std::collections::HashSet<_> =
            peer_block.transactions.iter().map(|tx| &tx.txid).collect();

        // Find transactions only in our block
        for tx_hash in our_tx_hashes.difference(&peer_tx_hashes) {
            transaction_mismatches.push(TransactionMismatch {
                tx_hash: (*tx_hash).clone(),
                present_on: vec![self.my_id.clone()],
                missing_on: vec![peer_ip.to_string()],
            });
        }

        // Find transactions only in peer block
        for tx_hash in peer_tx_hashes.difference(&our_tx_hashes) {
            transaction_mismatches.push(TransactionMismatch {
                tx_hash: (*tx_hash).clone(),
                present_on: vec![peer_ip.to_string()],
                missing_on: vec![self.my_id.clone()],
            });
        }

        // TODO: Compare coinbase rewards (masternode rewards)
    }

    /// Print differences for debugging
    fn print_differences(&self, differences: &BlockDifferences) {
        if !differences.transaction_mismatches.is_empty() {
            println!(
                "      📋 Transaction mismatches: {}",
                differences.transaction_mismatches.len()
            );
            for mismatch in &differences.transaction_mismatches {
                let tx_hash_display = if mismatch.tx_hash.len() > 16 {
                    format!("{}...", &mismatch.tx_hash[..16])
                } else {
                    mismatch.tx_hash.clone()
                };
                println!("         • {}", tx_hash_display);
                println!("           Present on: {:?}", mismatch.present_on);
                println!("           Missing on: {:?}", mismatch.missing_on);
            }
        }

        if !differences.masternode_mismatches.is_empty() {
            println!(
                "      💰 Masternode reward mismatches: {}",
                differences.masternode_mismatches.len()
            );
        }
    }

    /// Reconcile differences and create consensus block
    ///
    /// Rules:
    /// 1. Transactions must be validated by 2/3+ nodes to be included
    /// 2. Invalid transactions are rejected
    /// 3. Masternode rewards are recalculated based on consensus
    pub async fn reconcile_and_finalize(
        &self,
        block_num: u64,
        timestamp: DateTime<Utc>,
        _our_block: Block,
        peer_blocks: Vec<(String, Block)>,
        differences: BlockDifferences,
    ) -> Option<Block> {
        println!("\n🔧 Reconciling block differences...");

        // Step 1: Validate all unique transactions across the network
        let validated_transactions = self
            .validate_transactions_with_network(&differences.transaction_mismatches, &peer_blocks)
            .await;

        println!(
            "   ✓ Validated {} transactions",
            validated_transactions.len()
        );

        // Step 2: Get consensus on masternode list
        let consensus_masternodes = self.get_consensus_masternodes(&peer_blocks).await;

        println!(
            "   ✓ Consensus on {} masternodes",
            consensus_masternodes.len()
        );

        // Step 3: Recreate block with validated transactions and consensus masternodes
        let reconciled_block = self
            .create_deterministic_block(
                block_num,
                timestamp,
                &consensus_masternodes,
                validated_transactions,
                0, // TODO: Recalculate fees properly
            )
            .await;

        println!(
            "   ✓ Reconciled block: {}",
            if reconciled_block.hash.len() > 16 {
                format!("{}...", &reconciled_block.hash[..16])
            } else {
                reconciled_block.hash.clone()
            }
        );

        // Step 4: Verify reconciled block with peers
        // (In production, we'd do another round of comparison here)

        Some(reconciled_block)
    }

    /// Validate transactions with the network
    ///
    /// A transaction is valid if 2/3+ nodes have it in their mempool/block
    async fn validate_transactions_with_network(
        &self,
        _mismatches: &[TransactionMismatch],
        peer_blocks: &[(String, Block)],
    ) -> Vec<Transaction> {
        // Collect all unique transactions from peer blocks
        let mut transaction_votes: HashMap<String, (Transaction, usize)> = HashMap::new();

        for (_peer_ip, block) in peer_blocks {
            for tx in &block.transactions {
                // Skip coinbase (always first transaction)
                if tx.inputs.is_empty() {
                    continue;
                }

                transaction_votes
                    .entry(tx.txid.clone())
                    .and_modify(|(_, count)| *count += 1)
                    .or_insert((tx.clone(), 1));
            }
        }

        let required_votes = (peer_blocks.len() * 2).div_ceil(3);

        // Filter to transactions with 2/3+ votes
        transaction_votes
            .into_iter()
            .filter(|(_, (_, count))| *count >= required_votes)
            .map(|(_, (tx, _))| tx)
            .collect()
    }

    /// Get consensus masternode list from peer blocks
    async fn get_consensus_masternodes(&self, peer_blocks: &[(String, Block)]) -> Vec<String> {
        // For now, use the masternode list from the majority of blocks
        // In production, this would be more sophisticated

        let mut masternode_lists: HashMap<Vec<String>, usize> = HashMap::new();

        for (_peer_ip, block) in peer_blocks {
            // Extract masternode wallet addresses from coinbase outputs
            let coinbase = &block.transactions[0];
            let mut masternodes: Vec<String> = coinbase
                .outputs
                .iter()
                .map(|output| output.address.clone())
                .collect();

            masternodes.sort(); // Deterministic ordering

            *masternode_lists.entry(masternodes).or_insert(0) += 1;
        }

        // Return the most common masternode list
        masternode_lists
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(list, _)| list)
            .unwrap_or_default()
    }
}
