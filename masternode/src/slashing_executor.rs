//! Slashing execution with treasury integration
//!
//! This module coordinates the complete slashing workflow:
//! 1. Calculate slashing amount
//! 2. Deduct collateral from masternode
//! 3. Transfer slashed funds to treasury
//! 4. Record slashing event
//! 5. Publish event for monitoring
#![allow(missing_docs)]

use crate::slashing::SlashingRecord;
use serde::{Deserialize, Serialize};

/// Slashing event for monitoring and auditing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingEvent {
    /// The slashing record
    pub record: SlashingRecord,

    /// Whether treasury transfer was successful
    pub treasury_transfer_success: bool,

    /// Transaction ID of the treasury transfer (if successful)
    pub treasury_tx_id: Option<String>,

    /// Timestamp when event was published
    pub event_timestamp: u64,
}

impl SlashingEvent {
    pub fn new(
        record: SlashingRecord,
        treasury_transfer_success: bool,
        treasury_tx_id: Option<String>,
        event_timestamp: u64,
    ) -> Self {
        Self {
            record,
            treasury_transfer_success,
            treasury_tx_id,
            event_timestamp,
        }
    }

    /// Create an event with successful treasury transfer
    pub fn with_success(
        record: SlashingRecord,
        treasury_tx_id: String,
        event_timestamp: u64,
    ) -> Self {
        Self::new(record, true, Some(treasury_tx_id), event_timestamp)
    }

    /// Create an event with failed treasury transfer
    pub fn with_failure(record: SlashingRecord, event_timestamp: u64) -> Self {
        Self::new(record, false, None, event_timestamp)
    }
}

/// Treasury transfer result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryTransfer {
    /// Amount transferred
    pub amount: u64,

    /// Transaction ID
    pub tx_id: String,

    /// Source (slashed masternode address)
    pub source: String,

    /// Reason for transfer
    pub reason: String,

    /// Timestamp of transfer
    pub timestamp: u64,
}

/// Slashing executor that coordinates the full slashing workflow
pub struct SlashingExecutor {
    /// List of published slashing events
    events: Vec<SlashingEvent>,
}

impl SlashingExecutor {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Execute complete slashing workflow
    /// This is the main entry point for slashing execution
    pub fn execute_slashing(
        &mut self,
        record: SlashingRecord,
        timestamp: u64,
    ) -> Result<SlashingEvent, String> {
        // In a real implementation, this would:
        // 1. Create a transaction to transfer slashed funds to treasury
        // 2. Submit the transaction to the network
        // 3. Wait for confirmation
        // 4. Return the transaction ID
        //
        // For now, we simulate the treasury transfer
        let treasury_tx_id = self.transfer_to_treasury(&record)?;

        // Create and publish event
        let event = SlashingEvent::with_success(record, treasury_tx_id, timestamp);
        self.events.push(event.clone());

        Ok(event)
    }

    /// Transfer slashed funds to treasury
    /// Returns transaction ID on success
    fn transfer_to_treasury(&self, record: &SlashingRecord) -> Result<String, String> {
        // Validate the slashing record
        if record.amount == 0 {
            return Err("Cannot transfer zero amount to treasury".to_string());
        }

        // In a real implementation, this would:
        // 1. Create a transaction moving funds from masternode collateral to treasury
        // 2. Sign the transaction
        // 3. Submit to the network
        // 4. Return the transaction ID
        //
        // For now, we simulate by generating a transaction ID
        let tx_id = format!("treasury-transfer-{}", record.id);

        Ok(tx_id)
    }

    /// Get all published slashing events
    pub fn get_events(&self) -> &[SlashingEvent] {
        &self.events
    }

    /// Get events for a specific masternode
    pub fn get_events_for_masternode(&self, masternode_id: &str) -> Vec<&SlashingEvent> {
        self.events
            .iter()
            .filter(|e| e.record.masternode_id == masternode_id)
            .collect()
    }

    /// Get total amount slashed across all events
    pub fn total_slashed(&self) -> u64 {
        self.events.iter().map(|e| e.record.amount).sum()
    }

    /// Get total amount successfully transferred to treasury
    pub fn total_transferred_to_treasury(&self) -> u64 {
        self.events
            .iter()
            .filter(|e| e.treasury_transfer_success)
            .map(|e| e.record.amount)
            .sum()
    }
}

impl Default for SlashingExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slashing::Violation;

    fn create_test_record() -> SlashingRecord {
        SlashingRecord::new(
            "slash-1".to_string(),
            "node-1".to_string(),
            Violation::InvalidBlock {
                block_height: 1000,
                reason: "invalid tx".to_string(),
            },
            5_000_000,  // 0.05 TIME
            95_000_000, // 0.95 TIME remaining
            1234567890,
            1000,
        )
    }

    #[test]
    fn test_execute_slashing() {
        let mut executor = SlashingExecutor::new();
        let record = create_test_record();

        let event = executor
            .execute_slashing(record.clone(), 1234567890)
            .unwrap();

        assert_eq!(event.record.amount, 5_000_000);
        assert!(event.treasury_transfer_success);
        assert!(event.treasury_tx_id.is_some());
        assert_eq!(executor.get_events().len(), 1);
    }

    #[test]
    fn test_slashing_event_creation() {
        let record = create_test_record();

        let event = SlashingEvent::with_success(record.clone(), "tx-123".to_string(), 1234567890);

        assert!(event.treasury_transfer_success);
        assert_eq!(event.treasury_tx_id, Some("tx-123".to_string()));

        let event = SlashingEvent::with_failure(record, 1234567890);
        assert!(!event.treasury_transfer_success);
        assert_eq!(event.treasury_tx_id, None);
    }

    #[test]
    fn test_get_events_for_masternode() {
        let mut executor = SlashingExecutor::new();

        let record1 = create_test_record();
        executor.execute_slashing(record1, 1234567890).unwrap();

        let mut record2 = create_test_record();
        record2.masternode_id = "node-2".to_string();
        executor.execute_slashing(record2, 1234567891).unwrap();

        let events = executor.get_events_for_masternode("node-1");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].record.masternode_id, "node-1");
    }

    #[test]
    fn test_total_slashed() {
        let mut executor = SlashingExecutor::new();

        let record1 = create_test_record();
        executor.execute_slashing(record1, 1234567890).unwrap();

        let record2 = create_test_record();
        executor.execute_slashing(record2, 1234567891).unwrap();

        assert_eq!(executor.total_slashed(), 10_000_000);
        assert_eq!(executor.total_transferred_to_treasury(), 10_000_000);
    }
}
