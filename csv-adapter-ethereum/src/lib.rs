//! Ethereum Adapter for CSV (Client-Side Validation)
//!
//! This adapter implements the AnchorLayer trait for Ethereum,
//! using storage slots as single-use seals and LOG events for commitment publication.

#![warn(missing_docs)]
#![allow(missing_docs)]
#![allow(dead_code)]

pub mod adapter;
pub mod config;
pub mod error;
pub mod finality;
pub mod mpt;
pub mod proofs;
pub mod rpc;
pub mod seal;
pub mod seal_contract;
pub mod signatures;
pub mod types;

#[cfg(feature = "rpc")]
pub mod real_rpc;

#[cfg(feature = "rpc")]
pub use real_rpc::{publish, publish_seal_consumption, verify_seal_consumption_in_receipt, AlloyRpcError, RealEthereumRpc};

pub use adapter::EthereumAnchorLayer;
pub use config::EthereumConfig;
pub use types::{EthereumSealRef, EthereumAnchorRef, EthereumFinalityProof, EthereumInclusionProof};
pub use rpc::{EthereumRpc, MockEthereumRpc};
pub use finality::{FinalityChecker, FinalityConfig};
pub use seal_contract::CsvSealAbi;
