//! Supply management and tracking

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyStats {
    pub total_minted: u64,
    pub total_burned: u64,
    pub circulating_supply: u64,
    pub treasury_balance: u64,
    pub locked_in_masternodes: u64,
}

#[derive(Debug, Clone)]
pub struct SupplyManager {
    stats: SupplyStats,
}

impl SupplyManager {
    pub fn new() -> Self {
        Self {
            stats: SupplyStats {
                total_minted: 0,
                total_burned: 0,
                circulating_supply: 0,
                treasury_balance: 0,
                locked_in_masternodes: 0,
            },
        }
    }

    pub fn mint(&mut self, amount: u64) {
        self.stats.total_minted += amount;
        self.stats.circulating_supply += amount;
    }

    pub fn burn(&mut self, amount: u64) {
        self.stats.total_burned += amount;
        self.stats.circulating_supply = self.stats.circulating_supply.saturating_sub(amount);
    }

    pub fn stats(&self) -> &SupplyStats {
        &self.stats
    }

    pub fn net_supply(&self) -> u64 {
        self.stats
            .total_minted
            .saturating_sub(self.stats.total_burned)
    }
}

impl Default for SupplyManager {
    fn default() -> Self {
        Self::new()
    }
}
