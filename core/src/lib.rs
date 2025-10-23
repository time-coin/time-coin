//! TIME Coin Core Library
//!
//! Core blockchain functionality for TIME Coin

/// Smallest unit constant (1 COIN in satoshi-like units)
pub const COIN: u64 = 100_000_000;
pub mod finalizer;
pub mod state;

// Re-export main types
pub use finalizer::{BlockFinalizer, FinalizedBlock};
pub use state::{Address, DailyState, MasternodeInfo, StateSnapshot, Transaction, TxHash};
pub mod transaction;
pub use transaction::ValidationError;
pub mod mempool;
pub use mempool::TransactionPool;
pub mod masternode_tx;
