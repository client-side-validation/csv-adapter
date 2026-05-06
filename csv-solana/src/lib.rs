//! Solana Adapter for CSV (Client-Side Validation)
//!
//! This adapter implements the SealProtocol trait for Solana,
//! using program accounts as single-use seals and program instructions for commitment publication.

#![warn(missing_docs)]
#![allow(missing_docs)]
#![allow(dead_code)]

pub mod seal_protocol;
pub mod backend;
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

pub use seal_protocol::SolanaSealProtocol;
pub use backend::{create_solana_adapter, SolanaRpcClient, SolanaWallet};
pub use config::{Network, SolanaConfig};
pub use deploy::{deploy_csv_program, deploy_csv_seal_program, ProgramDeployer, ProgramDeployment};
pub use error::{SolanaError, SolanaResult};
pub use mint::mint_sanad_from_hex_key;
pub use rpc::SolanaRpc;
pub use types::{SolanaCommitAnchor, SolanaFinalityProof, SolanaInclusionProof, SolanaSealPoint};
pub use wallet::{ProgramWallet, WalletError};

// Chain operations exports
pub use chain_operations::SolanaBackend;

#[cfg(feature = "rpc")]
pub use rpc::RealSolanaRpc;
