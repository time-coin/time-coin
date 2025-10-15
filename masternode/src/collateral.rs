//! Masternode collateral tier management

use serde::{Deserialize, Serialize};
use crate::error::{MasternodeError, Result};

/// TIME unit (1 TIME = 100,000,000 smallest units)
pub const TIME_UNIT: u64 = 100_000_000;

/// Collateral tiers for masternodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CollateralTier {
    /// 1,000 TIME - Entry tier
    Bronze,
    /// 5,000 TIME - Mid tier
    Silver,
    /// 10,000 TIME - High tier
    Gold,
    /// 50,000 TIME - Premium tier
    Platinum,
    /// 100,000 TIME - Elite tier
    Diamond,
}

impl CollateralTier {
    /// Get required collateral amount in smallest units
    pub fn required_collateral(&self) -> u64 {
        match self {
            Self::Bronze => 1_000 * TIME_UNIT,
            Self::Silver => 5_000 * TIME_UNIT,
            Self::Gold => 10_000 * TIME_UNIT,
            Self::Platinum => 50_000 * TIME_UNIT,
            Self::Diamond => 100_000 * TIME_UNIT,
        }
    }

    /// Get required collateral in TIME (human readable)
    pub fn required_collateral_time(&self) -> u64 {
        match self {
            Self::Bronze => 1_000,
            Self::Silver => 5_000,
            Self::Gold => 10_000,
            Self::Platinum => 50_000,
            Self::Diamond => 100_000,
        }
    }

    /// Get voting power multiplier
    pub fn voting_power(&self) -> u64 {
        match self {
            Self::Bronze => 1,
            Self::Silver => 5,
            Self::Gold => 10,
            Self::Platinum => 50,
            Self::Diamond => 100,
        }
    }

    /// Get reward share multiplier (higher tiers get more)
    pub fn reward_multiplier(&self) -> f64 {
        match self {
            Self::Bronze => 1.0,
            Self::Silver => 5.2,    // Slightly more than 5x
            Self::Gold => 10.5,     // Slightly more than 10x
            Self::Platinum => 52.5, // Slightly more than 50x
            Self::Diamond => 105.0, // Slightly more than 100x
        }
    }

    /// Get APY range for tier (percentage)
    pub fn apy_range(&self) -> (f64, f64) {
        match self {
            Self::Bronze => (18.0, 22.0),
            Self::Silver => (20.0, 24.0),
            Self::Gold => (22.0, 26.0),
            Self::Platinum => (24.0, 28.0),
            Self::Diamond => (26.0, 30.0),
        }
    }

    /// Parse tier from collateral amount
    pub fn from_collateral(amount: u64) -> Result<Self> {
        if amount >= Self::Diamond.required_collateral() {
            Ok(Self::Diamond)
        } else if amount >= Self::Platinum.required_collateral() {
            Ok(Self::Platinum)
        } else if amount >= Self::Gold.required_collateral() {
            Ok(Self::Gold)
        } else if amount >= Self::Silver.required_collateral() {
            Ok(Self::Silver)
        } else if amount >= Self::Bronze.required_collateral() {
            Ok(Self::Bronze)
        } else {
            Err(MasternodeError::InsufficientCollateral {
                required: Self::Bronze.required_collateral(),
                provided: amount,
            })
        }
    }

    /// Parse tier from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "bronze" => Ok(Self::Bronze),
            "silver" => Ok(Self::Silver),
            "gold" => Ok(Self::Gold),
            "platinum" => Ok(Self::Platinum),
            "diamond" => Ok(Self::Diamond),
            _ => Err(MasternodeError::InvalidTier(s.to_string())),
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Bronze => "Bronze",
            Self::Silver => "Silver",
            Self::Gold => "Gold",
            Self::Platinum => "Platinum",
            Self::Diamond => "Diamond",
        }
    }

    /// Get all tiers
    pub fn all() -> Vec<Self> {
        vec![
            Self::Bronze,
            Self::Silver,
            Self::Gold,
            Self::Platinum,
            Self::Diamond,
        ]
    }
}

