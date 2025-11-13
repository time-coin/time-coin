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
    use time_core::constants::COIN;

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
