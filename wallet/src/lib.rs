//! TIME Coin Wallet Module
//!
//! Improved implementation with:
//! - UTXO model for flexible transactions
//! - thiserror for clean error handling
//! - NetworkType enum for type safety
//! - Fee support in transactions
//! - Auto-incrementing nonce
//! - Convenience methods for key export/import
//! - BIP-39 mnemonic phrase support for deterministic key generation

pub mod address;
pub mod keypair;
pub mod mnemonic;
pub mod transaction;
pub mod wallet;

pub use address::{Address, AddressError, NetworkType};
pub use keypair::{Keypair, KeypairError};
pub use mnemonic::{generate_mnemonic, mnemonic_to_keypair, validate_mnemonic, MnemonicError, MnemonicPhrase};
pub use transaction::{Transaction, TransactionError, TxInput, TxOutput};
pub use wallet::{Wallet, WalletError, UTXO};
