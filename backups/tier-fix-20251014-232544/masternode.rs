//! Masternode tier definitions and voting power

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MasternodeTier {
    Bronze,   // 1,000 TIME
    Silver,   // 5,000 TIME
    Gold,     // 10,000 TIME
    Platinum, // 50,000 TIME
    Diamond,  // 100,000 TIME
}

impl MasternodeTier {
    pub fn from_collateral(amount: u64) -> Option<Self> {
        let time = amount / crate::TIME_UNIT;
        
        match time {
            1_000..=4_999 => Some(MasternodeTier::Bronze),
            5_000..=9_999 => Some(MasternodeTier::Silver),
            10_000..=49_999 => Some(MasternodeTier::Gold),
            50_000..=99_999 => Some(MasternodeTier::Platinum),
            100_000.. => Some(MasternodeTier::Diamond),
            _ => None,
        }
    }
    
    pub fn voting_power(&self) -> u64 {
        match self {
            MasternodeTier::Bronze => 1,
            MasternodeTier::Silver => 5,
            MasternodeTier::Gold => 10,
            MasternodeTier::Platinum => 50,
            MasternodeTier::Diamond => 100,
        }
    }
    
    pub fn required_collateral(&self) -> u64 {
        match self {
            MasternodeTier::Bronze => 1_000 * crate::TIME_UNIT,
            MasternodeTier::Silver => 5_000 * crate::TIME_UNIT,
            MasternodeTier::Gold => 10_000 * crate::TIME_UNIT,
            MasternodeTier::Platinum => 50_000 * crate::TIME_UNIT,
            MasternodeTier::Diamond => 100_000 * crate::TIME_UNIT,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Masternode {
    pub id: String,
    pub address: String,
    pub tier: MasternodeTier,
    pub collateral: u64,
    pub active: bool,
    pub last_vote: Option<u64>,
}

impl Masternode {
    pub fn voting_power(&self) -> u64 {
        if self.active {
            self.tier.voting_power()
        } else {
            0
        }
    }
}
