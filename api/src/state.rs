use std::collections::HashMap;
use std::sync::Arc;
use time_core::state::BlockchainState;
use time_network::{PeerDiscovery, PeerManager};
use tokio::sync::RwLock;

use crate::{ApiError, ApiResult};

pub struct ApiState {
    pub dev_mode: bool,
    pub network: String,
    pub discovery: Arc<RwLock<PeerDiscovery>>,
    pub peer_manager: Arc<PeerManager>,
    pub admin_token: Option<String>,
    pub blockchain: Arc<RwLock<BlockchainState>>,
    pub consensus: Arc<time_consensus::ConsensusEngine>,
    pub balances: Arc<RwLock<HashMap<String, u64>>>,
    pub mempool: Option<Arc<time_mempool::Mempool>>,
    pub tx_consensus: Option<Arc<time_consensus::tx_consensus::TxConsensusManager>>,
    pub block_consensus: Option<Arc<time_consensus::block_consensus::BlockConsensusManager>>,
    pub tx_broadcaster: Option<Arc<time_network::tx_broadcast::TransactionBroadcaster>>,
}

impl ApiState {
    pub fn new(
        dev_mode: bool,
        network: String,
        discovery: Arc<RwLock<PeerDiscovery>>,
        peer_manager: Arc<PeerManager>,
        admin_token: Option<String>,
        blockchain: Arc<RwLock<BlockchainState>>,
        consensus: Arc<time_consensus::ConsensusEngine>,
    ) -> Self {
        Self {
            dev_mode,
            network,
            discovery,
            peer_manager,
            admin_token,
            blockchain,
            consensus,
            balances: Arc::new(RwLock::new(HashMap::new())),
            mempool: None,
            tx_consensus: None,
            block_consensus: None,
            tx_broadcaster: None,
        }
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
            consensus: self.consensus.clone(),
            balances: self.balances.clone(),
            mempool: self.mempool.clone(),
            tx_consensus: self.tx_consensus.clone(),
            block_consensus: self.block_consensus.clone(),
            tx_broadcaster: self.tx_broadcaster.clone(),
        }
    }
}
