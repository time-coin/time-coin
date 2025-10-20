//! TIME Coin Core Library
//!
//! Core blockchain functionality for TIME Coin

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
