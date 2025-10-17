//! Blockchain state management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub address: String,
    pub balance: u64,
    pub nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainState {
    pub accounts: HashMap<String, Account>,
    pub total_supply: u64,
    pub current_block: u64,
}

impl ChainState {
    pub fn new() -> Self {
        ChainState {
            accounts: HashMap::new(),
            total_supply: 0,
            current_block: 0,
        }
    }

    pub fn get_balance(&self, address: &str) -> u64 {
        self.accounts
            .get(address)
            .map(|acc| acc.balance)
            .unwrap_or(0)
    }

    pub fn transfer(&mut self, from: &str, to: &str, amount: u64, fee: u64) -> Result<(), String> {
        // Get sender account
        let sender = self.accounts.get_mut(from)
            .ok_or("Sender account not found")?;

        // Check balance
        let total = amount.checked_add(fee)
            .ok_or("Amount overflow")?;
        if sender.balance < total {
            return Err("Insufficient balance".to_string());
        }

        // Deduct from sender
        sender.balance -= total;
        sender.nonce += 1;

        // Add to recipient
        let recipient = self.accounts.entry(to.to_string())
            .or_insert(Account {
                address: to.to_string(),
                balance: 0,
                nonce: 0,
            });
        recipient.balance += amount;

        Ok(())
    }

    pub fn mint(&mut self, recipient: &str, amount: u64) -> Result<(), String> {
        // Check max supply
        let new_supply = self.total_supply.checked_add(amount)
            .ok_or("Supply overflow")?;
        if new_supply > crate::constants::MAX_SUPPLY {
            return Err("Max supply exceeded".to_string());
        }

        // Add to recipient
        let account = self.accounts.entry(recipient.to_string())
            .or_insert(Account {
                address: recipient.to_string(),
                balance: 0,
                nonce: 0,
            });
        account.balance += amount;
        self.total_supply += amount;

        Ok(())
    }

    pub fn increment_block(&mut self) {
        self.current_block += 1;
    }
}

impl Default for ChainState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mint() {
        let mut state = ChainState::new();
        
        let result = state.mint("alice", 1000);
        assert!(result.is_ok());
        assert_eq!(state.get_balance("alice"), 1000);
        assert_eq!(state.total_supply, 1000);
    }

    #[test]
    fn test_transfer() {
        let mut state = ChainState::new();
        
        // Mint to alice
        state.mint("alice", 1000).unwrap();
        
        // Transfer to bob
        let result = state.transfer("alice", "bob", 500, 10);
        assert!(result.is_ok());
        assert_eq!(state.get_balance("alice"), 490);
        assert_eq!(state.get_balance("bob"), 500);
    }
}
