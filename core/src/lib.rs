//! TIME Core Module
//! 
//! Core blockchain functionality for TIME coin.

pub mod block;
pub mod transaction;
pub mod state;

pub use block::Block;
pub use transaction::Transaction;
pub use state::ChainState;

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
}
