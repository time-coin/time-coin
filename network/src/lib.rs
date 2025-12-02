//! TIME Coin Network Module - P2P Networking Layer
pub mod config;
pub mod connection;
pub mod discovery;
pub mod error;
pub mod heartbeat;
pub mod lock_ordering; // OPTIMIZATION (Quick Win #7): Compile-time deadlock prevention
pub mod manager;
pub mod message_auth;
pub mod message_handler;
pub mod peer_info; // Canonical peer information structure
pub mod protocol;
pub mod quarantine;
pub mod rate_limiter;
pub mod sync;
pub mod sync_gate; // Fork prevention: Block creation gating
pub mod tx_broadcast;
pub mod tx_sync; // 🆕 Transaction synchronization for block proposals
pub mod unified_connection;
pub mod upnp;
pub mod utxo_handler;
pub mod utxo_sync; // UTXO instant synchronization
pub mod voting;

pub use config::NetworkConfig;
pub use connection::{PeerConnection, PeerListener};
pub use discovery::{DnsDiscovery, HttpDiscovery, NetworkType, PeerDiscovery, SeedNodes};
pub use error::{NetworkError, NetworkResult};
pub use lock_ordering::LockOrdering; // Export for external use
pub use manager::PeerManager;
pub use message_auth::{AuthError, AuthenticatedMessage, NonceTracker};
pub use message_handler::MessageHandler;
pub use peer_info::PeerInfo;
pub use protocol::{HandshakeMessage, NetworkMessage, ProtocolVersion, TransactionMessage};
pub use protocol::{TransactionValidation, PROTOCOL_VERSION, VERSION};
pub use quarantine::{
    PeerQuarantine, QuarantineConfig, QuarantineReason, QuarantineSeverity, QuarantineStats,
};
pub use rate_limiter::{RateLimitError, RateLimiter, RateLimiterConfig};
pub use sync_gate::SyncGate; // Export for consensus layer
pub use tx_broadcast::TransactionBroadcaster;
pub use tx_sync::TransactionSyncManager; // 🆕 Export transaction sync manager
pub use upnp::UpnpManager;
pub use utxo_handler::UTXOProtocolHandler;
pub use utxo_sync::{MasternodeTier, UtxoInput, UtxoOutput, UtxoSyncManager};

pub mod peer_exchange;
