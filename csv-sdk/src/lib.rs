//! CSV Adapter — Unified Meta-Crate
//!
//! This crate provides a single entry point for all CSV (Client-Side Validation)
//! operations, unifying the individual chain adapter crates behind a coherent,
//! ergonomic API.
//!
//! # Architecture
//!
//! ```text
//! csv-adapter (this crate)
//! ├── csv-adapter-core       (always included)
//! ├── csv-adapter-bitcoin    (optional, feature: "bitcoin")
//! ├── csv-adapter-ethereum   (optional, feature: "ethereum")
//! ├── csv-adapter-sui        (optional, feature: "sui")
//! ├── csv-adapter-aptos      (optional, feature: "aptos")
//! └── csv-adapter-store      (optional, feature: "sqlite")
//! ```
//!
//! # Quick Start
//!
//! ```no_run
//! use csv_adapter::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Build a client with Bitcoin support
//!     let client = CsvClient::builder()
//!         .with_chain(Chain::Bitcoin)
//!         .with_store_backend(StoreBackend::InMemory)
//!         .build()?;
//!
//!     // Access managers
//!     let rights = client.rights();
//!     let transfers = client.transfers();
//!     let proofs = client.proofs();
//!
//!     Ok(())
//! }
//! ```
//!
//! # Feature Flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `bitcoin` | Enable Bitcoin adapter |
//! | `ethereum` | Enable Ethereum adapter |
//! | `sui` | Enable Sui adapter |
//! | `aptos` | Enable Aptos adapter |
//! | `all-chains` | Enable all chain adapters |
//! | `tokio` | Enable tokio async runtime (default) |
//! | `async-std` | Enable async-std runtime |
//! | `sqlite` | Enable SQLite persistence |
//! | `in-memory` | Enable in-memory store backend |
//! | `wallet` | Enable unified wallet management |
//!
//! # Key Concepts
//!
//! - **Right**: A verifiable, single-use digital right that can be transferred
//!   cross-chain. Exists in client state, not on any chain.
//! - **Seal**: The on-chain mechanism that enforces a Right's single-use.
//!   Chain-specific and exists on one chain only.
//! - **Client-Side Validation (CSV)**: The client does the verification, not
//!   the blockchain. The chain only records commitments and enforces single-use.

#![warn(missing_docs)]

// Internal modules
pub mod builder;
pub mod client;
pub mod config;
pub mod cross_chain;
pub mod deploy;
pub mod errors;
pub mod events;
pub mod facade;
pub mod prelude;
pub mod proofs;
pub mod rights;
pub mod scalable_builder;
pub mod transfers;
pub mod wallet;

// Re-export core types from csv-adapter-core (🔒 STABLE API only by default)
pub use csv_adapter_core::{
    AnchorLayer, AnchorRef, Commitment, Consignment, CrossChainLockEvent, DAGNode, DAGSegment,
    FinalityProof, Genesis, Hash, InclusionProof, OwnedState, OwnershipProof, ProofBundle, Right,
    RightId, Schema, SealRef, StateRef, Transition, CONSIGNMENT_VERSION, SCHEMA_VERSION,
};

// Re-export canonical protocol types (🔒 STABLE + 🟡 BETA)
pub use csv_adapter_core::protocol_version::{
    Capabilities, Chain, ErrorCode, ProtocolVersion, SyncStatus, TransferStatus, PROTOCOL_VERSION,
};

// Re-export error types (🔒 STABLE)
pub use csv_adapter_core::{AdapterError, Result as CoreResult, StoreError};

// ===========================================================================
// Experimental re-exports (feature-gated)
// ===========================================================================

/// Re-exports of experimental modules — requires `experimental` feature.
///
/// These APIs may change or be removed without notice.
#[cfg(feature = "experimental")]
pub mod experimental {
    pub use csv_adapter_core::mpc::{MpcLeaf, MpcProof, MpcTree};
    pub use csv_adapter_core::rgb_compat::{
        CrossChainError, RgbValidationError, RgbValidationResult,
    };
    pub use csv_adapter_core::vm::{
        execute_transition, DeterministicVM, VMError, VMInputs, VMOutputs,
    };
}

/// Re-export error types
pub use errors::CsvError;

/// Re-export client
pub use client::CsvClient;

/// Re-export builder types
pub use builder::{ClientBuilder, StoreBackend};

/// Re-export facade types
pub use facade::{ChainFacade, AdapterFacade, AdapterConfig, AdapterBuilder};

/// Re-export deployment types
pub use deploy::{ContractDeployment, DeploymentError, DeploymentManager, DeploymentResult};

/// Unified result type alias.
///
/// Equivalent to `Result<T, CsvError>`.
pub type Result<T> = core::result::Result<T, CsvError>;

// Note: TransferStatus is already re-exported from protocol_version module above

// ===========================================================================
// Chain-specific deployment re-exports
// ===========================================================================

#[cfg(feature = "deploy-ethereum")]
pub use csv_adapter_ethereum::deploy::deploy_csv_lock;

#[cfg(feature = "deploy-sui")]
pub use csv_adapter_sui::deploy::publish_csv_package;

#[cfg(feature = "deploy-aptos")]
pub use csv_adapter_aptos::deploy::publish_csv_module;

#[cfg(feature = "deploy-solana")]
pub use csv_adapter_solana::deploy::deploy_csv_program;
