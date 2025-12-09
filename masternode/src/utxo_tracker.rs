//! UTXO Tracker - Tracks all UTXOs and matches them to wallet xpubs
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use time_core::{Block, Transaction};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtxoInfo {
    pub txid: String,
    pub vout: u32,
    pub address: String,
    pub amount: u64,
    pub block_height: Option<u64>,
    pub confirmations: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSubscription {
    pub xpub: String,
    pub known_addresses: HashSet<String>,
    pub last_sync: u64,
}

pub struct UtxoTracker {
    /// All UTXOs by address
    utxos: Arc<RwLock<HashMap<String, Vec<UtxoInfo>>>>,

    /// Wallet subscriptions by xpub
    subscriptions: Arc<RwLock<HashMap<String, WalletSubscription>>>,

    /// Spent UTXOs (txid:vout format)
    spent: Arc<RwLock<HashSet<String>>>,

    /// Current blockchain height
    height: Arc<RwLock<u64>>,
}

impl UtxoTracker {
    pub fn new() -> Self {
        Self {
            utxos: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            spent: Arc::new(RwLock::new(HashSet::new())),
            height: Arc::new(RwLock::new(0)),
        }
    }

    /// Register a wallet's xpub for tracking
    pub async fn subscribe_xpub(&self, xpub: String) -> Result<(), String> {
        let mut subs = self.subscriptions.write().await;

        if !subs.contains_key(&xpub) {
            subs.insert(
                xpub.clone(),
                WalletSubscription {
                    xpub: xpub.clone(),
                    known_addresses: HashSet::new(), // Start empty, wallet will register addresses
                    last_sync: 0,
                },
            );

            log::info!(
                "📝 Subscribed wallet xpub: {}...",
                &xpub[..std::cmp::min(20, xpub.len())]
            );
        }

        Ok(())
    }

    /// Register addresses for a subscribed xpub
    pub async fn register_addresses(
        &self,
        xpub: &str,
        addresses: Vec<String>,
    ) -> Result<(), String> {
        let mut subs = self.subscriptions.write().await;

        let subscription = subs
            .get_mut(xpub)
            .ok_or_else(|| format!("xpub not subscribed: {}", xpub))?;

        for addr in addresses {
            subscription.known_addresses.insert(addr);
        }

        log::info!(
            "📝 Registered {} addresses for xpub",
            subscription.known_addresses.len()
        );
        Ok(())
    }

    /// Get all UTXOs for a given xpub
    pub async fn get_utxos_for_xpub(&self, xpub: &str) -> Result<Vec<UtxoInfo>, String> {
        let subs = self.subscriptions.read().await;
        let utxos = self.utxos.read().await;
        let spent = self.spent.read().await;
        let height = *self.height.read().await;

        let subscription = subs
            .get(xpub)
            .ok_or_else(|| format!("xpub not subscribed: {}", xpub))?;

        let mut result = Vec::new();

        // Find all UTXOs for addresses in this wallet
        for address in &subscription.known_addresses {
            if let Some(addr_utxos) = utxos.get(address) {
                for utxo in addr_utxos {
                    let utxo_key = format!("{}:{}", utxo.txid, utxo.vout);

                    // Only include unspent UTXOs
                    if !spent.contains(&utxo_key) {
                        let mut utxo_with_confirmations = utxo.clone();
                        if let Some(block_height) = utxo.block_height {
                            utxo_with_confirmations.confirmations =
                                height.saturating_sub(block_height);
                        }
                        result.push(utxo_with_confirmations);
                    }
                }
            }
        }

        Ok(result)
    }

    /// Process a new block and update UTXOs
    pub async fn process_block(&self, block: &Block) -> Result<(), String> {
        log::info!(
            "🔍 Processing block {} for UTXOs",
            block.header.block_number
        );

        let mut utxos = self.utxos.write().await;
        let mut spent = self.spent.write().await;
        let mut height = self.height.write().await;

        *height = block.header.block_number;

        // Process all transactions in the block
        for tx in &block.transactions {
            // Mark inputs as spent
            for input in &tx.inputs {
                let utxo_key = format!(
                    "{}:{}",
                    input.previous_output.txid, input.previous_output.vout
                );
                spent.insert(utxo_key);
            }

            // Add outputs as new UTXOs
            for (vout, output) in tx.outputs.iter().enumerate() {
                let utxo = UtxoInfo {
                    txid: tx.txid.clone(),
                    vout: vout as u32,
                    address: output.address.clone(),
                    amount: output.amount,
                    block_height: Some(block.header.block_number),
                    confirmations: 0,
                };

                utxos
                    .entry(output.address.clone())
                    .or_insert_with(Vec::new)
                    .push(utxo);
            }
        }

        log::info!(
            "✅ Block {} processed: {} UTXOs tracked",
            block.header.block_number,
            utxos.len()
        );
        Ok(())
    }

    /// Process a mempool transaction (unconfirmed)
    pub async fn process_mempool_tx(&self, tx: &Transaction) -> Result<(), String> {
        let mut utxos = self.utxos.write().await;

        // Add outputs as unconfirmed UTXOs
        for (vout, output) in tx.outputs.iter().enumerate() {
            let utxo = UtxoInfo {
                txid: tx.txid.clone(),
                vout: vout as u32,
                address: output.address.clone(),
                amount: output.amount,
                block_height: None, // Unconfirmed
                confirmations: 0,
            };

            utxos
                .entry(output.address.clone())
                .or_insert_with(Vec::new)
                .push(utxo);
        }

        Ok(())
    }

    /// Scan existing blockchain for UTXOs belonging to registered addresses
    pub async fn scan_blockchain_for_addresses(
        &self,
        blocks: Vec<time_core::Block>,
        addresses: &[String],
    ) -> Result<(), String> {
        log::info!(
            "🔍 Scanning {} blocks for {} addresses",
            blocks.len(),
            addresses.len()
        );

        for block in &blocks {
            // Process all transactions in the block
            for tx in &block.transactions {
                // Check outputs for matching addresses
                for (vout, output) in tx.outputs.iter().enumerate() {
                    if addresses.contains(&output.address) {
                        let utxo = UtxoInfo {
                            txid: tx.txid.clone(),
                            vout: vout as u32,
                            address: output.address.clone(),
                            amount: output.amount,
                            block_height: Some(block.header.block_number),
                            confirmations: 0,
                        };

                        let mut utxos = self.utxos.write().await;
                        utxos
                            .entry(output.address.clone())
                            .or_insert_with(Vec::new)
                            .push(utxo);

                        log::info!(
                            "💰 Found UTXO: {} TIME at address {}",
                            output.amount,
                            output.address
                        );
                    }
                }

                // Mark spent outputs
                for input in &tx.inputs {
                    let utxo_key = format!(
                        "{}:{}",
                        input.previous_output.txid, input.previous_output.vout
                    );
                    let mut spent = self.spent.write().await;
                    spent.insert(utxo_key);
                }
            }
        }

        // Update height
        if let Some(last_block) = blocks.last() {
            let mut height = self.height.write().await;
            *height = last_block.header.block_number;
        }

        log::info!("✅ Blockchain scan complete");
        Ok(())
    }

    /// Get transactions for a specific xpub (for wallet sync)
    pub async fn get_transactions_for_xpub(&self, xpub: &str) -> Result<Vec<Transaction>, String> {
        // This will be implemented when we have transaction storage
        // For now, we'll use the UTXO data to reconstruct basic transaction info
        let _utxos = self.get_utxos_for_xpub(xpub).await?;

        // TODO: Load full transactions from storage
        Ok(Vec::new())
    }

    /// Add a single UTXO directly (used by blockchain scanner)
    pub async fn add_utxo(&self, utxo: UtxoInfo) {
        let mut utxos = self.utxos.write().await;
        utxos
            .entry(utxo.address.clone())
            .or_insert_with(Vec::new)
            .push(utxo);
    }

    /// Get statistics
    pub async fn stats(&self) -> UtxoStats {
        let utxos = self.utxos.read().await;
        let spent = self.spent.read().await;
        let subs = self.subscriptions.read().await;
        let height = *self.height.read().await;

        let total_utxos: usize = utxos.values().map(|v| v.len()).sum();

        UtxoStats {
            total_utxos,
            spent_count: spent.len(),
            subscribed_wallets: subs.len(),
            current_height: height,
        }
    }
}

impl Default for UtxoTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UtxoStats {
    pub total_utxos: usize,
    pub spent_count: usize,
    pub subscribed_wallets: usize,
    pub current_height: u64,
}
