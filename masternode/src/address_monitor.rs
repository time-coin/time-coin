use crate::error::MasternodeError;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use wallet::{xpub_to_address, NetworkType};

const ADDRESS_GAP_LIMIT: u32 = 20;

#[derive(Clone)]
pub struct AddressMonitor {
    /// Map of xpub -> (external addresses, internal addresses, last used indices)
    monitored: Arc<RwLock<HashMap<String, MonitoredXpub>>>,
}

struct MonitoredXpub {
    xpub_str: String,
    external_addresses: Vec<String>,
    internal_addresses: Vec<String>,
    last_external_used: u32,
    last_internal_used: u32,
}

impl Default for AddressMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl AddressMonitor {
    pub fn new() -> Self {
        Self {
            monitored: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an xpub for monitoring
    pub async fn register_xpub(&self, xpub_str: &str) -> Result<(), MasternodeError> {
        let mut monitored = self.monitored.write().await;

        if monitored.contains_key(xpub_str) {
            return Ok(()); // Already registered
        }

        // Generate initial addresses (gap limit)
        let external_addresses = Self::derive_addresses(xpub_str, 0, 0, ADDRESS_GAP_LIMIT)?;
        let internal_addresses = Self::derive_addresses(xpub_str, 1, 0, ADDRESS_GAP_LIMIT)?;

        monitored.insert(
            xpub_str.to_string(),
            MonitoredXpub {
                xpub_str: xpub_str.to_string(),
                external_addresses,
                internal_addresses,
                last_external_used: 0,
                last_internal_used: 0,
            },
        );

        tracing::info!(
            "📝 Registered xpub for monitoring with {} addresses",
            ADDRESS_GAP_LIMIT * 2
        );
        Ok(())
    }

    /// Check if an address belongs to any monitored xpub
    pub async fn is_monitored_address(&self, address: &str) -> bool {
        let monitored = self.monitored.read().await;

        for xpub_data in monitored.values() {
            if xpub_data.external_addresses.contains(&address.to_string())
                || xpub_data.internal_addresses.contains(&address.to_string())
            {
                return true;
            }
        }

        false
    }

    /// Get all monitored addresses across all xpubs
    pub async fn get_all_monitored_addresses(&self) -> HashSet<String> {
        let monitored = self.monitored.read().await;
        let mut addresses = HashSet::new();

        for xpub_data in monitored.values() {
            addresses.extend(xpub_data.external_addresses.iter().cloned());
            addresses.extend(xpub_data.internal_addresses.iter().cloned());
        }

        addresses
    }

    /// Get xpubs that are monitoring a specific address
    pub async fn get_xpubs_for_address(&self, address: &str) -> Vec<String> {
        let monitored = self.monitored.read().await;
        let mut xpubs = Vec::new();

        for (xpub_str, xpub_data) in monitored.iter() {
            if xpub_data.external_addresses.contains(&address.to_string())
                || xpub_data.internal_addresses.contains(&address.to_string())
            {
                xpubs.push(xpub_str.clone());
            }
        }

        xpubs
    }

    /// Update address usage and generate more if needed
    pub async fn update_address_usage(&self, address: &str) -> Result<(), MasternodeError> {
        let mut monitored = self.monitored.write().await;

        for xpub_data in monitored.values_mut() {
            // Check external addresses
            if let Some(pos) = xpub_data
                .external_addresses
                .iter()
                .position(|a| a == address)
            {
                let index = pos as u32;
                if index > xpub_data.last_external_used {
                    xpub_data.last_external_used = index;

                    // Generate more addresses if needed
                    let current_count = xpub_data.external_addresses.len() as u32;
                    if index + ADDRESS_GAP_LIMIT > current_count {
                        let new_addresses = Self::derive_addresses(
                            &xpub_data.xpub_str,
                            0,
                            current_count,
                            ADDRESS_GAP_LIMIT,
                        )?;
                        xpub_data.external_addresses.extend(new_addresses);
                        tracing::debug!(
                            "Generated {} more external addresses for xpub",
                            ADDRESS_GAP_LIMIT
                        );
                    }
                }
            }

            // Check internal addresses
            if let Some(pos) = xpub_data
                .internal_addresses
                .iter()
                .position(|a| a == address)
            {
                let index = pos as u32;
                if index > xpub_data.last_internal_used {
                    xpub_data.last_internal_used = index;

                    // Generate more addresses if needed
                    let current_count = xpub_data.internal_addresses.len() as u32;
                    if index + ADDRESS_GAP_LIMIT > current_count {
                        let new_addresses = Self::derive_addresses(
                            &xpub_data.xpub_str,
                            1,
                            current_count,
                            ADDRESS_GAP_LIMIT,
                        )?;
                        xpub_data.internal_addresses.extend(new_addresses);
                        tracing::debug!(
                            "Generated {} more internal addresses for xpub",
                            ADDRESS_GAP_LIMIT
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Derive addresses from xpub
    fn derive_addresses(
        xpub_str: &str,
        change: u32,
        start: u32,
        count: u32,
    ) -> Result<Vec<String>, MasternodeError> {
        let mut addresses = Vec::new();

        for i in start..(start + count) {
            let address = xpub_to_address(xpub_str, change, i, NetworkType::Mainnet)
                .map_err(|e| MasternodeError::AddressDerivation(e.to_string()))?;

            addresses.push(address);
        }

        Ok(addresses)
    }

    /// Get statistics about monitored xpubs
    pub async fn get_stats(&self) -> MonitorStats {
        let monitored = self.monitored.read().await;

        let mut total_external = 0;
        let mut total_internal = 0;

        for xpub_data in monitored.values() {
            total_external += xpub_data.external_addresses.len();
            total_internal += xpub_data.internal_addresses.len();
        }

        MonitorStats {
            xpub_count: monitored.len(),
            total_external_addresses: total_external,
            total_internal_addresses: total_internal,
            total_addresses: total_external + total_internal,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MonitorStats {
    pub xpub_count: usize,
    pub total_external_addresses: usize,
    pub total_internal_addresses: usize,
    pub total_addresses: usize,
}
