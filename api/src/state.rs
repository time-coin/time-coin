use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use time_core::state::BlockchainState;
use time_network::{PeerDiscovery, PeerManager, PeerQuarantine};
use tokio::sync::RwLock;

use crate::{ApiError, ApiResult};

pub struct ApiState {
    pub dev_mode: bool,
    pub network: String,
    pub discovery: Arc<RwLock<PeerDiscovery>>,
    pub peer_manager: Arc<PeerManager>,
    pub admin_token: Option<String>,
    pub blockchain: Arc<RwLock<BlockchainState>>,
    pub wallet_address: String,
    pub consensus: Arc<time_consensus::ConsensusEngine>,
    pub balances: Arc<RwLock<HashMap<String, u64>>>,
    pub mempool: Option<Arc<time_mempool::Mempool>>,
    pub tx_consensus: Option<Arc<time_consensus::tx_consensus::TxConsensusManager>>,
    pub block_consensus: Option<Arc<time_consensus::block_consensus::BlockConsensusManager>>,
    pub tx_broadcaster: Option<Arc<time_network::tx_broadcast::TransactionBroadcaster>>,
    /// Track recently processed peer broadcasts to prevent duplicates
    pub recent_broadcasts: Arc<RwLock<HashMap<String, Instant>>>,
    /// Peer quarantine system
    pub quarantine: Option<Arc<PeerQuarantine>>,
}

impl ApiState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        dev_mode: bool,
        network: String,
        discovery: Arc<RwLock<PeerDiscovery>>,
        peer_manager: Arc<PeerManager>,
        admin_token: Option<String>,
        blockchain: Arc<RwLock<BlockchainState>>,
        wallet_address: String,
        consensus: Arc<time_consensus::ConsensusEngine>,
    ) -> Self {
        let state = Self {
            dev_mode,
            network,
            discovery,
            peer_manager,
            admin_token,
            blockchain,
            wallet_address,
            consensus,
            balances: Arc::new(RwLock::new(HashMap::new())),
            mempool: None,
            tx_consensus: None,
            block_consensus: None,
            tx_broadcaster: None,
            recent_broadcasts: Arc::new(RwLock::new(HashMap::new())),
            quarantine: None,
        };

        // Spawn cleanup task for recent_broadcasts
        let recent_broadcasts_clone = state.recent_broadcasts.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                let mut broadcasts = recent_broadcasts_clone.write().await;
                let now = Instant::now();
                broadcasts.retain(|_, &mut last_seen| {
                    now.duration_since(last_seen) < std::time::Duration::from_secs(300)
                    // 5 minutes
                });
            }
        });

        state
    }

    pub fn with_mempool(mut self, mempool: Arc<time_mempool::Mempool>) -> Self {
        self.mempool = Some(mempool);
        self
    }

    pub fn with_tx_consensus(
        mut self,
        tx_consensus: Arc<time_consensus::tx_consensus::TxConsensusManager>,
    ) -> Self {
        self.tx_consensus = Some(tx_consensus);
        self
    }

    pub fn with_block_consensus(
        mut self,
        block_consensus: Arc<time_consensus::block_consensus::BlockConsensusManager>,
    ) -> Self {
        self.block_consensus = Some(block_consensus);
        self
    }

    pub fn with_tx_broadcaster(
        mut self,
        tx_broadcaster: Arc<time_network::tx_broadcast::TransactionBroadcaster>,
    ) -> Self {
        self.tx_broadcaster = Some(tx_broadcaster);
        self
    }

    pub fn with_quarantine(mut self, quarantine: Arc<PeerQuarantine>) -> Self {
        self.quarantine = Some(quarantine);
        self
    }

    pub fn require_admin(&self, token: Option<String>) -> ApiResult<()> {
        if let Some(expected) = &self.admin_token {
            if let Some(provided) = token {
                if &provided == expected {
                    return Ok(());
                }
            }
            return Err(ApiError::Unauthorized(
                "Invalid or missing admin token".to_string(),
            ));
        }
        Ok(())
    }
}

impl Clone for ApiState {
    fn clone(&self) -> Self {
        Self {
            dev_mode: self.dev_mode,
            network: self.network.clone(),
            discovery: self.discovery.clone(),
            peer_manager: self.peer_manager.clone(),
            admin_token: self.admin_token.clone(),
            blockchain: self.blockchain.clone(),
            wallet_address: self.wallet_address.clone(),
            consensus: self.consensus.clone(),
            balances: self.balances.clone(),
            mempool: self.mempool.clone(),
            tx_consensus: self.tx_consensus.clone(),
            block_consensus: self.block_consensus.clone(),
            tx_broadcaster: self.tx_broadcaster.clone(),
            recent_broadcasts: self.recent_broadcasts.clone(),
            quarantine: self.quarantine.clone(),
        }
    }
}
