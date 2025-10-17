//! TIME Core - Core blockchain functionality

pub mod block;
pub mod transaction;
pub mod state;
pub mod constants;

pub use block::{Block, BlockHeader};
pub use transaction::{Transaction, TransactionType};
pub use state::ChainState;
pub use constants::*;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }
}
pub mod finalizer;
