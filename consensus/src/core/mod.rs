//! Core consensus traits and shared abstractions
//!
//! This module contains reusable traits and implementations that eliminate
//! code duplication across the consensus system.

pub mod vrf;
pub mod collector;
pub mod strategy;

pub use vrf::{VRFSelector, DefaultVRFSelector};
pub use collector::{Vote, VoteCollector};
pub use strategy::FallbackStrategy;