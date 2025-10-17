//! Masternode collateral tiers and benefits

use serde::{Deserialize, Serialize};
use time_core::COIN;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CollateralTier {
    Bronze,  // 1,000 TIME
    Silver,  // 5,000 TIME
    Gold,    // 10,000 TIME
    Platinum, // 50,000 TIME
    Diamond, // 100,000 TIME
}

impl CollateralTier {
    pub fn from_amount(amount: u64) -> Result<Self, String> {
        match amount {
            x if x >= 100_000 * COIN => Ok(CollateralTier::Diamond),
            x if x >= 50_000 * COIN => Ok(CollateralTier::Platinum),
            x if x >= 10_000 * COIN => Ok(CollateralTier::Gold),
            x if x >= 5_000 * COIN => Ok(CollateralTier::Silver),
            x if x >= 1_000 * COIN => Ok(CollateralTier::Bronze),
            _ => Err("Collateral amount too low".to_string()),
        }
    }

    pub fn required_collateral(&self) -> u64 {
        match self {
            CollateralTier::Bronze => 1_000 * COIN,
            CollateralTier::Silver => 5_000 * COIN,
            CollateralTier::Gold => 10_000 * COIN,
            CollateralTier::Platinum => 50_000 * COIN,
            CollateralTier::Diamond => 100_000 * COIN,
        }
    }

    pub fn apy(&self) -> f64 {
        match self {
            CollateralTier::Bronze => 18.0,
            CollateralTier::Silver => 19.8,
            CollateralTier::Gold => 22.5,
            CollateralTier::Platinum => 27.0,
            CollateralTier::Diamond => 30.0,
        }
    }

    pub fn voting_multiplier(&self) -> u64 {
        match self {
            CollateralTier::Bronze => 1,
            CollateralTier::Silver => 5,
            CollateralTier::Gold => 10,
            CollateralTier::Platinum => 50,
            CollateralTier::Diamond => 100,
        }
    }

    pub fn reward_multiplier(&self) -> f64 {
        1.0 + (self.apy() - 18.0) / 100.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierBenefits {
    pub tier: CollateralTier,
    pub collateral_required: u64,
    pub apy: f64,
    pub voting_power: u64,
    pub reward_multiplier: f64,
}

impl TierBenefits {
    pub fn for_tier(tier: CollateralTier) -> Self {
        TierBenefits {
            tier,
            collateral_required: tier.required_collateral(),
            apy: tier.apy(),
            voting_power: tier.voting_multiplier(),
            reward_multiplier: tier.reward_multiplier(),
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::for_tier(CollateralTier::Bronze),
            Self::for_tier(CollateralTier::Silver),
            Self::for_tier(CollateralTier::Gold),
            Self::for_tier(CollateralTier::Platinum),
            Self::for_tier(CollateralTier::Diamond),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_from_amount() {
        assert_eq!(
            CollateralTier::from_amount(1_000 * COIN).unwrap(),
            CollateralTier::Bronze
        );
        assert_eq!(
            CollateralTier::from_amount(100_000 * COIN).unwrap(),
            CollateralTier::Diamond
        );
    }

    #[test]
    fn test_voting_power() {
        assert_eq!(CollateralTier::Bronze.voting_multiplier(), 1);
        assert_eq!(CollateralTier::Diamond.voting_multiplier(), 100);
    }
}
