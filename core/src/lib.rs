//! TIME Core Module
//! 
//! Core blockchain functionality for TIME coin.

pub mod block;
pub mod transaction;
pub mod state;
pub mod checkpoint;

pub use block::Block;
pub use transaction::Transaction;
pub use state::ChainState;
pub use checkpoint::Checkpoint;

/// TIME token unit (8 decimal places)
pub const TIME_UNIT: u64 = 100_000_000;

/// Block time in seconds
pub const BLOCK_TIME: u64 = 5;

/// Blocks per day
pub const BLOCKS_PER_DAY: u64 = 17_280;

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
        assert_eq!(TIME_UNIT, 100_000_000);
        assert_eq!(BLOCK_TIME, 5);
        assert_eq!(BLOCKS_PER_DAY, 17_280);
    }
}
