//! Masternode registry for tracking all masternodes

use crate::types::*;
use crate::collateral::CollateralTier;
use std::collections::HashMap;
use treasury::TreasuryPool;

pub struct MasternodeRegistry {
    masternodes: HashMap<MasternodeId, Masternode>,
    by_owner: HashMap<String, Vec<MasternodeId>>,
    total_collateral: u64,
    treasury: TreasuryPool,
}

impl MasternodeRegistry {
    pub fn new() -> Self {
        MasternodeRegistry {
            masternodes: HashMap::new(),
            by_owner: HashMap::new(),
            total_collateral: 0,
            treasury: TreasuryPool::new(),
        }
    }

    pub fn register(
        &mut self,
        owner: String,
        collateral: u64,
        network_info: NetworkInfo,
        reputation: u64,
        timestamp: u64,
    ) -> Result<MasternodeId, String> {
        let tier = CollateralTier::from_amount(collateral)?;
        let mut masternode = Masternode::new(owner.clone(), collateral, tier, network_info);
        masternode.reputation = reputation;
        
        let id = masternode.id.clone();
        
        // Lock collateral in treasury
        self.treasury
            .lock_collateral(
                format!("lock-{}", id),
                id.clone(),
                owner.clone(),
                collateral,
                timestamp,
            )
            .map_err(|e| format!("Failed to lock collateral: {}", e))?;
        
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

    /// Initiate decommissioning of a masternode (starts cooldown period)
    pub fn decommission(&mut self, id: &str, timestamp: u64) -> Result<(), String> {
        let masternode = self.masternodes.get_mut(id)
            .ok_or("Masternode not found")?;
        
        // Deactivate the masternode
        masternode.deactivate();
        
        // Start cooldown period in treasury
        let lock_id = format!("lock-{}", id);
        self.treasury
            .start_collateral_cooldown(&lock_id, timestamp)
            .map_err(|e| format!("Failed to start cooldown: {}", e))?;
        
        Ok(())
    }

    /// Complete withdrawal of collateral after cooldown period
    pub fn withdraw_collateral(&mut self, id: &str, timestamp: u64) -> Result<u64, String> {
        let masternode = self.masternodes.get(id)
            .ok_or("Masternode not found")?;
        
        if masternode.is_active() {
            return Err("Cannot withdraw collateral from active masternode. Decommission first.".to_string());
        }
        
        // Unlock collateral from treasury
        let lock_id = format!("lock-{}", id);
        let amount = self.treasury
            .unlock_collateral(&lock_id, timestamp)
            .map_err(|e| format!("Failed to unlock collateral: {}", e))?;
        
        Ok(amount)
    }

    /// Check if collateral can be withdrawn
    pub fn can_withdraw_collateral(&self, id: &str, timestamp: u64) -> Result<bool, String> {
        let lock_id = format!("lock-{}", id);
        self.treasury
            .can_unlock_collateral(&lock_id, timestamp)
            .map_err(|e| format!("Failed to check unlock status: {}", e))
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

    /// Get reference to the treasury
    pub fn treasury(&self) -> &TreasuryPool {
        &self.treasury
    }

    /// Get mutable reference to the treasury
    pub fn treasury_mut(&mut self) -> &mut TreasuryPool {
        &mut self.treasury
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
    use time_core::constants::COIN;
    use treasury::DEFAULT_LOCK_PERIOD;

    #[test]
    fn test_register_masternode() {
        let mut registry = MasternodeRegistry::new();
        
        // Fund treasury first - need enough for 10,000 TIME collateral
        for i in 1..=3000 {
            registry.treasury_mut().deposit_block_reward(i, (i as u64) * 1000).unwrap();
        }
        
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
            1000,
        ).unwrap();

        assert!(registry.get(&id).is_some());
        assert_eq!(registry.count(), 1);
        
        // Verify collateral is locked in treasury
        let lock_id = format!("lock-{}", id);
        let lock = registry.treasury().get_collateral_lock(&lock_id).unwrap();
        assert_eq!(lock.amount, 10_000 * COIN);
        assert_eq!(lock.masternode_id, id);
    }

    #[test]
    fn test_activate_masternode() {
        let mut registry = MasternodeRegistry::new();
        
        // Fund treasury - need enough for 10,000 TIME collateral
        for i in 1..=3000 {
            registry.treasury_mut().deposit_block_reward(i, (i as u64) * 1000).unwrap();
        }
        
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
            1000,
        ).unwrap();

        registry.activate(&id).unwrap();
        
        let mn = registry.get(&id).unwrap();
        assert!(mn.is_active());
        assert_eq!(registry.active_count(), 1);
    }

    #[test]
    fn test_decommission_masternode() {
        let mut registry = MasternodeRegistry::new();
        
        // Fund treasury - need enough for 10,000 TIME collateral
        for i in 1..=3000 {
            registry.treasury_mut().deposit_block_reward(i, (i as u64) * 1000).unwrap();
        }
        
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
            1000,
        ).unwrap();

        registry.activate(&id).unwrap();
        
        // Decommission - should start cooldown
        registry.decommission(&id, 2000).unwrap();
        
        let mn = registry.get(&id).unwrap();
        assert!(!mn.is_active());
        
        // Verify cooldown was started
        let lock_id = format!("lock-{}", id);
        let lock = registry.treasury().get_collateral_lock(&lock_id).unwrap();
        assert_eq!(lock.status, treasury::CollateralLockStatus::Cooldown);
    }

