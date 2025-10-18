//! TIME Coin Core Library
//! 
//! Core blockchain functionality for TIME Coin

pub mod state;
pub mod finalizer;

// Re-export main types
pub use state::{DailyState, Transaction, MasternodeInfo, StateSnapshot, Address, TxHash};
pub use finalizer::{BlockFinalizer, FinalizedBlock};
pub mod transaction;
pub use transaction::ValidationError;
pub mod mempool;
pub use mempool::TransactionPool;
pub mod masternode_tx;
