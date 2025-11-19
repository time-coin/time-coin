//! UTXO Protocol Integration for Masternodes
//!
//! Integrates the UTXO state protocol with the P2P network layer,
//! enabling instant finality through masternode consensus.

use crate::{utxo_tracker::UtxoTracker, voting::VoteTracker, TransactionNotification, WsBridge};
use std::net::IpAddr;
use std::sync::Arc;
use time_consensus::utxo_state_protocol::{UTXOStateManager, UTXOStateNotification};
use time_mempool::Mempool;
use time_network::{PeerManager, UTXOProtocolHandler};
use tracing::{debug, info, warn};

/// Masternode UTXO Protocol Integration
///
/// Coordinates between:
/// - UTXO State Protocol (consensus layer)
/// - P2P Network (communication layer)
/// - Instant Finality (voting layer)
/// - WebSocket clients (wallet notifications)
/// - Mempool (transaction management)
/// - UTXO Tracker (wallet-specific transaction tracking)
/// - Address Monitor (xpub address tracking)
#[derive(Clone)]
pub struct MasternodeUTXOIntegration {
    /// UTXO state manager
    utxo_manager: Arc<UTXOStateManager>,
    /// P2P network message handler
    utxo_handler: Arc<UTXOProtocolHandler>,
    /// P2P peer manager
    peer_manager: Arc<PeerManager>,
    /// WebSocket bridge for wallet notifications
    ws_bridge: Option<Arc<WsBridge>>,
    /// Vote tracker for instant finality
    vote_tracker: Arc<VoteTracker>,
    /// Node identifier
    node_id: String,
    /// Transaction mempool
    mempool: Arc<Mempool>,
    /// UTXO tracker for wallet subscriptions
    utxo_tracker: Arc<UtxoTracker>,
    /// Address monitor for xpub tracking
    address_monitor: Option<Arc<crate::address_monitor::AddressMonitor>>,
}

impl MasternodeUTXOIntegration {
    /// Create a new masternode UTXO integration
    pub fn new(
        node_id: String,
        utxo_manager: Arc<UTXOStateManager>,
        peer_manager: Arc<PeerManager>,
    ) -> Self {
        let utxo_handler = Arc::new(UTXOProtocolHandler::new(utxo_manager.clone()));
        let vote_tracker = Arc::new(VoteTracker::new(2)); // Require 2 votes for consensus
        let mempool = Arc::new(Mempool::new("mainnet".to_string()));
        let utxo_tracker = Arc::new(UtxoTracker::new());

        Self {
            utxo_manager,
            utxo_handler,
            peer_manager,
            ws_bridge: None,
            vote_tracker,
            node_id,
            mempool,
            utxo_tracker,
            address_monitor: None,
        }
    }

    /// Set WebSocket bridge for wallet notifications
    pub fn set_ws_bridge(&mut self, ws_bridge: Arc<WsBridge>) {
        self.ws_bridge = Some(ws_bridge);
        info!(node = %self.node_id, "WebSocket bridge connected for transaction notifications");
    }

    /// Set address monitor for xpub tracking
    pub fn set_address_monitor(
        &mut self,
        address_monitor: Arc<crate::address_monitor::AddressMonitor>,
    ) {
        self.address_monitor = Some(address_monitor);
        info!(node = %self.node_id, "Address monitor connected for xpub tracking");
    }

