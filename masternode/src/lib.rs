//! TIME Coin Masternode Implementation
//!
//! 3-tier masternode system with performance-based rewards

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time_core::Transaction;
use wallet::Address;

pub const COIN: u64 = 100_000_000; // 1 TIME = 100,000,000 satoshis

/// Masternode collateral tiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CollateralTier {
    Community,    // 1,000 TIME, 18% APY, 90% uptime required
    Verified,     // 10,000 TIME, 24% APY, 95% uptime required
    Professional, // 100,000 TIME, 30% APY, 98% uptime required
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

    pub fn base_apy(&self) -> f64 {
        match self {
            CollateralTier::Community => 0.18,
            CollateralTier::Verified => 0.24,
            CollateralTier::Professional => 0.30,
        }
    }

    pub fn min_uptime(&self) -> f64 {
        match self {
            CollateralTier::Community => 0.90,
            CollateralTier::Verified => 0.95,
            CollateralTier::Professional => 0.98,
        }
    }

    pub fn voting_weight(&self) -> u32 {
        match self {
            CollateralTier::Community => 1,
            CollateralTier::Verified => 10,
            CollateralTier::Professional => 100,
        }
    }

    pub fn can_verify_purchases(&self) -> bool {
        matches!(
            self,
            CollateralTier::Verified | CollateralTier::Professional
        )
    }

    pub fn can_create_proposals(&self) -> bool {
        matches!(self, CollateralTier::Professional)
    }

    pub fn voting_multiplier(&self) -> u32 {
        match self {
            CollateralTier::Community => 1,
            CollateralTier::Verified => 10,
            CollateralTier::Professional => 50,
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

/// Masternode configuration and state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Masternode {
    pub address: Address,
    pub collateral_tx: String,
    pub collateral_amount: u64,
    pub tier: CollateralTier,
    pub registered_at: i64,
    pub last_seen: i64,
    pub uptime_score: f64,
    pub kyc_verified: bool,
    pub is_slashed: bool,
    pub slashing_history: Vec<slashing::SlashingRecord>,
}

impl Masternode {
    pub fn new(
        address: Address,
        collateral_tx: String,
        collateral_amount: u64,
    ) -> Result<Self, String> {
        let tier = CollateralTier::from_amount(collateral_amount)?;
        let now = Utc::now().timestamp();
        Ok(Self {
            address,
            collateral_tx,
            collateral_amount,
            tier,
            registered_at: now,
            last_seen: now,
            uptime_score: 1.0,
            kyc_verified: false,
            is_slashed: false,
            slashing_history: Vec::new(),
        })
    }

    pub fn effective_apy(&self) -> f64 {
        let mut apy = self.tier.base_apy();

        if self.kyc_verified && self.tier.can_verify_purchases() {
            apy *= match self.tier {
                CollateralTier::Verified => 1.12,
                CollateralTier::Professional => 1.18,
                _ => 1.0,
            };
        }

        apy *= self.uptime_score;
        apy
    }

    pub fn monthly_reward(&self) -> u64 {
        let annual = self.collateral_amount as f64 * self.effective_apy();
        (annual / 12.0) as u64
    }

    pub fn is_active(&self) -> bool {
        let now = Utc::now().timestamp();
        now - self.last_seen < 300 // Active if seen within 5 minutes
    }

    pub fn meets_requirements(&self) -> bool {
        self.uptime_score >= self.tier.min_uptime() && self.is_active()
    }

    /// Execute slashing for a violation
    /// Returns the slash amount and updates the masternode state
    pub fn execute_slash(
        &mut self,
        violation: slashing::Violation,
        timestamp: i64,
        block_height: u64,
    ) -> Result<slashing::SlashingRecord, String> {
        if self.is_slashed {
            return Err("Masternode is already slashed".to_string());
        }

        // Calculate slash amount
        let slash_amount = slashing::calculate_slash_amount(&violation, self.collateral_amount);

        // Deduct collateral
        if slash_amount > self.collateral_amount {
            return Err("Slash amount exceeds collateral".to_string());
        }

        self.collateral_amount -= slash_amount;

        // Update tier based on remaining collateral and mark as slashed if below minimum
        match CollateralTier::from_amount(self.collateral_amount) {
            Ok(new_tier) => {
                self.tier = new_tier;
                // Node can continue with lower tier
            }
            Err(_) => {
                // Collateral below minimum tier requirement (1,000 COIN)
                self.is_slashed = true;
            }
        }

        // Create slashing record
        let record_id = format!("slash-{}-{}", self.address, timestamp);
        let record = slashing::SlashingRecord::new(
            record_id,
            self.address.to_string(),
            violation,
            slash_amount,
            self.collateral_amount,
            timestamp as u64,
            block_height,
        );

        // Add to history
        self.slashing_history.push(record.clone());

        Ok(record)
    }
}

/// Masternode network manager
#[derive(Debug)]
pub struct MasternodeNetwork {
    nodes: HashMap<Address, Masternode>,
    quorum_size: usize,
}

impl MasternodeNetwork {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            quorum_size: 7, // BFT requires 2f+1 for f Byzantine faults
        }
    }

    pub fn register(&mut self, node: Masternode) -> Result<(), String> {
        if self.nodes.contains_key(&node.address) {
            return Err("Masternode already registered".to_string());
        }
        self.nodes.insert(node.address.clone(), node);
        Ok(())
    }

    pub fn deregister(&mut self, address: &Address) -> Result<(), String> {
        self.nodes
            .remove(address)
            .map(|_| ())
            .ok_or_else(|| "Masternode not found".to_string())
    }

    pub fn get_node(&self, address: &Address) -> Option<&Masternode> {
        self.nodes.get(address)
    }

    pub fn active_nodes(&self) -> Vec<&Masternode> {
        self.nodes
            .values()
            .filter(|n| n.is_active() && n.meets_requirements())
            .collect()
    }

    pub fn select_quorum(&self) -> Vec<Address> {
        let mut active: Vec<_> = self
            .active_nodes()
            .iter()
            .map(|n| n.address.clone())
            .collect();

        // Sort deterministically by the string representation
        active.sort_by_key(|a| a.to_string());

        active.into_iter().take(self.quorum_size).collect()
    }

    pub fn validate_transaction(&self, _tx: &Transaction) -> bool {
        true // Placeholder
    }

    pub fn total_collateral(&self) -> u64 {
        self.nodes.values().map(|n| n.collateral_amount).sum()
    }

    pub fn tier_distribution(&self) -> HashMap<CollateralTier, usize> {
        let mut dist = HashMap::new();
        for node in self.nodes.values() {
            *dist.entry(node.tier).or_insert(0) += 1;
        }
        dist
    }

    /// Execute slashing on a masternode
    /// Returns the slashing record and the amount to be transferred to treasury
    pub fn slash_masternode(
        &mut self,
        address: &Address,
        violation: slashing::Violation,
        timestamp: i64,
        block_height: u64,
    ) -> Result<slashing::SlashingRecord, String> {
        let node = self
            .nodes
            .get_mut(address)
            .ok_or_else(|| "Masternode not found".to_string())?;

        // Execute the slash on the masternode
        let record = node.execute_slash(violation, timestamp, block_height)?;

        Ok(record)
    }

    /// Get all slashing records across all masternodes
    pub fn get_all_slashing_records(&self) -> Vec<&slashing::SlashingRecord> {
        self.nodes
            .values()
            .flat_map(|node| &node.slashing_history)
            .collect()
    }

    /// Get slashing records for a specific masternode
    pub fn get_slashing_records(&self, address: &Address) -> Vec<&slashing::SlashingRecord> {
        self.nodes
            .get(address)
            .map(|node| node.slashing_history.iter().collect())
            .unwrap_or_default()
    }

    /// Check if a masternode is slashed
    pub fn is_slashed(&self, address: &Address) -> bool {
        self.nodes
            .get(address)
            .map(|node| node.is_slashed)
            .unwrap_or(false)
    }
}

