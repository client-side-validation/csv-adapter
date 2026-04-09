//! CSV Adapter Core - Chain-agnostic traits and types for Client-Side Validation
//!
//! This crate provides the foundational abstractions for anchoring CSV logic
//! to heterogeneous base layers without modifying the CSV core.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
#![allow(missing_docs)]

extern crate alloc;

// Core types
pub mod commitment;
pub mod hash;
pub mod right;
pub mod seal;
pub mod tagged_hash;

// Production hardening
pub mod hardening;

// State machine types (Phase 1: Consignment Wire Format)
pub mod consignment;
pub mod genesis;
pub mod schema;
pub mod state;
pub mod transition;

// MPC Tree (Phase 2)
pub mod mpc;

// Deterministic VM (Phase 3)
pub mod vm;

// DAG and proof types
pub mod dag;
pub mod proof;
pub mod proof_verify;
pub mod signature;

// Error handling and traits
pub mod error;
pub mod traits;

// Cross-cutting (Phase 10)
pub mod monitor;
pub mod store;

// Client-side validation (Sprint 2)
pub mod client;
pub mod validator;
pub mod commitment_chain;
pub mod state_store;
pub mod seal_registry;

// Cross-chain transfer (Sprint 4 - NORTH STAR)
pub mod cross_chain;

// RGB protocol compatibility (Sprint 5)
pub mod rgb_compat;

// Tapret verification (Sprint 0.5) - requires bitcoin dependency
#[cfg(feature = "tapret")]
pub mod tapret_verify;

// Re-exports: core
pub use commitment::Commitment;
pub use right::{Right, RightId, RightError, OwnershipProof};
pub use hardening::{
    BoundedQueue, CircuitBreaker, CircuitState, MemoryLimits, TimeoutConfig,
    DEFAULT_CIRCUIT_MAX_FAILURES, DEFAULT_CIRCUIT_RESET_TIMEOUT, DEFAULT_HEALTH_CHECK_TIMEOUT,
    DEFAULT_RPC_TIMEOUT, MAX_CACHE_SIZE, MAX_REGISTRY_SIZE, MAX_SEAL_REGISTRY_SIZE,
};
pub use hash::Hash;
pub use seal::{AnchorRef, SealRef};

// Re-exports: state machine (Phase 1)
pub use consignment::CONSIGNMENT_VERSION;
pub use consignment::{Anchor as ConsignmentAnchor, Consignment, ConsignmentError, SealAssignment};
pub use genesis::Genesis;
pub use schema::SCHEMA_VERSION;
pub use schema::{
    GlobalStateType, OwnedStateType, Schema, SchemaError, StateDataType, TransitionDef,
    TransitionValidationError,
};
pub use state::{GlobalState, Metadata, OwnedState, StateAssignment, StateRef, StateTypeId};
pub use transition::Transition;

// Re-exports: MPC (Phase 2)
pub use mpc::{MerkleBranchNode, MpcLeaf, MpcProof, MpcTree, ProtocolId};

// Re-exports: VM (Phase 3)
pub use vm::{execute_transition, DeterministicVM, PassthroughVM, VMError, VMInputs, VMOutputs};

// Re-exports: DAG and proofs
pub use dag::{DAGNode, DAGSegment};
pub use proof::{FinalityProof, InclusionProof, ProofBundle};
pub use proof_verify::verify_proof;
pub use signature::{parse_signatures_from_bytes, verify_signatures, Signature, SignatureScheme};

// Re-exports: errors and traits
pub use error::{AdapterError, Result};
pub use traits::AnchorLayer;

// Re-exports: cross-cutting (Phase 10)
pub use monitor::{PendingPublication, PublicationTracker, ReorgEvent, ReorgMonitor};
pub use store::{AnchorRecord, InMemorySealStore, SealRecord, SealStore, StoreError};
