//! TIME Coin Masternode Module
//!
//! Implements the masternode system with:
//! - Collateral tiers (1K, 5K, 10K, 50K, 100K TIME)
//! - Reputation tracking
//! - Reward distribution (95 TIME per block)
//! - Weighted voting for governance
//! - Liveness monitoring
//! - Slashing for misbehavior

pub mod node;
pub mod collateral;
pub mod reputation;
pub mod rewards;
pub mod selection;
pub mod registry;
pub mod voting;
pub mod heartbeat;
pub mod error;

pub use node::{Masternode, MasternodeStatus, MasternodeInfo};
pub use collateral::{CollateralTier, CollateralManager};
pub use reputation::{Reputation, ReputationScore, ReputationManager};
pub use rewards::{RewardCalculator, RewardDistribution};
pub use selection::{MasternodeSelector, SelectionAlgorithm};
pub use registry::{MasternodeRegistry, RegistrationRequest};
pub use voting::{VotingPower, Vote};
pub use heartbeat::{HeartbeatMonitor, HeartbeatStatus};
pub use error::{MasternodeError, Result};

/// Masternode block reward (95 TIME per block)
pub const MASTERNODE_BLOCK_REWARD: u64 = 95 * 100_000_000;

/// Minimum masternodes required for network
pub const MIN_MASTERNODES: usize = 10;

/// Maximum offline time before penalization (24 hours)
pub const MAX_OFFLINE_SECONDS: u64 = 86400;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(MASTERNODE_BLOCK_REWARD, 9_500_000_000);
        assert_eq!(MIN_MASTERNODES, 10);
        assert_eq!(MAX_OFFLINE_SECONDS, 86400);
    }
}
