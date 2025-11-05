//! UTXO-based transaction model for TIME Coin

use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::fmt;

#[derive(Debug, Clone)]
pub enum TransactionError {
    InvalidAmount,
    InvalidSignature,
    InvalidInput,
    InsufficientFunds,
    DuplicateInput,
    InvalidOutputIndex,
    SerializationError,
}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TransactionError::InvalidAmount => write!(f, "Invalid transaction amount"),
            TransactionError::InvalidSignature => write!(f, "Invalid signature"),
            TransactionError::InvalidInput => write!(f, "Invalid transaction input"),
            TransactionError::InsufficientFunds => write!(f, "Insufficient funds"),
            TransactionError::DuplicateInput => write!(f, "Duplicate input detected"),
            TransactionError::InvalidOutputIndex => write!(f, "Invalid output index"),
            TransactionError::SerializationError => write!(f, "Serialization error"),
        }
    }
}

impl std::error::Error for TransactionError {}

/// Reference to a previous transaction output (UTXO)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct OutPoint {
    /// Transaction ID being spent
    pub txid: String,
    /// Output index in the transaction
    pub vout: u32,
}

impl OutPoint {
    pub fn new(txid: String, vout: u32) -> Self {
        Self { txid, vout }
    }
}

/// Transaction input spending a UTXO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxInput {
    /// Reference to the UTXO being spent
    pub previous_output: OutPoint,
    /// Public key of the spender
    pub public_key: Vec<u8>,
    /// Signature proving ownership
    pub signature: Vec<u8>,
    /// Sequence number (for future use with timelocks)
    pub sequence: u32,
}

impl TxInput {
    pub fn new(txid: String, vout: u32, public_key: Vec<u8>, signature: Vec<u8>) -> Self {
        Self {
            previous_output: OutPoint::new(txid, vout),
            public_key,
            signature,
            sequence: 0xFFFFFFFF, // Default: no timelock
        }
    }
}

/// Transaction output creating a new UTXO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxOutput {
    /// Amount in the smallest unit (satoshi equivalent)
    pub amount: u64,
    /// Address that can spend this output
    pub address: String,
}

impl TxOutput {
    pub fn new(amount: u64, address: String) -> Self {
        Self { amount, address }
    }
}

