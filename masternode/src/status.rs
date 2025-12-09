//! Masternode status with grace period and sync requirements
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Masternode operational status and network information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasternodeStatus {
    /// Public key for verification
    pub public_key: String,
    /// IP address for network connectivity
    pub ip_address: String,
    /// Network port
    pub port: u16,
    /// Block height at registration
    pub registration_block: u64,
    /// Unix timestamp of registration
    pub registration_time: i64,
    /// Last heartbeat timestamp
    pub last_heartbeat: i64,
    /// Whether the masternode is active
    pub is_active: bool,
    /// Current blockchain sync status
    pub sync_status: SyncStatus,
    /// Uptime score (0.0-100.0)
    pub uptime_score: f64,
}

/// Blockchain synchronization status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    /// Not yet started syncing
    NotSynced,
    /// Currently syncing
    Syncing {
        /// Current block height
        current_block: u64,
        /// Target block height
        target_block: u64,
    },
    /// Fully synchronized
    Synced,
}

/// Grace period before marking a masternode as inactive (30 minutes)
pub const GRACE_PERIOD_SECS: i64 = 1800;

/// Threshold for considering a masternode offline (5 minutes)
pub const INACTIVE_THRESHOLD_SECS: i64 = 300;

impl MasternodeStatus {
    /// Creates a new masternode status
    pub fn new(public_key: String, ip_address: String, port: u16, registration_block: u64) -> Self {
        let now = current_timestamp();
        Self {
            public_key,
            ip_address,
            port,
            registration_block,
            registration_time: now,
            last_heartbeat: now,
            is_active: false,
            sync_status: SyncStatus::NotSynced,
            uptime_score: 100.0,
        }
    }
    /// Checks if masternode can participate in voting
    pub fn can_vote(&self) -> bool {
        self.is_active && self.is_synced() && self.is_online()
    }

    /// Check if masternode can vote at a specific block height
    /// This enforces vote maturity delays to prevent instant takeover by newly coordinated malicious nodes
    pub fn can_vote_at_height(&self, current_block: u64, tier: &crate::CollateralTier) -> bool {
        if !self.can_vote() {
            return false;
        }

        // Calculate blocks since registration
        let blocks_since_registration = current_block.saturating_sub(self.registration_block);

        // Check if maturity period has passed
        blocks_since_registration >= tier.vote_maturity_blocks()
    }
    /// Checks if blockchain is fully synchronized
    pub fn is_synced(&self) -> bool {
        matches!(self.sync_status, SyncStatus::Synced)
    }
    /// Checks if masternode is currently online
    pub fn is_online(&self) -> bool {
        (current_timestamp() - self.last_heartbeat) < INACTIVE_THRESHOLD_SECS
    }
    /// Checks if masternode is within grace period
    pub fn is_within_grace_period(&self) -> bool {
        (current_timestamp() - self.last_heartbeat) < GRACE_PERIOD_SECS
    }
    /// Updates the last heartbeat timestamp
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = current_timestamp();
        if !self.is_active && self.is_within_grace_period() && self.is_synced() {
            self.is_active = true;
        }
    }
    /// Updates the blockchain sync status
    pub fn update_sync_status(&mut self, status: SyncStatus) {
        self.sync_status = status.clone();
        if matches!(status, SyncStatus::Synced) && !self.is_active {
            self.is_active = true;
        }
    }
}
fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Vote maturity configuration for emergency updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteMaturityConfig {
    /// Maturity period in blocks for Community tier
    pub community_maturity_blocks: u64,
    /// Maturity period in blocks for Verified tier
    pub verified_maturity_blocks: u64,
    /// Maturity period in blocks for Professional tier
    pub professional_maturity_blocks: u64,
}

impl Default for VoteMaturityConfig {
    fn default() -> Self {
        Self {
            community_maturity_blocks: 1,
            verified_maturity_blocks: 3,
            professional_maturity_blocks: 10,
        }
    }
}