    /// Initialize the integration - sets up notification handlers
    pub async fn initialize(&self) -> Result<(), String> {
        info!(node = %self.node_id, "Initializing UTXO protocol integration");

        // Load mempool from disk if it exists
        let mempool_path = format!("data/{}/mempool.json", self.node_id);
        match self.mempool.load_from_disk(&mempool_path).await {
            Ok(count) => {
                if count > 0 {
                    info!(node = %self.node_id, count = count, "Loaded transactions from mempool cache");
                }
            }
            Err(e) => {
                warn!(node = %self.node_id, error = %e, "Failed to load mempool from disk");
            }
        }

        // Set up notification handler to route UTXO state changes to P2P network
        let peer_manager = self.peer_manager.clone();
        let node_id = self.node_id.clone();

        // Get network-aware port
        let network_port = match peer_manager.network {
            time_network::discovery::NetworkType::Mainnet => 24000,
            time_network::discovery::NetworkType::Testnet => 24100,
        };

        self.utxo_handler
            .setup_notification_handler(move |peer_ip: IpAddr, message| {
                let peer_manager = peer_manager.clone();
                let node_id = node_id.clone();
                Box::pin(async move {
                    if let Err(e) = peer_manager
                        .send_message_to_peer(
                            std::net::SocketAddr::new(peer_ip, network_port),
                            message,
                        )
                        .await
                    {
                        debug!(
                            node = %node_id,
                            peer = %peer_ip,
                            error = %e,
                            "Failed to send UTXO notification to peer"
                        );
                    }
                })
            })
            .await;

        info!(node = %self.node_id, "UTXO protocol integration initialized");
        Ok(())
    }