/// Complete UTXO-based transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction ID (hash of the transaction)
    pub txid: String,
    /// Transaction version
    pub version: u32,
    /// Input UTXOs being spent
    pub inputs: Vec<TxInput>,
    /// Output UTXOs being created
    pub outputs: Vec<TxOutput>,
    /// Lock time (block height or timestamp)
    pub lock_time: u32,
    /// Transaction timestamp
    pub timestamp: i64,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(inputs: Vec<TxInput>, outputs: Vec<TxOutput>) -> Self {
        let mut tx = Self {
            txid: String::new(),
            version: 1,
            inputs,
            outputs,
            lock_time: 0,
            timestamp: chrono::Utc::now().timestamp(),
        };

        tx.txid = tx.calculate_txid();
        tx
    }

    /// Calculate the transaction ID (double SHA3-256 hash)
    pub fn calculate_txid(&self) -> String {
        let data = self.serialize_for_signing();
        let hash1 = Sha3_256::digest(&data);
        let hash2 = Sha3_256::digest(&hash1);
        hex::encode(hash2)
    }

    /// Serialize transaction data for signing (excludes signatures)
    pub fn serialize_for_signing(&self) -> Vec<u8> {
        let mut data = Vec::new();

        // Version
        data.extend_from_slice(&self.version.to_le_bytes());

        // Inputs (without signatures)
        data.extend_from_slice(&(self.inputs.len() as u32).to_le_bytes());
        for input in &self.inputs {
            data.extend_from_slice(input.previous_output.txid.as_bytes());
            data.extend_from_slice(&input.previous_output.vout.to_le_bytes());
            data.extend_from_slice(&input.public_key);
            data.extend_from_slice(&input.sequence.to_le_bytes());
        }

        // Outputs
        data.extend_from_slice(&(self.outputs.len() as u32).to_le_bytes());
        for output in &self.outputs {
            data.extend_from_slice(&output.amount.to_le_bytes());
            data.extend_from_slice(output.address.as_bytes());
        }

        // Lock time and timestamp
        data.extend_from_slice(&self.lock_time.to_le_bytes());
        data.extend_from_slice(&self.timestamp.to_le_bytes());

        data
    }

    /// Get total input amount (requires UTXO lookup)
    pub fn total_input(
        &self,
        utxo_set: &std::collections::HashMap<OutPoint, TxOutput>,
    ) -> Result<u64, TransactionError> {
        let mut total = 0u64;
        for input in &self.inputs {
            let utxo = utxo_set
                .get(&input.previous_output)
                .ok_or(TransactionError::InvalidInput)?;
            total = total
                .checked_add(utxo.amount)
                .ok_or(TransactionError::InvalidAmount)?;
        }
        Ok(total)
    }

    /// Get total output amount
    pub fn total_output(&self) -> Result<u64, TransactionError> {
        let mut total = 0u64;
        for output in &self.outputs {
            total = total
                .checked_add(output.amount)
                .ok_or(TransactionError::InvalidAmount)?;
        }
        Ok(total)
    }

    /// Calculate transaction fee
    pub fn fee(
        &self,
        utxo_set: &std::collections::HashMap<OutPoint, TxOutput>,
    ) -> Result<u64, TransactionError> {
        let input_total = self.total_input(utxo_set)?;
        let output_total = self.total_output()?;

        input_total
            .checked_sub(output_total)
            .ok_or(TransactionError::InsufficientFunds)
    }

    /// Basic validation (structure checks)
    pub fn validate_structure(&self) -> Result<(), TransactionError> {
        // Must have at least one input and output
        // Coinbase transactions have no inputs
        if self.inputs.is_empty() && !self.is_coinbase() {
            return Err(TransactionError::InvalidInput);
        }
        if self.outputs.is_empty() {
            return Err(TransactionError::InvalidAmount);
        }

        // Check for duplicate inputs
        let mut seen = std::collections::HashSet::new();
        for input in &self.inputs {
            if !seen.insert(&input.previous_output) {
                return Err(TransactionError::DuplicateInput);
            }
        }

        // All outputs must have positive amounts
        for output in &self.outputs {
            if output.amount == 0 {
                return Err(TransactionError::InvalidAmount);
            }
        }

        Ok(())
    }

    /// Check if this is a coinbase transaction (no inputs, generates new coins)
    pub fn is_coinbase(&self) -> bool {
        self.inputs.is_empty()
    }
}

/// Special transaction types for TIME Coin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpecialTransaction {
    /// Coinbase transaction (block reward)
    Coinbase {
        block_height: u64,
        outputs: Vec<TxOutput>,
    },
    /// Masternode registration
    MasternodeRegistration {
        collateral_tx: Transaction,
        masternode_address: String,
        operator_pubkey: Vec<u8>,
        voting_address: String,
    },
    /// Governance proposal
    GovernanceProposal {
        proposal_hash: String,
        payment_amount: u64,
        payment_address: String,
    },
}

impl SpecialTransaction {
    /// Convert special transaction to regular transaction
    pub fn to_transaction(&self) -> Transaction {
        match self {
            SpecialTransaction::Coinbase {
                block_height,
                outputs,
            } => {
                Transaction {
                    txid: format!("coinbase_{}", block_height),
                    version: 1,
                    inputs: vec![], // Coinbase has no inputs
                    outputs: outputs.clone(),
                    lock_time: 0,
                    timestamp: chrono::Utc::now().timestamp(),
                }
            }
            SpecialTransaction::MasternodeRegistration { collateral_tx, .. } => {
                collateral_tx.clone()
            }
            SpecialTransaction::GovernanceProposal {
                proposal_hash: _,
                payment_amount,
                payment_address,
            } => {
                Transaction::new(
                    vec![], // Will be filled by treasury
                    vec![TxOutput::new(*payment_amount, payment_address.clone())],
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_creation() {
        let input = TxInput::new("prev_tx_id".to_string(), 0, vec![1, 2, 3], vec![4, 5, 6]);
        let output = TxOutput::new(1000, "recipient_address".to_string());

        let tx = Transaction::new(vec![input], vec![output]);

        assert!(!tx.txid.is_empty());
        assert_eq!(tx.inputs.len(), 1);
        assert_eq!(tx.outputs.len(), 1);
    }

    #[test]
    fn test_outpoint_equality() {
        let op1 = OutPoint::new("txid".to_string(), 0);
        let op2 = OutPoint::new("txid".to_string(), 0);
        let op3 = OutPoint::new("txid".to_string(), 1);

        assert_eq!(op1, op2);
        assert_ne!(op1, op3);
    }
}
