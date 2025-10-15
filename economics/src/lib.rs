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
    
    /// Block time in seconds (5 seconds)
    pub const BLOCK_TIME: u64 = 5;
    
    /// Blocks per day (17,280)
    pub const BLOCKS_PER_DAY: u64 = 86400 / BLOCK_TIME;
    
    /// Total block reward (100 TIME)
    pub const BLOCK_REWARD: u64 = 100 * TIME_UNIT;
    
    /// Treasury portion (5 TIME)
    pub const TREASURY_REWARD: u64 = 5 * TIME_UNIT;
    
    /// Masternode portion (95 TIME)
    pub const MASTERNODE_REWARD: u64 = 95 * TIME_UNIT;
    
    /// Treasury fee percentage (50%)
    pub const TREASURY_FEE_PERCENTAGE: u64 = 50;
}
