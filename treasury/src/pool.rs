//! Treasury Pool Management
//!
//! Manages the TIME Coin community treasury that receives:
//! - 50% of all transaction fees
//! - 5 TIME from each block reward (95 TIME goes to masternodes)
//!
//! Funds are distributed through approved governance proposals with
//! milestone-based payments and complete audit trails.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Import error types
use crate::error::{Result, TreasuryError};

/// TIME token unit (8 decimal places)
pub const TIME_UNIT: u64 = 100_000_000;

/// Percentage of transaction fees going to treasury (50%)
pub const TREASURY_FEE_PERCENTAGE: u64 = 50;

/// Treasury portion of block reward (5 TIME)
pub const TREASURY_BLOCK_REWARD: u64 = 5 * TIME_UNIT;

/// Masternode portion of block reward (95 TIME)
pub const MASTERNODE_BLOCK_REWARD: u64 = 95 * TIME_UNIT;

/// Source of treasury funds
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TreasurySource {
    /// Transaction fee (50% of total fee)
    TransactionFee { tx_id: String, fee: u64 },

    /// Block reward (5 TIME per block)
    BlockReward { block_number: u64 },

    /// Community donation
    Donation { donor: String },

    /// Recovered funds (from failed proposals, penalties, etc.)
    Recovered { reason: String },
}

/// Scheduled or executed withdrawal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryWithdrawal {
    pub id: String,
    pub proposal_id: String,
    pub milestone_id: Option<String>,
    pub amount: u64,
    pub recipient: String,
    pub scheduled_time: u64,
    pub executed_time: Option<u64>,
    pub status: WithdrawalStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WithdrawalStatus {
    Scheduled,
    Executed,
    Cancelled,
    Failed,
}

/// Complete transaction record for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryTransaction {
    pub id: String,
    pub timestamp: u64,
    pub transaction_type: TransactionType,
    pub amount: u64,
    pub balance_after: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    Deposit(TreasurySource),
    Withdrawal {
        proposal_id: String,
        recipient: String,
    },
}

/// Treasury statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryStats {
    pub total_deposits: u64,
    pub total_withdrawals: u64,
    pub transaction_count: usize,
    pub active_proposals: usize,
    pub deposits_by_source: HashMap<String, u64>,
}

/// Financial report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryReport {
    pub period_start: u64,
    pub period_end: u64,
    pub opening_balance: u64,
    pub closing_balance: u64,
    pub total_deposits: u64,
    pub total_withdrawals: u64,
    pub transaction_count: usize,
    pub largest_deposit: u64,
    pub largest_withdrawal: u64,
}

/// Main treasury pool manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryPool {
    /// Current balance in smallest units (satoshis)
    balance: u64,

    /// Complete transaction history
    transactions: Vec<TreasuryTransaction>,

    /// Scheduled withdrawals
    scheduled_withdrawals: HashMap<String, TreasuryWithdrawal>,

    /// Statistics tracking
    stats: TreasuryStats,

    /// Next transaction ID
    next_tx_id: u64,
}

impl TreasuryPool {
    /// Create a new empty treasury pool
    pub fn new() -> Self {
        Self {
            balance: 0,
            transactions: Vec::new(),
            scheduled_withdrawals: HashMap::new(),
            stats: TreasuryStats {
                total_deposits: 0,
                total_withdrawals: 0,
                transaction_count: 0,
                active_proposals: 0,
                deposits_by_source: HashMap::new(),
            },
            next_tx_id: 1,
        }
    }

    /// Get current balance in smallest units
    pub fn balance(&self) -> u64 {
        self.balance
    }

    /// Get balance in TIME (with decimal places)
    pub fn balance_time(&self) -> f64 {
        self.balance as f64 / TIME_UNIT as f64
    }

    /// Deposit transaction fee (treasury receives 50%)
    pub fn deposit_transaction_fee(
        &mut self,
        tx_id: String,
        total_fee: u64,
        timestamp: u64,
    ) -> Result<u64> {
        if total_fee == 0 {
            return Err(TreasuryError::InvalidAmount(
                "Fee cannot be zero".to_string(),
            ));
        }

        // Calculate treasury portion (50%)
        let treasury_amount = (total_fee * TREASURY_FEE_PERCENTAGE) / 100;

        let source = TreasurySource::TransactionFee {
            tx_id: tx_id.clone(),
            fee: treasury_amount,
        };

        self.deposit_internal(source, treasury_amount, timestamp)?;

        Ok(treasury_amount)
    }

