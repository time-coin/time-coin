//! TIME Coin In-Memory State Manager
//! 
//! Manages current day state (cleared every 24 hours)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Timelike};

pub type Address = String;
pub type TxHash = String;

/// Current day in-memory state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyState {
    /// When this day started (00:00 UTC)
    pub day_start: DateTime<Utc>,
    
    /// Current block height (day number since genesis)
    pub current_height: u64,
    
    /// All transactions received today
    pub transactions: Vec<Transaction>,
    
    /// Current account balances (UTXO-style)
    pub balances: HashMap<Address, u64>,
    
    /// Active masternodes
    pub masternodes: HashMap<Address, MasternodeInfo>,
    
    /// Pending transactions (mempool)
    pub mempool: Vec<Transaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub txid: TxHash,
    pub from: Address,
    pub to: Address,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: i64,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasternodeInfo {
    pub address: Address,
    pub collateral: u64,
    pub tier: String,
    pub active_since: i64,
    pub last_seen: i64,
}

impl DailyState {
    /// Create new state for today
    pub fn new(height: u64) -> Self {
        let now = Utc::now();
        let day_start = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        
        Self {
            day_start,
            current_height: height,
            transactions: Vec::new(),
            balances: HashMap::new(),
            masternodes: HashMap::new(),
            mempool: Vec::new(),
        }
    }
    
    /// Check if we should finalize (past midnight UTC)
    pub fn should_finalize(&self) -> bool {
        let now = Utc::now();
        now.date_naive() > self.day_start.date_naive()
    }
    
    /// Get time until finalization
    pub fn seconds_until_finalize(&self) -> i64 {
        let next_midnight = self.day_start + chrono::Duration::days(1);
        (next_midnight - Utc::now()).num_seconds()
    }
    
    /// Add transaction to mempool
    pub fn add_to_mempool(&mut self, tx: Transaction) {
        self.mempool.push(tx);
    }
    
    /// Process transaction (move from mempool to confirmed)
    pub fn confirm_transaction(&mut self, txid: &str) -> Option<Transaction> {
        if let Some(pos) = self.mempool.iter().position(|tx| tx.txid == txid) {
            let tx = self.mempool.remove(pos);
            
            // Update balances
            if let Some(balance) = self.balances.get_mut(&tx.from) {
                *balance = balance.saturating_sub(tx.amount + tx.fee);
            }
            
            *self.balances.entry(tx.to.clone()).or_insert(0) += tx.amount;
            
            self.transactions.push(tx.clone());
            Some(tx)
        } else {
            None
        }
    }
    
    /// Get balance
    pub fn get_balance(&self, address: &Address) -> u64 {
        *self.balances.get(address).unwrap_or(&0)
    }
    
    /// Set balance (for genesis/rewards)
    pub fn set_balance(&mut self, address: Address, amount: u64) {
        self.balances.insert(address, amount);
    }
    
    /// Create state snapshot for block
    pub fn create_snapshot(&self) -> StateSnapshot {
        StateSnapshot {
            balances: self.balances.clone(),
            transaction_count: self.transactions.len() as u64,
            total_fees: self.transactions.iter().map(|tx| tx.fee).sum(),
        }
    }
}

/// Snapshot of state (saved in block)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub balances: HashMap<Address, u64>,
    pub transaction_count: u64,
    pub total_fees: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_daily_state() {
        let mut state = DailyState::new(1);
        
        state.set_balance("addr1".to_string(), 1000);
        assert_eq!(state.get_balance(&"addr1".to_string()), 1000);
        
        let tx = Transaction {
            txid: "tx1".to_string(),
            from: "addr1".to_string(),
            to: "addr2".to_string(),
            amount: 100,
            fee: 1,
            timestamp: Utc::now().timestamp(),
            signature: vec![],
        };
        
        state.add_to_mempool(tx);
        assert_eq!(state.mempool.len(), 1);
        
        state.confirm_transaction("tx1");
        assert_eq!(state.mempool.len(), 0);
        assert_eq!(state.transactions.len(), 1);
        assert_eq!(state.get_balance(&"addr1".to_string()), 899);
        assert_eq!(state.get_balance(&"addr2".to_string()), 100);
    }
}