    /// Handle incoming network message - delegates to UTXO handler if relevant
    pub async fn handle_network_message(
        &self,
        message: &time_network::protocol::NetworkMessage,
        peer_ip: IpAddr,
    ) -> Result<Option<time_network::protocol::NetworkMessage>, String> {
        // Check if this is a UTXO protocol message
        match message {
            time_network::protocol::NetworkMessage::UTXOStateQuery { .. }
            | time_network::protocol::NetworkMessage::UTXOStateResponse { .. }
            | time_network::protocol::NetworkMessage::UTXOStateNotification { .. }
            | time_network::protocol::NetworkMessage::UTXOSubscribe { .. }
            | time_network::protocol::NetworkMessage::UTXOUnsubscribe { .. } => {
                // Handle via UTXO protocol handler
                self.utxo_handler.handle_message(message, peer_ip).await
            }
            // Handle incoming transaction broadcasts - add to mempool
            time_network::protocol::NetworkMessage::TransactionBroadcast(tx) => {
                info!(
                    node = %self.node_id,
                    txid = %tx.txid,
                    "Received transaction broadcast from peer"
                );

                // Add to mempool if not already present
                if !self.mempool.contains(&tx.txid).await {
                    match self.mempool.add_transaction(tx.clone()).await {
                        Ok(_) => {
                            info!(
                                node = %self.node_id,
                                txid = %tx.txid,
                                "Added broadcasted transaction to mempool"
                            );

                            // Track in UTXO tracker for wallet notifications
                            if let Err(e) = self.track_mempool_transaction(tx).await {
                                warn!(
                                    node = %self.node_id,
                                    txid = %tx.txid,
                                    error = %e,
                                    "Failed to track transaction in UTXO tracker"
                                );
                            }

                            // Notify connected wallets about the new transaction
                            if let Some(ref bridge) = self.ws_bridge {
                                self.notify_wallets_of_transaction(bridge, tx).await;
                            }
                        }
                        Err(e) => {
                            warn!(
                                node = %self.node_id,
                                txid = %tx.txid,
                                error = %e,
                                "Failed to add transaction to mempool"
                            );
                        }
                    }
                }

                // Also handle via UTXO protocol handler for any additional processing
                self.utxo_handler.handle_message(message, peer_ip).await
            }
            // NEW: Handle transaction notifications from P2P network
            time_network::protocol::NetworkMessage::NewTransactionNotification { transaction } => {
                info!(
                    node = %self.node_id,
                    txid = %transaction.tx_hash,
                    "Received transaction notification from P2P network"
                );

                // Broadcast to WebSocket clients if bridge is available
                if let Some(ref bridge) = self.ws_bridge {
                    self.broadcast_transaction_to_wallets(bridge, transaction)
                        .await;
                }

                // Return Ok - transaction notification doesn't need a response
                Ok(None)
            }
            // NEW: Handle instant finality vote requests
            time_network::protocol::NetworkMessage::InstantFinalityRequest(tx) => {
                info!(
                    node = %self.node_id,
                    txid = %tx.txid,
                    "Received instant finality vote request"
                );

                // Validate the transaction
                let is_valid = self.validate_transaction(tx).await;

                info!(
                    node = %self.node_id,
                    txid = %tx.txid,
                    valid = %is_valid,
                    "Transaction validation result"
                );

                // Create vote response
                let vote = time_network::protocol::NetworkMessage::InstantFinalityVote {
                    txid: tx.txid.clone(),
                    voter: self.node_id.clone(),
                    approve: is_valid,
                    timestamp: chrono::Utc::now().timestamp() as u64,
                };

                // Broadcast vote to all peers (not just requester)
                self.peer_manager.broadcast_message(vote.clone()).await;

                // Also return vote as response to requester
                Ok(Some(vote))
            }
            // NEW: Handle incoming votes from other masternodes
            time_network::protocol::NetworkMessage::InstantFinalityVote {
                txid,
                voter,
                approve,
                timestamp,
            } => {
                info!(
                    node = %self.node_id,
                    txid = %txid,
                    voter = %voter,
                    approve = %approve,
                    "Received vote from masternode"
                );

                // Record the vote
                let vote = crate::voting::Vote {
                    txid: txid.clone(),
                    voter: voter.clone(),
                    approve: *approve,
                    timestamp: *timestamp,
                };

                // Check if consensus reached
                if let Some(consensus) = self.vote_tracker.record_vote(vote).await {
                    info!(
                        node = %self.node_id,
                        txid = %txid,
                        approved = %consensus,
                        "🎉 CONSENSUS REACHED for transaction"
                    );

                    // Notify wallet via WebSocket
                    if let Some(ref bridge) = self.ws_bridge {
                        self.notify_wallet_consensus(bridge, txid, consensus).await;
                    }

                    // If approved, finalize the transaction
                    if consensus {
                        if let Err(e) = self.finalize_transaction(txid).await {
                            warn!(
                                node = %self.node_id,
                                txid = %txid,
                                error = %e,
                                "Failed to finalize transaction after consensus"
                            );
                        }
                    }
                }

                // No response needed for votes
                Ok(None)
            }
            // NEW: Handle mempool query requests
            time_network::protocol::NetworkMessage::MempoolQuery => {
                info!(
                    node = %self.node_id,
                    "Received mempool query request"
                );

                // Get all transactions from our local mempool
                let transactions = self.mempool.get_all_transactions().await;

                info!(
                    node = %self.node_id,
                    count = transactions.len(),
                    "Sending mempool response"
                );

                // Return mempool response
                Ok(Some(
                    time_network::protocol::NetworkMessage::MempoolResponse(transactions),
                ))
            }
            // NEW: Handle mempool response from other nodes
            time_network::protocol::NetworkMessage::MempoolResponse(transactions) => {
                info!(
                    node = %self.node_id,
                    count = transactions.len(),
                    "Received mempool response from peer"
                );

                // Add missing transactions to our mempool
                let mut added_count = 0;
                for tx in transactions {
                    // Check if we already have this transaction
                    if !self.mempool.contains(&tx.txid).await {
                        match self.mempool.add_transaction(tx.clone()).await {
                            Ok(_) => {
                                info!(
                                    node = %self.node_id,
                                    txid = %tx.txid,
                                    "Added transaction from peer mempool"
                                );
                                added_count += 1;

                                // Notify connected wallets about synced transaction
                                if let Some(ref bridge) = self.ws_bridge {
                                    self.notify_wallets_of_transaction(bridge, tx).await;
                                }
                            }
                            Err(e) => {
                                debug!(
                                    node = %self.node_id,
                                    txid = %tx.txid,
                                    error = %e,
                                    "Failed to add transaction from peer mempool"
                                );
                            }
                        }
                    }
                }

                if added_count > 0 {
                    info!(
                        node = %self.node_id,
                        added = added_count,
                        total = transactions.len(),
                        "Synchronized {} new transactions from peer",
                        added_count
                    );
                }

                // No response needed
                Ok(None)
            }
            // NEW: Handle xpub registration for wallet address tracking
            time_network::protocol::NetworkMessage::RegisterXpub { xpub } => {
                info!(
                    node = %self.node_id,
                    xpub = %xpub,
                    "Received xpub registration request"
                );

                if let Some(ref monitor) = self.address_monitor {
                    match monitor.register_xpub(xpub).await {
                        Ok(_) => {
                            let stats = monitor.get_stats().await;
                            info!(
                                node = %self.node_id,
                                xpub_count = stats.xpub_count,
                                addresses = stats.total_addresses,
                                "Xpub registered successfully, now scanning for UTXOs..."
                            );

                            // Get all monitored addresses for this xpub
                            let addresses: Vec<String> = monitor
                                .get_all_monitored_addresses()
                                .await
                                .into_iter()
                                .collect();

                            // Subscribe the xpub in UTXO tracker
                            if let Err(e) = self.utxo_tracker.subscribe_xpub(xpub.clone()).await {
                                warn!(
                                    node = %self.node_id,
                                    error = %e,
                                    "Failed to subscribe xpub to UTXO tracker"
                                );
                                return Ok(Some(
                                    time_network::protocol::NetworkMessage::XpubRegistered {
                                        success: false,
                                        message: format!(
                                            "Failed to subscribe to UTXO tracker: {}",
                                            e
                                        ),
                                    },
                                ));
                            }

                            // Register the addresses for this xpub
                            if let Err(e) = self
                                .utxo_tracker
                                .register_addresses(xpub, addresses.clone())
                                .await
                            {
                                warn!(
                                    node = %self.node_id,
                                    error = %e,
                                    "Failed to register addresses for xpub"
                                );
                                return Ok(Some(
                                    time_network::protocol::NetworkMessage::XpubRegistered {
                                        success: false,
                                        message: format!("Failed to register addresses: {}", e),
                                    },
                                ));
                            }

                            // Scan existing blockchain for UTXOs
                            // TODO: Need to add blockchain access to scan existing blocks
                            info!(
                                node = %self.node_id,
                                address_count = addresses.len(),
                                "Registered {} addresses. Wallet will receive UTXOs from new blocks.",
                                addresses.len()
                            );

                            // Get existing UTXOs for this xpub
                            match self.utxo_tracker.get_utxos_for_xpub(xpub).await {
                                Ok(utxos) => {
                                    info!(
                                        node = %self.node_id,
                                        utxo_count = utxos.len(),
                                        "Found {} UTXOs for xpub, sending to wallet",
                                        utxos.len()
                                    );

                                    // Convert masternode UtxoInfo to network UtxoInfo
                                    let network_utxos: Vec<time_network::protocol::UtxoInfo> =
                                        utxos
                                            .into_iter()
                                            .map(|u| time_network::protocol::UtxoInfo {
                                                txid: u.txid,
                                                vout: u.vout,
                                                address: u.address,
                                                amount: u.amount,
                                                block_height: u.block_height,
                                                confirmations: u.confirmations,
                                            })
                                            .collect();

                                    // Send UTXOs back to wallet
                                    Ok(Some(time_network::protocol::NetworkMessage::UtxoUpdate {
                                        xpub: xpub.clone(),
                                        utxos: network_utxos,
                                    }))
                                }
                                Err(e) => {
                                    warn!(
                                        node = %self.node_id,
                                        error = %e,
                                        "Failed to get UTXOs for xpub"
                                    );
                                    Ok(Some(
                                        time_network::protocol::NetworkMessage::XpubRegistered {
                                            success: false,
                                            message: format!("Failed to get UTXOs: {}", e),
                                        },
                                    ))
                                }
                            }
                        }
                        Err(e) => {
                            warn!(
                                node = %self.node_id,
                                error = %e,
                                "Failed to register xpub"
                            );
                            Ok(Some(
                                time_network::protocol::NetworkMessage::XpubRegistered {
                                    success: false,
                                    message: format!("Failed to register xpub: {}", e),
                                },
                            ))
                        }
                    }
                } else {
                    Ok(Some(
                        time_network::protocol::NetworkMessage::XpubRegistered {
                            success: false,
                            message: "Address monitor not available".to_string(),
                        },
                    ))
                }
            }
            // Handle peer list request
            time_network::protocol::NetworkMessage::GetPeerList => {
                info!(
                    node = %self.node_id,
                    "Received peer list request"
                );

                // Get all peers from peer manager
                let peer_ips = self.peer_manager.get_peer_ips().await;
                let network = self.peer_manager.network;
                let peer_addresses: Vec<time_network::protocol::PeerAddress> = peer_ips
                    .into_iter()
                    .map(|ip| time_network::protocol::PeerAddress {
                        ip: ip.to_string(),
                        port: match network {
                            time_network::discovery::NetworkType::Mainnet => 24101,
                            time_network::discovery::NetworkType::Testnet => 24100,
                        },
                        version: "1.0.0".to_string(),
                    })
                    .collect();

                info!(
                    node = %self.node_id,
                    peer_count = peer_addresses.len(),
                    "Sending peer list response"
                );

                // Return peer list
                Ok(Some(time_network::protocol::NetworkMessage::PeerList(
                    peer_addresses,
                )))
            }
            _ => {
                // Not a UTXO protocol message
                Ok(None)
            }
        }
    }

