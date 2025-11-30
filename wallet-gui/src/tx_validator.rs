// Transaction validator for TIME Coin network
// Validates transactions against network rules before broadcast

use std::collections::HashSet;
use wallet::Transaction;

/// TIME Coin network parameters
pub const MAX_SUPPLY: u64 = 21_000_000 * 100_000_000; // 21M TIME in satoshis
pub const DUST_THRESHOLD: u64 = 546; // Minimum output amount (satoshis)
pub const MAX_TRANSACTION_SIZE: usize = 100_000; // 100KB max transaction
pub const MAX_OUTPUTS: usize = 2000; // Prevent memory DOS
pub const MIN_FEE_PER_BYTE: u64 = 1; // Minimum 1 sat/byte

/// Transaction validation errors
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Transaction has no inputs")]
    NoInputs,

    #[error("Transaction has no outputs")]
    NoOutputs,

    #[error("Transaction size {0} bytes exceeds maximum {1}")]
    TooLarge(usize, usize),

    #[error("Output count {0} exceeds maximum {1}")]
    TooManyOutputs(usize, usize),

    #[error("Output amount {0} is below dust threshold {1}")]
    BelowDustThreshold(u64, u64),

    #[error("Total output amount {0} exceeds maximum supply {1}")]
    ExceedsMaxSupply(u64, u64),

    #[error("Fee {0} is below minimum {1} for transaction size {2} bytes")]
    FeeTooLow(u64, u64, usize),

    #[error("Fee {0} exceeds recommended maximum {1} (possible user error)")]
    FeeTooHigh(u64, u64),

    #[error("Invalid address format: {0}")]
    InvalidAddress(String),

    #[error("Arithmetic overflow in amount calculation")]
    AmountOverflow,

    #[error("Signature verification failed")]
    InvalidSignature,

    #[error("Possible double-spend detected: {0}")]
    PossibleDoubleSpend(String),
}

/// Fee market statistics for fee estimation
#[derive(Debug, Clone)]
pub struct FeeMarket {
    pub low_priority: u64,     // 10th percentile (sat/byte)
    pub standard: u64,         // 50th percentile (sat/byte)
    pub high_priority: u64,    // 90th percentile (sat/byte)
    pub congestion_level: f64, // 0.0 = empty, 1.0 = full
    pub mempool_size: usize,   // Number of pending transactions
    pub last_updated: std::time::SystemTime,
}

impl Default for FeeMarket {
    fn default() -> Self {
        Self {
            low_priority: 1,
            standard: 5,
            high_priority: 10,
            congestion_level: 0.0,
            mempool_size: 0,
            last_updated: std::time::SystemTime::now(),
        }
    }
}

/// Transaction validator
#[derive(Debug)]
pub struct TransactionValidator {
    fee_market: FeeMarket,
    known_mempool_txids: HashSet<String>,
    max_fee_multiplier: u64, // Alert if fee exceeds this * standard
}

impl Default for TransactionValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionValidator {
    pub fn new() -> Self {
        Self {
            fee_market: FeeMarket::default(),
            known_mempool_txids: HashSet::new(),
            max_fee_multiplier: 100, // Alert if fee > 100x standard
        }
    }

    /// Update fee market data from network
    pub fn update_fee_market(&mut self, fee_market: FeeMarket) {
        self.fee_market = fee_market;
    }

    /// Update known mempool transactions
    pub fn update_mempool(&mut self, txids: Vec<String>) {
        self.known_mempool_txids = txids.into_iter().collect();
    }

    /// Get current fee market data
    pub fn get_fee_market(&self) -> &FeeMarket {
        &self.fee_market
    }

    /// Estimate fee for transaction size and priority
    pub fn estimate_fee(&self, tx_size_bytes: usize, priority: FeePriority) -> u64 {
        let rate = match priority {
            FeePriority::Low => self.fee_market.low_priority,
            FeePriority::Standard => self.fee_market.standard,
            FeePriority::High => self.fee_market.high_priority,
        };

        (tx_size_bytes as u64).saturating_mul(rate)
    }

    /// Validate transaction against TIME Coin network rules
    pub fn validate_transaction(
        &self,
        tx: &Transaction,
        tx_size_bytes: usize,
    ) -> Result<ValidationReport, ValidationError> {
        let mut warnings = Vec::new();

        // 1. Basic structure validation
        if tx.inputs.is_empty() {
            return Err(ValidationError::NoInputs);
        }
        if tx.outputs.is_empty() {
            return Err(ValidationError::NoOutputs);
        }

        // 2. Size limits
        if tx_size_bytes > MAX_TRANSACTION_SIZE {
            return Err(ValidationError::TooLarge(
                tx_size_bytes,
                MAX_TRANSACTION_SIZE,
            ));
        }

        // 3. Output limits
        if tx.outputs.len() > MAX_OUTPUTS {
            return Err(ValidationError::TooManyOutputs(
                tx.outputs.len(),
                MAX_OUTPUTS,
            ));
        }

        // 4. Dust threshold check
        for (i, output) in tx.outputs.iter().enumerate() {
            if output.amount < DUST_THRESHOLD {
                return Err(ValidationError::BelowDustThreshold(
                    output.amount,
                    DUST_THRESHOLD,
                ));
            }
        }

        // 5. Total amount validation
        let total_output = tx
            .outputs
            .iter()
            .try_fold(0u64, |acc, output| acc.checked_add(output.amount))
            .ok_or(ValidationError::AmountOverflow)?;

        if total_output > MAX_SUPPLY {
            return Err(ValidationError::ExceedsMaxSupply(total_output, MAX_SUPPLY));
        }

        // 6. Address format validation
        for output in &tx.outputs {
            if !self.validate_address(&output.address) {
                return Err(ValidationError::InvalidAddress(output.address.clone()));
            }
        }

        // 7. Fee estimation (assuming total_output is less than total_input)
        // Note: wallet Transaction doesn't track input amounts, so we estimate fee
        // This should be enhanced when we have access to UTXO amounts
        let estimated_fee = tx_size_bytes as u64 * self.fee_market.standard;
        let fee_rate = estimated_fee / (tx_size_bytes as u64).max(1);

        // 8. Fee rate analysis
        if fee_rate < self.fee_market.low_priority {
            warnings.push(format!(
                "Estimated fee rate {} sat/byte may be below network low priority ({}). Transaction may be slow.",
                fee_rate, self.fee_market.low_priority
            ));
        }

        // 9. Double-spend detection (check against known mempool)
        for input in &tx.inputs {
            let input_key = format!("{:?}:{}", input.prev_tx, input.prev_index);
            if self.known_mempool_txids.contains(&input_key) {
                warnings.push(format!(
                    "Input {}:{} may already be in mempool (possible double-spend)",
                    hex::encode(&input.prev_tx[..8]),
                    input.prev_index
                ));
            }
        }

        Ok(ValidationReport {
            valid: true,
            warnings,
            fee_rate,
            estimated_confirmation_time: self.estimate_confirmation_time(fee_rate),
        })
    }

