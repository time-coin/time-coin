//! Service layer for business logic
//!
//! This module contains services that encapsulate business logic,
//! separating it from HTTP handling concerns. Services are:
//! - Testable without HTTP layer
//! - Reusable across different interfaces (API, CLI, gRPC)
//! - Focused on domain logic

pub mod blockchain;
pub mod treasury;

pub use blockchain::BlockchainService;
pub use treasury::TreasuryService;
