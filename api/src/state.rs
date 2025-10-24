//! API State Management
use std::sync::Arc;
use time_network::PeerManager;
use time_network::discovery::PeerDiscovery;
use tokio::sync::RwLock;

use std::collections::HashMap;
#[derive(Clone)]
pub struct ApiState {
    pub balances: Arc<RwLock<HashMap<String, u64>>>,
    pub transactions: Arc<RwLock<HashMap<String, TransactionData>>>,
    pub grants: Arc<RwLock<Vec<GrantData>>>,
    pub start_time: std::time::Instant,
    pub dev_mode: bool,
    pub network: String,
    pub peer_discovery: Arc<RwLock<PeerDiscovery>>,
    pub peer_manager: Arc<PeerManager>,
    pub admin_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TransactionData {
    pub txid: String,
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: i64,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct GrantData {
    pub email: String,
    pub verification_token: String,
    pub verified: bool,
    pub status: String,
    pub grant_amount: u64,
    pub applied_at: i64,
    pub verified_at: Option<i64>,
    pub activated_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub masternode_address: Option<String>,
    pub public_key: Option<String>,
}

impl ApiState {
    pub fn new(dev_mode: bool, network: String, peer_discovery: Arc<RwLock<PeerDiscovery>>, peer_manager: Arc<PeerManager>, admin_token: Option<String>) -> Self {
        let mut balances = HashMap::new();

        // Initialize genesis balances (1M TIME)
        balances.insert(
            "TIME1treasury00000000000000000000000000".to_string(),
            50_000_000_000_000, // 500,000 TIME for grants
        );
        balances.insert(
            "TIME1development0000000000000000000000".to_string(),
            10_000_000_000_000, // 100,000 TIME
        );
        balances.insert(
            "TIME1operations0000000000000000000000".to_string(),
            10_000_000_000_000, // 100,000 TIME
        );
        balances.insert(
            "TIME1rewards000000000000000000000000000".to_string(),
            30_000_000_000_000, // 300,000 TIME
        );

        Self {
            balances: Arc::new(RwLock::new(balances)),
            transactions: Arc::new(RwLock::new(HashMap::new())),
            grants: Arc::new(RwLock::new(Vec::new())),
            start_time: std::time::Instant::now(),
            dev_mode,
            network,
            peer_discovery,
            admin_token,
            peer_manager,
        }
    }
}
