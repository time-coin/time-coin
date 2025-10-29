//! TIME Coin Network Module
pub mod discovery;
pub mod protocol;
pub mod sync;
pub mod connection;
pub mod manager;

pub use discovery::{DnsDiscovery, HttpDiscovery, NetworkType, PeerDiscovery, PeerInfo, SeedNodes};
pub use protocol::{HandshakeMessage, ProtocolVersion, VERSION, PROTOCOL_VERSION};
pub use protocol::{TransactionMessage, TransactionValidation, NetworkMessage};
pub use connection::PeerConnection;
pub use manager::PeerManager;
pub use connection::PeerListener;
pub use manager::Snapshot;
