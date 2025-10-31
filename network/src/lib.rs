//! TIME Coin Network Module
pub mod discovery;
pub mod protocol;
pub mod sync;
pub mod connection;
pub mod manager;

pub use discovery::{DnsDiscovery, HttpDiscovery, NetworkType, PeerDiscovery, PeerInfo, SeedNodes};
pub use protocol::{HandshakeMessage, ProtocolVersion, VERSION, PROTOCOL_VERSION};
pub use protocol::{TransactionMessage, TransactionValidation, NetworkMessage};
pub use connection::PeerConnection;
pub use manager::PeerManager;
pub use connection::PeerListener;
pub use manager::Snapshot;

pub mod peer_exchange;

/// Transaction broadcasting functionality
pub mod tx_broadcast {
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use time_core::Transaction;
    use time_mempool::Mempool;
    use reqwest;

    pub struct TransactionBroadcaster {
        mempool: Arc<Mempool>,
        peer_ips: Arc<RwLock<Vec<String>>>,
    }

    impl TransactionBroadcaster {
        pub fn new(mempool: Arc<Mempool>) -> Self {
            Self {
                mempool,
                peer_ips: Arc::new(RwLock::new(Vec::new())),
            }
        }

        /// Update peer list
        pub async fn update_peers(&self, peers: Vec<String>) {
            let mut peer_ips = self.peer_ips.write().await;
            *peer_ips = peers;
        }

        /// Broadcast a transaction to all peers
        pub async fn broadcast_transaction(&self, tx: Transaction) {
            let peers = self.peer_ips.read().await.clone();
            
            println!("📡 Broadcasting transaction {} to {} peers", 
                &tx.txid[..16], peers.len());

            for peer in peers {
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    let client = reqwest::Client::new();
                    let url = format!("http://{}:24101/mempool/add", peer);
                    
                    match client.post(&url)
                        .json(&tx_clone)
                        .timeout(std::time::Duration::from_secs(5))
                        .send()
                        .await
                    {
                        Ok(_) => {
                            println!("   ✓ Sent to {}", peer);
                        }
                        Err(e) => {
                            println!("   ✗ Failed to send to {}: {}", peer, e);
                        }
                    }
                });
            }
        }

        /// Sync mempool with a peer (on startup or reconnection)
        pub async fn sync_mempool_from_peer(&self, peer: &str) -> Result<Vec<Transaction>, String> {
            let client = reqwest::Client::new();
            let url = format!("http://{}:24101/mempool/all", peer);
            
            println!("🔄 Syncing mempool from {}...", peer);
            
            match client.get(&url)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
            {
                Ok(response) => {
                    match response.json::<Vec<Transaction>>().await {
                        Ok(transactions) => {
                            println!("   ✓ Received {} transactions", transactions.len());
                            
                            // Add to local mempool
                            for tx in &transactions {
                                let _ = self.mempool.add_transaction(tx.clone()).await;
                            }
                            
                            Ok(transactions)
                        }
                        Err(e) => Err(format!("Failed to parse response: {}", e))
                    }
                }
                Err(e) => Err(format!("Request failed: {}", e))
            }
        }

        /// Broadcast transaction proposal (which transactions should go in block)
        pub async fn broadcast_tx_proposal(&self, proposal: serde_json::Value) {
            let peers = self.peer_ips.read().await.clone();
            
            println!("📡 Broadcasting transaction proposal to {} peers", peers.len());

            for peer in peers {
                let proposal_clone = proposal.clone();
                tokio::spawn(async move {
                    let client = reqwest::Client::new();
                    let url = format!("http://{}:24101/consensus/tx-proposal", peer);
                    
                    let _ = client.post(&url)
                        .json(&proposal_clone)
                        .timeout(std::time::Duration::from_secs(5))
                        .send()
                        .await;
                });
            }
        }

        /// Broadcast vote on transaction set
        pub async fn broadcast_tx_vote(&self, vote: serde_json::Value) {
            let peers = self.peer_ips.read().await.clone();

            for peer in peers {
                let vote_clone = vote.clone();
                tokio::spawn(async move {
                    let client = reqwest::Client::new();
                    let url = format!("http://{}:24101/consensus/tx-vote", peer);
                    
                    let _ = client.post(&url)
                        .json(&vote_clone)
                        .timeout(std::time::Duration::from_secs(5))
                        .send()
                        .await;
                });
            }
        }
    }
}
