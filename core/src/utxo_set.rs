//! UTXO Set Management for TIME Coin
//! 
//! Tracks all unspent transaction outputs in the blockchain

use crate::transaction::{Transaction, OutPoint, TxOutput, TransactionError};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Manages the UTXO set (all unspent transaction outputs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UTXOSet {
    /// Map from OutPoint to TxOutput
    utxos: HashMap<OutPoint, TxOutput>,
    /// Total supply currently in circulation
    total_supply: u64,
}

impl UTXOSet {
    /// Create a new empty UTXO set
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
            total_supply: 0,
        }
    }

    /// Get a UTXO by outpoint
    pub fn get(&self, outpoint: &OutPoint) -> Option<&TxOutput> {
        self.utxos.get(outpoint)
    }

    /// Check if a UTXO exists
    pub fn contains(&self, outpoint: &OutPoint) -> bool {
        self.utxos.contains_key(outpoint)
    }

    /// Add a new UTXO
    pub fn add_utxo(&mut self, outpoint: OutPoint, output: TxOutput) {
        self.total_supply = self.total_supply.saturating_add(output.amount);
        self.utxos.insert(outpoint, output);
    }

    /// Remove a spent UTXO
    /// Get all UTXOs for a specific address
    pub fn get_utxos_by_address(&self, address: &str) -> Vec<(OutPoint, &TxOutput)> {
        self.utxos
            .iter()
            .filter(|(_, output)| output.address == address)
            .map(|(outpoint, output)| (outpoint.clone(), output))
            .collect()
    }


    pub fn remove_utxo(&mut self, outpoint: &OutPoint) -> Option<TxOutput> {
        if let Some(output) = self.utxos.remove(outpoint) {
            self.total_supply = self.total_supply.saturating_sub(output.amount);
            Some(output)
        } else {
            None
        }
    }

    /// Apply a transaction to the UTXO set
    pub fn apply_transaction(&mut self, tx: &Transaction) -> Result<(), TransactionError> {
        // First validate that all inputs exist (except for coinbase)
        if !tx.is_coinbase() {
            for input in &tx.inputs {
                if !self.contains(&input.previous_output) {
                    return Err(TransactionError::InvalidInput);
                }
            }
        }

        // Remove spent UTXOs
        for input in &tx.inputs {
            self.remove_utxo(&input.previous_output)
                .ok_or(TransactionError::InvalidInput)?;
        }

        // Add new UTXOs
        for (vout, output) in tx.outputs.iter().enumerate() {
            let outpoint = OutPoint::new(tx.txid.clone(), vout as u32);
            self.add_utxo(outpoint, output.clone());
        }

        Ok(())
    }

    /// Revert a transaction (for blockchain reorganization)
    pub fn revert_transaction(&mut self, tx: &Transaction) -> Result<(), TransactionError> {
        // Remove UTXOs created by this transaction
        for (vout, _output) in tx.outputs.iter().enumerate() {
            let outpoint = OutPoint::new(tx.txid.clone(), vout as u32);
            self.remove_utxo(&outpoint);
        }

        // Note: To fully revert, we'd need to restore the spent UTXOs
        // This requires keeping transaction history or undo data
        // For now, this is a simplified version

        Ok(())
    }

    /// Get all UTXOs for a specific address
    pub fn get_utxos_for_address(&self, address: &str) -> Vec<(OutPoint, TxOutput)> {
        self.utxos
            .iter()
            .filter(|(_, output)| output.address == address)
            .map(|(outpoint, output)| (outpoint.clone(), output.clone()))
            .collect()
    }

    /// Get balance for an address
    pub fn get_balance(&self, address: &str) -> u64 {
        self.utxos
            .values()
            .filter(|output| output.address == address)
            .map(|output| output.amount)
            .sum()
    }

    /// Get total number of UTXOs
    pub fn len(&self) -> usize {
        self.utxos.len()
    }

    /// Check if UTXO set is empty
    pub fn is_empty(&self) -> bool {
        self.utxos.is_empty()
    }

    /// Get total supply
    pub fn total_supply(&self) -> u64 {
        self.total_supply
    }

    /// Get a reference to the underlying map
    pub fn utxos(&self) -> &HashMap<OutPoint, TxOutput> {
        &self.utxos
    }

    /// Validate the entire UTXO set consistency
    pub fn validate(&self) -> Result<(), TransactionError> {
        let calculated_supply: u64 = self.utxos.values().map(|o| o.amount).sum();
        
        if calculated_supply != self.total_supply {
            return Err(TransactionError::InvalidAmount);
        }

        Ok(())
    }

    /// Create a snapshot for rollback
    pub fn snapshot(&self) -> UTXOSetSnapshot {
        UTXOSetSnapshot {
            utxos: self.utxos.clone(),
            total_supply: self.total_supply,
        }
    }

    /// Restore from a snapshot
    pub fn restore(&mut self, snapshot: UTXOSetSnapshot) {
        self.utxos = snapshot.utxos;
        self.total_supply = snapshot.total_supply;
    }
}

