//! Masternode type definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type MasternodeId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MasternodeStatus {
    Registered,
    Active,
    Inactive,
    Banned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub ip_address: String,
    pub port: u16,
    pub protocol_version: u32,
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Masternode {
    pub id: MasternodeId,
    pub owner: String,
    pub collateral: u64,
    pub tier: crate::collateral::CollateralTier,
    pub status: MasternodeStatus,
    pub network_info: NetworkInfo,
    pub registered_at: DateTime<Utc>,
    pub activated_at: Option<DateTime<Utc>>,
    pub reputation: u64,
}

impl Masternode {
    pub fn new(
        owner: String,
        collateral: u64,
        tier: crate::collateral::CollateralTier,
        network_info: NetworkInfo,
    ) -> Self {
        Masternode {
            id: Uuid::new_v4().to_string(),
            owner,
            collateral,
            tier,
            status: MasternodeStatus::Registered,
            network_info,
            registered_at: Utc::now(),
            activated_at: None,
            reputation: 100,
        }
    }

    pub fn activate(&mut self) {
        self.status = MasternodeStatus::Active;
        self.activated_at = Some(Utc::now());
    }

    pub fn deactivate(&mut self) {
        self.status = MasternodeStatus::Inactive;
    }

    pub fn is_active(&self) -> bool {
        self.status == MasternodeStatus::Active
    }

    pub fn voting_power(&self) -> u64 {
        self.tier.voting_multiplier()
    }
}
