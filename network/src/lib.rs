//! TIME Coin Network Module
pub mod connection;
pub mod discovery;
pub mod manager;
pub mod protocol;
pub mod quarantine;
pub mod sync;
pub mod utxo_handler;

pub use connection::PeerConnection;
pub use connection::PeerListener;
pub use discovery::{DnsDiscovery, HttpDiscovery, NetworkType, PeerDiscovery, SeedNodes};
pub use manager::PeerManager;
pub use manager::Snapshot;
pub use protocol::{HandshakeMessage, ProtocolVersion, PROTOCOL_VERSION, VERSION};
pub use protocol::{NetworkMessage, TransactionMessage, TransactionValidation};
pub use quarantine::{
    PeerQuarantine, QuarantineConfig, QuarantineReason, QuarantineSeverity, QuarantineStats,
};

pub mod peer_exchange;

// Re-export PeerInfo from discovery
pub use discovery::PeerInfo;

// Re-export UTXO handler
pub use utxo_handler::UTXOProtocolHandler;

/// Transaction broadcasting functionality
pub mod tx_broadcast {
    use crate::manager::PeerManager;
    use std::sync::Arc;
    use time_core::Transaction;
    use time_mempool::Mempool;
    use tracing::debug;

    pub struct TransactionBroadcaster {
        #[allow(dead_code)]
        mempool: Arc<Mempool>,
        peer_manager: Arc<PeerManager>,
    }

    impl TransactionBroadcaster {
        pub fn new(mempool: Arc<Mempool>, peer_manager: Arc<PeerManager>) -> Self {
            Self {
                mempool,
                peer_manager,
            }
        }

        /// Broadcast a transaction to all peers via TCP
        pub async fn broadcast_transaction(&self, tx: Transaction) {
            let peers = self.peer_manager.get_connected_peers().await;

            println!(
                "📡 Broadcasting transaction {} to {} peers",
                &tx.txid[..16],
                peers.len()
            );

            let message = crate::protocol::NetworkMessage::TransactionBroadcast(tx.clone());

            for peer_info in peers {
                let peer_addr = peer_info.address;
                let msg_clone = message.clone();
                let manager = self.peer_manager.clone();

                tokio::spawn(async move {
                    match manager.send_message_to_peer(peer_addr, msg_clone).await {
                        Ok(_) => {
                            println!("   ✓ Sent to {}", peer_addr);
                        }
                        Err(e) => {
                            println!("   ✗ Failed to send to {}: {}", peer_addr, e);
                        }
                    }
                });
            }
        }

        /// Sync mempool with a peer via TCP
        pub async fn sync_mempool_from_peer(
            &self,
            peer_addr: &str,
        ) -> Result<Vec<Transaction>, String> {
            let addr: std::net::SocketAddr = peer_addr
                .parse()
                .map_err(|e| format!("Invalid peer address: {}", e))?;

            println!("🔄 Syncing mempool from {}...", peer_addr);

            // Send mempool query via TCP
            let query_msg = crate::protocol::NetworkMessage::MempoolQuery;
            self.peer_manager
                .send_message_to_peer(addr, query_msg)
                .await
                .map_err(|e| format!("Failed to send query: {}", e))?;

            // For now, return empty as we need a response mechanism
            // This will be implemented when we add request/response handling
            println!("   ⚠️  Mempool sync via TCP not yet fully implemented");
            Ok(vec![])
        }

        /// Broadcast transaction proposal via TCP
        pub async fn broadcast_tx_proposal(&self, proposal: serde_json::Value) {
            let peers = self.peer_manager.get_connected_peers().await;

            println!(
                "📡 Broadcasting transaction proposal to {} peers via TCP",
                peers.len()
            );

            let proposal_json = proposal.to_string();
            let message = crate::protocol::NetworkMessage::ConsensusTxProposal(proposal_json);

            for peer_info in peers {
                let peer_ip = peer_info.address.ip();
                let msg_clone = message.clone();
                let manager_clone = self.peer_manager.clone();

                tokio::spawn(async move {
                    if let Err(e) = manager_clone.send_to_peer_tcp(peer_ip, msg_clone).await {
                        debug!(peer = %peer_ip, error = %e, "Failed to send tx proposal via TCP");
                    }
                });
            }
        }

        /// Broadcast vote on transaction set via TCP
        pub async fn broadcast_tx_vote(&self, vote: serde_json::Value) {
            let peers = self.peer_manager.get_connected_peers().await;

            let vote_json = vote.to_string();
            let message = crate::protocol::NetworkMessage::ConsensusTxVote(vote_json);

            for peer_info in peers {
                let peer_ip = peer_info.address.ip();
                let msg_clone = message.clone();
                let manager_clone = self.peer_manager.clone();

                tokio::spawn(async move {
                    if let Err(e) = manager_clone.send_to_peer_tcp(peer_ip, msg_clone).await {
                        debug!(peer = %peer_ip, error = %e, "Failed to send tx vote via TCP");
                    }
                });
            }
        }

        /// Request instant finality votes from all peers via TCP
        pub async fn request_instant_finality_votes(&self, tx: Transaction) {
            let peers = self.peer_manager.get_connected_peers().await;

            println!(
                "📡 Requesting instant finality votes from {} peers",
                peers.len()
            );

            let message = crate::protocol::NetworkMessage::InstantFinalityRequest(tx.clone());

            for peer_info in peers {
                let peer_addr = peer_info.address;
                let msg_clone = message.clone();
                let manager = self.peer_manager.clone();

                tokio::spawn(async move {
                    match manager.send_message_to_peer(peer_addr, msg_clone).await {
                        Ok(_) => {
                            println!("   ✓ Vote request sent to {}", peer_addr);
                        }
                        Err(e) => {
                            println!("   ✗ Failed to send vote request to {}: {}", peer_addr, e);
                        }
                    }
                });
            }
        }

        /// Broadcast instant finality vote to all peers via TCP
        pub async fn broadcast_instant_finality_vote(&self, vote: serde_json::Value) {
            let peers = self.peer_manager.get_connected_peers().await;

            // Extract vote details from JSON
            let txid = vote
                .get("txid")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let voter = vote
                .get("voter")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let approve = vote
                .get("approve")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let timestamp = vote.get("timestamp").and_then(|v| v.as_u64()).unwrap_or(0);

            let message = crate::protocol::NetworkMessage::InstantFinalityVote {
                txid,
                voter,
                approve,
                timestamp,
            };

            for peer_info in peers {
                let peer_addr = peer_info.address;
                let msg_clone = message.clone();
                let manager = self.peer_manager.clone();

                tokio::spawn(async move {
                    let _ = manager.send_message_to_peer(peer_addr, msg_clone).await;
                });
            }
        }
    }
}
