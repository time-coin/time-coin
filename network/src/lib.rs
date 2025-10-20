//! TIME Coin Network Module
//!
//! Handles peer-to-peer networking and discovery

pub mod discovery;

pub use discovery::{DnsDiscovery, HttpDiscovery, NetworkType, PeerDiscovery, PeerInfo, SeedNodes};

#[cfg(test)]
mod tests {
    #[test]
    fn test_network_module() {}
}
pub mod sync;
