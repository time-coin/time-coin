//! API Constants - Single source of truth for protocol parameters

/// Satoshis per TIME coin (8 decimal places)
pub const SATOSHIS_PER_TIME: u64 = 100_000_000;

/// Block reward in satoshis (500 TIME)
pub const TIME_BLOCK_REWARD: u64 = 50_000_000_000;

/// Treasury percentage of block reward
pub const TREASURY_PERCENTAGE: u64 = 10;

/// Grant system constants
pub const GRANT_AMOUNT_SATOSHIS: u64 = 100_000_000_000; // 1000 TIME
pub const GRANT_ACTIVATION_DAYS: i64 = 30;
pub const GRANT_DECOMMISSION_DAYS: i64 = 90;

/// Consensus thresholds
pub const BFT_THRESHOLD_NUMERATOR: u64 = 2;
pub const BFT_THRESHOLD_DENOMINATOR: u64 = 3;

/// Masternode requirements
pub const MIN_MASTERNODE_CONFIRMATIONS: u64 = 10;
pub const MASTERNODE_COLLATERAL_BRONZE: u64 = 500 * SATOSHIS_PER_TIME;
pub const MASTERNODE_COLLATERAL_SILVER: u64 = 2000 * SATOSHIS_PER_TIME;
pub const MASTERNODE_COLLATERAL_GOLD: u64 = 5000 * SATOSHIS_PER_TIME;

/// Network timeouts
pub const UTXO_BROADCAST_TIMEOUT_SECS: u64 = 5;
pub const CONSENSUS_VOTE_TIMEOUT_SECS: u64 = 10;

/// Validation limits
pub const MIN_PORT: u16 = 1024;
pub const MAX_PORT: u16 = 65535;
pub const MAX_TRANSACTION_SIZE: usize = 1_000_000; // 1 MB

/// Helper conversion functions
pub fn satoshis_to_time(satoshis: u64) -> f64 {
    satoshis as f64 / SATOSHIS_PER_TIME as f64
}

pub fn time_to_satoshis(time: f64) -> u64 {
    (time * SATOSHIS_PER_TIME as f64) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_satoshis_to_time() {
        assert_eq!(satoshis_to_time(100_000_000), 1.0);
        assert_eq!(satoshis_to_time(50_000_000), 0.5);
        assert_eq!(satoshis_to_time(0), 0.0);
    }

    #[test]
    fn test_time_to_satoshis() {
        assert_eq!(time_to_satoshis(1.0), 100_000_000);
        assert_eq!(time_to_satoshis(0.5), 50_000_000);
        assert_eq!(time_to_satoshis(0.0), 0);
    }

    #[test]
    fn test_grant_amount() {
        assert_eq!(satoshis_to_time(GRANT_AMOUNT_SATOSHIS), 1000.0);
    }

    #[test]
    fn test_collateral_amounts() {
        assert_eq!(satoshis_to_time(MASTERNODE_COLLATERAL_BRONZE), 500.0);
        assert_eq!(satoshis_to_time(MASTERNODE_COLLATERAL_SILVER), 2000.0);
        assert_eq!(satoshis_to_time(MASTERNODE_COLLATERAL_GOLD), 5000.0);
    }
}