    /// Process a transaction - lock UTXOs and prepare for voting
    pub async fn process_transaction(&self, tx: &time_core::Transaction) -> Result<(), String> {
        info!(
            node = %self.node_id,
            txid = %tx.txid,
            "Processing transaction - locking UTXOs"
        );

        // Lock all input UTXOs
        for input in &tx.inputs {
            let outpoint = time_core::OutPoint {
                txid: input.previous_output.txid.clone(),
                vout: input.previous_output.vout,
            };

            match self
                .utxo_manager
                .lock_utxo(&outpoint, tx.txid.clone())
                .await
            {
                Ok(_) => {
                    debug!(
                        node = %self.node_id,
                        outpoint = ?outpoint,
                        txid = %tx.txid,
                        "UTXO locked successfully"
                    );
                }
                Err(e) => {
                    warn!(
                        node = %self.node_id,
                        outpoint = ?outpoint,
                        txid = %tx.txid,
                        error = %e,
                        "Failed to lock UTXO"
                    );

                    // Unlock any UTXOs we already locked
                    for (i, prev_input) in tx.inputs.iter().enumerate() {
                        if i >= tx
                            .inputs
                            .iter()
                            .position(|inp| std::ptr::eq(inp, input))
                            .unwrap_or(usize::MAX)
                        {
                            break;
                        }
                        let prev_outpoint = time_core::OutPoint {
                            txid: prev_input.previous_output.txid.clone(),
                            vout: prev_input.previous_output.vout,
                        };
                        let _ = self.utxo_manager.unlock_utxo(&prev_outpoint).await;
                    }

                    return Err(format!("Failed to lock UTXO: {}", e));
                }
            }
        }

        // Broadcast transaction to network
        let tx_msg = time_network::protocol::NetworkMessage::TransactionBroadcast(tx.clone());
        self.peer_manager.broadcast_message(tx_msg).await;

        // Notify connected wallets about the new transaction
        if let Some(ref bridge) = self.ws_bridge {
            self.notify_wallets_of_transaction(bridge, tx).await;
        }

        info!(
            node = %self.node_id,
            txid = %tx.txid,
            "Transaction processed and broadcast to network"
        );

        Ok(())
    }

