//! Core masternode implementation
//!
//! This module contains the canonical [`Masternode`] struct and related types.
//! It handles masternode registration, status management, and lifecycle operations.
//!
//! # Masternode Lifecycle
//!
//! 1. **Pending**: Initial state after registration
//! 2. **Active**: Operational and validating blocks
//! 3. **Offline**: Temporarily unavailable
//! 4. **Slashed**: Penalized for violations
//! 5. **Deregistered**: Permanently removed
//!
//! # Example
//!
//! ```
//! use time_masternode::{Masternode, CollateralTier, MasternodeStatus};
//!
//! let mut mn = Masternode::new(
//!     "mn1".into(),
//!     "pubkey".into(),
//!     CollateralTier::Community,
//!     "192.168.1.1".into(),
//!     24000,
//!     1234567890
//! );
//!
//! assert_eq!(mn.status, MasternodeStatus::Pending);
//! ```

use crate::collateral::CollateralTier;
use crate::error::{MasternodeError, Result};
use crate::reputation::Reputation;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MasternodeStatus {
    /// Waiting for activation
    Pending,
    /// Active and can validate blocks
    Active,
    /// Temporarily offline
    Offline,
    /// Slashed for misbehavior
    Slashed,
    /// Deregistered
    Deregistered,
}

impl std::fmt::Display for MasternodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Active => write!(f, "Active"),
            Self::Offline => write!(f, "Offline"),
            Self::Slashed => write!(f, "Slashed"),
            Self::Deregistered => write!(f, "Deregistered"),
        }
    }
}

/// Core masternode structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Masternode {
    /// Unique identifier
    pub id: String,
    /// Owner's public key
    pub public_key: String,
    /// Collateral tier
    pub tier: CollateralTier,
    /// Current status
    pub status: MasternodeStatus,
    /// Reputation
    pub reputation: Reputation,
    /// Registration timestamp
    pub registered_at: u64,
    /// Last heartbeat timestamp
    pub last_heartbeat: u64,
    /// IP address
    pub ip_address: String,
    /// Port
    pub port: u16,
    /// Total blocks validated
    pub blocks_validated: u64,
    /// Total rewards earned
    pub total_rewards: u64,
}

impl Masternode {
    pub fn new(
        id: String,
        public_key: String,
        tier: CollateralTier,
        ip_address: String,
        port: u16,
        timestamp: u64,
    ) -> Self {
        Self {
            id: id.clone(),
            public_key,
            tier,
            status: MasternodeStatus::Pending,
            reputation: Reputation::new(id, timestamp),
            registered_at: timestamp,
            last_heartbeat: timestamp,
            ip_address,
            port,
            blocks_validated: 0,
            total_rewards: 0,
        }
    }

    /// Activate masternode
    pub fn activate(&mut self, timestamp: u64) -> Result<()> {
        if self.status != MasternodeStatus::Pending {
            return Err(MasternodeError::InvalidStatus(format!(
                "Cannot activate from status: {}",
                self.status
            )));
        }
        self.status = MasternodeStatus::Active;
        self.last_heartbeat = timestamp;
        Ok(())
    }

    /// Update heartbeat
    pub fn heartbeat(&mut self, timestamp: u64) -> Result<()> {
        if self.status == MasternodeStatus::Slashed || self.status == MasternodeStatus::Deregistered
        {
            return Err(MasternodeError::InvalidStatus(format!(
                "Cannot update heartbeat for status: {}",
                self.status
            )));
        }

        self.last_heartbeat = timestamp;

        // Reactivate if was offline
        if self.status == MasternodeStatus::Offline {
            self.status = MasternodeStatus::Active;
        }

        Ok(())
    }

    /// Check if online (heartbeat within last 5 minutes)
    pub fn is_online(&self, current_time: u64) -> bool {
        current_time - self.last_heartbeat < 300
    }

    /// Mark as offline
    pub fn mark_offline(&mut self, timestamp: u64) {
        if self.status == MasternodeStatus::Active {
            self.status = MasternodeStatus::Offline;
            self.reputation.record_block_missed(timestamp);
        }
    }

