//! Aptos Adapter for CSV (Client-Side Validation)
//!
//! This adapter implements the SealProtocol trait for Aptos,
//! using resources with key + delete as seals.
//!
//! ## Architecture
//!
//! - **Seals**: Move resources that can be deleted once (via `move_from`)
//! - **Anchors**: Events emitted when seal resources are deleted
//! - **Finality**: HotStuff consensus provides deterministic finality via 2f+1 certification
//!
//! ## Usage
//!
//! ```no_run
//! use csv_adapter_aptos::{AptosSealProtocol, AptosConfig, AptosNetwork};
//!
//! // Create adapter with test RPC for testing
//! let adapter = AptosSealProtocol::with_test().unwrap();
//!
//! // Or with configuration
//! let config = AptosConfig::new(AptosNetwork::Devnet);
//! // let rpc = ...;
//! // let adapter = AptosSealProtocol::from_config(config, rpc).unwrap();
//! ```
//!
//! ## Production
//!
//! Enable the `rpc` feature to use real Aptos RPC calls:
//! ```toml
//! [dependencies]
//! csv-adapter-aptos = { version = "0.1", features = ["rpc"] }
//! ```

#![warn(missing_docs)]
#![allow(missing_docs)]
#![allow(dead_code)]

pub mod seal_protocol;
pub mod backend;
pub mod chain_operations;
pub mod checkpoint;
pub mod config;
pub mod deploy;
pub mod error;
pub mod merkle;
pub mod proofs;
pub mod rpc;
pub mod seal;
pub mod signatures;
pub mod types;

#[cfg(feature = "rpc")]
pub mod node;

pub use seal_protocol::AptosSealProtocol;
pub use backend::{create_aptos_adapter, AptosWallet};
pub use deploy::deploy_csv_seal_module;
pub use deploy::{ModuleDeployer, ModuleDeployment};

pub use checkpoint::CheckpointVerifier;
pub use config::{AptosConfig, AptosNetwork, CheckpointConfig};
#[cfg(feature = "aptos-sdk")]
pub use deploy::publish_csv_module;
pub use error::AptosError;
pub use proofs::{
    CommitmentEventBuilder, EventProof, EventProofVerifier, StateProof, StateProofVerifier,
    TransactionProof,
};
#[cfg(feature = "rpc")]
pub use node::AptosRpcClient;
#[cfg(test)]
pub use rpc::MockAptosRpc;
pub use rpc::{
    AptosBlockInfo, AptosEvent, AptosLedgerInfo, AptosResource, AptosRpc, AptosTransaction,
};
pub use seal::{SealRecord, SealRegistry, SealStore};
pub use types::{AptosCommitAnchor, AptosFinalityProof, AptosInclusionProof, AptosSealPoint};

// Chain operations exports
pub use chain_operations::AptosBackend;
