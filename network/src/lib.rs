//! TIME Coin Network Module
//!
//! Handles peer-to-peer networking and discovery

pub mod discovery;

pub use discovery::{
    PeerDiscovery, PeerInfo, NetworkType, SeedNodes,
    HttpDiscovery, DnsDiscovery,
};

#[cfg(test)]
mod tests {
    #[test]
    fn test_network_module() {
        assert!(true);
    }
}
pub mod sync;
