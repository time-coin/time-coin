//! API State Management

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// Shared API state
#[derive(Clone)]
pub struct ApiState {
    /// In-memory balances (for dev mode)
    pub balances: Arc<RwLock<HashMap<String, u64>>>,
    
    /// Transaction pool
    pub transactions: Arc<RwLock<HashMap<String, TransactionData>>>,
    
    /// Node start time
    pub start_time: std::time::Instant,
    
    /// Dev mode enabled
    pub dev_mode: bool,
    
    /// Network type
    pub network: String,
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

impl ApiState {
    pub fn new(dev_mode: bool, network: String) -> Self {
        let mut balances = HashMap::new();
        
        // Initialize genesis balances
        balances.insert(
            "TIME1treasury00000000000000000000000000".to_string(),
            10_000_000_000_000, // 100,000 TIME
        );
        balances.insert(
            "TIME1development0000000000000000000000".to_string(),
            5_000_000_000_000, // 50,000 TIME
        );
        balances.insert(
            "TIME1masternode00000000000000000000000".to_string(),
            85_000_000_000_000, // 850,000 TIME
        );
        
        Self {
            balances: Arc::new(RwLock::new(balances)),
            transactions: Arc::new(RwLock::new(HashMap::new())),
            start_time: std::time::Instant::now(),
            dev_mode,
            network,
        }
    }
}
