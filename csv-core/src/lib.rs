//! CSV Core — Client-Side Validation for Cross-Chain Sanads
//!
//! This crate provides the foundational types and traits for the CSV protocol:
//!
//! - **[`Sanad`]** — A verifiable, single-use digital sanad (deed) that can be
//!   transferred cross-chain
//! - **[`struct@Hash`]** — A 32-byte cryptographic hash (SHA-256 based)
//! - **[`Commitment`]** — A binding between a sanad's state and its anchor
//!   on a blockchain
//! - **[`SealPoint`]** / **[`CommitAnchor`]** — References to consumed seals
//!   and published anchors
//! - **[`InclusionProof`]** / **[`FinalityProof`]** / **[`ProofBundle`]** —
//!   Cryptographic proofs that a sanad was locked on the source chain
//! - **[`SealProtocol`]** — The core seal protocol trait each chain backend implements
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

// No-std compatible collections
pub mod collections;

// Re-exports
pub use commitment::Commitment;
pub use commitments_ext::CommitmentScheme;
pub use hash::Hash;
pub use merkle::{MerkleProof, MerkleTree};
pub use proof_pipeline::{ValidationStep};
pub use proof::ProofBundle;
pub use provenance::{AdapterSignature, ProofProvenance, VerificationStep, VerificationStepType};
pub use certification::{ProofCertification, VerificationInputs, VerificationOutputs};
pub use replay_record::{GlobalReplayRecord, ReplayState};
pub use replay_registry::{ReplayEntry, ReplayKey, ReplayRegistry, ReplayRegistryBackend};
pub use sanad::{OwnershipProof, Sanad, SanadEnvelope, SanadId};
pub use seal_protocol::SealProtocol;
pub use nullifier::SealStatus;

// Advanced commitment types
pub mod commitments_ext;

// Transfer state machine (Phase 2)
pub mod transfer_state;

// Recovery engine (Phase 2)
pub mod recovery_engine;

// Finality state model (Phase 3)
pub mod finality;

// Reorg detection and handling (Phase 3)
pub mod reorg;

// RPC quorum client (Phase 4)
pub mod rpc;

// Protocol version and canonical contract (🔒 STABLE + 🟡 BETA)
pub mod protocol_version;

// Canonical serialization (Priority 0 — Constitutional hardening)
pub mod canonical;

// Agent-friendly types (AI agent support) - 🟡 BETA
pub mod mcp;

// Lease management for cross-chain transfers
pub mod lease;

// Re-exports: Canonical serialization
pub use canonical::{canonical_hash, from_canonical_cbor, to_canonical_cbor};

// Production hardening - 🔒 STABLE
pub mod hardening;

// State machine types (Phase 1: Consignment Wire Format) - 🟡 BETA
pub mod consignment;
pub mod genesis;
pub mod schema;
pub mod state;
pub mod transition;

// CommitMux (Phase 2) - 🧪 EXPERIMENTAL (re-exports gated, module always available)
pub mod commit_mux;

// Deterministic VM (Phase 3) - 🧪 EXPERIMENTAL
#[cfg(feature = "experimental")]
pub mod vm;

// Core types
pub mod commitment;
pub mod hash;
pub mod merkle;
pub mod seal;
pub mod domain_hash;
pub mod domains;
pub mod tagged_hash;
pub mod proof_pipeline;
pub mod provenance;
pub mod certification;
#[allow(missing_docs)]
pub mod replay_record;
pub mod replay_registry;
pub mod sanad;
pub mod seal_protocol;

  // Adapter boundary — adapters are data providers, not verifiers
#[allow(missing_docs)]
pub mod adapter;

// Typed finality guarantee — chain-agnostic, runtime-enforceable
#[allow(missing_docs)]
pub mod finality_guarantee;

// Canonical Sanad Envelope — chain-agnostic, version-stable identity
#[allow(missing_docs)]
pub mod envelope;

// Canonical event model with causality chains
#[allow(missing_docs)]
pub mod event;

// DAG and proof types - 🔒 STABLE
pub mod dag;
pub mod proof;
pub mod signature;
pub mod verifier;
/// Trust package primitives for offline verification bootstrapping.
pub mod trust_package;

// Trust package re-exports
pub use trust_package::{
    OfflineVerificationContext, TrustPackage, TrustPackageError,
};

/// Proof provenance metadata for forensic and deterministic verification.
pub mod proof_provenance;

