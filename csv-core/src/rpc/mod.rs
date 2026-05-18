//! RPC Client with Quorum Support
//!
//! This module provides RPC client functionality with quorum-based
//! consensus to prevent single-point-of-failure or malicious provider attacks.

pub mod quorum_client;
pub mod quorum_types;

// Re-exports
pub use quorum_client::{QuorumClient, QuorumConfig, RpcProvider};
pub use quorum_types::{FinalityGrade, QuorumDecision, RpcObservation};
