//! TIME Coin Treasury Module
//!
//! Manages the community-governed treasury that receives:
//! - 50% of all transaction fees
//! - 5 TIME from each block reward
//!
//! Funds are distributed through approved governance proposals.

pub mod error;
pub mod pool;

pub use pool::{
    TreasuryPool, TreasuryReport, TreasurySource, TreasuryStats, TreasuryTransaction,
    TreasuryWithdrawal, MASTERNODE_BLOCK_REWARD, TIME_UNIT, TREASURY_BLOCK_REWARD,
    TREASURY_FEE_PERCENTAGE,
};

pub use error::{Result, TreasuryError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_constants() {
        assert_eq!(TIME_UNIT, 100_000_000);
        assert_eq!(TREASURY_FEE_PERCENTAGE, 50);
        assert_eq!(TREASURY_BLOCK_REWARD, 5 * TIME_UNIT);
        assert_eq!(MASTERNODE_BLOCK_REWARD, 95 * TIME_UNIT);
    }
}