impl VoteMaturityConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Get maturity period for a specific tier
    pub fn get_maturity_blocks(&self, tier: &crate::CollateralTier) -> u64 {
        match tier {
            crate::CollateralTier::Community => self.community_maturity_blocks,
            crate::CollateralTier::Verified => self.verified_maturity_blocks,
            crate::CollateralTier::Professional => self.professional_maturity_blocks,
        }
    }

    /// Admin function: Update maturity period for Community tier
    pub fn set_community_maturity(&mut self, blocks: u64) {
        self.community_maturity_blocks = blocks;
    }

    /// Admin function: Update maturity period for Verified tier
    pub fn set_verified_maturity(&mut self, blocks: u64) {
        self.verified_maturity_blocks = blocks;
    }

    /// Admin function: Update maturity period for Professional tier
    pub fn set_professional_maturity(&mut self, blocks: u64) {
        self.professional_maturity_blocks = blocks;
    }

    /// Admin function: Emergency override to disable all maturity checks (set to 0)
    pub fn emergency_disable_maturity(&mut self) {
        self.community_maturity_blocks = 0;
        self.verified_maturity_blocks = 0;
        self.professional_maturity_blocks = 0;
    }

    /// Admin function: Emergency override to set custom maturity for all tiers
    pub fn emergency_set_all_maturity(&mut self, blocks: u64) {
        self.community_maturity_blocks = blocks;
        self.verified_maturity_blocks = blocks;
        self.professional_maturity_blocks = blocks;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CollateralTier;

    #[test]
    fn test_can_vote_at_height_community() {
        let mut status =
            MasternodeStatus::new("pubkey".to_string(), "127.0.0.1".to_string(), 9000, 100);
        status.is_active = true;
        status.sync_status = SyncStatus::Synced;

        let tier = CollateralTier::Community;

        // At registration block, cannot vote (needs 1 block)
        assert!(!status.can_vote_at_height(100, &tier));

        // At block 101, can vote (1 block has passed)
        assert!(status.can_vote_at_height(101, &tier));
    }

    #[test]
    fn test_can_vote_at_height_verified() {
        let mut status =
            MasternodeStatus::new("pubkey".to_string(), "127.0.0.1".to_string(), 9000, 100);
        status.is_active = true;
        status.sync_status = SyncStatus::Synced;

        let tier = CollateralTier::Verified;

        // At registration block, cannot vote (needs 3 blocks)
        assert!(!status.can_vote_at_height(100, &tier));

        // At block 102, cannot vote (only 2 blocks)
        assert!(!status.can_vote_at_height(102, &tier));

        // At block 103, can vote (3 blocks have passed)
        assert!(status.can_vote_at_height(103, &tier));
    }

    #[test]
    fn test_can_vote_at_height_professional() {
        let mut status =
            MasternodeStatus::new("pubkey".to_string(), "127.0.0.1".to_string(), 9000, 100);
        status.is_active = true;
        status.sync_status = SyncStatus::Synced;

        let tier = CollateralTier::Professional;

        // At registration block, cannot vote (needs 10 blocks)
        assert!(!status.can_vote_at_height(100, &tier));

        // At block 109, cannot vote (only 9 blocks)
        assert!(!status.can_vote_at_height(109, &tier));

        // At block 110, can vote (10 blocks have passed)
        assert!(status.can_vote_at_height(110, &tier));
    }

    #[test]
    fn test_can_vote_at_height_requires_active() {
        let mut status =
            MasternodeStatus::new("pubkey".to_string(), "127.0.0.1".to_string(), 9000, 100);
        // Not active
        status.sync_status = SyncStatus::Synced;

        let tier = CollateralTier::Community;

        // Even after maturity period, cannot vote if not active
        assert!(!status.can_vote_at_height(110, &tier));
    }

    #[test]
    fn test_vote_maturity_config_defaults() {
        let config = VoteMaturityConfig::default();
        assert_eq!(config.community_maturity_blocks, 1);
        assert_eq!(config.verified_maturity_blocks, 3);
        assert_eq!(config.professional_maturity_blocks, 10);
    }

    #[test]
    fn test_vote_maturity_config_get() {
        let config = VoteMaturityConfig::default();
        assert_eq!(config.get_maturity_blocks(&CollateralTier::Community), 1);
        assert_eq!(config.get_maturity_blocks(&CollateralTier::Verified), 3);
        assert_eq!(
            config.get_maturity_blocks(&CollateralTier::Professional),
            10
        );
    }

    #[test]
    fn test_vote_maturity_config_admin_updates() {
        let mut config = VoteMaturityConfig::default();

        config.set_community_maturity(5);
        assert_eq!(config.community_maturity_blocks, 5);

        config.set_verified_maturity(7);
        assert_eq!(config.verified_maturity_blocks, 7);

        config.set_professional_maturity(15);
        assert_eq!(config.professional_maturity_blocks, 15);
    }

    #[test]
    fn test_vote_maturity_config_emergency_disable() {
        let mut config = VoteMaturityConfig::default();
        config.emergency_disable_maturity();

        assert_eq!(config.community_maturity_blocks, 0);
        assert_eq!(config.verified_maturity_blocks, 0);
        assert_eq!(config.professional_maturity_blocks, 0);
    }

    #[test]
    fn test_vote_maturity_config_emergency_set_all() {
        let mut config = VoteMaturityConfig::default();
        config.emergency_set_all_maturity(20);

        assert_eq!(config.community_maturity_blocks, 20);
        assert_eq!(config.verified_maturity_blocks, 20);
        assert_eq!(config.professional_maturity_blocks, 20);
    }
}