/// Startup-time config validation helpers to assert capability alignment.
pub mod config_validation;

/// Runtime health and degraded-mode types used by runtime orchestration.
pub mod runtime_health;

/// Restart-safe finality anchoring — canonical chain snapshot persistence.
pub mod finality_anchor;

/// Protocol version compatibility matrix for version negotiation.
pub mod compatibility;

/// Chain-specific finality grades (SolanaCommitmentGrade, EthereumFinalityStage).
pub mod chain_specific;

/// Data authority tags — prevent explorer-authoritative state interpretation.
pub mod data_authority;

/// Persisted state transitions — atomic coupling of proofs and state changes.
pub mod persisted_transition;

/// Proof material provider trait — adapters become pure data providers.
pub mod proof_material;

/// Wallet capability separation and signing provider abstraction.
pub mod wallet_types;

// Error handling and traits - 🔒 STABLE
pub mod error;

// Chain operation traits (Production Guarantee Plan Phase 2) - 🔒 STABLE
pub mod backend;

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

// Chain configuration system
pub mod chain_config;

// Multi-dimensional verification result types (Phase 1)
pub mod verified;

// RGB protocol compatibility (Sprint 5) - 🧪 EXPERIMENTAL
#[cfg(feature = "experimental")]
pub mod rgb;

// Tapret verification (Sprint 0.5) - requires bitcoin dependency
#[cfg(feature = "tapret")]
pub mod tapret_verify;

// ZK proof infrastructure (Phase 5)
pub mod zk_proof;

// Atomic swap / HTLSE (Phase 3)
pub mod atomic_swap;

// Stealth addresses (Phase 3.3)
pub mod stealth;

// ===========================================================================
// Re-exports: Protocol Contract (🔒 STABLE + 🟡 BETA)
// ===========================================================================

// Protocol version, chain IDs, transfer status, error codes, capabilities
pub use protocol_version::{
    Capabilities, ChainId, ErrorCode, PROTOCOL_VERSION, ProtocolVersion, SimplifiedTransferStatus,
    SyncStatus, TransferStatus, builtin, simplified_to_full,
};

// ===========================================================================
// Re-exports: Stable API (will not change without semver-major bump)
// ===========================================================================

pub use seal::{CommitAnchor, SealPoint};
pub use signature::{
    Signature, SignatureScheme, parse_signatures_from_bytes, verify_signatures, PQ_DEFAULT_SCHEME,
};

// DAG and proofs
pub use dag::{DAGNode, DAGSegment};
pub use proof::{FinalityProof, InclusionProof, ProofPhase, ReplayId};
pub use verifier::verify_proof;

// Errors and traits
pub use error::{ProtocolError, Result};

// Chain operations (Production Guarantee Plan Phase 2)
pub use backend::{
    BalanceInfo, ChainBackend, ChainBroadcaster, ChainCapability, ChainDeployer, ChainOpError,
    ChainOpResult, ChainProofProvider, ChainQuery, ChainSanadOps, ChainSigner, ContractStatus,
    DeploymentStatus, FinalityStatus, SanadOperation, SanadOperationResult, TokenBalance,
    TransactionInfo, TransactionStatus,
};

// Event schemas (Production Guarantee Plan Phase 6)
pub use events::{
    CsvEvent, EventData, EventFilter, EventFinalityStatus, EventIndexer, EventIndexerRegistry,
    event_names, metadata_fields,
};

// Cross-chain transfer
pub use client::{ValidationClient, ValidationResult};
pub use cross_chain::{
    CrossChainHashAlgorithm, CrossChainLockEvent, CrossChainRegistry, CrossChainRegistryEntry,
    CrossChainTransferProof, StandardTransferVerifier, CrossChainDomain,
};
pub use nullifier::{
    DoubleSpendError, OptimizedSealNullifier, SealConsumption, SealNullifier,
};

// Adapter boundary
pub use adapter::{
    ChainAdapter, ChainContext, InclusionVerifier, RawAnchorData,
    RawInclusionProof, VerifiedInclusion,
};
pub use adapter::InclusionProofType as AdapterInclusionProofType;

// Finality guarantee
pub use finality_guarantee::{FinalityGuarantee, FinalityPolicy, FinalityPolicyRegistry};

// Canonical envelope
pub use envelope::{CanonicalSanadEnvelope, EncodingType, SignatureScheme as EnvelopeSignatureScheme, TypeId, decode_envelope};

