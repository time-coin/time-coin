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

    /// Broadcast a finalized transaction to all peers via HTTP POST
    /// This ensures all masternodes receive and apply the finalized transaction to their UTXO sets
    pub async fn broadcast_finalized_transaction(&self, tx: Transaction) {
        let peers = self.peer_manager.get_connected_peers().await;
        println!(
            "📡 Broadcasting FINALIZED transaction {} to {} peers",
            &tx.txid[..16],
            peers.len()
        );

        let mut send_tasks = Vec::new();

        // Launch all HTTP POSTs in parallel
        for peer_info in &peers {
            let peer_addr = peer_info.address;
            let tx_clone = tx.clone();

            let task = tokio::spawn(async move {
                // Construct HTTP endpoint
                let url = format!("http://{}:{}/mempool/finalized", peer_addr.ip(), 24101);
                
                // 5 second timeout per peer
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(5))
                    .build()
                    .unwrap();

                match client.post(&url).json(&tx_clone).send().await {
                    Ok(response) if response.status().is_success() => {
                        debug!(peer = %peer_addr, "Finalized transaction broadcast success");
                        true
                    }
                    Ok(response) => {
                        debug!(
                            peer = %peer_addr,
                            status = %response.status(),
                            "Failed to broadcast finalized transaction"
                        );
                        false
                    }
                    Err(e) => {
                        debug!(peer = %peer_addr, error = %e, "Failed to send finalized transaction");
                        false
                    }
                }
            });
            send_tasks.push(task);
        }

        // Wait for all sends to complete
        let results = futures::future::join_all(send_tasks).await;
        let success_count = results
            .into_iter()
            .filter(|r| matches!(r, Ok(true)))
            .count();
        println!(
            "✅ Finalized transaction broadcast: {}/{} peers successful",
            success_count,
            peers.len()
        );
    }

    /// Broadcast a transaction to all peers via TCP - OPTIMIZED PARALLEL
    pub async fn broadcast_transaction(&self, tx: Transaction) {
        let peers = self.peer_manager.get_connected_peers().await;
        println!(
            "📡 Broadcasting transaction {} to {} peers",
            &tx.txid[..16],
            peers.len()
        );

        let message = NetworkMessage::TransactionBroadcast(tx);
        let mut send_tasks = Vec::new();

        // Launch all sends in parallel with timeout per send
        for peer_info in peers {
            let peer_addr = peer_info.address;
            let msg_clone = message.clone();
            let manager = self.peer_manager.clone();

            let task = tokio::spawn(async move {
                // 2 second timeout per peer to prevent blocking on slow peers
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(2),
                    manager.send_message_to_peer(peer_addr, msg_clone),
                )
                .await
                {
                    Ok(Ok(_)) => true,
                    Ok(Err(e)) => {
                        debug!(peer = %peer_addr, error = %e, "Failed to send transaction");
                        false
                    }
                    Err(_) => {
                        debug!(peer = %peer_addr, "Timeout sending transaction");
                        false
                    }
                }
            });
            send_tasks.push(task);
        }

        // Wait for all sends to complete (or timeout)
        let results = futures::future::join_all(send_tasks).await;
        let success_count = results
            .into_iter()
            .filter(|r| matches!(r, Ok(true)))
            .count();
        debug!(success = success_count, "Transaction broadcast completed");
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

    /// Broadcast transaction proposal via TCP - OPTIMIZED PARALLEL
    pub async fn broadcast_tx_proposal(&self, proposal: serde_json::Value) {
        let peers = self.peer_manager.get_connected_peers().await;
        let proposal_json = proposal.to_string();
        let message = NetworkMessage::ConsensusTxProposal(proposal_json);

        println!(
            "📡 Broadcasting transaction proposal to {} peers",
            peers.len()
        );

        let mut send_tasks = Vec::new();

        for peer_info in peers {
            let peer_ip = peer_info.address.ip();
            let msg_clone = message.clone();
            let manager_clone = self.peer_manager.clone();

            let task = tokio::spawn(async move {
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(2),
                    manager_clone.send_to_peer_tcp(peer_ip, msg_clone),
                )
                .await
                {
                    Ok(Ok(_)) => true,
                    Ok(Err(e)) => {
                        debug!(peer = %peer_ip, error = %e, "Failed to send tx proposal");
                        false
                    }
                    Err(_) => {
                        debug!(peer = %peer_ip, "Timeout sending tx proposal");
                        false
                    }
                }
            });
            send_tasks.push(task);
        }

        let results = futures::future::join_all(send_tasks).await;
        let success_count = results
            .into_iter()
            .filter(|r| matches!(r, Ok(true)))
            .count();
        debug!(success = success_count, "TX proposal broadcast completed");
    }

    /// Broadcast vote on transaction set via TCP - OPTIMIZED PARALLEL
    pub async fn broadcast_tx_vote(&self, vote: serde_json::Value) {
        let peers = self.peer_manager.get_connected_peers().await;
        let vote_json = vote.to_string();
        let message = NetworkMessage::ConsensusTxVote(vote_json);

        let mut send_tasks = Vec::new();

        for peer_info in peers {
            let peer_ip = peer_info.address.ip();
            let msg_clone = message.clone();
            let manager_clone = self.peer_manager.clone();

            let task = tokio::spawn(async move {
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(2),
                    manager_clone.send_to_peer_tcp(peer_ip, msg_clone),
                )
                .await
                {
                    Ok(Ok(_)) => true,
                    Ok(Err(e)) => {
                        debug!(peer = %peer_ip, error = %e, "Failed to send tx vote");
                        false
                    }
                    Err(_) => {
                        debug!(peer = %peer_ip, "Timeout sending tx vote");
                        false
                    }
                }
            });
            send_tasks.push(task);
        }

        let results = futures::future::join_all(send_tasks).await;
        let success_count = results
            .into_iter()
            .filter(|r| matches!(r, Ok(true)))
            .count();
        debug!(success = success_count, "TX vote broadcast completed");
    }

    /// Request instant finality votes from peers - ULTRA-FAST PARALLEL
    pub async fn request_instant_finality_votes(
        &self,
        tx: Transaction,
        consensus: Arc<time_consensus::ConsensusEngine>,
    ) -> usize {
        let peers = self.peer_manager.get_connected_peers().await;

        println!(
            "📡 ⚡ ULTRA-FAST parallel vote requests to {} peers",
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
                // OPTIMIZED: 1 second timeout (reduced from 3s) for instant finality
                if let Ok(Some(NetworkMessage::InstantFinalityVote {
                    txid: vote_txid,
                    voter,
                    approve,
                    timestamp: _,
                })) = manager
                    .send_message_to_peer_with_response(peer_addr, msg_clone, 1) // 1s timeout!
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

        // Wait for ALL votes in parallel
        let results = futures::future::join_all(vote_tasks).await;
        let votes_received = results.into_iter().filter_map(|r| r.ok().flatten()).count();

        println!("   ⚡ Collected {} votes in <1s", votes_received);
        votes_received
    }

    /// Broadcast instant finality vote to all peers via TCP - OPTIMIZED PARALLEL
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

        let mut send_tasks = Vec::new();

        for peer_info in peers {
            let peer_addr = peer_info.address;
            let msg_clone = message.clone();
            let manager = self.peer_manager.clone();

            let task = tokio::spawn(async move {
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(1),
                    manager.send_message_to_peer(peer_addr, msg_clone),
                )
                .await
                {
                    Ok(Ok(_)) => true,
                    Ok(Err(_)) => false,
                    Err(_) => false,
                }
            });
            send_tasks.push(task);
        }

        let results = futures::future::join_all(send_tasks).await;
        let success_count = results
            .into_iter()
            .filter(|r| matches!(r, Ok(true)))
            .count();
        debug!(
            success = success_count,
            "Instant finality vote broadcast completed"
        );
    }
}
