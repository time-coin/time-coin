//! TIME Coin Economics Module
//!
//! Implements the economic model including:
//! - Purchase-based minting
//! - Fee distribution
//! - Reward calculations
//! - Supply management

pub mod supply;
pub mod rewards;
pub mod pricing;

pub use supply::{SupplyManager, SupplyStats};
pub use rewards::{RewardCalculator, RewardDistribution};
pub use pricing::{PurchasePrice, PriceCalculator};

/// Economic constants
pub mod constants {
    /// TIME token unit (8 decimal places)
    pub const TIME_UNIT: u64 = 100_000_000;
    
    /// Block time in seconds (24 hours)
    pub const BLOCK_TIME: u64 = 86400;
    
    /// Blocks per day (one block every 24 hours)
    pub const BLOCKS_PER_DAY: u64 = 1;
    
    /// Total block reward (100 TIME per day)
    pub const BLOCK_REWARD: u64 = 100 * TIME_UNIT;
    
    /// Treasury portion (5 TIME per block)
    pub const TREASURY_REWARD: u64 = 5 * TIME_UNIT;
    
    /// Masternode portion (95 TIME per block)
    pub const MASTERNODE_REWARD: u64 = 95 * TIME_UNIT;
    
    /// Treasury fee percentage (50%)
    pub const TREASURY_FEE_PERCENTAGE: u64 = 50;
}
