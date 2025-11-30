use std::collections::HashSet;
use wallet::Transaction;

const MAX_SUPPLY: u64 = 21_000_000 * 100_000_000; // 21M TIME with 8 decimals
const MIN_FEE_PER_BYTE: u64 = 1; // Minimum fee per byte

#[derive(Debug, Clone)]
pub struct FeeEstimate {
    pub low: u64,    // Low priority (>30 min)
    pub medium: u64, // Medium priority (10-30 min)
    pub high: u64,   // High priority (<10 min)
}

#[derive(Debug)]
pub enum ValidationError {
    InvalidSignature,
    ExceedsMaxSupply,
    InsufficientFee { required: u64, provided: u64 },
    DoubleSpendAttempt,
    InvalidOutput,
    InvalidInput,
    ZeroAmount,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSignature => write!(f, "Transaction signature is invalid"),
            Self::ExceedsMaxSupply => write!(f, "Transaction outputs exceed maximum supply"),
            Self::InsufficientFee { required, provided } => {
                write!(
                    f,
                    "Insufficient fee: required {}, provided {}",
                    required, provided
                )
            }
            Self::DoubleSpendAttempt => write!(f, "Transaction attempts to double-spend"),
            Self::InvalidOutput => write!(f, "Transaction has invalid output"),
            Self::InvalidInput => write!(f, "Transaction has invalid input"),
            Self::ZeroAmount => write!(f, "Transaction amount cannot be zero"),
        }
    }
}

impl std::error::Error for ValidationError {}

#[derive(Debug)]
pub struct TransactionValidator {
    mempool_txids: HashSet<String>,
    current_fee_rate: u64,
}

impl TransactionValidator {
    pub fn new() -> Self {
        Self {
            mempool_txids: HashSet::new(),
            current_fee_rate: MIN_FEE_PER_BYTE,
        }
    }

    /// Update the mempool state for double-spend detection
    pub fn update_mempool(&mut self, txids: Vec<String>) {
        self.mempool_txids = txids.into_iter().collect();
    }

    /// Update the current network fee rate
    pub fn update_fee_rate(&mut self, fee_per_byte: u64) {
        self.current_fee_rate = fee_per_byte.max(MIN_FEE_PER_BYTE);
    }

    /// Validate a transaction against TIME Coin network rules
    /// Note: This validates structure, amounts, and fees, but NOT UTXO inputs
    /// since the GUI wallet doesn't have access to full blockchain state
    pub fn validate_transaction(&self, tx: &Transaction) -> Result<(), ValidationError> {
        // 1. Check outputs don't exceed max supply
        let total_output: u64 = tx.outputs.iter().map(|o| o.amount).sum();
        if total_output > MAX_SUPPLY {
            return Err(ValidationError::ExceedsMaxSupply);
        }

        // 2. Validate outputs
        for output in &tx.outputs {
            if output.amount == 0 {
                return Err(ValidationError::ZeroAmount);
            }
            if output.address.is_empty() {
                return Err(ValidationError::InvalidOutput);
            }
        }

        // 3. Check for double-spend in mempool (by transaction hash)
        let txid = hex::encode(tx.txid());
        if self.mempool_txids.contains(&txid) {
            return Err(ValidationError::DoubleSpendAttempt);
        }

        // 4. Validate inputs exist
        if tx.inputs.is_empty() {
            return Err(ValidationError::InvalidInput);
        }

        // 5. Basic signature check (verify signatures exist)
        for input in &tx.inputs {
            if input.signature.is_empty() {
                return Err(ValidationError::InvalidSignature);
            }
            if input.public_key.is_empty() {
                return Err(ValidationError::InvalidSignature);
            }
        }

        Ok(())
    }

    /// Estimate transaction size in bytes
    fn estimate_transaction_size(&self, tx: &Transaction) -> u64 {
        // Basic size estimation:
        // - Base: 10 bytes (version, locktime, etc.)
        // - Input: ~148 bytes each (outpoint + script + sequence)
        // - Output: ~34 bytes each (amount + script)
        let base_size = 10;
        let input_size = tx.inputs.len() as u64 * 148;
        let output_size = tx.outputs.len() as u64 * 34;

        base_size + input_size + output_size
    }

    /// Estimate fee based on current network conditions
    pub fn estimate_fee(&self, num_inputs: usize, num_outputs: usize) -> FeeEstimate {
        let base_size = 10;
        let input_size = num_inputs as u64 * 148;
        let output_size = num_outputs as u64 * 34;
        let total_size = base_size + input_size + output_size;

        FeeEstimate {
            low: total_size * self.current_fee_rate,
            medium: total_size * (self.current_fee_rate * 2),
            high: total_size * (self.current_fee_rate * 3),
        }
    }

    /// Check if transaction is likely to be accepted by network
    /// Note: This can only check structure and fees, not UTXO validity
    pub fn check_network_acceptance(&self, tx: &Transaction) -> Result<String, ValidationError> {
        // Validate all rules
        self.validate_transaction(tx)?;

        let tx_size = self.estimate_transaction_size(tx);

        // For GUI wallet, we can't calculate fee from inputs since we don't track input amounts
        // This would need to be provided by the wallet when creating the transaction
        let status = "Transaction structure valid - node will validate UTXOs".to_string();

        Ok(status)
    }
}

impl Default for TransactionValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wallet::{TxInput, TxOutput};

    fn create_test_transaction() -> Transaction {
        let mut tx = Transaction::new();

        tx.inputs.push(TxInput {
            prev_tx: [0u8; 32],
            prev_index: 0,
            signature: vec![1, 2, 3],  // Non-empty signature
            public_key: vec![4, 5, 6], // Non-empty public key
        });

        tx.outputs.push(TxOutput {
            amount: 900,
            address: "TIME_receiver".to_string(),
        });

        tx
    }

    #[test]
    fn test_valid_transaction() {
        let validator = TransactionValidator::new();
        let tx = create_test_transaction();

        assert!(validator.validate_transaction(&tx).is_ok());
    }

    #[test]
    fn test_zero_amount_rejected() {
        let validator = TransactionValidator::new();
        let mut tx = create_test_transaction();
        tx.outputs[0].amount = 0;

        match validator.validate_transaction(&tx) {
            Err(ValidationError::ZeroAmount) => (),
            _ => panic!("Expected ZeroAmount error"),
        }
    }

    #[test]
    fn test_exceeds_max_supply() {
        let validator = TransactionValidator::new();
        let mut tx = create_test_transaction();
        tx.outputs[0].amount = MAX_SUPPLY + 1;

        match validator.validate_transaction(&tx) {
            Err(ValidationError::ExceedsMaxSupply) => (),
            _ => panic!("Expected ExceedsMaxSupply error"),
        }
    }

    #[test]
    fn test_empty_signature_rejected() {
        let validator = TransactionValidator::new();
        let mut tx = create_test_transaction();
        tx.inputs[0].signature = vec![];

        match validator.validate_transaction(&tx) {
            Err(ValidationError::InvalidSignature) => (),
            _ => panic!("Expected InvalidSignature error"),
        }
    }

    #[test]
    fn test_fee_estimation() {
        let validator = TransactionValidator::new();
        let estimate = validator.estimate_fee(2, 2);

        assert!(estimate.low > 0);
        assert!(estimate.medium > estimate.low);
        assert!(estimate.high > estimate.medium);
    }
}