    /// Validate TIME Coin address format
    fn validate_address(&self, address: &str) -> bool {
        // TIME Coin addresses start with "TIME"
        if !address.starts_with("TIME") {
            return false;
        }

        // Must be reasonable length (base58 encoded)
        if address.len() < 30 || address.len() > 50 {
            return false;
        }

        // Basic character set validation (base58: no 0, O, I, l)
        address
            .chars()
            .all(|c| c.is_ascii_alphanumeric() && !matches!(c, '0' | 'O' | 'I' | 'l'))
    }

    /// Estimate confirmation time based on fee rate
    fn estimate_confirmation_time(&self, fee_rate: u64) -> &'static str {
        if fee_rate >= self.fee_market.high_priority {
            "Next block (~10 min)"
        } else if fee_rate >= self.fee_market.standard {
            "1-3 blocks (~10-30 min)"
        } else if fee_rate >= self.fee_market.low_priority {
            "3-6 blocks (~30-60 min)"
        } else {
            "6+ blocks (>60 min)"
        }
    }

    /// Perform dry-run simulation of transaction
    pub fn simulate_transaction(&self, tx: &Transaction) -> SimulationResult {
        let tx_size = self.estimate_transaction_size(tx);

        match self.validate_transaction(tx, tx_size) {
            Ok(report) => SimulationResult {
                success: true,
                estimated_size: tx_size,
                validation_report: Some(report),
                error: None,
            },
            Err(e) => SimulationResult {
                success: false,
                estimated_size: tx_size,
                validation_report: None,
                error: Some(e.to_string()),
            },
        }
    }

    /// Estimate transaction size in bytes
    fn estimate_transaction_size(&self, tx: &Transaction) -> usize {
        // Rough estimation:
        // - Version: 4 bytes
        // - Input count: 1-9 bytes (varint)
        // - Inputs: ~180 bytes each (txid + vout + scriptSig + sequence)
        // - Output count: 1-9 bytes (varint)
        // - Outputs: ~34 bytes each (amount + scriptPubKey)
        // - Locktime: 4 bytes

        let base_size = 10; // Version + locktime + varints
        let input_size = tx.inputs.len() * 180;
        let output_size = tx.outputs.len() * 34;

        base_size + input_size + output_size
    }
}

/// Fee priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeePriority {
    Low,
    Standard,
    High,
}

/// Validation report with warnings
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub valid: bool,
    pub warnings: Vec<String>,
    pub fee_rate: u64,
    pub estimated_confirmation_time: &'static str,
}

/// Simulation result for dry-run
#[derive(Debug)]
pub struct SimulationResult {
    pub success: bool,
    pub estimated_size: usize,
    pub validation_report: Option<ValidationReport>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use wallet::{TxInput, TxOutput};

    fn create_test_transaction() -> Transaction {
        let mut tx = Transaction::new();
        tx.add_input(TxInput::new([0; 32], 0));
        tx.add_output(TxOutput::new(
            99_000,
            wallet::Address::from_string("TIME1234567890abcdefghijklmnop").unwrap(),
        ))
        .unwrap();
        tx
    }

    #[test]
    fn test_valid_transaction() {
        let validator = TransactionValidator::new();
        let tx = create_test_transaction();
        let result = validator.validate_transaction(&tx, 250);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dust_threshold() {
        let validator = TransactionValidator::new();
        let mut tx = Transaction::new();
        tx.add_input(TxInput::new([0; 32], 0));
        tx.add_output(TxOutput::new(
            100,
            wallet::Address::from_string("TIME1234567890abcdefghijklmnop").unwrap(),
        ))
        .unwrap(); // Below dust threshold

        let result = validator.validate_transaction(&tx, 250);
        assert!(matches!(
            result,
            Err(ValidationError::BelowDustThreshold(_, _))
        ));
    }

    #[test]
    fn test_fee_estimation() {
        let validator = TransactionValidator::new();
        let fee = validator.estimate_fee(250, FeePriority::Standard);
        assert_eq!(fee, 250 * 5); // Standard rate is 5 sat/byte
    }

    #[test]
    fn test_invalid_address() {
        let validator = TransactionValidator::new();
        assert!(!validator.validate_address("BTC1234")); // Wrong prefix
        assert!(!validator.validate_address("TIME")); // Too short
        assert!(!validator.validate_address("TIME0Il")); // Contains invalid chars
        assert!(validator.validate_address("TIME1234567890abcdefghijklmnop"));
    }
}
