//! CSV Core — Client-Side Validation for Cross-Chain Rights
//!
//! This crate provides the foundational types and traits for the CSV protocol:
//!
//! - **[`Sanad`]** — A verifiable, single-use digital title (deed) that can be
//!   transferred cross-chain
//! - **[`struct@Hash`]** — A 32-byte cryptographic hash (SHA-256 based)
//! - **[`Commitment`]** — A binding between a sanad's state and its anchor
//!   on a blockchain
//! - **[`SealPoint`]** / **[`CommitAnchor`]** — References to consumed seals
//!   and published anchors
//! - **[`InclusionProof`]** / **[`FinalityProof`]** / **[`ProofBundle`]** —
//!   Cryptographic proofs that a sanad was locked on the source chain
//! - **[`SealProtocol`]** — The core trait each blockchain adapter implements
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
pub mod right;      // Legacy: being replaced by title
pub mod seal;
pub mod tagged_hash;
pub mod title;      // New: Sanad/Title types (replaces right)

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

// Chain operation traits (Production Guarantee Plan Phase 2) - 🔒 STABLE
pub mod chain_operations;
pub mod ops;        // New: refactored chain operations (replaces chain_operations)

// Shared event schemas (Production Guarantee Plan Phase 6) - 🔒 STABLE
pub mod events;

// Cross-cutting (Phase 10) - 🟡 BETA
pub mod monitor;
pub mod performance;
pub mod store;

// Client-side validation (Sprint 2)// Cross-chain transfer
pub mod client;
pub mod commitment_chain;
pub mod cross_chain;
pub mod nullifier;
pub mod state_store;
pub mod validator;

// Chain adapter system for dynamic chain support
pub mod adapter_factory;
pub mod adapters;
pub mod chain_adapter;
pub mod chain_config;
pub mod chain_discovery;
pub mod chain_plugin;

// RGB protocol compatibility (Sprint 5) - 🧪 EXPERIMENTAL
#[cfg(feature = "experimental")]
pub mod rgb_compat;

// Tapret verification (Sprint 0.5) - requires bitcoin dependency
#[cfg(feature = "tapret")]
pub mod tapret_verify;

// ZK proof infrastructure (Phase 5)
pub mod zk_proof;

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
pub use seal::{AnchorRef, CommitAnchor, SealPoint, SealRef};
pub use title::{Sanad, SanadError, SanadId, OwnershipProof as TitleOwnershipProof};
pub use signature::{parse_signatures_from_bytes, verify_signatures, Signature, SignatureScheme};

// DAG and proofs
pub use dag::{DAGNode, DAGSegment};
pub use proof::{FinalityProof, InclusionProof, ProofBundle};
pub use proof_verify::verify_proof;

// Errors and traits
pub use error::{AdapterError, Result};
pub use traits::AnchorLayer;

// Chain operations (Production Guarantee Plan Phase 2)
pub use chain_operations::{
    BalanceInfo, ChainBroadcaster, ChainCapability, ChainDeployer, ChainOpError, ChainOpResult,
    ChainProofProvider, ChainQuery, ChainRightOps, ChainSigner, ContractStatus, DeploymentStatus,
    FinalityStatus, FullChainAdapter, RightOperation, RightOperationResult, TokenBalance,
    TransactionInfo, TransactionStatus,
};

// New refactored chain operations (replaces chain_operations)
pub use ops::{
    BalanceInfo as OpsBalanceInfo, ChainBackend, ChainBroadcaster as OpsChainBroadcaster,
    ChainCapability as OpsChainCapability, ChainDeployer as OpsChainDeployer,
    ChainOpError as OpsChainOpError, ChainOpResult as OpsChainOpResult,
    ChainProofProvider as OpsChainProofProvider, ChainQuery as OpsChainQuery,
    ChainSanadOps, ChainSigner as OpsChainSigner, ContractStatus as OpsContractStatus,
    DeploymentStatus as OpsDeploymentStatus, FinalityStatus as OpsFinalityStatus,
    SanadOperation, SanadOperationResult, SanadOperationResult as OpsSanadOperationResult,
    TokenBalance as OpsTokenBalance, TransactionInfo as OpsTransactionInfo,
    TransactionStatus as OpsTransactionStatus,
};

// Event schemas (Production Guarantee Plan Phase 6)
pub use events::{
    CsvEvent, EventData, EventFilter, EventFinalityStatus, EventIndexer, EventIndexerRegistry,
    event_names, metadata_fields,
};

// Cross-chain transfer
pub use client::{ValidationClient, ValidationResult};
pub use cross_chain::{CrossChainLockEvent, CrossChainRegistry, CrossChainRegistryEntry};
pub use nullifier::{
    ChainId, SealNullifier, DoubleSpendError, OptimizedSealNullifier,
    SealConsumption, SealStatus,
};

// ===========================================================================
// Re-exports: Beta API (may receive additive changes)
// ===========================================================================

// Advanced commitment types
pub use advanced_commitments::{
    CommitmentScheme, EnhancedCommitment, FinalityProofType, InclusionProofType, ProofMetadata,
};

// Agent-friendly types
pub use agent_types::{ErrorSuggestion, FixAction, HasErrorSuggestion, error_codes};

// Production hardening
pub use hardening::{
    BoundedQueue, CircuitBreaker, CircuitState, MemoryLimits, TimeoutConfig,
    DEFAULT_CIRCUIT_MAX_FAILURES, DEFAULT_CIRCUIT_RESET_TIMEOUT, DEFAULT_HEALTH_CHECK_TIMEOUT,
    DEFAULT_RPC_TIMEOUT, MAX_CACHE_SIZE, MAX_REGISTRY_SIZE, MAX_SEAL_NULLIFIER_SIZE,
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
pub use performance::{
    BloomFilter, CacheStats, FilterStats, PerformanceMetrics, PerformanceStats, ProofCache,
    SealRegistryFilter, SequentialVerifier, VerificationResult,
};
pub use store::{AnchorRecord, InMemorySealStore, RightRecord, RightStore, SealRecord, SealStore, StoreError};

// Chain adapter system (Beta API)
pub use adapter_factory::{create_adapter, is_chain_supported, AdapterFactory};
pub use chain_adapter::{
    ChainAdapter, ChainAdapterExt, ChainError, ChainRegistry, ChainResult, RpcClient, Wallet,
};
pub use chain_config::{AccountModel, ChainCapabilities, ChainConfig, ChainConfigLoader};
pub use chain_discovery::{ChainDiscovery, ChainInfo};
pub use chain_plugin::{
    ChainPlugin, ChainPluginBuildError, ChainPluginBuilder, ChainPluginMetadata,
    ChainPluginRegistry,
};

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
pub use vm::{
    execute_transition, AluVmAdapter, DeterministicVM, MeteredVMAdapter, PassthroughVM, VMError,
    VMInputs, VMOutputs,
};

/// Experimental module — feature-gated behind `experimental`.
/// These APIs may change or be removed without notice.
#[cfg(feature = "experimental")]
pub use rgb_compat::{RgbConsignmentValidator, RgbValidationError, RgbValidationResult};
