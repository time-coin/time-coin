use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transaction {
    Transfer {
        from: String,
        to: String,
        amount: u64,
        fee: u64,
    },
    Mint {
        recipient: String,
        amount: u64,
    },
}

impl Transaction {
    pub fn new_transfer(from: String, to: String, amount: u64, fee: u64) -> Self {
        Transaction::Transfer { from, to, amount, fee }
    }

    pub fn new_mint(recipient: String, amount: u64) -> Self {
        Transaction::Mint { recipient, amount }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_creation() {
        let tx = Transaction::new_transfer(
            "alice".to_string(),
            "bob".to_string(),
            100,
            1,
        );
        
        match tx {
            Transaction::Transfer { amount, .. } => assert_eq!(amount, 100),
            _ => panic!("Expected Transfer"),
        }
    }
}
