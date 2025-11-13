//! Masternode reward calculation and distribution

use serde::{Deserialize, Serialize};
use time_core::constants::MASTERNODE_REWARD;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardCalculation {
    pub masternode_id: String,
    pub base_reward: u64,
    pub tier_multiplier: f64,
    pub total_reward: u64,
}

pub struct RewardCalculator;

impl RewardCalculator {
    pub fn calculate_reward(masternode_id: String, tier_multiplier: f64) -> RewardCalculation {
        let base_reward = MASTERNODE_REWARD;
        let total_reward = (base_reward as f64 * tier_multiplier) as u64;

        RewardCalculation {
            masternode_id,
            base_reward,
            tier_multiplier,
            total_reward,
        }
    }

    pub fn calculate_daily_rewards(tier_multiplier: f64) -> u64 {
        (MASTERNODE_REWARD as f64 * tier_multiplier) as u64
    }

    pub fn calculate_annual_rewards(tier_multiplier: f64) -> u64 {
        Self::calculate_daily_rewards(tier_multiplier) * 365
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reward_calculation() {
        let reward = RewardCalculator::calculate_reward("mn123".to_string(), 1.0);

        assert_eq!(reward.base_reward, MASTERNODE_REWARD);
        assert_eq!(reward.total_reward, MASTERNODE_REWARD);
    }
}
