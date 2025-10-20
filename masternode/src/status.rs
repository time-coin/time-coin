//! Masternode status with grace period and sync requirements
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
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    NotSynced,
    Syncing {
        current_block: u64,
        target_block: u64,
    },
    Synced,
}
pub const GRACE_PERIOD_SECS: i64 = 1800;
pub const INACTIVE_THRESHOLD_SECS: i64 = 300;
impl MasternodeStatus {
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
    pub fn can_vote(&self) -> bool {
        self.is_active && self.is_synced() && self.is_online()
    }
    pub fn is_synced(&self) -> bool {
        matches!(self.sync_status, SyncStatus::Synced)
    }
    pub fn is_online(&self) -> bool {
        (current_timestamp() - self.last_heartbeat) < INACTIVE_THRESHOLD_SECS
    }
    pub fn is_within_grace_period(&self) -> bool {
        (current_timestamp() - self.last_heartbeat) < GRACE_PERIOD_SECS
    }
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = current_timestamp();
        if !self.is_active && self.is_within_grace_period() && self.is_synced() {
            self.is_active = true;
        }
    }
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
