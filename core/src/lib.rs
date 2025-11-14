//! Core blockchain components for TIME Coin

pub mod block;
pub mod checkpoint;
pub mod constants;
pub mod db;
pub mod finalizer;
pub mod masternode_tx;
pub mod mempool;
pub mod snapshot;
pub mod snapshot_service;
pub mod state;
pub mod transaction;
pub mod treasury_manager;
pub mod utxo_set;

// Re-export commonly used types
pub use block::{
    calculate_total_masternode_reward, calculate_treasury_reward, Block, BlockHeader,
    MasternodeCounts, MasternodeTier,
};
pub use transaction::{
    OutPoint, SpecialTransaction, Transaction, TransactionError, TxInput, TxOutput,
};
pub use utxo_set::{UTXOSet, UTXOSetSnapshot};

// Note: Mempool and BlockchainState will be re-exported once they're properly defined
// pub use mempool::{Mempool, MempoolError, MempoolStats};
pub use state::{
    BlockchainState, ChainStats, MasternodeInfo, StateError, Treasury, TreasuryAllocation,
    TreasurySource, TreasuryStats, TreasuryWithdrawal,
};
pub use treasury_manager::{
    CreateProposalParams, ProposalStatus, TreasuryManager, TreasuryProposal, Vote, VoteChoice,
};
