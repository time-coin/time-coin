//! Masternode collateral tiers and benefits

use serde::{Deserialize, Serialize};
use time_core::constants::COIN;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CollateralTier {
    Community,    // 1,000 TIME - Entry level
    Verified,     // 10,000 TIME - Balanced
    Professional, // 50,000 TIME - Premium
}

impl CollateralTier {
    pub fn from_amount(amount: u64) -> Result<Self, String> {
        match amount {
            x if x >= 100_000 * COIN => Ok(CollateralTier::Professional),
            x if x >= 10_000 * COIN => Ok(CollateralTier::Verified),
            x if x >= 1_000 * COIN => Ok(CollateralTier::Community),
            _ => Err("Minimum collateral is 1,000 TIME".to_string()),
        }
    }

    pub fn required_collateral(&self) -> u64 {
        match self {
            CollateralTier::Community => 1_000 * COIN,
            CollateralTier::Verified => 10_000 * COIN,
            CollateralTier::Professional => 100_000 * COIN,
        }
    }

    pub fn apy(&self) -> f64 {
        match self {
            CollateralTier::Community => 18.0,
            CollateralTier::Verified => 24.0,
            CollateralTier::Professional => 30.0,
        }
    }

    pub fn voting_multiplier(&self) -> u64 {
        match self {
            CollateralTier::Community => 1,
            CollateralTier::Verified => 10,
            CollateralTier::Professional => 50,
        }
    }

    pub fn reward_multiplier(&self) -> f64 {
        match self {
            CollateralTier::Community => 1.0,
            CollateralTier::Verified => 1.33,     // 33% boost
            CollateralTier::Professional => 1.67, // 67% boost
        }
    }

    pub fn min_uptime(&self) -> f64 {
        match self {
            CollateralTier::Community => 0.90,    // 90%
            CollateralTier::Verified => 0.95,     // 95%
            CollateralTier::Professional => 0.98, // 98%
        }
    }

    /// Vote maturity period in blocks before a newly registered masternode can vote
    /// This prevents instant takeover by newly coordinated malicious nodes
    pub fn vote_maturity_blocks(&self) -> u64 {
        match self {
            CollateralTier::Community => 1,     // 1 block for Community tier
            CollateralTier::Verified => 3,      // 3 blocks for Verified tier
            CollateralTier::Professional => 10, // 10 blocks for Professional tier
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierBenefits {
    pub tier: CollateralTier,
    pub name: String,
    pub collateral_required: u64,
    pub apy: f64,
    pub voting_power: u64,
    pub reward_multiplier: f64,
    pub min_uptime: f64,
}

impl TierBenefits {
    pub fn for_tier(tier: CollateralTier) -> Self {
        let name = match tier {
            CollateralTier::Community => "Community",
            CollateralTier::Verified => "Verified",
            CollateralTier::Professional => "Professional",
        };

        TierBenefits {
            tier,
            name: name.to_string(),
            collateral_required: tier.required_collateral(),
            apy: tier.apy(),
            voting_power: tier.voting_multiplier(),
            reward_multiplier: tier.reward_multiplier(),
            min_uptime: tier.min_uptime(),
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::for_tier(CollateralTier::Community),
            Self::for_tier(CollateralTier::Verified),
            Self::for_tier(CollateralTier::Professional),
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
            CollateralTier::Community
        );
        assert_eq!(
            CollateralTier::from_amount(10_000 * COIN).unwrap(),
            CollateralTier::Verified
        );
        assert_eq!(
            CollateralTier::from_amount(100_000 * COIN).unwrap(),
            CollateralTier::Professional
        );
    }

    #[test]
    fn test_voting_power() {
        assert_eq!(CollateralTier::Community.voting_multiplier(), 1);
        assert_eq!(CollateralTier::Verified.voting_multiplier(), 10);
        assert_eq!(CollateralTier::Professional.voting_multiplier(), 50);
    }

    #[test]
    fn test_apy() {
        assert_eq!(CollateralTier::Community.apy(), 18.0);
        assert_eq!(CollateralTier::Verified.apy(), 24.0);
        assert_eq!(CollateralTier::Professional.apy(), 30.0);
    }

    #[test]
    fn test_tier_count() {
        let tiers = TierBenefits::all();
        assert_eq!(tiers.len(), 3);
    }

    #[test]
    fn test_vote_maturity_blocks() {
        assert_eq!(CollateralTier::Community.vote_maturity_blocks(), 1);
        assert_eq!(CollateralTier::Verified.vote_maturity_blocks(), 3);
        assert_eq!(CollateralTier::Professional.vote_maturity_blocks(), 10);
    }
}
