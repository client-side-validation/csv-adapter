//! Solana Adapter for CSV (Client-Side Validation)
//!
//! This adapter implements the AnchorLayer trait for Solana,
//! using program accounts as single-use seals and program instructions for commitment publication.

#![warn(missing_docs)]
#![allow(missing_docs)]
#![allow(dead_code)]

pub mod adapter;
pub mod chain_adapter_impl;
pub mod chain_operations;
pub mod config;
pub mod deploy;
pub mod error;
pub mod mint;
pub mod program;
pub mod proofs;
pub mod rpc;
pub mod seal;
pub mod types;
pub mod wallet;

pub use adapter::SolanaAnchorLayer;
pub use chain_adapter_impl::{create_solana_adapter, SolanaRpcClient, SolanaWallet};
pub use config::{Network, SolanaConfig};
pub use deploy::{deploy_csv_program, deploy_csv_seal_program, ProgramDeployer, ProgramDeployment};
pub use error::{SolanaError, SolanaResult};
pub use mint::mint_right_from_hex_key;
pub use rpc::SolanaRpc;
pub use types::{SolanaAnchorRef, SolanaFinalityProof, SolanaInclusionProof, SolanaSealRef};
pub use wallet::{ProgramWallet, WalletError};

// Chain operations exports
pub use chain_operations::SolanaChainOperations;

#[cfg(feature = "rpc")]
pub use rpc::RealSolanaRpc;
