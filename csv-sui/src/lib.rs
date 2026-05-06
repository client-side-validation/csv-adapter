//! Sui Adapter for CSV (Client-Side Validation)
//!
//! This adapter implements the AnchorLayer trait for Sui,
//! using owned objects with one_time attributes as seals.
//!
//! ## Architecture
//!
//! - **Seals**: Sui objects that can be transferred and consumed once
//! - **Anchors**: Dynamic fields created when seal objects are consumed
//! - **Finality**: Narwhal consensus provides deterministic finality via checkpoint certification
//!
//! ## Usage
//!
//! ```no_run
//! use csv_adapter_sui::{SuiAnchorLayer, SuiConfig, SuiNetwork};
//!
//! // Create adapter with test RPC for testing
//! let adapter = SuiAnchorLayer::with_test().unwrap();
//!
//! // Or with configuration
//! let config = SuiConfig::new(SuiNetwork::Testnet);
//! // let rpc = ...;
//! // let adapter = SuiAnchorLayer::from_config(config, rpc).unwrap();
//! ```
//!
//! ## Production
//!
//! Enable the `rpc` feature to use real Sui RPC calls:
//! ```toml
//! [dependencies]
//! csv-adapter-sui = { version = "0.1", features = ["rpc"] }
//! ```

#![warn(missing_docs)]
#![allow(missing_docs)]
#![allow(dead_code)]

pub mod adapter;
pub mod chain_adapter_impl;
pub mod chain_operations;
pub mod checkpoint;
pub mod config;
pub mod deploy;
pub mod error;
pub mod mint;
pub mod proofs;
pub mod rpc;
pub mod seal;
pub mod signatures;
pub mod types;

#[cfg(feature = "rpc")]
pub mod real_rpc;

pub use adapter::SuiAnchorLayer;
pub use chain_adapter_impl::{create_sui_adapter, SuiWallet};
pub use deploy::deploy_csv_seal_package;
pub use deploy::{PackageDeployer, PackageDeployment};

pub use checkpoint::CheckpointVerifier;
pub use config::{CheckpointConfig, SealContractConfig, SuiConfig, SuiNetwork, TransactionConfig};
#[cfg(feature = "rpc")]
pub use deploy::publish_csv_package;
pub use error::SuiError;
#[cfg(feature = "rpc")]
pub use mint::mint_right;
pub use proofs::{
    CommitmentEventBuilder, EventProof, EventProofVerifier, StateProof, StateProofVerifier,
    TransactionProof,
};
#[cfg(feature = "rpc")]
pub use real_rpc::SuiRpcClient;
#[cfg(test)]
pub use rpc::MockSuiRpc;
pub use rpc::{SuiCheckpoint, SuiEvent, SuiLedgerInfo, SuiObject, SuiRpc, SuiTransactionBlock};
pub use seal::{SealRecord, SealRegistry, SealStore};
pub use types::{SuiAnchorRef, SuiFinalityProof, SuiInclusionProof, SuiSealRef};

// Chain operations exports
pub use chain_operations::SuiChainOperations;
