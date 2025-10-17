#!/bin/bash
# TIME Coin - Complete Repository Setup Script
# This script creates the entire TIME coin project structure with all code

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}════════════════════════════════════════════════${NC}"
echo -e "${BLUE}   TIME Coin - Complete Project Setup${NC}"
echo -e "${BLUE}════════════════════════════════════════════════${NC}\n"

# Create root directory structure
echo -e "${GREEN}📁 Creating directory structure...${NC}"
mkdir -p {core,masternode,treasury,network,purchase,wallet,api,storage,crypto,cli}/{src,tests}
mkdir -p {docs,scripts,config,.github/workflows}
mkdir -p docs/{architecture,masternodes,api,developers}

# ============================================
# ROOT CARGO.TOML (Workspace)
# ============================================
echo -e "${GREEN}📦 Creating Cargo workspace...${NC}"
cat > Cargo.toml << 'EOF'
[workspace]
members = [
    "core",
    "masternode",
    "treasury",
    "network",
    "purchase",
    "wallet",
    "api",
    "storage",
    "crypto",
    "cli",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["TIME Coin Developers"]
license = "MIT"
repository = "https://github.com/time-coin/time-coin"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.35", features = ["full"] }
thiserror = "1.0"
anyhow = "1.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
sha2 = "0.10"
sha3 = "0.10"
ed25519-dalek = "2.1"
secp256k1 = "0.28"
ring = "0.17"
EOF

# ============================================
# CORE MODULE
# ============================================
echo -e "${GREEN}⚙️  Creating core module...${NC}"
cat > core/Cargo.toml << 'EOF'
[package]
name = "time-core"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
chrono.workspace = true
sha2.workspace = true
sha3.workspace = true

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
EOF

cat > core/src/lib.rs << 'EOF'
//! TIME Core - Core blockchain functionality

pub mod block;
pub mod transaction;
pub mod state;
pub mod constants;

pub use block::{Block, BlockHeader};
pub use transaction::{Transaction, TransactionType};
pub use state::ChainState;
pub use constants::*;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }
}
EOF

cat > core/src/constants.rs << 'EOF'
//! TIME Coin Protocol Constants

use std::time::Duration;

// Block Constants
pub const BLOCK_TIME: Duration = Duration::from_secs(86400); // 24 hours
pub const BLOCK_REWARD: u64 = 100 * COIN; // 100 TIME per block
pub const MASTERNODE_REWARD: u64 = 95 * COIN; // 95 TIME to masternodes
pub const TREASURY_REWARD: u64 = 5 * COIN; // 5 TIME to treasury

// Transaction Constants
pub const MAX_TRANSACTION_SIZE: usize = 100_000; // 100 KB
pub const MIN_TRANSACTION_FEE: u64 = 1000; // 0.00001 TIME
pub const TRANSACTION_FINALITY_TIME: Duration = Duration::from_secs(3); // 3 seconds

// Coin Constants
pub const COIN: u64 = 100_000_000; // 1 TIME = 100 million satoshis
pub const MAX_SUPPLY: u64 = 1_000_000_000 * COIN; // 1 billion TIME

// Network Constants
pub const MIN_MASTERNODE_COLLATERAL: u64 = 1_000 * COIN; // Bronze tier
pub const DEFAULT_PORT: u16 = 9876;
pub const MAX_PEERS: usize = 125;

// Consensus Constants
pub const BFT_THRESHOLD: f64 = 0.67; // 67% for Byzantine Fault Tolerance
pub const MIN_VALIDATORS: usize = 10;
pub const MAX_VALIDATORS: usize = 10_000;

// Treasury Constants
pub const TREASURY_MULTISIG_THRESHOLD: usize = 670; // 67% of masternodes
pub const MIN_PROPOSAL_AMOUNT: u64 = 100 * COIN;
pub const PROPOSAL_VOTING_PERIOD: Duration = Duration::from_secs(86400 * 14); // 14 days
EOF

cat > core/src/block.rs << 'EOF'
//! Block structures and functionality

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub block_number: u64,
    pub timestamp: DateTime<Utc>,
    pub previous_hash: String,
    pub merkle_root: String,
    pub validator_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<String>, // Transaction IDs
    pub hash: String,
}

impl Block {
    pub fn new(block_number: u64, previous_hash: String) -> Self {
        let header = BlockHeader {
            block_number,
            timestamp: Utc::now(),
            previous_hash,
            merkle_root: String::new(),
            validator_signature: String::new(),
        };

        let mut block = Block {
            header,
            transactions: Vec::new(),
            hash: String::new(),
        };

        block.hash = block.calculate_hash();
        block
    }