    /// Handle instant finality vote - update UTXO states when consensus reached
    pub async fn handle_instant_finality_achieved(
        &self,
        txid: &str,
        votes: usize,
    ) -> Result<(), String> {
        info!(
            node = %self.node_id,
            txid = %txid,
            votes = votes,
            "Instant finality achieved - finalizing UTXOs"
        );

        // Note: In a full implementation, we would look up the transaction
        // and finalize all its input UTXOs. For now, this is a placeholder.

        Ok(())
    }

    /// Broadcast UTXO state change to all masternodes
    pub async fn broadcast_state_change(&self, notification: UTXOStateNotification) {
        debug!(
            node = %self.node_id,
            outpoint = ?notification.outpoint,
            "Broadcasting UTXO state change to network"
        );

        self.peer_manager
            .broadcast_utxo_notification(&notification)
            .await;
    }

    /// Get UTXO handler for direct access
    pub fn utxo_handler(&self) -> &Arc<UTXOProtocolHandler> {
        &self.utxo_handler
    }

    /// Get UTXO manager for direct access
    pub fn utxo_manager(&self) -> &Arc<UTXOStateManager> {
        &self.utxo_manager
    }

    /// Get subscription count
    pub async fn get_subscription_count(&self) -> usize {
        self.utxo_handler.subscription_count().await
    }

