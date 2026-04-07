//! Aptos Adapter for CSV (Client-Side Validation)
//!
//! This adapter implements the AnchorLayer trait for Aptos,
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
//! use csv_adapter_aptos::{AptosAnchorLayer, AptosConfig, AptosNetwork};
//!
//! // Create adapter with mock RPC for testing
//! let adapter = AptosAnchorLayer::with_mock().unwrap();
//!
//! // Or with configuration
//! let config = AptosConfig::new(AptosNetwork::Devnet);
//! // let rpc = ...;
//! // let adapter = AptosAnchorLayer::from_config(config, rpc).unwrap();
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

pub mod adapter;
pub mod checkpoint;
pub mod config;
pub mod error;
pub mod proofs;
pub mod rpc;
pub mod seal;
pub mod signatures;
pub mod types;

pub use adapter::AptosAnchorLayer;
pub use config::{AptosConfig, AptosNetwork, CheckpointConfig};
pub use types::{AptosSealRef, AptosAnchorRef, AptosFinalityProof, AptosInclusionProof};
pub use rpc::{AptosRpc, MockAptosRpc, AptosResource, AptosEvent, AptosTransaction, AptosBlockInfo, AptosLedgerInfo};
pub use checkpoint::CheckpointVerifier;
pub use seal::{SealRegistry, SealRecord, SealStore};
pub use proofs::{StateProof, StateProofVerifier, EventProof, EventProofVerifier, TransactionProof, CommitmentEventBuilder};
pub use error::AptosError;