    /// Slash for misbehavior
    pub fn slash(&mut self, timestamp: u64) {
        self.status = MasternodeStatus::Slashed;
        self.reputation.record_slash(timestamp);
    }

    /// Deregister
    pub fn deregister(&mut self) {
        self.status = MasternodeStatus::Deregistered;
    }

    /// Check if eligible for rewards
    pub fn is_eligible_for_rewards(&self) -> bool {
        self.status == MasternodeStatus::Active && self.reputation.is_eligible()
    }

    /// Record block validation
    pub fn record_block_validation(&mut self, reward: u64, timestamp: u64) {
        self.blocks_validated += 1;
        self.total_rewards += reward;
        self.reputation.record_block_validated(timestamp);
    }

    /// Get info summary
    pub fn info(&self) -> MasternodeInfo {
        MasternodeInfo {
            id: self.id.clone(),
            tier: self.tier,
            status: self.status,
            reputation_score: self.reputation.score,
            uptime: self.reputation.uptime_percentage(),
            blocks_validated: self.blocks_validated,
            total_rewards: self.total_rewards,
        }
    }

    /// Record a violation (affects reputation)
    pub fn record_violation(&mut self, reputation_penalty: i32, timestamp: u64) {
        self.reputation.update_score(reputation_penalty, timestamp);
    }
}

/// Violation tracking state for a masternode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationState {
    pub masternode_id: String,
    /// Number of violations detected
    pub violation_count: u32,
    /// Last violation timestamp
    pub last_violation: Option<u64>,
    /// Consecutive data request failures
    pub consecutive_data_failures: u32,
    /// Last successful data request
    pub last_successful_data_request: Option<u64>,
}

impl ViolationState {
    pub fn new(masternode_id: String) -> Self {
        Self {
            masternode_id,
            violation_count: 0,
            last_violation: None,
            consecutive_data_failures: 0,
            last_successful_data_request: None,
        }
    }

    /// Record a violation
    pub fn record_violation(&mut self, timestamp: u64) {
        self.violation_count += 1;
        self.last_violation = Some(timestamp);
    }

    /// Record a data request failure
    pub fn record_data_failure(&mut self) {
        self.consecutive_data_failures += 1;
    }

    /// Record a successful data request (resets failure counter)
    pub fn record_data_success(&mut self, timestamp: u64) {
        self.consecutive_data_failures = 0;
        self.last_successful_data_request = Some(timestamp);
    }

    /// Reset violation state
    pub fn reset(&mut self) {
        self.violation_count = 0;
        self.last_violation = None;
        self.consecutive_data_failures = 0;
    }
}

/// Masternode information summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasternodeInfo {
    pub id: String,
    pub tier: CollateralTier,
    pub status: MasternodeStatus,
    pub reputation_score: i32,
    pub uptime: f64,
    pub blocks_validated: u64,
    pub total_rewards: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_masternode() {
        let mn = Masternode::new(
            "mn1".to_string(),
            "pubkey123".to_string(),
            CollateralTier::Verified,
            "127.0.0.1".to_string(),
            9999,
            1000,
        );

        assert_eq!(mn.status, MasternodeStatus::Pending);
        assert_eq!(mn.tier, CollateralTier::Verified);
    }

    #[test]
    fn test_activation() {
        let mut mn = Masternode::new(
            "mn1".to_string(),
            "pubkey123".to_string(),
            CollateralTier::Verified,
            "127.0.0.1".to_string(),
            9999,
            1000,
        );

        mn.activate(1001).unwrap();
        assert_eq!(mn.status, MasternodeStatus::Active);
    }

    #[test]
    fn test_is_online() {
        let mut mn = Masternode::new(
            "mn1".to_string(),
            "pubkey123".to_string(),
            CollateralTier::Verified,
            "127.0.0.1".to_string(),
            9999,
            1000,
        );

        mn.activate(1001).unwrap();

        // Within 5 minutes - online
        assert!(mn.is_online(1002));

        // More than 5 minutes - offline
        assert!(!mn.is_online(1000 + 301));
    }
}
