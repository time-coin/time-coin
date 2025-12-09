//! Blockchain Scanner for UTXO Tracking
//!
//! Scans the blockchain database to find UTXOs belonging to registered xpubs.
//! This ensures wallets can sync their transaction history from existing blocks.
#![allow(missing_docs)]

use crate::address_monitor::AddressMonitor;
use crate::utxo_tracker::{UtxoInfo, UtxoTracker};
use std::sync::Arc;
use time_core::db::BlockchainDB;
use tracing::info;

/// Blockchain scanner that finds UTXOs for registered addresses
pub struct BlockchainScanner {
    /// Blockchain database
    db: Arc<BlockchainDB>,
    /// Address monitor to check which addresses we care about
    address_monitor: Arc<AddressMonitor>,
    /// UTXO tracker to store found UTXOs
    utxo_tracker: Arc<UtxoTracker>,
    /// Node identifier for logging
    node_id: String,
}

impl BlockchainScanner {
    /// Create a new blockchain scanner
    pub fn new(
        db: Arc<BlockchainDB>,
        address_monitor: Arc<AddressMonitor>,
        utxo_tracker: Arc<UtxoTracker>,
        node_id: String,
    ) -> Self {
        Self {
            db,
            address_monitor,
            utxo_tracker,
            node_id,
        }
    }

    /// Scan the entire blockchain for UTXOs belonging to registered xpubs
    pub async fn scan_blockchain(&self) -> Result<usize, String> {
        info!(node = %self.node_id, "Starting blockchain scan for registered xpubs");

        // Load all blocks from the database
        let blocks = self
            .db
            .load_all_blocks()
            .map_err(|e| format!("Failed to load blocks: {}", e))?;

        if blocks.is_empty() {
            info!(node = %self.node_id, "No blocks found in blockchain");
            return Ok(0);
        }

        info!(
            node = %self.node_id,
            block_count = blocks.len(),
            "Loaded blocks from blockchain"
        );

        let mut total_utxos_found = 0;

        // Scan each block
        for (block_index, block) in blocks.iter().enumerate() {
            let block_height = block.header.block_number;

            // Process all transactions in the block (including coinbase as first transaction)
            for (tx_index, transaction) in block.transactions.iter().enumerate() {
                // Check each output to see if it belongs to a monitored address
                for (output_index, output) in transaction.outputs.iter().enumerate() {
                    if self
                        .address_monitor
                        .is_monitored_address(&output.address)
                        .await
                    {
                        info!(
                            node = %self.node_id,
                            block = block_height,
                            tx_index = tx_index,
                            output_index = output_index,
                            address = %output.address,
                            amount = output.amount,
                            "Found UTXO for monitored address"
                        );

                        // Create UTXO info
                        let utxo = UtxoInfo {
                            txid: transaction.txid.clone(),
                            vout: output_index as u32,
                            address: output.address.clone(),
                            amount: output.amount,
                            block_height: Some(block_height),
                            confirmations: 0, // Will be updated by tracker
                        };

                        // Add to UTXO tracker
                        self.utxo_tracker.add_utxo(utxo).await;
                        total_utxos_found += 1;
                    }
                }
            }

            // Log progress every 100 blocks
            if block_index % 100 == 0 {
                info!(
                    node = %self.node_id,
                    progress = format!("{}/{}", block_index + 1, blocks.len()),
                    utxos_found = total_utxos_found,
                    "Blockchain scan progress"
                );
            }
        }

        info!(
            node = %self.node_id,
            blocks_scanned = blocks.len(),
            utxos_found = total_utxos_found,
            "Blockchain scan completed"
        );

        Ok(total_utxos_found)
    }

    /// Scan blockchain for a specific xpub that was just registered
    pub async fn scan_for_xpub(&self, xpub: &str) -> Result<usize, String> {
        info!(
            node = %self.node_id,
            xpub = %xpub,
            "Starting blockchain scan for newly registered xpub"
        );

        // Generate addresses from xpub (scan first 100 addresses in both chains)
        let mut addresses_to_check = Vec::new();

        // External chain (receiving addresses) - check first 100
        for index in 0..100 {
            if let Ok(address) =
                wallet::xpub_to_address(xpub, 0, index, wallet::NetworkType::Mainnet)
            {
                addresses_to_check.push(address);
            }
        }

        // Internal chain (change addresses) - check first 100
        for index in 0..100 {
            if let Ok(address) =
                wallet::xpub_to_address(xpub, 1, index, wallet::NetworkType::Mainnet)
            {
                addresses_to_check.push(address);
            }
        }

        info!(
            node = %self.node_id,
            address_count = addresses_to_check.len(),
            "Generated addresses from xpub for scanning"
        );

        // Load all blocks
        let blocks = self
            .db
            .load_all_blocks()
            .map_err(|e| format!("Failed to load blocks: {}", e))?;

        if blocks.is_empty() {
            return Ok(0);
        }

        let mut total_utxos_found = 0;

        // Scan each block
        for block in blocks.iter() {
            let block_height = block.header.block_number;

            // Check transaction outputs
            for transaction in &block.transactions {
                for (output_index, output) in transaction.outputs.iter().enumerate() {
                    if addresses_to_check.contains(&output.address) {
                        let utxo = UtxoInfo {
                            txid: transaction.txid.clone(),
                            vout: output_index as u32,
                            address: output.address.clone(),
                            amount: output.amount,
                            block_height: Some(block_height),
                            confirmations: 0,
                        };

                        self.utxo_tracker.add_utxo(utxo).await;
                        total_utxos_found += 1;
                    }
                }
            }
        }

        info!(
            node = %self.node_id,
            xpub = %xpub,
            utxos_found = total_utxos_found,
            "Completed blockchain scan for xpub"
        );

        Ok(total_utxos_found)
    }
}