impl Default for UTXOSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of UTXO set for rollback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UTXOSetSnapshot {
    utxos: HashMap<OutPoint, TxOutput>,
    total_supply: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::TxInput;

    #[test]
    fn test_utxo_set_basic_operations() {
        let mut utxo_set = UTXOSet::new();
        
        let outpoint = OutPoint::new("tx1".to_string(), 0);
        let output = TxOutput::new(1000, "addr1".to_string());
        
        utxo_set.add_utxo(outpoint.clone(), output.clone());
        
        assert_eq!(utxo_set.len(), 1);
        assert_eq!(utxo_set.total_supply(), 1000);
        assert!(utxo_set.contains(&outpoint));
        
        utxo_set.remove_utxo(&outpoint);
        
        assert_eq!(utxo_set.len(), 0);
        assert_eq!(utxo_set.total_supply(), 0);
    }

    #[test]
    fn test_apply_transaction() {
        let mut utxo_set = UTXOSet::new();
        
        // Create initial UTXO
        let prev_outpoint = OutPoint::new("prev_tx".to_string(), 0);
        let prev_output = TxOutput::new(2000, "addr1".to_string());
        utxo_set.add_utxo(prev_outpoint.clone(), prev_output);
        
        // Create transaction spending it
        let input = TxInput::new("prev_tx".to_string(), 0, vec![], vec![]);
        let outputs = vec![
            TxOutput::new(1500, "addr2".to_string()),
            TxOutput::new(450, "addr1".to_string()), // Change
        ];
        let tx = Transaction::new(vec![input], outputs);
        
        utxo_set.apply_transaction(&tx).unwrap();
        
        // Old UTXO should be gone
        assert!(!utxo_set.contains(&prev_outpoint));
        
        // New UTXOs should exist
        let new_outpoint1 = OutPoint::new(tx.txid.clone(), 0);
        let new_outpoint2 = OutPoint::new(tx.txid.clone(), 1);
        assert!(utxo_set.contains(&new_outpoint1));
        assert!(utxo_set.contains(&new_outpoint2));
        
        // Total supply should remain same (minus implicit fee)
        assert_eq!(utxo_set.total_supply(), 1950);
    }

    #[test]
    fn test_get_balance() {
        let mut utxo_set = UTXOSet::new();
        
        utxo_set.add_utxo(
            OutPoint::new("tx1".to_string(), 0),
            TxOutput::new(1000, "addr1".to_string())
        );
        utxo_set.add_utxo(
            OutPoint::new("tx2".to_string(), 0),
            TxOutput::new(500, "addr1".to_string())
        );
        utxo_set.add_utxo(
            OutPoint::new("tx3".to_string(), 0),
            TxOutput::new(2000, "addr2".to_string())
        );
        
        assert_eq!(utxo_set.get_balance("addr1"), 1500);
        assert_eq!(utxo_set.get_balance("addr2"), 2000);
        assert_eq!(utxo_set.get_balance("addr3"), 0);
    }
}