    /// Deposit block reward (5 TIME per block)
    pub fn deposit_block_reward(&mut self, block_number: u64, timestamp: u64) -> Result<u64> {
        let source = TreasurySource::BlockReward { block_number };

        self.deposit_internal(source, TREASURY_BLOCK_REWARD, timestamp)?;

        Ok(TREASURY_BLOCK_REWARD)
    }

    /// Deposit community donation
    pub fn deposit_donation(&mut self, donor: String, amount: u64, timestamp: u64) -> Result<u64> {
        if amount == 0 {
            return Err(TreasuryError::InvalidAmount(
                "Donation cannot be zero".to_string(),
            ));
        }

        let source = TreasurySource::Donation { donor };

        self.deposit_internal(source, amount, timestamp)?;

        Ok(amount)
    }

    /// Deposit recovered funds
    pub fn deposit_recovered(
        &mut self,
        reason: String,
        amount: u64,
        timestamp: u64,
    ) -> Result<u64> {
        if amount == 0 {
            return Err(TreasuryError::InvalidAmount(
                "Recovery amount cannot be zero".to_string(),
            ));
        }

        let source = TreasurySource::Recovered { reason };

        self.deposit_internal(source, amount, timestamp)?;

        Ok(amount)
    }

    /// Internal deposit handler
    fn deposit_internal(
        &mut self,
        source: TreasurySource,
        amount: u64,
        timestamp: u64,
    ) -> Result<()> {
        // Update balance
        self.balance = self
            .balance
            .checked_add(amount)
            .ok_or_else(|| TreasuryError::InvalidAmount("Balance overflow".to_string()))?;

        // Record transaction
        let tx = TreasuryTransaction {
            id: format!("tx-{}", self.next_tx_id),
            timestamp,
            transaction_type: TransactionType::Deposit(source.clone()),
            amount,
            balance_after: self.balance,
        };

        self.transactions.push(tx);
        self.next_tx_id += 1;

        // Update stats
        self.stats.total_deposits += amount;
        self.stats.transaction_count += 1;

        let source_key = match source {
            TreasurySource::TransactionFee { .. } => "fees",
            TreasurySource::BlockReward { .. } => "blocks",
            TreasurySource::Donation { .. } => "donations",
            TreasurySource::Recovered { .. } => "recovered",
        };

        *self
            .stats
            .deposits_by_source
            .entry(source_key.to_string())
            .or_insert(0) += amount;

        Ok(())
    }

    /// Schedule a withdrawal for an approved proposal
    pub fn schedule_withdrawal(&mut self, withdrawal: TreasuryWithdrawal) -> Result<()> {
        if withdrawal.amount > self.balance {
            return Err(TreasuryError::InsufficientBalance {
                requested: withdrawal.amount,
                available: self.balance,
            });
        }

        if withdrawal.amount == 0 {
            return Err(TreasuryError::InvalidAmount(
                "Withdrawal cannot be zero".to_string(),
            ));
        }

        self.scheduled_withdrawals
            .insert(withdrawal.id.clone(), withdrawal);

        Ok(())
    }

    /// Execute a scheduled withdrawal
    pub fn execute_withdrawal(&mut self, withdrawal_id: &str, timestamp: u64) -> Result<u64> {
        let mut withdrawal = self
            .scheduled_withdrawals
            .get(withdrawal_id)
            .ok_or_else(|| TreasuryError::ProposalNotFound(withdrawal_id.to_string()))?
            .clone();

        if withdrawal.status != WithdrawalStatus::Scheduled {
            return Err(TreasuryError::InvalidAmount(format!(
                "Withdrawal {} is not scheduled",
                withdrawal_id
            )));
        }

        if withdrawal.amount > self.balance {
            return Err(TreasuryError::InsufficientBalance {
                requested: withdrawal.amount,
                available: self.balance,
            });
        }

        // Update balance
        self.balance -= withdrawal.amount;

        // Record transaction
        let tx = TreasuryTransaction {
            id: format!("tx-{}", self.next_tx_id),
            timestamp,
            transaction_type: TransactionType::Withdrawal {
                proposal_id: withdrawal.proposal_id.clone(),
                recipient: withdrawal.recipient.clone(),
            },
            amount: withdrawal.amount,
            balance_after: self.balance,
        };

        self.transactions.push(tx);
        self.next_tx_id += 1;

        // Update withdrawal status
        withdrawal.executed_time = Some(timestamp);
        withdrawal.status = WithdrawalStatus::Executed;
        self.scheduled_withdrawals
            .insert(withdrawal_id.to_string(), withdrawal.clone());

        // Update stats
        self.stats.total_withdrawals += withdrawal.amount;
        self.stats.transaction_count += 1;

        Ok(withdrawal.amount)
    }

