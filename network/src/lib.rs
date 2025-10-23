//! TIME Coin Network Module
//!
//! Handles peer-to-peer networking and discovery

pub mod discovery;
pub mod protocol;
pub mod sync;

pub use discovery::{DnsDiscovery, HttpDiscovery, NetworkType, PeerDiscovery, PeerInfo, SeedNodes};
pub use protocol::{HandshakeMessage, ProtocolVersion, VERSION, PROTOCOL_VERSION};

/// Current TIME Coin version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Protocol version for compatibility checking
pub const PROTOCOL_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    #[test]
    fn test_network_module() {}
}