impl Default for MasternodeNetwork {
    fn default() -> Self {
        Self::new()
    }
}

/// Counts of masternodes per tier
#[derive(Debug, Clone, Default)]
pub struct MasternodeCounts {
    pub community: usize,
    pub verified: usize,
    pub professional: usize,
}

// Optional status module
pub mod status;

// Slashing module
pub mod slashing;

// Slashing executor module
pub mod slashing_executor;

// Module declarations for the new registry implementation
pub mod collateral;
pub mod registry;
pub mod rewards;
pub mod types;

// Violation detection modules
pub mod detector;
pub mod violations;

// Supporting modules
pub mod error;
pub mod heartbeat;
pub mod node;
pub mod reputation;
pub mod selection;
pub mod voting;

pub use registry::MasternodeRegistry;

// --- Tests
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
        assert!(CollateralTier::from_amount(500 * COIN).is_err());
    }

    #[test]
    fn test_tier_requirements() {
        let community = CollateralTier::Community;
        assert_eq!(community.required_collateral(), 1_000 * COIN);
        assert_eq!(community.base_apy(), 0.18);
        assert_eq!(community.voting_weight(), 1);
        assert!(!community.can_verify_purchases());

        let professional = CollateralTier::Professional;
        assert_eq!(professional.required_collateral(), 100_000 * COIN);
        assert_eq!(professional.base_apy(), 0.30);
        assert_eq!(professional.voting_weight(), 100);
        assert!(professional.can_verify_purchases());
        assert!(professional.can_create_proposals());
    }

    #[test]
    fn test_masternode_rewards() {
        let mut node = Masternode::new(
            Address::from_public_key(&[0u8; 32], wallet::NetworkType::Mainnet).unwrap(),
            "tx_hash".to_string(),
            10_000 * COIN,
        )
        .unwrap();

        assert_eq!(node.tier, CollateralTier::Verified);

        let base_monthly = (10_000.0 * COIN as f64 * 0.24 / 12.0) as u64;
        assert_eq!(node.monthly_reward(), base_monthly);

        node.kyc_verified = true;
        let kyc_monthly = (10_000.0 * COIN as f64 * 0.24 * 1.12 / 12.0) as u64;
        assert_eq!(node.monthly_reward(), kyc_monthly);
    }

    #[test]
    fn test_network_operations() {
        let mut network = MasternodeNetwork::new();

        let node1 = Masternode::new(
            Address::from_public_key(&[0u8; 32], wallet::NetworkType::Mainnet).unwrap(),
            "tx1".to_string(),
            1_000 * COIN,
        )
        .unwrap();

        // Use a different public key for node2 so it produces a different Address
        let node2 = Masternode::new(
            Address::from_public_key(&[1u8; 32], wallet::NetworkType::Mainnet).unwrap(),
            "tx2".to_string(),
            100_000 * COIN,
        )
        .unwrap();

        assert!(network.register(node1.clone()).is_ok());
        assert!(network.register(node2).is_ok());
        assert!(network.register(node1).is_err());

        assert_eq!(network.active_nodes().len(), 2);
        assert_eq!(network.total_collateral(), 101_000 * COIN);

        let dist = network.tier_distribution();
        assert_eq!(dist.get(&CollateralTier::Community), Some(&1));
        assert_eq!(dist.get(&CollateralTier::Professional), Some(&1));
    }
}
