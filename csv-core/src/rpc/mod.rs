//! RPC Client with Quorum Support
//!
//! This module provides RPC client functionality with quorum-based
//! consensus to prevent single-point-of-failure or malicious provider attacks.
//!
//! The quorum engine aggregates observations from multiple RPC providers
//! and produces consensus decisions using weighted voting with Byzantine
//! fault tolerance.

#[cfg(not(target_arch = "wasm32"))]
pub mod quorum_client;
#[cfg(not(target_arch = "wasm32"))]
pub mod quorum_engine;
pub mod quorum_types;

// Re-exports
#[cfg(not(target_arch = "wasm32"))]
pub use quorum_client::{QuorumClient, QuorumConfig, RpcProvider};
pub use quorum_types::{
    ByzantineConfig, ConsensusRound, FinalityGrade, ProviderHealth, ProviderWeight,
    QuorumDecision, QuorumError, QuorumFinality, QuorumResult, QuorumThreshold, RpcObservation,
};
#[cfg(not(target_arch = "wasm32"))]
pub use quorum_engine::QuorumEngine;
