//! Masternode status tracking with grace period and sync requirements

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasternodeStatus {
    pub public_key: String,
    pub ip_address: String,
    pub port: u16,
    pub registration_block: u64,
    pub registration_time: i64,
    pub last_heartbeat: i64,
    pub is_active: bool,
    pub sync_status: SyncStatus,
    pub uptime_score: f64,
    pub total_votes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    NotSynced,
    Syncing { current_block: u64, target_block: u64 },
    Synced,
}

/// Grace period: 30 minutes for updates
pub const GRACE_PERIOD_SECS: i64 = 1800;

/// Time before marking masternode inactive (5 minutes)
pub const INACTIVE_THRESHOLD_SECS: i64 = 300;

impl MasternodeStatus {
    pub fn new(
        public_key: String,
        ip_address: String,
        port: u16,
        registration_block: u64,
    ) -> Self {
        let now = current_timestamp();
        Self {
            public_key,
            ip_address,
            port,
            registration_block,
            registration_time: now,
            last_heartbeat: now,
            is_active: false, // Starts inactive until synced
            sync_status: SyncStatus::NotSynced,
            uptime_score: 100.0,
            total_votes: 0,
        }
    }
    
    /// Can this masternode participate in voting?
    pub fn can_vote(&self) -> bool {
        self.is_active 
            && self.is_synced()
            && self.is_online()
    }
    
    /// Is the masternode fully synced?
    pub fn is_synced(&self) -> bool {
        matches!(self.sync_status, SyncStatus::Synced)
    }
    
    /// Is the masternode currently online?
    pub fn is_online(&self) -> bool {
        let now = current_timestamp();
        (now - self.last_heartbeat) < INACTIVE_THRESHOLD_SECS
    }
    
    /// Is the masternode within grace period after going offline?
    pub fn is_within_grace_period(&self) -> bool {
        let now = current_timestamp();
        let offline_duration = now - self.last_heartbeat;
        offline_duration < GRACE_PERIOD_SECS
    }
    
    /// Update last seen timestamp
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = current_timestamp();
        
        // Reactivate if within grace period
        if !self.is_active && self.is_within_grace_period() && self.is_synced() {
            self.is_active = true;
        }
    }
    
    /// Update sync status
    pub fn update_sync_status(&mut self, status: SyncStatus) {
        self.sync_status = status.clone();
        
        // Activate once synced
        if matches!(status, SyncStatus::Synced) && !self.is_active {
            self.is_active = true;
        }
    }
    
    /// Calculate uptime percentage
    pub fn calculate_uptime(&mut self) {
        let total_time = current_timestamp() - self.registration_time;
        if total_time > 0 {
            let online_time = if self.is_online() {
                total_time
            } else {
                total_time - (current_timestamp() - self.last_heartbeat)
            };
            self.uptime_score = (online_time as f64 / total_time as f64) * 100.0;
        }
    }
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_masternode_not_active() {
        let mn = MasternodeStatus::new(
            "pubkey".to_string(),
            "1.2.3.4".to_string(),
            24100,
            1,
        );
        
        assert!(!mn.is_active);
        assert!(!mn.can_vote());
        assert!(!mn.is_synced());
    }

    #[test]
    fn test_can_vote_requires_sync_and_active() {
        let mut mn = MasternodeStatus::new(
            "pubkey".to_string(),
            "1.2.3.4".to_string(),
            24100,
            1,
        );
        
        // Not synced - can't vote
        assert!(!mn.can_vote());
        
        // Mark as synced
        mn.update_sync_status(SyncStatus::Synced);
        mn.update_heartbeat();
        
        // Now can vote
        assert!(mn.can_vote());
    }
}
