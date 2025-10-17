//! BFT Consensus implementation for TIME Coin

use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct ConsensusEngine {
    dev_mode: bool,
    validators: Arc<RwLock<Vec<String>>>,
}

impl ConsensusEngine {
    pub fn new(dev_mode: bool) -> Self {
        Self {
            dev_mode,
            validators: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub async fn add_validator(&self, address: String) {
        let mut validators = self.validators.write().await;
        validators.push(address);
    }
    
    pub fn is_dev_mode(&self) -> bool {
        self.dev_mode
    }
    
    pub async fn validate_transaction(&self, _tx: &time_core::Transaction) -> bool {
        if self.dev_mode {
            // In dev mode, auto-approve all transactions
            true
        } else {
            // TODO: Implement proper BFT consensus
            // For now, require 2/3+ validator approval
            false
        }
    }
}