    /// Get UTXO statistics
    pub async fn get_utxo_stats(&self) -> time_consensus::utxo_state_protocol::UTXOStateStats {
        self.utxo_manager.get_stats().await
    }

    /// Get vote tracker for direct access
    pub fn vote_tracker(&self) -> &Arc<VoteTracker> {
        &self.vote_tracker
    }

    /// Start background cleanup task for old votes
    pub fn start_cleanup_task(&self) {
        let vote_tracker = self.vote_tracker.clone();
        let node_id = self.node_id.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await; // Every hour

                info!(node = %node_id, "Running vote tracker cleanup");
                vote_tracker.cleanup_old(3600).await; // Remove votes older than 1 hour
            }
        });
    }

    /// Broadcast transaction to WebSocket wallet clients
    async fn broadcast_transaction_to_wallets(
        &self,
        bridge: &Arc<WsBridge>,
        transaction: &time_network::protocol::WalletTransaction,
    ) {
        // WalletTransaction already has from/to addresses
        let input_addresses = vec![transaction.from_address.clone()];
        let output_addresses = vec![transaction.to_address.clone()];

        // Create notification
        let notification = TransactionNotification {
            txid: transaction.tx_hash.clone(),
            inputs: input_addresses,
            outputs: output_addresses,
            amount: transaction.amount,
            timestamp: transaction.timestamp as i64,
        };

        // Broadcast to subscribed clients
        bridge.broadcast_transaction(notification).await;

        info!(
            node = %self.node_id,
            txid = %transaction.tx_hash,
            from = %transaction.from_address,
            to = %transaction.to_address,
            amount = %transaction.amount,
            "Broadcasted transaction to WebSocket clients"
        );
    }

    /// Notify wallets of a new transaction (from time_core::Transaction)
    async fn notify_wallets_of_transaction(
        &self,
        bridge: &Arc<WsBridge>,
        tx: &time_core::Transaction,
    ) {
        // Extract addresses from transaction
        let input_addresses = Vec::new();
        let mut output_addresses = Vec::new();
        let mut total_amount = 0u64;

        // Get input addresses (would need UTXO lookup in real implementation)
        // For now, we'll use the output addresses
        for output in &tx.outputs {
            output_addresses.push(output.address.clone());
            total_amount += output.amount;
        }

        // Check if any monitored addresses are involved
        if let Some(address_monitor) = &self.address_monitor {
            let mut relevant_addresses = Vec::new();
            for addr in &output_addresses {
                if address_monitor.is_monitored_address(addr).await {
                    relevant_addresses.push(addr.clone());
                }
            }

            if !relevant_addresses.is_empty() {
                info!(
                    node = %self.node_id,
                    txid = %tx.txid,
                    addresses = ?relevant_addresses,
                    "Transaction involves monitored wallet addresses"
                );
            }
        }

        // Create notification
        let notification = TransactionNotification {
            txid: tx.txid.clone(),
            inputs: input_addresses,
            outputs: output_addresses,
            amount: total_amount,
            timestamp: tx.timestamp,
        };

        // Broadcast to subscribed clients
        bridge.broadcast_transaction(notification).await;

        info!(
            node = %self.node_id,
            txid = %tx.txid,
            outputs = %tx.outputs.len(),
            amount = %total_amount,
            "Notified WebSocket clients of new transaction"
        );
    }

    /// Notify wallet of consensus result
    async fn notify_wallet_consensus(&self, _bridge: &Arc<WsBridge>, txid: &str, approved: bool) {
        info!(
            node = %self.node_id,
            txid = %txid,
            approved = %approved,
            "Notifying wallet of consensus result"
        );

        // TODO: Send proper consensus notification to wallet
        // For now, we'll use the existing notification system
        // In a full implementation, add a ConsensusNotification type
    }

    /// Finalize a transaction after consensus approval
    async fn finalize_transaction(&self, txid: &str) -> Result<(), String> {
        info!(
            node = %self.node_id,
            txid = %txid,
            "Finalizing transaction after consensus approval"
        );

        // TODO: In a full implementation:
        // 1. Update UTXO states from Locked to Spent
        // 2. Create new UTXOs for outputs
        // 3. Broadcast finalization to other masternodes
        // 4. Store in blockchain

        Ok(())
    }

    /// Validate a transaction
    async fn validate_transaction(&self, tx: &time_core::Transaction) -> bool {
        // Basic validation checks
        info!(
            node = %self.node_id,
            txid = %tx.txid,
            "Validating transaction"
        );

        // 1. Check transaction has inputs and outputs
        if tx.inputs.is_empty() || tx.outputs.is_empty() {
            warn!(
                node = %self.node_id,
                txid = %tx.txid,
                "Transaction has no inputs or outputs"
            );
            return false;
        }

        // 2. Verify all input UTXOs exist and are unspent
        let mut total_input: u64 = 0;

        for input in &tx.inputs {
            let outpoint = time_core::OutPoint {
                txid: input.previous_output.txid.clone(),
                vout: input.previous_output.vout,
            };

            match self.utxo_manager.get_utxo_info(&outpoint).await {
                Some(utxo) => {
                    // UTXO exists
                    total_input += utxo.output.amount;

                    // Check if it's locked or spent
                    if utxo.state.is_locked_or_spent() {
                        if let Some(locked_txid) = utxo.state.txid() {
                            if locked_txid != tx.txid {
                                warn!(
                                    node = %self.node_id,
                                    txid = %tx.txid,
                                    outpoint = ?outpoint,
                                    locked_by = %locked_txid,
                                    "UTXO is locked/spent by another transaction"
                                );
                                return false;
                            }
                        }
                    }
                }
                None => {
                    warn!(
                        node = %self.node_id,
                        txid = %tx.txid,
                        outpoint = ?outpoint,
                        "UTXO does not exist (already spent or invalid)"
                    );
                    return false;
                }
            }
        }

        // 3. Check input amounts >= output amounts (must account for fees)
        let total_output: u64 = tx.outputs.iter().map(|o| o.amount).sum();

        if total_input < total_output {
            warn!(
                node = %self.node_id,
                txid = %tx.txid,
                input = %total_input,
                output = %total_output,
                "Transaction outputs exceed inputs"
            );
            return false;
        }

        let fee = total_input - total_output;
        info!(
            node = %self.node_id,
            txid = %tx.txid,
            input = %total_input,
            output = %total_output,
            fee = %fee,
            "Transaction validation PASSED"
        );

        true
    }

    /// Subscribe wallet xpub for UTXO tracking
    pub async fn subscribe_xpub(&self, xpub: String) -> Result<(), String> {
        info!(
            node = %self.node_id,
            xpub = %&xpub[..20],
            "Subscribing wallet xpub for UTXO tracking"
        );

        self.utxo_tracker.subscribe_xpub(xpub).await
    }

    /// Get UTXOs for a subscribed xpub
    pub async fn get_utxos_for_xpub(
        &self,
        xpub: &str,
    ) -> Result<Vec<crate::utxo_tracker::UtxoInfo>, String> {
        self.utxo_tracker.get_utxos_for_xpub(xpub).await
    }

    /// Process a new block and update UTXO tracker
    pub async fn process_block(&self, block: &time_core::Block) -> Result<(), String> {
        info!(
            node = %self.node_id,
            height = block.header.block_number,
            "Processing block for UTXO tracking"
        );

        self.utxo_tracker.process_block(block).await
    }

    /// Get UTXO tracker statistics
    pub async fn get_tracker_stats(&self) -> crate::utxo_tracker::UtxoStats {
        self.utxo_tracker.stats().await
    }

    /// Process mempool transaction in UTXO tracker
    async fn track_mempool_transaction(&self, tx: &time_core::Transaction) -> Result<(), String> {
        self.utxo_tracker.process_mempool_tx(tx).await
    }

    /// Start periodic mempool synchronization task
    pub fn start_mempool_sync_task(&self) {
        let peer_manager = self.peer_manager.clone();
        let node_id = self.node_id.clone();
        let mempool = self.mempool.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

            loop {
                interval.tick().await;

                info!(node = %node_id, "🔄 Starting mempool synchronization");

                // Save mempool to disk
                let mempool_path = format!("data/{}/mempool.json", node_id);
                let had_error = mempool.save_to_disk(&mempool_path).await.is_err();

                if !had_error {
                    let size = mempool.size().await;
                    if size > 0 {
                        debug!(node = %node_id, transactions = size, "Mempool saved to disk");
                    }
                } else {
                    warn!(node = %node_id, "Failed to save mempool to disk");
                }

                // Get connected peers
                let peers = peer_manager.get_connected_peers().await;

                if peers.is_empty() {
                    debug!(node = %node_id, "No connected peers for mempool sync");
                    continue;
                }

                // Request mempool from a random peer
                if let Some(peer) = peers.first() {
                    let peer_addr = peer.address.to_string();
                    info!(
                        node = %node_id,
                        peer = %peer_addr,
                        "Requesting mempool from peer"
                    );

                    // Send mempool query
                    peer_manager
                        .broadcast_message(time_network::protocol::NetworkMessage::MempoolQuery)
                        .await;
                }
            }
        });

        info!(node = %self.node_id, "✅ Mempool sync task started");
    }

    /// Start background task to retry instant finality for unfinalized transactions
    /// Checks every 15 seconds and retries voting for transactions that haven't reached consensus
    pub fn start_finality_retry_task(&self) {
        let node_id = self.node_id.clone();
        let mempool = self.mempool.clone();
        let peer_manager = self.peer_manager.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(15));

            loop {
                interval.tick().await;

                // Get all unfinalized transactions
                let unfinalized = mempool.get_unfinalized_transactions().await;

                if unfinalized.is_empty() {
                    continue;
                }

                info!(
                    node = %node_id,
                    count = unfinalized.len(),
                    "🔄 Retrying instant finality for unfinalized transactions"
                );

                for tx in unfinalized {
                    let txid = tx.txid.clone();

                    // Check if we have enough connected peers
                    let peers = peer_manager.get_connected_peers().await;
                    if peers.len() < 2 {
                        debug!(
                            node = %node_id,
                            txid = %txid,
                            peers = peers.len(),
                            "Not enough peers connected for voting, skipping retry"
                        );
                        continue;
                    }

                    info!(
                        node = %node_id,
                        txid = %txid,
                        "♻️  Retrying instant finality vote"
                    );

                    // Broadcast instant finality request to all peers
                    let request =
                        time_network::protocol::NetworkMessage::InstantFinalityRequest(tx);
                    peer_manager.broadcast_message(request).await;
                }
            }
        });

        info!(node = %self.node_id, "✅ Finality retry task started");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time_network::discovery::NetworkType;

    #[tokio::test]
    async fn test_integration_creation() {
        let utxo_manager = Arc::new(UTXOStateManager::new("test-node".to_string()));
        let peer_manager = Arc::new(PeerManager::new(
            NetworkType::Testnet,
            "127.0.0.1:24000".parse().unwrap(),
            "127.0.0.1:24000".parse().unwrap(),
        ));

        let integration =
            MasternodeUTXOIntegration::new("test-node".to_string(), utxo_manager, peer_manager);

        assert!(integration.initialize().await.is_ok());
        assert_eq!(integration.get_subscription_count().await, 0);
    }
}
