//! Reward calculation and distribution

use crate::constants::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardDistribution {
    pub block_number: u64,
    pub treasury_reward: u64,
    pub masternode_reward: u64,
    pub total_reward: u64,
}

pub struct RewardCalculator;

impl RewardCalculator {
    pub fn calculate_block_reward(block_number: u64) -> RewardDistribution {
        RewardDistribution {
            block_number,
            treasury_reward: TREASURY_REWARD,
            masternode_reward: MASTERNODE_REWARD,
            total_reward: BLOCK_REWARD,
        }
    }

    pub fn split_transaction_fee(fee: u64) -> (u64, u64) {
        let treasury_portion = (fee * TREASURY_FEE_PERCENTAGE) / 100;
        let masternode_portion = fee - treasury_portion;
        (treasury_portion, masternode_portion)
    }

    pub fn calculate_masternode_apy(collateral: u64, daily_rewards: u64) -> f64 {
        if collateral == 0 {
            return 0.0;
        }

        let annual_rewards = daily_rewards as f64 * 365.0;
        (annual_rewards / collateral as f64) * 100.0
    }
}