impl std::fmt::Display for CollateralTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Manages masternode collateral
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollateralManager {
    /// Locked collateral by masternode ID
    locked: std::collections::HashMap<String, LockedCollateral>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedCollateral {
    pub masternode_id: String,
    pub amount: u64,
    pub tier: CollateralTier,
    pub locked_at: u64,
    pub lock_tx_hash: String,
}

impl CollateralManager {
    pub fn new() -> Self {
        Self {
            locked: std::collections::HashMap::new(),
        }
    }

    /// Lock collateral for a masternode
    pub fn lock_collateral(
        &mut self,
        masternode_id: String,
        amount: u64,
        lock_tx_hash: String,
        timestamp: u64,
    ) -> Result<CollateralTier> {
        // Determine tier
        let tier = CollateralTier::from_collateral(amount)?;

        // Check if already locked
        if self.locked.contains_key(&masternode_id) {
            return Err(MasternodeError::AlreadyRegistered(masternode_id));
        }

        // Lock collateral
        let locked = LockedCollateral {
            masternode_id: masternode_id.clone(),
            amount,
            tier,
            locked_at: timestamp,
            lock_tx_hash,
        };

        self.locked.insert(masternode_id, locked);

        Ok(tier)
    }

    /// Unlock collateral (when masternode deregisters)
    pub fn unlock_collateral(&mut self, masternode_id: &str) -> Result<u64> {
        let locked = self.locked.remove(masternode_id)
            .ok_or_else(|| MasternodeError::NotFound(masternode_id.to_string()))?;
        
        Ok(locked.amount)
    }

    /// Get locked collateral info
    pub fn get_collateral(&self, masternode_id: &str) -> Option<&LockedCollateral> {
        self.locked.get(masternode_id)
    }

    /// Get tier for masternode
    pub fn get_tier(&self, masternode_id: &str) -> Result<CollateralTier> {
        self.locked
            .get(masternode_id)
            .map(|c| c.tier)
            .ok_or_else(|| MasternodeError::NotFound(masternode_id.to_string()))
    }

    /// Get total locked collateral
    pub fn total_locked(&self) -> u64 {
        self.locked.values().map(|c| c.amount).sum()
    }

    /// Count masternodes by tier
    pub fn count_by_tier(&self) -> std::collections::HashMap<CollateralTier, usize> {
        let mut counts = std::collections::HashMap::new();
        for locked in self.locked.values() {
            *counts.entry(locked.tier).or_insert(0) += 1;
        }
        counts
    }
}

impl Default for CollateralManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collateral_tiers() {
        assert_eq!(CollateralTier::Bronze.required_collateral_time(), 1_000);
        assert_eq!(CollateralTier::Silver.required_collateral_time(), 5_000);
        assert_eq!(CollateralTier::Gold.required_collateral_time(), 10_000);
        assert_eq!(CollateralTier::Platinum.required_collateral_time(), 50_000);
        assert_eq!(CollateralTier::Diamond.required_collateral_time(), 100_000);
    }

    #[test]
    fn test_voting_power() {
        assert_eq!(CollateralTier::Bronze.voting_power(), 1);
        assert_eq!(CollateralTier::Silver.voting_power(), 5);
        assert_eq!(CollateralTier::Gold.voting_power(), 10);
        assert_eq!(CollateralTier::Platinum.voting_power(), 50);
        assert_eq!(CollateralTier::Diamond.voting_power(), 100);
    }

    #[test]
    fn test_from_collateral() {
        let bronze = CollateralTier::from_collateral(1_000 * TIME_UNIT).unwrap();
        assert_eq!(bronze, CollateralTier::Bronze);

        let diamond = CollateralTier::from_collateral(100_000 * TIME_UNIT).unwrap();
        assert_eq!(diamond, CollateralTier::Diamond);

        // Insufficient should error
        let result = CollateralTier::from_collateral(500 * TIME_UNIT);
        assert!(result.is_err());
    }

    #[test]
    fn test_collateral_manager() {
        let mut manager = CollateralManager::new();

        // Lock collateral
        let tier = manager.lock_collateral(
            "mn1".to_string(),
            10_000 * TIME_UNIT,
            "tx123".to_string(),
            1000,
        ).unwrap();

        assert_eq!(tier, CollateralTier::Gold);
        assert_eq!(manager.total_locked(), 10_000 * TIME_UNIT);

        // Unlock collateral
        let amount = manager.unlock_collateral("mn1").unwrap();
        assert_eq!(amount, 10_000 * TIME_UNIT);
        assert_eq!(manager.total_locked(), 0);
    }
}
