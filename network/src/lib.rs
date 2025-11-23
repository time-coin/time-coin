//! TIME Coin Network Module
pub mod connection;
pub mod discovery;
pub mod manager;
pub mod message_auth;
pub mod protocol;
pub mod quarantine;
pub mod rate_limiter;
pub mod sync;
pub mod tx_broadcast;
pub mod utxo_handler;

pub use connection::{PeerConnection, PeerListener};
pub use discovery::{DnsDiscovery, HttpDiscovery, NetworkType, PeerDiscovery, PeerInfo, SeedNodes};
pub use manager::PeerManager;
pub use message_auth::{AuthError, AuthenticatedMessage, NonceTracker};
pub use protocol::{HandshakeMessage, NetworkMessage, ProtocolVersion, TransactionMessage};
pub use protocol::{TransactionValidation, PROTOCOL_VERSION, VERSION};
pub use quarantine::{
    PeerQuarantine, QuarantineConfig, QuarantineReason, QuarantineSeverity, QuarantineStats,
};
pub use rate_limiter::{RateLimitError, RateLimiter, RateLimiterConfig};
pub use tx_broadcast::TransactionBroadcaster;
pub use utxo_handler::UTXOProtocolHandler;

pub mod peer_exchange;
