//! Core blockchain components for TIME Coin

pub mod block;
pub mod transaction;
pub mod utxo_set;
pub mod mempool;
pub mod state;
pub mod db;
pub mod snapshot;
pub mod snapshot_service;
pub mod constants;
pub mod checkpoint;
pub mod finalizer;
pub mod masternode_tx;

// Re-export commonly used types
pub use block::{Block, BlockHeader, MasternodeCounts, MasternodeTier, calculate_treasury_reward, calculate_total_masternode_reward};
pub use transaction::{Transaction, TxInput, TxOutput, OutPoint, TransactionError, SpecialTransaction};
pub use utxo_set::{UTXOSet, UTXOSetSnapshot};

// Note: Mempool and BlockchainState will be re-exported once they're properly defined
// pub use mempool::{Mempool, MempoolError, MempoolStats};
pub use state::{BlockchainState, StateError, MasternodeInfo, ChainStats};