    /// Cancel a scheduled withdrawal
    pub fn cancel_withdrawal(&mut self, withdrawal_id: &str) -> Result<()> {
        let withdrawal = self
            .scheduled_withdrawals
            .get_mut(withdrawal_id)
            .ok_or_else(|| TreasuryError::ProposalNotFound(withdrawal_id.to_string()))?;

        if withdrawal.status != WithdrawalStatus::Scheduled {
            return Err(TreasuryError::InvalidAmount(format!(
                "Cannot cancel withdrawal {} with status {:?}",
                withdrawal_id, withdrawal.status
            )));
        }

        withdrawal.status = WithdrawalStatus::Cancelled;

        Ok(())
    }

    /// Get all transactions
    pub fn transactions(&self) -> &[TreasuryTransaction] {
        &self.transactions
    }

    /// Get scheduled withdrawals
    pub fn scheduled_withdrawals(&self) -> &HashMap<String, TreasuryWithdrawal> {
        &self.scheduled_withdrawals
    }

    /// Get treasury statistics
    pub fn stats(&self) -> &TreasuryStats {
        &self.stats
    }

    /// Generate financial report for a time period
    pub fn generate_report(&self, period_start: u64, period_end: u64) -> TreasuryReport {
        let mut opening_balance = 0u64;
        let mut period_deposits = 0u64;
        let mut period_withdrawals = 0u64;
        let mut period_tx_count = 0usize;
        let mut largest_deposit = 0u64;
        let mut largest_withdrawal = 0u64;

        for tx in &self.transactions {
            if tx.timestamp < period_start {
                opening_balance = tx.balance_after;
            } else if tx.timestamp >= period_start && tx.timestamp <= period_end {
                period_tx_count += 1;

                match &tx.transaction_type {
                    TransactionType::Deposit(_) => {
                        period_deposits += tx.amount;
                        largest_deposit = largest_deposit.max(tx.amount);
                    }
                    TransactionType::Withdrawal { .. } => {
                        period_withdrawals += tx.amount;
                        largest_withdrawal = largest_withdrawal.max(tx.amount);
                    }
                }
            }
        }

        TreasuryReport {
            period_start,
            period_end,
            opening_balance,
            closing_balance: self.balance,
            total_deposits: period_deposits,
            total_withdrawals: period_withdrawals,
            transaction_count: period_tx_count,
            largest_deposit,
            largest_withdrawal,
        }
    }
}

