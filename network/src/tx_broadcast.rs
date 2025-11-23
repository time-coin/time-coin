use crate::manager::PeerManager;
use crate::protocol::NetworkMessage;
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

    #[allow(dead_code)]
    async fn broadcast_to_peers<F>(&self, message: NetworkMessage, log_msg: &str, send_fn: F)
    where
        F: Fn(Arc<PeerManager>, std::net::IpAddr, NetworkMessage) -> tokio::task::JoinHandle<()>
            + Send
            + 'static,
    {
        let peers = self.peer_manager.get_connected_peers().await;
        println!("{} to {} peers", log_msg, peers.len());

        for peer_info in peers {
            let peer_ip = peer_info.address.ip();
            let msg_clone = message.clone();
            let manager_clone = self.peer_manager.clone();

            send_fn(manager_clone, peer_ip, msg_clone);
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

        let message = NetworkMessage::TransactionBroadcast(tx);

        for peer_info in peers {
            let peer_addr = peer_info.address;
            let msg_clone = message.clone();
            let manager = self.peer_manager.clone();

            tokio::spawn(async move {
                if let Err(e) = manager.send_message_to_peer(peer_addr, msg_clone).await {
                    debug!(peer = %peer_addr, error = %e, "Failed to send transaction");
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

        let query_msg = NetworkMessage::MempoolQuery;
        self.peer_manager
            .send_message_to_peer(addr, query_msg)
            .await
            .map_err(|e| format!("Failed to send query: {}", e))?;

        println!("   ⚠️  Mempool sync via TCP not yet fully implemented");
        Ok(vec![])
    }

    /// Broadcast transaction proposal via TCP
    pub async fn broadcast_tx_proposal(&self, proposal: serde_json::Value) {
        let peers = self.peer_manager.get_connected_peers().await;
        let proposal_json = proposal.to_string();
        let message = NetworkMessage::ConsensusTxProposal(proposal_json);

        println!(
            "📡 Broadcasting transaction proposal to {} peers",
            peers.len()
        );

        for peer_info in peers {
            let peer_ip = peer_info.address.ip();
            let msg_clone = message.clone();
            let manager_clone = self.peer_manager.clone();

            tokio::spawn(async move {
                if let Err(e) = manager_clone.send_to_peer_tcp(peer_ip, msg_clone).await {
                    debug!(peer = %peer_ip, error = %e, "Failed to send tx proposal");
                }
            });
        }
    }

    /// Broadcast vote on transaction set via TCP
    pub async fn broadcast_tx_vote(&self, vote: serde_json::Value) {
        let peers = self.peer_manager.get_connected_peers().await;
        let vote_json = vote.to_string();
        let message = NetworkMessage::ConsensusTxVote(vote_json);

        for peer_info in peers {
            let peer_ip = peer_info.address.ip();
            let msg_clone = message.clone();
            let manager_clone = self.peer_manager.clone();

            tokio::spawn(async move {
                if let Err(e) = manager_clone.send_to_peer_tcp(peer_ip, msg_clone).await {
                    debug!(peer = %peer_ip, error = %e, "Failed to send tx vote");
                }
            });
        }
    }

    /// Request instant finality votes from peers and collect responses
    pub async fn request_instant_finality_votes(
        &self,
        tx: Transaction,
        consensus: Arc<time_consensus::ConsensusEngine>,
    ) -> usize {
        let peers = self.peer_manager.get_connected_peers().await;

        println!(
            "📡 Requesting instant finality votes from {} peers",
            peers.len()
        );

        let message = NetworkMessage::InstantFinalityRequest(tx.clone());
        let mut vote_tasks = Vec::new();

        for peer_info in peers {
            let peer_addr = peer_info.address;
            let msg_clone = message.clone();
            let manager = self.peer_manager.clone();
            let consensus_clone = consensus.clone();
            let txid = tx.txid.clone();

            let task = tokio::spawn(async move {
                if let Ok(Some(NetworkMessage::InstantFinalityVote {
                    txid: vote_txid,
                    voter,
                    approve,
                    timestamp: _,
                })) = manager
                    .send_message_to_peer_with_response(peer_addr, msg_clone, 3)
                    .await
                {
                    if vote_txid == txid {
                        let _ = consensus_clone
                            .vote_on_transaction(&txid, voter.clone(), approve)
                            .await;
                        return Some(approve);
                    }
                }
                None
            });

            vote_tasks.push(task);
        }

        let mut votes_received = 0;
        for task in vote_tasks {
            if let Ok(Some(_)) = task.await {
                votes_received += 1;
            }
        }

        println!("   📊 Collected {} votes from peers", votes_received);
        votes_received
    }

    /// Broadcast instant finality vote to all peers via TCP
    pub async fn broadcast_instant_finality_vote(&self, vote: serde_json::Value) {
        let peers = self.peer_manager.get_connected_peers().await;

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

        let message = NetworkMessage::InstantFinalityVote {
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
