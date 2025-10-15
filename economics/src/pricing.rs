//! Purchase pricing calculations

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchasePrice {
    pub time_amount: u64,
    pub usd_price: f64,
    pub exchange_rate: f64,
}

pub struct PriceCalculator;

impl PriceCalculator {
    pub fn calculate_purchase_price(usd_amount: f64, time_usd_rate: f64) -> PurchasePrice {
        let time_amount =
            ((usd_amount / time_usd_rate) * crate::constants::TIME_UNIT as f64) as u64;

        PurchasePrice {
            time_amount,
            usd_price: usd_amount,
            exchange_rate: time_usd_rate,
        }
    }
}
