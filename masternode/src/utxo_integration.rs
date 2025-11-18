//! UTXO Protocol Integration for Masternodes
//!
//! Integrates the UTXO state protocol with the P2P network layer,
//! enabling instant finality through masternode consensus.

use crate::{TransactionNotification, WsBridge};
use std::net::IpAddr;
use std::sync::Arc;
use time_consensus::utxo_state_protocol::{UTXOStateManager, UTXOStateNotification};
use time_network::{PeerManager, UTXOProtocolHandler};
use tracing::{debug, info, warn};

/// Masternode UTXO Protocol Integration
///
/// Coordinates between:
/// - UTXO State Protocol (consensus layer)
/// - P2P Network (communication layer)
/// - Instant Finality (voting layer)
/// - WebSocket clients (wallet notifications)
pub struct MasternodeUTXOIntegration {
    /// UTXO state manager
    utxo_manager: Arc<UTXOStateManager>,
    /// P2P network message handler
    utxo_handler: Arc<UTXOProtocolHandler>,
    /// P2P peer manager
    peer_manager: Arc<PeerManager>,
    /// WebSocket bridge for wallet notifications
    ws_bridge: Option<Arc<WsBridge>>,
    /// Node identifier
    node_id: String,
}

impl MasternodeUTXOIntegration {
    /// Create a new masternode UTXO integration
    pub fn new(
        node_id: String,
        utxo_manager: Arc<UTXOStateManager>,
        peer_manager: Arc<PeerManager>,
    ) -> Self {
        let utxo_handler = Arc::new(UTXOProtocolHandler::new(utxo_manager.clone()));

        Self {
            utxo_manager,
            utxo_handler,
            peer_manager,
            ws_bridge: None,
            node_id,
        }
    }

    /// Set WebSocket bridge for wallet notifications
    pub fn set_ws_bridge(&mut self, ws_bridge: Arc<WsBridge>) {
        self.ws_bridge = Some(ws_bridge);
        info!(node = %self.node_id, "WebSocket bridge connected for transaction notifications");
    }

    /// Initialize the integration - sets up notification handlers
    pub async fn initialize(&self) -> Result<(), String> {
        info!(node = %self.node_id, "Initializing UTXO protocol integration");

        // Set up notification handler to route UTXO state changes to P2P network
        let peer_manager = self.peer_manager.clone();
        let node_id = self.node_id.clone();

        self.utxo_handler
            .setup_notification_handler(move |peer_ip: IpAddr, message| {
                let peer_manager = peer_manager.clone();
                let node_id = node_id.clone();
                Box::pin(async move {
                    if let Err(e) = peer_manager
                        .send_message_to_peer(std::net::SocketAddr::new(peer_ip, 24000), message)
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
            | time_network::protocol::NetworkMessage::UTXOUnsubscribe { .. }
            | time_network::protocol::NetworkMessage::TransactionBroadcast(_) => {
                // Handle via UTXO protocol handler
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

                // Return vote as response
                Ok(Some(vote))
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
