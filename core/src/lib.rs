//! TIME Core Module
//! 
//! Core blockchain functionality for TIME coin.

pub mod block;
pub mod transaction;
pub mod state;

pub use block::Block;
pub use transaction::Transaction;
pub use state::ChainState;

/// TIME token unit (8 decimal places)
pub const TIME_UNIT: u64 = 100_000_000;

/// Block time in seconds (24 hours)
pub const BLOCK_TIME: u64 = 86400;

/// Blocks per day (one block every 24 hours)
pub const BLOCKS_PER_DAY: u64 = 1;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version(), "0.1.0");
    }
    
    #[test]
    fn test_constants() {
        // Verify whitepaper specifications
        assert_eq!(TIME_UNIT, 100_000_000, "TIME_UNIT should be 100,000,000 (8 decimals)");
        assert_eq!(BLOCK_TIME, 86400, "BLOCK_TIME should be 86400 seconds (24 hours)");
        assert_eq!(BLOCKS_PER_DAY, 1, "BLOCKS_PER_DAY should be 1 (one block per day)");
    }
    
    #[test]
    fn test_block_time_is_24_hours() {
        assert_eq!(BLOCK_TIME, 24 * 60 * 60, "Block time should equal 24 hours in seconds");
    }
    
    #[test]
    fn test_blocks_per_year() {
        let blocks_per_year = BLOCKS_PER_DAY * 365;
        assert_eq!(blocks_per_year, 365, "Should have 365 blocks per year");
    }
}