    #[test]
    fn test_withdraw_collateral_after_cooldown() {
        let mut registry = MasternodeRegistry::new();
        
        // Fund treasury - need enough for 10,000 TIME collateral
        for i in 1..=3000 {
            registry.treasury_mut().deposit_block_reward(i, (i as u64) * 1000).unwrap();
        }
        
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
            1000,
        ).unwrap();

        registry.activate(&id).unwrap();
        
        // Decommission
        registry.decommission(&id, 2000).unwrap();
        
        // Cannot withdraw before cooldown
        let can_withdraw = registry.can_withdraw_collateral(&id, 2000 + DEFAULT_LOCK_PERIOD - 1).unwrap();
        assert!(!can_withdraw);
        
        // Can withdraw after cooldown
        let can_withdraw = registry.can_withdraw_collateral(&id, 2000 + DEFAULT_LOCK_PERIOD).unwrap();
        assert!(can_withdraw);
        
        // Withdraw collateral
        let amount = registry.withdraw_collateral(&id, 2000 + DEFAULT_LOCK_PERIOD).unwrap();
        assert_eq!(amount, 10_000 * COIN);
    }

    #[test]
    fn test_withdraw_collateral_from_active_masternode_fails() {
        let mut registry = MasternodeRegistry::new();
        
        // Fund treasury - need enough for 10,000 TIME collateral
        for i in 1..=3000 {
            registry.treasury_mut().deposit_block_reward(i, (i as u64) * 1000).unwrap();
        }
        
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
            1000,
        ).unwrap();

        registry.activate(&id).unwrap();
        
        // Try to withdraw without decommissioning
        let result = registry.withdraw_collateral(&id, 2000);
        assert!(result.is_err());
    }

    #[test]
    fn test_early_withdrawal_fails() {
        let mut registry = MasternodeRegistry::new();
        
        // Fund treasury - need enough for 10,000 TIME collateral
        for i in 1..=3000 {
            registry.treasury_mut().deposit_block_reward(i, (i as u64) * 1000).unwrap();
        }
        
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
            1000,
        ).unwrap();

        registry.activate(&id).unwrap();
        registry.decommission(&id, 2000).unwrap();
        
        // Try to withdraw immediately (before cooldown period)
        let result = registry.withdraw_collateral(&id, 2001);
        assert!(result.is_err());
    }
}