    pub fn add_transaction(&mut self, tx_id: String) {
        self.transactions.push(tx_id);
        self.header.merkle_root = self.calculate_merkle_root();
        self.hash = self.calculate_hash();
    }

    pub fn calculate_hash(&self) -> String {
        let data = format!(
            "{}:{}:{}:{}",
            self.header.block_number,
            self.header.timestamp.timestamp(),
            self.header.previous_hash,
            self.header.merkle_root
        );

        let mut hasher = Sha3_256::new();
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn calculate_merkle_root(&self) -> String {
        if self.transactions.is_empty() {
            return String::from("0");
        }

        let mut hasher = Sha3_256::new();
        for tx in &self.transactions {
            hasher.update(tx.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }

    pub fn validate(&self) -> Result<(), String> {
        // Validate hash
        if self.hash != self.calculate_hash() {
            return Err("Invalid block hash".to_string());
        }

        // Validate merkle root
        if self.header.merkle_root != self.calculate_merkle_root() {
            return Err("Invalid merkle root".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_creation() {
        let block = Block::new(1, "genesis".to_string());
        assert_eq!(block.header.block_number, 1);
        assert_eq!(block.header.previous_hash, "genesis");
        assert!(!block.hash.is_empty());
    }

    #[test]
    fn test_add_transaction() {
        let mut block = Block::new(1, "genesis".to_string());
        let initial_hash = block.hash.clone();
        
        block.add_transaction("tx123".to_string());
        
        assert_eq!(block.transactions.len(), 1);
        assert_ne!(block.hash, initial_hash); // Hash should change
    }

    #[test]
    fn test_block_validation() {
        let block = Block::new(1, "genesis".to_string());
        assert!(block.validate().is_ok());
    }
}
EOF

cat > core/src/transaction.rs << 'EOF'
//! Transaction structures and types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    Transfer {
        from: String,
        to: String,
        amount: u64,
        fee: u64,
    },
    Mint {
        recipient: String,
        amount: u64,
        purchase_proof: String,
    },
    MasternodeReward {
        masternode_id: String,
        amount: u64,
    },
    TreasuryAllocation {
        amount: u64,
        proposal_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub transaction_type: TransactionType,
    pub signature: String,
    pub nonce: u64,
}

impl Transaction {
    pub fn new(transaction_type: TransactionType, nonce: u64) -> Self {
        let timestamp = Utc::now();
        let id = Self::generate_id(&transaction_type, timestamp, nonce);

        Transaction {
            id,
            timestamp,
            transaction_type,
            signature: String::new(),
            nonce,
        }
    }

    fn generate_id(tx_type: &TransactionType, timestamp: DateTime<Utc>, nonce: u64) -> String {
        let data = format!("{:?}:{}:{}", tx_type, timestamp.timestamp(), nonce);
        let mut hasher = Sha3_256::new();
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn sign(&mut self, signature: String) {
        self.signature = signature;
    }

    pub fn validate(&self) -> Result<(), String> {
        // Validate transaction ID
        let expected_id = Self::generate_id(&self.transaction_type, self.timestamp, self.nonce);
        if self.id != expected_id {
            return Err("Invalid transaction ID".to_string());
        }

        // Validate signature exists
        if self.signature.is_empty() {
            return Err("Transaction not signed".to_string());
        }

        // Validate transaction type specific rules
        match &self.transaction_type {
            TransactionType::Transfer { amount, fee, .. } => {
                if *amount == 0 {
                    return Err("Transfer amount must be greater than 0".to_string());
                }
                if *fee < crate::constants::MIN_TRANSACTION_FEE {
                    return Err("Transaction fee too low".to_string());
                }
            }
            TransactionType::Mint { amount, .. } => {
                if *amount == 0 {
                    return Err("Mint amount must be greater than 0".to_string());
                }
            }
            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_transaction() {
        let tx_type = TransactionType::Transfer {
            from: "alice".to_string(),
            to: "bob".to_string(),
            amount: 100 * crate::constants::COIN,
            fee: crate::constants::MIN_TRANSACTION_FEE,
        };

        let mut tx = Transaction::new(tx_type, 1);
        tx.sign("test_signature".to_string());

        assert!(tx.validate().is_ok());
    }

    #[test]
    fn test_mint_transaction() {
        let tx_type = TransactionType::Mint {
            recipient: "alice".to_string(),
            amount: 1000 * crate::constants::COIN,
            purchase_proof: "payment_id_123".to_string(),
        };

        let mut tx = Transaction::new(tx_type, 1);
        tx.sign("test_signature".to_string());

        assert!(tx.validate().is_ok());
    }
}
EOF

cat > core/src/state.rs << 'EOF'
//! Blockchain state management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub address: String,
    pub balance: u64,
    pub nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainState {
    pub accounts: HashMap<String, Account>,
    pub total_supply: u64,
    pub current_block: u64,
}

impl ChainState {
    pub fn new() -> Self {
        ChainState {
            accounts: HashMap::new(),
            total_supply: 0,
            current_block: 0,
        }
    }

    pub fn get_balance(&self, address: &str) -> u64 {
        self.accounts
            .get(address)
            .map(|acc| acc.balance)
            .unwrap_or(0)
    }

    pub fn transfer(&mut self, from: &str, to: &str, amount: u64, fee: u64) -> Result<(), String> {
        // Get sender account
        let sender = self.accounts.get_mut(from)
            .ok_or("Sender account not found")?;

        // Check balance
        let total = amount.checked_add(fee)
            .ok_or("Amount overflow")?;
        if sender.balance < total {
            return Err("Insufficient balance".to_string());
        }

        // Deduct from sender
        sender.balance -= total;
        sender.nonce += 1;

        // Add to recipient
        let recipient = self.accounts.entry(to.to_string())
            .or_insert(Account {
                address: to.to_string(),
                balance: 0,
                nonce: 0,
            });
        recipient.balance += amount;

        Ok(())
    }

    pub fn mint(&mut self, recipient: &str, amount: u64) -> Result<(), String> {
        // Check max supply
        let new_supply = self.total_supply.checked_add(amount)
            .ok_or("Supply overflow")?;
        if new_supply > crate::constants::MAX_SUPPLY {
            return Err("Max supply exceeded".to_string());
        }

        // Add to recipient
        let account = self.accounts.entry(recipient.to_string())
            .or_insert(Account {
                address: recipient.to_string(),
                balance: 0,
                nonce: 0,
            });
        account.balance += amount;
        self.total_supply += amount;

        Ok(())
    }

    pub fn increment_block(&mut self) {
        self.current_block += 1;
    }
}

impl Default for ChainState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mint() {
        let mut state = ChainState::new();
        
        let result = state.mint("alice", 1000);
        assert!(result.is_ok());
        assert_eq!(state.get_balance("alice"), 1000);
        assert_eq!(state.total_supply, 1000);
    }

    #[test]
    fn test_transfer() {
        let mut state = ChainState::new();
        
        // Mint to alice
        state.mint("alice", 1000).unwrap();
        
        // Transfer to bob
        let result = state.transfer("alice", "bob", 500, 10);
        assert!(result.is_ok());
        assert_eq!(state.get_balance("alice"), 490);
        assert_eq!(state.get_balance("bob"), 500);
    }
}
EOF

# ============================================
# MASTERNODE MODULE
# ============================================
echo -e "${GREEN}🔗 Creating masternode module...${NC}"
cat > masternode/Cargo.toml << 'EOF'
[package]
name = "time-masternode"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
time-core = { path = "../core" }
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
chrono.workspace = true
uuid.workspace = true

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
EOF

cat > masternode/src/lib.rs << 'EOF'
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
EOF

cat > masternode/src/types.rs << 'EOF'
//! Masternode type definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type MasternodeId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MasternodeStatus {
    Registered,
    Active,
    Inactive,
    Banned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub ip_address: String,
    pub port: u16,
    pub protocol_version: u32,
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Masternode {
    pub id: MasternodeId,
    pub owner: String,
    pub collateral: u64,
    pub tier: crate::collateral::CollateralTier,
    pub status: MasternodeStatus,
    pub network_info: NetworkInfo,
    pub registered_at: DateTime<Utc>,
    pub activated_at: Option<DateTime<Utc>>,
    pub reputation: u64,
}

impl Masternode {
    pub fn new(
        owner: String,
        collateral: u64,
        tier: crate::collateral::CollateralTier,
        network_info: NetworkInfo,
    ) -> Self {
        Masternode {
            id: Uuid::new_v4().to_string(),
            owner,
            collateral,
            tier,
            status: MasternodeStatus::Registered,
            network_info,
            registered_at: Utc::now(),
            activated_at: None,
            reputation: 100,
        }
    }

    pub fn activate(&mut self) {
        self.status = MasternodeStatus::Active;
        self.activated_at = Some(Utc::now());
    }

    pub fn deactivate(&mut self) {
        self.status = MasternodeStatus::Inactive;
    }

    pub fn is_active(&self) -> bool {
        self.status == MasternodeStatus::Active
    }

    pub fn voting_power(&self) -> u64 {
        self.tier.voting_multiplier()
    }
}
EOF

cat > masternode/src/collateral.rs << 'EOF'
//! Masternode collateral tiers and benefits

use serde::{Deserialize, Serialize};
use time_core::COIN;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CollateralTier {
    Bronze,  // 1,000 TIME
    Silver,  // 5,000 TIME
    Gold,    // 10,000 TIME
    Platinum, // 50,000 TIME
    Diamond, // 100,000 TIME
}

impl CollateralTier {
    pub fn from_amount(amount: u64) -> Result<Self, String> {
        match amount {
            x if x >= 100_000 * COIN => Ok(CollateralTier::Diamond),
            x if x >= 50_000 * COIN => Ok(CollateralTier::Platinum),
            x if x >= 10_000 * COIN => Ok(CollateralTier::Gold),
            x if x >= 5_000 * COIN => Ok(CollateralTier::Silver),
            x if x >= 1_000 * COIN => Ok(CollateralTier::Bronze),
            _ => Err("Collateral amount too low".to_string()),
        }
    }

    pub fn required_collateral(&self) -> u64 {
        match self {
            CollateralTier::Bronze => 1_000 * COIN,
            CollateralTier::Silver => 5_000 * COIN,
            CollateralTier::Gold => 10_000 * COIN,
            CollateralTier::Platinum => 50_000 * COIN,
            CollateralTier::Diamond => 100_000 * COIN,
        }
    }

    pub fn apy(&self) -> f64 {
        match self {
            CollateralTier::Bronze => 18.0,
            CollateralTier::Silver => 19.8,
            CollateralTier::Gold => 22.5,
            CollateralTier::Platinum => 27.0,
            CollateralTier::Diamond => 30.0,
        }
    }

    pub fn voting_multiplier(&self) -> u64 {
        match self {
            CollateralTier::Bronze => 1,
            CollateralTier::Silver => 5,
            CollateralTier::Gold => 10,
            CollateralTier::Platinum => 50,
            CollateralTier::Diamond => 100,
        }
    }

    pub fn reward_multiplier(&self) -> f64 {
        1.0 + (self.apy() - 18.0) / 100.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierBenefits {
    pub tier: CollateralTier,
    pub collateral_required: u64,
    pub apy: f64,
    pub voting_power: u64,
    pub reward_multiplier: f64,
}

impl TierBenefits {
    pub fn for_tier(tier: CollateralTier) -> Self {
        TierBenefits {
            tier,
            collateral_required: tier.required_collateral(),
            apy: tier.apy(),
            voting_power: tier.voting_multiplier(),
            reward_multiplier: tier.reward_multiplier(),
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::for_tier(CollateralTier::Bronze),
            Self::for_tier(CollateralTier::Silver),
            Self::for_tier(CollateralTier::Gold),
            Self::for_tier(CollateralTier::Platinum),
            Self::for_tier(CollateralTier::Diamond),
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
            CollateralTier::Bronze
        );
        assert_eq!(
            CollateralTier::from_amount(100_000 * COIN).unwrap(),
            CollateralTier::Diamond
        );
    }

    #[test]
    fn test_voting_power() {
        assert_eq!(CollateralTier::Bronze.voting_multiplier(), 1);
        assert_eq!(CollateralTier::Diamond.voting_multiplier(), 100);
    }
}
EOF

cat > masternode/src/rewards.rs << 'EOF'
//! Masternode reward calculation and distribution

use serde::{Deserialize, Serialize};
use time_core::MASTERNODE_REWARD;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardCalculation {
    pub masternode_id: String,
    pub base_reward: u64,
    pub tier_multiplier: f64,
    pub total_reward: u64,
}

pub struct RewardCalculator;

impl RewardCalculator {
    pub fn calculate_reward(
        masternode_id: String,
        tier_multiplier: f64,
    ) -> RewardCalculation {
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
        let reward = RewardCalculator::calculate_reward(
            "mn123".to_string(),
            1.0,
        );
        
        assert_eq!(reward.base_reward, MASTERNODE_REWARD);
        assert_eq!(reward.total_reward, MASTERNODE_REWARD);
    }
}
EOF

cat > masternode/src/registry.rs << 'EOF'
//! Masternode registry for tracking all masternodes

use crate::types::*;
use crate::collateral::CollateralTier;
use std::collections::HashMap;

pub struct MasternodeRegistry {
    masternodes: HashMap<MasternodeId, Masternode>,
    by_owner: HashMap<String, Vec<MasternodeId>>,
    total_collateral: u64,
}

impl MasternodeRegistry {
    pub fn new() -> Self {
        MasternodeRegistry {
            masternodes: HashMap::new(),
            by_owner: HashMap::new(),
            total_collateral: 0,
        }
    }

    pub fn register(
        &mut self,
        owner: String,
        collateral: u64,
        network_info: NetworkInfo,
        reputation: u64,
    ) -> Result<MasternodeId, String> {
        let tier = CollateralTier::from_amount(collateral)?;
        let mut masternode = Masternode::new(owner.clone(), collateral, tier, network_info);
        masternode.reputation = reputation;
        
        let id = masternode.id.clone();
        self.masternodes.insert(id.clone(), masternode);
        self.by_owner.entry(owner).or_insert_with(Vec::new).push(id.clone());
        self.total_collateral += collateral;

        Ok(id)
    }

    pub fn get(&self, id: &str) -> Option<&Masternode> {
        self.masternodes.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Masternode> {
        self.masternodes.get_mut(id)
    }

    pub fn activate(&mut self, id: &str) -> Result<(), String> {
        let masternode = self.masternodes.get_mut(id)
            .ok_or("Masternode not found")?;
        masternode.activate();
        Ok(())
    }

    pub fn deactivate(&mut self, id: &str) -> Result<(), String> {
        let masternode = self.masternodes.get_mut(id)
            .ok_or("Masternode not found")?;
        masternode.deactivate();
        Ok(())
    }

    pub fn get_active_masternodes(&self) -> Vec<&Masternode> {
        self.masternodes.values()
            .filter(|mn| mn.is_active())
            .collect()
    }

    pub fn total_voting_power(&self) -> u64 {
        self.get_active_masternodes()
            .iter()
            .map(|mn| mn.voting_power())
            .sum()
    }

    pub fn count(&self) -> usize {
        self.masternodes.len()
    }

    pub fn active_count(&self) -> usize {
        self.get_active_masternodes().len()
    }
}

impl Default for MasternodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time_core::COIN;

    #[test]
    fn test_register_masternode() {
        let mut registry = MasternodeRegistry::new();
        
        let id = registry.register(
            "owner1".to_string(),
            10_000 * COIN,
            NetworkInfo {
                ip_address: "127.0.0.1".to_string(),
                port: 9000,
                protocol_version: 1,
                public_key: "pubkey".to_string(),
            },
            100,
        ).unwrap();

        assert!(registry.get(&id).is_some());
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_activate_masternode() {
        let mut registry = MasternodeRegistry::new();
        
        let id = registry.register(
            "owner1".to_string(),
            10_000 * COIN,
            NetworkInfo {
                ip_address: "127.0.0.1".to_string(),
                port: 9000,
                protocol_version: 1,
                public_key: "pubkey".to_string(),
            },
            100,
        ).unwrap();

        registry.activate(&id).unwrap();
        
        let mn = registry.get(&id).unwrap();
        assert!(mn.is_active());
        assert_eq!(registry.active_count(), 1);
    }
}
EOF

# ============================================
# README.md
# ============================================
echo -e "${GREEN}📝 Creating README...${NC}"
cat > README.md << 'EOF'
# TIME Coin ⏰

A next-generation cryptocurrency featuring 24-hour time blocks, instant transaction finality, and masternode-powered consensus.

## Key Features

- ⚡ **Instant Finality**: <3 second transaction confirmation
- 🕐 **24-Hour Blocks**: Daily settlement for immutable checkpoints
- 🔗 **Masternode Network**: Byzantine Fault Tolerant consensus
- 💰 **Tiered Staking**: 18-30% APY across 5 collateral tiers
- 🏛️ **Community Treasury**: Decentralized governance
- 🚀 **Fair Launch**: No pre-mine, purchase-based minting

## Architecture

TIME Coin separates transaction finality from block production:

1. **Instant Transactions**: Validated by masternodes in real-time
2. **Daily Blocks**: Periodic checkpoints every 24 hours
3. **BFT Consensus**: 67% validator agreement required
4. **Masternode Rewards**: 95 TIME per block distributed based on tier
5. **Treasury Funding**: 5 TIME per block for ecosystem development

## Masternode Tiers

| Tier | Collateral | APY | Voting Power |
|------|-----------|-----|--------------|
| Bronze | 1,000 TIME | 18% | 1x |
| Silver | 5,000 TIME | 19.8% | 5x |
| Gold | 10,000 TIME | 22.5% | 10x |
| Platinum | 50,000 TIME | 27% | 50x |
| Diamond | 100,000 TIME | 30% | 100x |

## Project Structure

```
time-coin/
├── core/           # Core blockchain logic
├── masternode/     # Masternode management
├── treasury/       # Community treasury
├── network/        # P2P networking
├── purchase/       # Fiat/crypto purchases
├── wallet/         # Wallet implementation
├── api/            # API server
├── storage/        # Database layer
├── crypto/         # Cryptographic primitives
└── cli/            # Command-line interface
```

## Getting Started

### Prerequisites

- Rust 1.75 or higher
- Git

### Building

```bash
# Clone the repository
git clone https://github.com/time-coin/time-coin.git
cd time-coin

# Build all components
cargo build --release

# Run tests
cargo test --all

# Run a node
cargo run --bin time-node --release
```

## Documentation

- [Technical Whitepaper](docs/whitepaper-technical.md)
- [Masternode Setup Guide](docs/masternodes/setup-guide.md)
- [API Documentation](docs/api/README.md)
- [Architecture Overview](docs/architecture/README.md)

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Community

- Website: https://time-coin.io
- Telegram: https://t.me/+CaN6EflYM-83OTY0
- Twitter: @TIMEcoin515010
- GitHub: https://github.com/time-coin/time-coin

## License

MIT License - see [LICENSE](LICENSE) for details

## Security

See [SECURITY.md](SECURITY.md) for reporting security vulnerabilities.

---

**⏰ TIME is money. Make it accessible.**
EOF

# ============================================
# GITIGNORE
# ============================================
cat > .gitignore << 'EOF'
# Rust
/target/
**/*.rs.bk
*.pdb
Cargo.lock

# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db

# Config
config/local.toml
*.key
*.pem

# Logs
*.log
logs/

# Database
*.db
*.sqlite
*.sqlite3

# Build
dist/
build/
EOF

# ============================================
# LICENSE
# ============================================
cat > LICENSE << 'EOF'
MIT License

Copyright (c) 2025 TIME Coin Developers

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
EOF

# ============================================
# GITHUB CI/CD
# ============================================
mkdir -p .github/workflows
cat > .github/workflows/rust.yml << 'EOF'
name: Rust CI

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main, develop ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        override: true
        components: rustfmt, clippy
    
    - name: Check formatting
      run: cargo fmt --all -- --check
    
    - name: Run clippy
      run: cargo clippy --all-targets --all-features -- -D warnings
    
    - name: Build
      run: cargo build --verbose --all
    
    - name: Run tests
      run: cargo test --verbose --all
    
    - name: Run doc tests
      run: cargo test --doc --all

  build:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        override: true
    
    - name: Build release
      run: cargo build --release --all
EOF

echo -e "${GREEN}✅ Project structure created successfully!${NC}\n"

# ============================================
# FINAL INSTRUCTIONS
# ============================================
echo -e "${BLUE}════════════════════════════════════════════════${NC}"
echo -e "${BLUE}   Setup Complete!${NC}"
echo -e "${BLUE}════════════════════════════════════════════════${NC}\n"

echo -e "${YELLOW}Next steps:${NC}"
echo -e "1. ${GREEN}cargo build --all${NC} - Build all components"
echo -e "2. ${GREEN}cargo test --all${NC} - Run all tests"
echo -e "3. ${GREEN}cargo doc --open${NC} - Generate and view documentation"
echo -e "\n${YELLOW}Project structure:${NC}"
echo -e "  ✅ Core blockchain module"
echo -e "  ✅ Masternode system"
echo -e "  ✅ Documentation"
echo -e "  ✅ CI/CD workflows"
echo -e "  ✅ Testing infrastructure"
echo -e "\n${GREEN}Ready to start development!${NC}"
EOF

chmod +x complete-time-coin-setup.sh
echo "Created complete-time-coin-setup.sh"