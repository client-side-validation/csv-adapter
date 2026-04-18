//! CSV Core — Client-Side Validation for Cross-Chain Rights
//!
//! This crate provides the foundational types and traits for the CSV protocol:
//!
//! - **[`Right`]** — A verifiable, single-use digital right that can be
//!   transferred cross-chain
//! - **[`struct@Hash`]** — A 32-byte cryptographic hash (SHA-256 based)
//! - **[`Commitment`]** — A binding between a right's state and its anchor
//!   on a blockchain
//! - **[`SealRef`]** / **[`AnchorRef`]** — References to consumed seals
//!   and published anchors
//! - **[`InclusionProof`]** / **[`FinalityProof`]** / **[`ProofBundle`]** —
//!   Cryptographic proofs that a right was locked on the source chain
//! - **[`AnchorLayer`]** — The core trait each blockchain adapter implements
//! - **[`SignatureScheme`]** — Supported signing algorithms (secp256k1, ed25519)
//!
//! ## Stability Tiers
//!
//! Items in this crate are categorized into three tiers:
//!
//! ### 🔒 Stable API
//! The public re-exports at the top level of this module are **stable API**.
//! They will not change without a semver-major version bump.
//!
//! ### 🟡 Beta API
//! Modules like `consignment`, `genesis`, `schema`, `state`, `transition` are
//! maturing and may receive additive changes. Breaking changes require a minor
//! version bump with deprecation warnings.
//!
//! ### 🧪 Experimental API
//! Modules like `vm`, `mpc`, `rgb_compat` are experimental and feature-gated
//! behind the `experimental` Cargo feature. They may change or be removed
//! without notice.
//!
//! ## Protocol Contract
//!
//! The canonical protocol types (chain IDs, transfer status, error codes,
//! capability flags) live in [`protocol_version`]. These types MUST be mirrored
//! across all protocol consumers: CLI, TypeScript SDK, MCP server, Explorer, Wallet.
//!
//! ## Stability
//!
//! The types re-exported from this module are considered **stable API**.
//! They will not change without a semver-major version bump. Internal modules
//! (state machine, VM, MPC) may evolve as the protocol matures.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

extern crate alloc;

// Core types
pub mod commitment;
pub mod hash;
pub mod right;
pub mod seal;
pub mod tagged_hash;

// Advanced commitment types
pub mod advanced_commitments;

// Protocol version and canonical contract (🔒 STABLE + 🟡 BETA)
pub mod protocol_version;

// Agent-friendly types (AI agent support) - 🟡 BETA
pub mod agent_types;

// Production hardening - 🔒 STABLE
pub mod hardening;

// State machine types (Phase 1: Consignment Wire Format) - 🟡 BETA
pub mod consignment;
pub mod genesis;
pub mod schema;
pub mod state;
pub mod transition;

// MPC Tree (Phase 2) - 🧪 EXPERIMENTAL (re-exports gated, module always available)
pub mod mpc;

// Deterministic VM (Phase 3) - 🧪 EXPERIMENTAL
#[cfg(feature = "experimental")]
pub mod vm;

// DAG and proof types - 🔒 STABLE
pub mod dag;
pub mod proof;
pub mod proof_verify;
pub mod signature;

// Error handling and traits - 🔒 STABLE
pub mod error;
pub mod traits;

// Cross-cutting (Phase 10) - 🟡 BETA
pub mod monitor;
pub mod performance;
pub mod store;

// Client-side validation (Sprint 2)// Cross-chain transfer
pub mod client;
pub mod cross_chain;
pub mod commitment_chain;
pub mod seal_registry;
pub mod state_store;
pub mod validator;

// Chain adapter system for dynamic chain support
pub mod chain_config;
pub mod chain_system;
pub mod chain_adapter;
pub mod chain_discovery;
pub mod adapters;

