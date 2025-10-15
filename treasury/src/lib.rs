//! TIME Coin Treasury Module
//!
//! Manages the community-governed treasury that receives:
//! - 50% of all transaction fees
//! - 5 TIME from each block reward
//!
//! Funds are distributed through approved governance proposals.

pub mod pool;
pub mod error;

pub use pool::{
    TreasuryPool,
    TreasurySource,
    TreasuryWithdrawal,
    TreasuryTransaction,
    TreasuryReport,
    TreasuryStats,
    TIME_UNIT,
    TREASURY_FEE_PERCENTAGE,
    TREASURY_BLOCK_REWARD,
    MASTERNODE_BLOCK_REWARD,
};

pub use error::{TreasuryError, Result};

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