// Canonical events
pub use event::{CanonicalEvent, EventLog, EventType, InMemoryEventLog};

// ===========================================================================
// Re-exports: Beta API (may receive additive changes)
// ===========================================================================

// Advanced commitment types
pub use commitments_ext::{
    EnhancedCommitment, FinalityProofType, InclusionProofType, ProofMetadata,
};

// Agent-friendly types
pub use mcp::{
    AgentChainAdapterInfo, AgentCreateSealResult, AgentExportProofResult, AgentGetSanadsResult,
    AgentProtocolInfoResult, AgentRpcStatus, AgentSanadSummary, AgentSealStatus,
    AgentTransferResult, AgentTransferStatus, AgentVerifyProofResult, ErrorSuggestion,
    FixAction, HasErrorSuggestion, VerificationLevel, error_codes,
};

// Production hardening
pub use hardening::{
    BoundedQueue, CircuitBreaker, CircuitState, DEFAULT_CIRCUIT_MAX_FAILURES,
    DEFAULT_CIRCUIT_RESET_TIMEOUT_SECS, DEFAULT_HEALTH_CHECK_TIMEOUT_SECS, DEFAULT_RPC_TIMEOUT_SECS,
    MAX_CACHE_SIZE, MAX_REGISTRY_SIZE, MAX_SEAL_NULLIFIER_SIZE, MemoryLimits, TimeoutConfig,
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

// Finality (Phase 1 - FinalityVerifier trait)
pub use finality::{FinalityEvidence, FinalityProof as FinalityVerifierProof, FinalityVerifier};

// Cross-cutting (Phase 10)
pub use monitor::{PendingPublication, PublicationTracker, ReorgEvent, ReorgMonitor};
pub use performance::{
    BloomFilter, CacheStats, FilterStats, PerformanceMetrics, PerformanceStats, ProofCache,
    SealRegistryFilter, SequentialVerifier, VerificationResult,
};
pub use store::{
    AnchorRecord, InMemorySealStore, SanadRecord, SanadStore, SealRecord, SealStore, StoreError,
};

// Chain configuration system (Beta API)
pub use chain_config::{
    ChainCapabilities, ChainConfig, ChainConfigLoader, ChainRole, FinalityModel, ProofModel,
    ReplayProtectionModel, ReorgRisk, StateModel,
};

// Verification result types (Phase 1)
pub use verified::{
    FinalityStrength, InclusionStrength, VerificationAssurance, VerificationFailure,
    VerifiedComponents,
};

// ===========================================================================
// Re-exports: Experimental API (feature-gated, may change)
// ===========================================================================

/// Experimental module — feature-gated behind `experimental`.
/// These APIs may change or be removed without notice.
#[cfg(feature = "experimental")]
pub use commit_mux::{CommitMux, MerkleBranchNode, MuxLeaf, MuxProof, ProtocolId};

/// Experimental module — feature-gated behind `experimental`.
/// These APIs may change or be removed without notice.
#[cfg(feature = "experimental")]
pub use vm::{
    AluVmAdapter, DeterministicVM, MeteredVMAdapter, PassthroughVM, VMError, VMInputs, VMOutputs,
    execute_transition,
};

/// Experimental module — feature-gated behind `experimental`.
/// These APIs may change or be removed without notice.
#[cfg(feature = "experimental")]
pub use rgb::{RgbConsignmentValidator, RgbValidationError, RgbValidationResult};

// ===========================================================================
// Re-exports: Phase 3 (Atomic Swap / HTLSE)
// ===========================================================================

pub use atomic_swap::{
    AtomicSwapBackend, AtomicSwapError, AtomicSwapOffer, AtomicSwapRegistry, AtomicSwapState,
    DefaultTimeouts, HashLock, SwapDirection, SwapRecord, blocks_to_duration, compute_swap_id,
    derive_hash_lock, is_timeout_valid, verify_hash_lock,
};

// ===========================================================================
// Re-exports: Phase 3 (Stealth Addresses)
// ===========================================================================

pub use stealth::{
    EphemeralPoint, ScanPublicKey, SpendPublicKey, StealthAddress, StealthAddressPair,
    StealthScanEntry, StealthWallet, compute_ephemeral_point, derive_stealth_base,
};

// ===========================================================================
// Re-exports: Phase 3 (Pedersen Commitments) - feature-gated
// ===========================================================================

#[cfg(feature = "zk")]
pub use zk_proof::pedersen;
