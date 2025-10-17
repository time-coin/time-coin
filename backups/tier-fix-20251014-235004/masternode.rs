//! Masternode tier definitions and voting power

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MasternodeTier {
    Bronze, // 1,000 TIME
    Silver, // 10,000 TIME
    Gold,   // 100,000 TIME
}

impl MasternodeTier {
    pub fn from_collateral(amount: u64) -> Option<Self> {
        let time = amount / crate::TIME_UNIT;

        match time {
            1_000..=9_999 => Some(MasternodeTier::Bronze),
            10_000..=99_999 => Some(MasternodeTier::Silver),
            100_000.. => Some(MasternodeTier::Gold),
            _ => None,
        }
    }

    pub fn voting_power(&self) -> u64 {
        match self {
            MasternodeTier::Bronze => 1,
            MasternodeTier::Silver => 10,
            MasternodeTier::Gold => 100,
        }
    }

    pub fn required_collateral(&self) -> u64 {
        match self {
            MasternodeTier::Bronze => 1_000 * crate::TIME_UNIT,
            MasternodeTier::Silver => 10_000 * crate::TIME_UNIT,
            MasternodeTier::Gold => 100_000 * crate::TIME_UNIT,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            MasternodeTier::Bronze => "Bronze",
            MasternodeTier::Silver => "Silver",
            MasternodeTier::Gold => "Gold",
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
    pub registration_time: u64,
    pub last_active: u64,
}

impl Masternode {
    pub fn voting_power(&self) -> u64 {
        if self.active {
            self.tier.voting_power()
        } else {
            0
        }
    }

    pub fn weighted_voting_power(&self, current_time: u64) -> u64 {
        if !self.active {
            return 0;
        }

        let base_power = self.tier.voting_power();
        let longevity_multiplier = self.calculate_longevity_multiplier(current_time);

        // Total Weight = Tier Weight × Longevity Multiplier
        (base_power as f64 * longevity_multiplier) as u64
    }

    pub fn calculate_longevity_multiplier(&self, current_time: u64) -> f64 {
        let days_active = (current_time - self.registration_time) / 86400;

        // Formula: 1 + (Days Active ÷ 365) × 0.5
        // Maximum: 3.0× (after 4 years = 1460 days)
        let multiplier = 1.0 + ((days_active as f64) / 365.0) * 0.5;

        // Cap at 3.0×
        multiplier.min(3.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_from_collateral() {
        assert_eq!(
            MasternodeTier::from_collateral(1_000 * crate::TIME_UNIT),
            Some(MasternodeTier::Bronze)
        );

        assert_eq!(
            MasternodeTier::from_collateral(10_000 * crate::TIME_UNIT),
            Some(MasternodeTier::Silver)
        );

        assert_eq!(
            MasternodeTier::from_collateral(100_000 * crate::TIME_UNIT),
            Some(MasternodeTier::Gold)
        );
    }

    #[test]
    fn test_voting_power() {
        assert_eq!(MasternodeTier::Bronze.voting_power(), 1);
        assert_eq!(MasternodeTier::Silver.voting_power(), 10);
        assert_eq!(MasternodeTier::Gold.voting_power(), 100);
    }

    #[test]
    fn test_longevity_multiplier() {
        let mn = Masternode {
            id: "test".to_string(),
            address: "addr".to_string(),
            tier: MasternodeTier::Gold,
            collateral: 100_000 * crate::TIME_UNIT,
            active: true,
            registration_time: 0,
            last_active: 0,
        };

        // New node (0 days)
        assert!((mn.calculate_longevity_multiplier(0) - 1.0).abs() < 0.01);

        // 6 months (180 days)
        let six_months = 180 * 86400;
        assert!((mn.calculate_longevity_multiplier(six_months) - 1.25).abs() < 0.01);

        // 1 year (365 days)
        let one_year = 365 * 86400;
        assert!((mn.calculate_longevity_multiplier(one_year) - 1.5).abs() < 0.01);

        // 2 years (730 days)
        let two_years = 730 * 86400;
        assert!((mn.calculate_longevity_multiplier(two_years) - 2.0).abs() < 0.01);

        // 4 years (1460 days) - maximum
        let four_years = 1460 * 86400;
        assert!((mn.calculate_longevity_multiplier(four_years) - 3.0).abs() < 0.01);

        // 5 years - should still be capped at 3.0
        let five_years = 1825 * 86400;
        assert!((mn.calculate_longevity_multiplier(five_years) - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_weighted_voting_power() {
        let mut mn = Masternode {
            id: "test".to_string(),
            address: "addr".to_string(),
            tier: MasternodeTier::Gold,
            collateral: 100_000 * crate::TIME_UNIT,
            active: true,
            registration_time: 0,
            last_active: 0,
        };

        // New Gold node: 100 × 1.0 = 100
        assert_eq!(mn.weighted_voting_power(0), 100);

        // Gold node after 1 year: 100 × 1.5 = 150
        let one_year = 365 * 86400;
        assert_eq!(mn.weighted_voting_power(one_year), 150);

        // Gold node after 4 years: 100 × 3.0 = 300 (maximum)
        let four_years = 1460 * 86400;
        assert_eq!(mn.weighted_voting_power(four_years), 300);

        // Inactive node has 0 power
        mn.active = false;
        assert_eq!(mn.weighted_voting_power(four_years), 0);
    }
}
