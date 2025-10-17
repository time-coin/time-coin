//! Transaction structures and types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    Transfer {
        from: String,
        to: String,
        amount: u64,
        fee: u64,
    },
    Mint {
        recipient: String,
        amount: u64,
        purchase_proof: String,
    },
    MasternodeReward {
        masternode_id: String,
        amount: u64,
    },
    TreasuryAllocation {
        amount: u64,
        proposal_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub transaction_type: TransactionType,
    pub signature: String,
    pub nonce: u64,
}

impl Transaction {
    pub fn new(transaction_type: TransactionType, nonce: u64) -> Self {
        let timestamp = Utc::now();
        let id = Self::generate_id(&transaction_type, timestamp, nonce);

        Transaction {
            id,
            timestamp,
            transaction_type,
            signature: String::new(),
            nonce,
        }
    }

    fn generate_id(tx_type: &TransactionType, timestamp: DateTime<Utc>, nonce: u64) -> String {
        let data = format!("{:?}:{}:{}", tx_type, timestamp.timestamp(), nonce);
        let mut hasher = Sha3_256::new();
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn sign(&mut self, signature: String) {
        self.signature = signature;
    }

    pub fn validate(&self) -> Result<(), String> {
        // Validate transaction ID
        let expected_id = Self::generate_id(&self.transaction_type, self.timestamp, self.nonce);
        if self.id != expected_id {
            return Err("Invalid transaction ID".to_string());
        }

        // Validate signature exists
        if self.signature.is_empty() {
            return Err("Transaction not signed".to_string());
        }

        // Validate transaction type specific rules
        match &self.transaction_type {
            TransactionType::Transfer { amount, fee, .. } => {
                if *amount == 0 {
                    return Err("Transfer amount must be greater than 0".to_string());
                }
                if *fee < crate::constants::MIN_TRANSACTION_FEE {
                    return Err("Transaction fee too low".to_string());
                }
            }
            TransactionType::Mint { amount, .. } => {
                if *amount == 0 {
                    return Err("Mint amount must be greater than 0".to_string());
                }
            }
            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_transaction() {
        let tx_type = TransactionType::Transfer {
            from: "alice".to_string(),
            to: "bob".to_string(),
            amount: 100 * crate::constants::COIN,
            fee: crate::constants::MIN_TRANSACTION_FEE,
        };

        let mut tx = Transaction::new(tx_type, 1);
        tx.sign("test_signature".to_string());

        assert!(tx.validate().is_ok());
    }

    #[test]
    fn test_mint_transaction() {
        let tx_type = TransactionType::Mint {
            recipient: "alice".to_string(),
            amount: 1000 * crate::constants::COIN,
            purchase_proof: "payment_id_123".to_string(),
        };

        let mut tx = Transaction::new(tx_type, 1);
        tx.sign("test_signature".to_string());

        assert!(tx.validate().is_ok());
    }
}
