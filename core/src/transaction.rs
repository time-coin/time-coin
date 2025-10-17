//! Transaction Processing
//! 
//! Create, sign, and validate transactions

use crate::state::{Transaction, DailyState, Address};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use chrono::Utc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    InsufficientBalance,
    InvalidSignature,
    InvalidAmount,
    InvalidFee,
    NonceReused,
    SelfTransfer,
}

/// Transaction builder
#[derive(Debug, Clone)]
pub struct TransactionBuilder {
    from: Address,
    to: Address,
    amount: u64,
    fee: u64,
    nonce: u64,
}

impl TransactionBuilder {
    pub fn new() -> Self {
        Self {
            from: String::new(),
            to: String::new(),
            amount: 0,
            fee: 0,
            nonce: 0,
        }
    }
    
    pub fn from(mut self, address: Address) -> Self {
        self.from = address;
        self
    }
    
    pub fn to(mut self, address: Address) -> Self {
        self.to = address;
        self
    }
    
    pub fn amount(mut self, amount: u64) -> Self {
        self.amount = amount;
        self
    }
    
    pub fn fee(mut self, fee: u64) -> Self {
        self.fee = fee;
        self
    }
    
    pub fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = nonce;
        self
    }
    
    /// Build unsigned transaction
    pub fn build(self) -> Transaction {
        let timestamp = Utc::now().timestamp();
        let txid = Self::calculate_txid(&self.from, &self.to, self.amount, self.nonce, timestamp);
        
        Transaction {
            txid,
            from: self.from,
            to: self.to,
            amount: self.amount,
            fee: self.fee,
            timestamp,
            signature: vec![],
        }
    }
    
    /// Calculate transaction ID
    fn calculate_txid(from: &str, to: &str, amount: u64, nonce: u64, timestamp: i64) -> String {
        let mut hasher = Sha256::new();
        hasher.update(from.as_bytes());
        hasher.update(to.as_bytes());
        hasher.update(amount.to_le_bytes());
        hasher.update(nonce.to_le_bytes());
        hasher.update(timestamp.to_le_bytes());
        format!("{:x}", hasher.finalize())
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Transaction {
    /// Get signable message
    pub fn signable_message(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(self.txid.as_bytes());
        message.extend_from_slice(self.from.as_bytes());
        message.extend_from_slice(self.to.as_bytes());
        message.extend_from_slice(&self.amount.to_le_bytes());
        message.extend_from_slice(&self.fee.to_le_bytes());
        message.extend_from_slice(&self.timestamp.to_le_bytes());
        message
    }
    
    /// Sign transaction
    pub fn sign(&mut self, signature: Vec<u8>) {
        self.signature = signature;
    }
}

/// Transaction validator
pub struct TransactionValidator;

impl TransactionValidator {
    /// Validate transaction against current state
    pub fn validate(
        tx: &Transaction,
        state: &DailyState,
    ) -> Result<(), ValidationError> {
        // Check amounts
        if tx.amount == 0 {
            return Err(ValidationError::InvalidAmount);
        }
        
        if tx.fee == 0 {
            return Err(ValidationError::InvalidFee);
        }
        
        // Check self-transfer
        if tx.from == tx.to {
            return Err(ValidationError::SelfTransfer);
        }
        
        // Check balance
        let balance = state.get_balance(&tx.from);
        let total_cost = tx.amount + tx.fee;
        
        if balance < total_cost {
            return Err(ValidationError::InsufficientBalance);
        }
        
        // Signature verification happens via crypto module
        // Placeholder for now
        if tx.signature.is_empty() {
            return Err(ValidationError::InvalidSignature);
        }
        
        Ok(())
    }
    
    /// Validate signature
    pub fn verify_signature(tx: &Transaction, public_key_hex: &str) -> Result<(), ValidationError> {
        let message = tx.signable_message();
        
        // Use time-crypto module for verification
        // This will be integrated with the crypto module
        if tx.signature.len() != 64 {
            return Err(ValidationError::InvalidSignature);
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_transaction_builder() {
        let tx = TransactionBuilder::new()
            .from("addr1".to_string())
            .to("addr2".to_string())
            .amount(100)
            .fee(1)
            .nonce(0)
            .build();
        
        assert_eq!(tx.from, "addr1");
        assert_eq!(tx.to, "addr2");
        assert_eq!(tx.amount, 100);
        assert_eq!(tx.fee, 1);
        assert!(!tx.txid.is_empty());
    }
    
    #[test]
    fn test_validation_insufficient_balance() {
        let mut state = DailyState::new(1);
        state.set_balance("addr1".to_string(), 50);
        
        let tx = TransactionBuilder::new()
            .from("addr1".to_string())
            .to("addr2".to_string())
            .amount(100)
            .fee(1)
            .build();
        
        let result = TransactionValidator::validate(&tx, &state);
        assert_eq!(result, Err(ValidationError::InsufficientBalance));
    }
    
    #[test]
    fn test_validation_self_transfer() {
        let state = DailyState::new(1);
        
        let tx = TransactionBuilder::new()
            .from("addr1".to_string())
            .to("addr1".to_string())
            .amount(100)
            .fee(1)
            .build();
        
        let result = TransactionValidator::validate(&tx, &state);
        assert_eq!(result, Err(ValidationError::SelfTransfer));
    }
    
    #[test]
    fn test_signable_message() {
        let tx = TransactionBuilder::new()
            .from("addr1".to_string())
            .to("addr2".to_string())
            .amount(100)
            .fee(1)
            .build();
        
        let message = tx.signable_message();
        assert!(!message.is_empty());
    }
}