impl Default for TreasuryPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_pool() {
        let pool = TreasuryPool::new();
        assert_eq!(pool.balance(), 0);
        assert_eq!(pool.balance_time(), 0.0);
    }

    #[test]
    fn test_block_reward_deposit() {
        let mut pool = TreasuryPool::new();

        let amount = pool.deposit_block_reward(1, 1000).unwrap();

        assert_eq!(amount, TREASURY_BLOCK_REWARD);
        assert_eq!(pool.balance(), TREASURY_BLOCK_REWARD);
        assert_eq!(pool.balance_time(), 5.0);
    }

    #[test]
    fn test_transaction_fee_deposit() {
        let mut pool = TreasuryPool::new();
        let total_fee = TIME_UNIT; // 1 TIME

        let amount = pool
            .deposit_transaction_fee("tx123".to_string(), total_fee, 1000)
            .unwrap();

        // Should receive 50%
        assert_eq!(amount, TIME_UNIT / 2);
        assert_eq!(pool.balance(), TIME_UNIT / 2);
    }

    #[test]
    fn test_donation() {
        let mut pool = TreasuryPool::new();

        let amount = pool
            .deposit_donation("alice".to_string(), 10 * TIME_UNIT, 1000)
            .unwrap();

        assert_eq!(amount, 10 * TIME_UNIT);
        assert_eq!(pool.balance(), 10 * TIME_UNIT);
    }

    #[test]
    fn test_withdrawal_flow() {
        let mut pool = TreasuryPool::new();

        // Add funds
        pool.deposit_block_reward(1, 1000).unwrap();
        let initial_balance = pool.balance();

        // Schedule withdrawal
        let withdrawal = TreasuryWithdrawal {
            id: "w1".to_string(),
            proposal_id: "prop-1".to_string(),
            milestone_id: Some("m1".to_string()),
            amount: 2 * TIME_UNIT,
            recipient: "recipient".to_string(),
            scheduled_time: 2000,
            executed_time: None,
            status: WithdrawalStatus::Scheduled,
        };

        pool.schedule_withdrawal(withdrawal).unwrap();

        // Execute withdrawal
        let withdrawn = pool.execute_withdrawal("w1", 3000).unwrap();

        assert_eq!(withdrawn, 2 * TIME_UNIT);
        assert_eq!(pool.balance(), initial_balance - (2 * TIME_UNIT));
    }

    #[test]
    fn test_insufficient_balance() {
        let mut pool = TreasuryPool::new();

        let withdrawal = TreasuryWithdrawal {
            id: "w1".to_string(),
            proposal_id: "prop-1".to_string(),
            milestone_id: None,
            amount: 100 * TIME_UNIT,
            recipient: "recipient".to_string(),
            scheduled_time: 1000,
            executed_time: None,
            status: WithdrawalStatus::Scheduled,
        };

        let result = pool.schedule_withdrawal(withdrawal);
        assert!(result.is_err());
    }

    #[test]
    fn test_statistics() {
        let mut pool = TreasuryPool::new();

        pool.deposit_block_reward(1, 1000).unwrap();
        pool.deposit_transaction_fee("tx1".to_string(), TIME_UNIT, 1100)
            .unwrap();
        pool.deposit_donation("alice".to_string(), 5 * TIME_UNIT, 1200)
            .unwrap();

        let stats = pool.stats();
        assert_eq!(stats.transaction_count, 3);
        assert!(stats.total_deposits > 0);
    }

    #[test]
    fn test_financial_report() {
        let mut pool = TreasuryPool::new();

        pool.deposit_block_reward(1, 1000).unwrap();
        pool.deposit_block_reward(2, 2000).unwrap();
        pool.deposit_block_reward(3, 3000).unwrap();

        let report = pool.generate_report(1500, 2500);

        assert_eq!(report.total_deposits, TREASURY_BLOCK_REWARD);
        assert_eq!(report.transaction_count, 1);
    }

    #[test]
    fn test_cancel_withdrawal() {
        let mut pool = TreasuryPool::new();

        pool.deposit_block_reward(1, 1000).unwrap();

        let withdrawal = TreasuryWithdrawal {
            id: "w1".to_string(),
            proposal_id: "prop-1".to_string(),
            milestone_id: None,
            amount: TIME_UNIT,
            recipient: "recipient".to_string(),
            scheduled_time: 2000,
            executed_time: None,
            status: WithdrawalStatus::Scheduled,
        };

        pool.schedule_withdrawal(withdrawal).unwrap();
        pool.cancel_withdrawal("w1").unwrap();

        let w = pool.scheduled_withdrawals().get("w1").unwrap();
        assert_eq!(w.status, WithdrawalStatus::Cancelled);
    }

    #[test]
    fn test_transaction_history() {
        let mut pool = TreasuryPool::new();

        pool.deposit_block_reward(1, 1000).unwrap();
        pool.deposit_donation("alice".to_string(), 10 * TIME_UNIT, 2000)
            .unwrap();

        let txs = pool.transactions();
        assert_eq!(txs.len(), 2);

        assert_eq!(txs[0].amount, TREASURY_BLOCK_REWARD);
        assert_eq!(txs[1].amount, 10 * TIME_UNIT);
    }
}