// RGB protocol compatibility (Sprint 5) - 🧪 EXPERIMENTAL
#[cfg(feature = "experimental")]
pub mod rgb_compat;

// Tapret verification (Sprint 0.5) - requires bitcoin dependency
#[cfg(feature = "tapret")]
pub mod tapret_verify;

// ===========================================================================
// Re-exports: Protocol Contract (🔒 STABLE + 🟡 BETA)
// ===========================================================================

// Protocol version, chain IDs, transfer status, error codes, capabilities
pub use protocol_version::{
    Capabilities, Chain, ErrorCode, ProtocolVersion, SyncStatus, TransferStatus, PROTOCOL_VERSION,
};

// ===========================================================================
// Re-exports: Stable API (will not change without semver-major bump)
// ===========================================================================

pub use commitment::Commitment;
pub use hash::Hash;
pub use right::{OwnershipProof, Right, RightError, RightId};
pub use seal::{AnchorRef, SealRef};
pub use signature::{parse_signatures_from_bytes, verify_signatures, Signature, SignatureScheme};

// DAG and proofs
pub use dag::{DAGNode, DAGSegment};
pub use proof::{FinalityProof, InclusionProof, ProofBundle};
pub use proof_verify::verify_proof;

// Errors and traits
pub use error::{AdapterError, Result};
pub use traits::AnchorLayer;

// Cross-chain transfer
pub use client::{ValidationClient, ValidationResult};
pub use cross_chain::{CrossChainLockEvent, CrossChainRegistry, CrossChainRegistryEntry};

// ===========================================================================
// Re-exports: Beta API (may receive additive changes)
// ===========================================================================

// Advanced commitment types
pub use advanced_commitments::{CommitmentScheme, EnhancedCommitment, FinalityProofType, InclusionProofType, ProofMetadata};

// Agent-friendly types
pub use agent_types::{ErrorSuggestion, FixAction};

// Production hardening
pub use hardening::{
    BoundedQueue, CircuitBreaker, CircuitState, MemoryLimits, TimeoutConfig,
    DEFAULT_CIRCUIT_MAX_FAILURES, DEFAULT_CIRCUIT_RESET_TIMEOUT, DEFAULT_HEALTH_CHECK_TIMEOUT,
    DEFAULT_RPC_TIMEOUT, MAX_CACHE_SIZE, MAX_REGISTRY_SIZE, MAX_SEAL_REGISTRY_SIZE,
};

// State machine (Phase 1)
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

// Cross-cutting (Phase 10)
pub use monitor::{PendingPublication, PublicationTracker, ReorgEvent, ReorgMonitor};
pub use store::{AnchorRecord, InMemorySealStore, SealRecord, SealStore, StoreError};

// Chain adapter system (Beta API)
pub use chain_adapter::{
    ChainAdapter, ChainAdapterExt, ChainError, ChainRegistry, ChainResult, RpcClient, Wallet,
};
pub use chain_config::{
    ChainCapabilities, ChainConfig, ChainConfigLoader, AccountModel,
};
pub use chain_discovery::ChainDiscovery;
pub use chain_system::{ChainInfo, SimpleChainRegistry};

// ===========================================================================
// Re-exports: Experimental API (feature-gated, may change)
// ===========================================================================

/// Experimental module — feature-gated behind `experimental`.
/// These APIs may change or be removed without notice.
#[cfg(feature = "experimental")]
pub use mpc::{MerkleBranchNode, MpcLeaf, MpcProof, MpcTree, ProtocolId};

/// Experimental module — feature-gated behind `experimental`.
/// These APIs may change or be removed without notice.
#[cfg(feature = "experimental")]
pub use vm::{execute_transition, DeterministicVM, PassthroughVM, VMError, VMInputs, VMOutputs};

/// Experimental module — feature-gated behind `experimental`.
/// These APIs may change or be removed without notice.
#[cfg(feature = "experimental")]
pub use rgb_compat::{RGBCommitment, RGBError, RGBState};
