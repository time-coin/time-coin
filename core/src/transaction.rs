//! Transaction module for TIME Coin

use sha3::{Digest, Keccak256};
use std::fmt;

#[derive(Debug, Clone)]
pub enum ValidationError {
    InvalidAmount,
    InvalidSignature,
    InvalidAddress,
    InsufficientBalance,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValidationError::InvalidAmount => write!(f, "Invalid transaction amount"),
            ValidationError::InvalidSignature => write!(f, "Invalid signature"),
            ValidationError::InvalidAddress => write!(f, "Invalid address"),
            ValidationError::InsufficientBalance => write!(f, "Insufficient balance"),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Represents a TIME Coin transaction
#[derive(Debug, Clone)]
pub struct Transaction {
    pub txid: String,
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: i64,
    pub signature: String,
}

impl Transaction {
    pub fn new(from: String, to: String, amount: u64, fee: u64) -> Self {
        let timestamp = chrono::Utc::now().timestamp();
        let txid = format!("{}", uuid::Uuid::new_v4());
        
        Self {
            txid,
            from,
            to,
            amount,
            fee,
            timestamp,
            signature: String::new(),
        }
    }
    
    pub fn signable_message(&self) -> Vec<u8> {
        let message = format!(
            "{}{}{}{}{}",
            self.from, self.to, self.amount, self.fee, self.timestamp
        );
        message.into_bytes()
    }
    
    pub fn sign(&mut self, private_key: &str) -> Result<(), ValidationError> {
        let keypair = time_crypto::KeyPair::from_private_key(private_key)
            .map_err(|_| ValidationError::InvalidSignature)?;
        
        let message = self.signable_message();
        let signature = keypair.sign(&message);
        self.signature = hex::encode(signature);
        
        Ok(())
    }
    
    pub fn calculate_hash(&self) -> String {
        let data = format!(
            "{}{}{}{}{}{}",
            self.txid, self.from, self.to, self.amount, self.fee, self.timestamp
        );
        
        let mut hasher = Keccak256::new();
        hasher.update(data.as_bytes());
        let result = hasher.finalize();
        
        format!("{:x}", result)
    }
    
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.amount == 0 {
            return Err(ValidationError::InvalidAmount);
        }
        
        if self.from.is_empty() || self.to.is_empty() {
            return Err(ValidationError::InvalidAddress);
        }
        
        Ok(())
    }
}
