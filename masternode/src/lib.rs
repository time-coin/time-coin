//! TIME Masternode - Masternode management and consensus

pub mod registry;
pub mod types;
pub mod collateral;
pub mod rewards;

pub use registry::MasternodeRegistry;
pub use types::{Masternode, MasternodeId, MasternodeStatus, NetworkInfo};
pub use collateral::{CollateralTier, TierBenefits};
pub use rewards::RewardCalculator;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_masternode_module() {
        let _registry = MasternodeRegistry::new();
        assert!(true);
    }
}
